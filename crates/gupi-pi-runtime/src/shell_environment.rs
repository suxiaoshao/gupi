//! Capture the exported environment from the user's login shell for GUI launches on macOS.
//!
//! The shell starts this executable again with `--gupi-print-shell-environment`. The
//! helper writes a NUL-framed byte sequence so startup-file output cannot be
//! mistaken for captured variables, and non-UTF-8 values survive the round trip.

use std::{
    ffi::{CStr, OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{io::AsyncRead, process::Command, time::timeout};

#[cfg(unix)]
use std::os::unix::{
    ffi::{OsStrExt, OsStringExt},
    process::CommandExt,
};

const FRAME_MARKER: &[u8] = b"\0GUPI-SHELL-ENVIRONMENT-v1\0";
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
const HELPER_ENV: &str = "GUPI_SHELL_ENVIRONMENT_HELPER";

/// Capture exported variables from the current user's default login shell.
pub(crate) async fn capture() -> Result<Vec<(OsString, OsString)>, String> {
    let (shell, home) = login_shell_and_home()?;
    let helper =
        std::env::current_exe().map_err(|error| format!("resolve Gupi executable: {error}"))?;
    capture_with(&shell, &home, helper.as_os_str(), CAPTURE_TIMEOUT).await
}

/// Entry point used by `main` before GPUI starts in shell environment helper mode.
pub fn print_environment() -> io::Result<()> {
    let frame = environment_frame(std::env::vars_os());
    use io::Write;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&frame)?;
    stdout.flush()
}

fn environment_frame(variables: impl IntoIterator<Item = (OsString, OsString)>) -> Vec<u8> {
    let mut frame = FRAME_MARKER.to_vec();
    for (name, value) in variables {
        if name == "SHLVL" || name == HELPER_ENV {
            continue;
        }
        frame.extend_from_slice(name.as_bytes());
        frame.push(b'=');
        frame.extend_from_slice(value.as_bytes());
        frame.push(0);
    }
    frame.push(0);
    frame
}

fn parse_environment_frame(output: &[u8]) -> Result<Vec<(OsString, OsString)>, String> {
    let marker_start = output
        .windows(FRAME_MARKER.len())
        .position(|window| window == FRAME_MARKER)
        .ok_or_else(|| "login shell did not return an environment frame".to_owned())?;
    let mut rest = &output[marker_start + FRAME_MARKER.len()..];
    let mut variables = Vec::new();
    loop {
        let end = rest
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| "login shell returned an incomplete environment frame".to_owned())?;
        let entry = &rest[..end];
        if entry.is_empty() {
            return Ok(variables);
        }
        let separator = entry
            .iter()
            .position(|byte| *byte == b'=')
            .filter(|index| *index > 0)
            .ok_or_else(|| "login shell returned an invalid environment entry".to_owned())?;
        variables.push((
            OsString::from_vec(entry[..separator].to_vec()),
            OsString::from_vec(entry[separator + 1..].to_vec()),
        ));
        rest = &rest[end + 1..];
    }
}

fn capture_command(shell: &OsStr, home: &Path, helper: &OsStr) -> Result<Command, String> {
    const EXEC_HELPER: &str =
        "exec \"$GUPI_SHELL_ENVIRONMENT_HELPER\" --gupi-print-shell-environment";
    let mut command = Command::new(shell);
    match Path::new(shell).file_name().and_then(OsStr::to_str) {
        Some("sh" | "bash" | "zsh" | "dash" | "ksh") => {
            command.args(["-l", "-i", "-c", EXEC_HELPER]);
        }
        Some("fish") => {
            // Some version managers update PATH from fish_prompt hooks.
            command.args([
                "-l", "-i", "-c",
                "emit fish_prompt >/dev/null 2>/dev/null; exec \"$GUPI_SHELL_ENVIRONMENT_HELPER\" --gupi-print-shell-environment",
            ]);
        }
        Some("csh" | "tcsh") => {
            // These shells cannot combine -l with command arguments. A leading
            // '-' in argv[0] enables login startup files, including ~/.login.
            command.as_std_mut().arg0("-");
            command.args(["-i", "-c", EXEC_HELPER]);
        }
        Some("nu") => {
            // -e loads interactive/login configuration before executing. Nu
            // reads environment variables through $env, not POSIX expansion.
            // exec replaces the shell so it never enters the REPL afterwards.
            command.args([
                "-l",
                "-e",
                "exec $env.GUPI_SHELL_ENVIRONMENT_HELPER --gupi-print-shell-environment",
            ]);
        }
        _ => {
            return Err(format!(
                "unsupported login shell: {}",
                Path::new(shell).display()
            ));
        }
    }
    command
        .env(HELPER_ENV, helper)
        .current_dir(home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Keep an interactive shell or a version-manager hook from inheriting the
    // app's controlling terminal or leaving descendants holding our pipes.
    command.as_std_mut().process_group(0);
    Ok(command)
}

async fn capture_with(
    shell: &OsStr,
    home: &Path,
    helper: &OsStr,
    deadline: Duration,
) -> Result<Vec<(OsString, OsString)>, String> {
    let mut child = capture_command(shell, home, helper)?
        .spawn()
        .map_err(|error| format!("start login shell: {error}"))?;
    let pid = child
        .id()
        .ok_or_else(|| "login shell did not provide a process id".to_owned())?;
    let mut group = ProcessGroup::new(pid as i32);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "login shell stdout was unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "login shell stderr was unavailable".to_owned())?;
    let total = Arc::new(AtomicUsize::new(0));

    let result = timeout(deadline, async {
        let status = async {
            child
                .wait()
                .await
                .map_err(|error| format!("wait for login shell: {error}"))
        };
        let stdout = read_capped(stdout, total.clone(), true);
        let stderr = read_capped(stderr, total, false);
        let (status, stdout, _) = tokio::try_join!(status, stdout, stderr)?;
        if !status.success() {
            return Err("login shell exited unsuccessfully".to_owned());
        }
        parse_environment_frame(&stdout)
    })
    .await;

    match result {
        Ok(Ok(path)) => {
            group.disarm();
            Ok(path)
        }
        result => {
            group.kill();
            let _ = child.start_kill();
            let _ = child.wait().await;
            match result {
                Ok(Err(error)) => Err(error),
                _ => Err("login shell environment capture timed out".to_owned()),
            }
        }
    }
}

async fn read_capped(
    mut reader: impl AsyncRead + Unpin,
    total: Arc<AtomicUsize>,
    keep_bytes: bool,
) -> Result<Vec<u8>, String> {
    use tokio::io::AsyncReadExt;

    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let count = reader
            .read(&mut chunk)
            .await
            .map_err(|error| format!("read login shell output: {error}"))?;
        if count == 0 {
            break;
        }
        let previous = total.fetch_add(count, Ordering::Relaxed);
        if keep_bytes && previous < MAX_OUTPUT_BYTES {
            let keep = count.min(MAX_OUTPUT_BYTES - previous);
            bytes.extend_from_slice(&chunk[..keep]);
        }
        if previous.saturating_add(count) > MAX_OUTPUT_BYTES {
            return Err("login shell output exceeded 1 MiB".to_owned());
        }
    }
    Ok(bytes)
}

