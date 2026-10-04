mod installation;
use super::Event;
use super::plan::InstallPlan;
use installation::installed_culture;
use std::ffi::CString;
use std::ffi::OsString;
use std::ffi::c_char;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

static EVENTS: OnceLock<smol::channel::Sender<Event>> = OnceLock::new();
fn send(event: Event) {
    if let Some(sender) = EVENTS.get() {
        let _ = sender.try_send(event);
    }
}
extern "C" fn finished() {
    send(Event::Finished);
}
extern "C" fn failed() {
    send(Event::Failed);
}
extern "C" fn skipped() {
    send(Event::Skipped);
}
extern "C" fn shutdown() {} // The verified installer event owns Gupi's managed shutdown.
extern "C" fn can_shutdown() -> i32 {
    1
}
extern "C" fn installer(path: *const u16) -> i32 {
    // WinSparkle invokes this only after verifying its EdDSA signature. Copy the
    // download while the callback owns it, before WinSparkle removes its cache.
    let result = std::panic::catch_unwind(|| -> Result<(), String> {
        if path.is_null() {
            return Err("missing installer".into());
        }
        let mut len = 0;
        unsafe {
            while len < 32768 && *path.add(len) != 0 {
                len += 1;
            }
        }
        if len == 32768 {
            return Err("invalid installer path".into());
        }
        let path = PathBuf::from(OsString::from_wide(unsafe {
            std::slice::from_raw_parts(path, len)
        }));
        let directory = tempfile::Builder::new()
            .prefix("gupi-update-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        std::fs::copy(path, directory.path().join("update.msi")).map_err(|e| e.to_string())?;
        EVENTS
            .get()
            .ok_or("updater stopped")?
            .try_send(Event::Installer(directory))
            .map_err(|e| e.to_string())
    });
    if matches!(result, Ok(Ok(()))) { 1 } else { -1 }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub struct Driver {
    library: libloading::Library,
    culture: String,
}
impl Driver {
    pub fn new(sender: smol::channel::Sender<Event>) -> Result<Self, String> {
        if !cfg!(feature = "bundled") {
            return Err("source build".into());
        }
        let key = option_env!("GUPI_UPDATE_PUBLIC_KEY")
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or("update signing key is not configured")?;
        let culture =
            installed_culture().ok_or("Gupi is not running from a recognized MSI installation")?;
        let dll = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("WinSparkle.dll");
        // Restrict dependency lookup to this installation and system directories.
        let library: libloading::Library = unsafe {
            libloading::os::windows::Library::load_with_flags(&dll, 0x00000100 | 0x00001000)
        }
        .map_err(|e| e.to_string())?
        .into();
        let driver = Self { library, culture };
        let _ = EVENTS.set(sender);
        unsafe {
            driver.symbol::<unsafe extern "C" fn(*const u16, *const u16, *const u16)>(
                b"win_sparkle_set_app_details\0",
            )?(
                wide("suxiaoshao").as_ptr(),
                wide("Gupi").as_ptr(),
                wide(env!("CARGO_PKG_VERSION")).as_ptr(),
            );
            if driver.symbol::<unsafe extern "C" fn(*const c_char) -> i32>(
                b"win_sparkle_set_eddsa_public_key\0",
            )?(CString::new(key).map_err(|e| e.to_string())?.as_ptr())
                != 1
            {
                return Err("WinSparkle rejected the update public key".into());
            }
            driver.symbol::<unsafe extern "C" fn(i32)>(
                b"win_sparkle_set_automatic_check_for_updates\0",
            )?(0);
            for (name, callback) in [
                (
                    b"win_sparkle_set_update_skipped_callback\0".as_slice(),
                    skipped as extern "C" fn(),
                ),
                (
                    b"win_sparkle_set_error_callback\0".as_slice(),
                    failed as extern "C" fn(),
                ),
                (b"win_sparkle_set_update_dismissed_callback\0", finished),
                (b"win_sparkle_set_shutdown_request_callback\0", shutdown),
            ] {
                driver.symbol::<unsafe extern "C" fn(extern "C" fn())>(name)?(callback);
            }
            driver.symbol::<unsafe extern "C" fn(extern "C" fn() -> i32)>(
                b"win_sparkle_set_can_shutdown_callback\0",
            )?(can_shutdown);
            driver.symbol::<unsafe extern "C" fn(extern "C" fn(*const u16) -> i32)>(
                b"win_sparkle_set_user_run_installer_callback\0",
            )?(installer);
            driver.symbol::<unsafe extern "C" fn()>(b"win_sparkle_init\0")?();
        }
        Ok(driver)
    }
    unsafe fn symbol<T>(&self, name: &[u8]) -> Result<libloading::Symbol<'_, T>, String> {
        unsafe { self.library.get(name) }.map_err(|e| e.to_string())
    }
    pub fn install(&self, feed: &str) -> Result<(), String> {
        let feed = CString::new(feed).map_err(|e| e.to_string())?;
        unsafe {
            self.symbol::<unsafe extern "C" fn(*const c_char)>(b"win_sparkle_set_appcast_url\0")?(
                feed.as_ptr(),
            );
            self.symbol::<unsafe extern "C" fn()>(b"win_sparkle_check_update_with_ui\0")?();
        }
        Ok(())
    }
    pub fn feed_name(&self) -> String {
        format!("appcast-windows-{}.xml", self.culture)
    }
    pub fn show(&self) {
        if let Ok(show) =
            unsafe { self.symbol::<unsafe extern "C" fn()>(b"win_sparkle_check_update_with_ui\0") }
        {
            unsafe { show() };
        }
    }
}
impl Drop for Driver {
    fn drop(&mut self) {
        if let Ok(cleanup) =
            unsafe { self.symbol::<unsafe extern "C" fn()>(b"win_sparkle_cleanup\0") }
        {
            unsafe { cleanup() };
        }
    }
}

pub struct PreparedInstall {
    child: Option<tokio::process::Child>,
    input: tokio::process::ChildStdin,
    directory: Option<tempfile::TempDir>,
}
impl Drop for PreparedInstall {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.start_kill();
        }
    }
}
impl PreparedInstall {
    pub async fn commit(mut self) -> Result<(), String> {
        let sent = async {
            self.input.write_all(b"install\n").await?;
            self.input.flush().await
        }
        .await;
        if let Err(error) = sent {
            // Reap the failed helper before TempDir tries to remove its executable.
            if let Some(mut child) = self.child.take() {
                let _ = child.kill().await;
            }
            return Err(error.to_string());
        }
        // The relaunched application waits for the helper and removes this directory.
        let _ = self.directory.take().unwrap().keep();
        self.child.take(); // Detached helper waits for this process to exit before opening MSI.
        Ok(())
    }
}
pub async fn prepare_install(
    directory: tempfile::TempDir,
    failure_message: String,
    log: std::path::PathBuf,
) -> Result<PreparedInstall, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = directory.path().join("gupi-update-helper.exe");
    tokio::fs::copy(executable.with_file_name("gupi-update-helper.exe"), &helper)
        .await
        .map_err(|e| e.to_string())?;
    let plan = InstallPlan {
        parent: std::process::id(),
        executable,
        installer: directory.path().join("update.msi"),
        log,
        failure_message,
    };
    let mut child = tokio::process::Command::new(helper)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut output = tokio::io::BufReader::new(child.stdout.take().unwrap());
    let mut prepared = PreparedInstall {
        input: child.stdin.take().unwrap(),
        child: Some(child),
        directory: Some(directory),
    };
    let mut bytes = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    prepared
        .input
        .write_all(&bytes)
        .await
        .map_err(|e| e.to_string())?;
    let mut ready = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        output.read_line(&mut ready),
    )
    .await
    .map_err(|_| "update helper timed out")?
    .map_err(|e| e.to_string())?;
    if ready.trim() != "ready" {
        return Err("update helper did not acknowledge preparation".into());
    }
    Ok(prepared)
}