struct ProcessGroup {
    id: i32,
    armed: bool,
}

impl ProcessGroup {
    fn new(id: i32) -> Self {
        Self { id, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn kill(&mut self) {
        if self.armed {
            // SAFETY: the child was started in a fresh process group whose id is
            // its pid. A negative pid addresses only that group.
            unsafe {
                libc::kill(-self.id, libc::SIGKILL);
            }
            self.armed = false;
        }
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.kill();
    }
}

fn login_shell_and_home() -> Result<(OsString, PathBuf), String> {
    match account_shell_and_home() {
        Ok(Some(value)) => Ok(value),
        Ok(None) | Err(_) => {
            let shell = std::env::var_os("SHELL")
                .filter(|shell| !shell.is_empty())
                .ok_or_else(|| "could not determine the user's login shell".to_owned())?;
            let home = dirs_next::home_dir()
                .ok_or_else(|| "could not determine the user's home directory".to_owned())?;
            Ok((shell, home))
        }
    }
}

#[cfg(target_os = "macos")]
fn account_shell_and_home() -> Result<Option<(OsString, PathBuf)>, String> {
    use std::mem::MaybeUninit;

    let uid = unsafe { libc::getuid() };
    let configured = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    let capacity = if configured > 0 {
        (configured as usize).clamp(1024, 1024 * 1024)
    } else {
        16 * 1024
    };
    let mut buffer = vec![0u8; capacity];
    let mut entry = MaybeUninit::<libc::passwd>::zeroed();
    let mut result = std::ptr::null_mut();
    // SAFETY: `entry` is writable, `buffer` remains alive for the call, and
    // libc initializes `entry` and `result` on success.
    let code = unsafe {
        libc::getpwuid_r(
            uid,
            entry.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut result,
        )
    };
    if code != 0 {
        return Err(format!("read the user's account record: {code}"));
    }
    if result.is_null() {
        return Ok(None);
    }
    // SAFETY: a non-null result from successful getpwuid_r points at `entry`.
    let entry = unsafe { entry.assume_init() };
    if entry.pw_shell.is_null() || entry.pw_dir.is_null() {
        return Ok(None);
    }
    // SAFETY: both pointers refer to NUL-terminated strings in `buffer`.
    let shell = unsafe { CStr::from_ptr(entry.pw_shell) }.to_bytes();
    let home = unsafe { CStr::from_ptr(entry.pw_dir) }.to_bytes();
    if shell.is_empty() || home.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        OsString::from_vec(shell.to_vec()),
        PathBuf::from(OsString::from_vec(home.to_vec())),
    )))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    // Startup chatter and byte values must not corrupt variables passed to Pi.
    #[test]
    fn environment_frame_preserves_exports_and_excludes_capture_variables() {
        let variables = vec![
            (
                "PATH".into(),
                OsString::from_vec(b"/usr/bin:/sample/\xff-bin".to_vec()),
            ),
            ("HTTPS_PROXY".into(), "http://proxy.example:8080".into()),
            ("NODE_OPTIONS".into(), "--max-old-space-size=2048".into()),
            (
                "TEST_API_KEY".into(),
                "fixture-only=not-a-credential\nsecond line".into(),
            ),
            ("EMPTY".into(), "".into()),
        ];
        let mut exported = variables.clone();
        exported.push(("SHLVL".into(), "9".into()));
        exported.push((HELPER_ENV.into(), "/capture/helper".into()));
        let mut noisy = b"shell startup notice\n".to_vec();
        noisy.extend(environment_frame(exported));
        noisy.extend_from_slice(b"\ntrailing shell output");
        assert_eq!(parse_environment_frame(&noisy).unwrap(), variables);
        assert!(parse_environment_frame(b"no frame").is_err());
        let mut incomplete = environment_frame(variables);
        incomplete.pop();
        assert!(parse_environment_frame(&incomplete).is_err());
    }
}
