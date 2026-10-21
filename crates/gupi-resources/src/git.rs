//! Read-only system Git queries. No shell, Pi process, or repository mutations.
use smol::io::AsyncReadExt;
use smol::process::Command;

mod changes;
pub use changes::{
    Change, ChangeContent, ChangeKind, ChangeList, ChangeSource, ContentState, text_within_limits,
};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
}

impl Repository {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    pub async fn head(&self) -> Result<Head, Error> {
        let output = run(&self.root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).await?;
        if output.status.success() {
            return Ok(Head::Branch(value(&output.stdout)?));
        }
        if output.status.code() != Some(1) {
            return Err(failure(output));
        }
        let output = checked(&self.root, &["rev-parse", "--short", "HEAD"]).await?;
        Ok(Head::Detached(value(&output.stdout)?))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    Detached(String),
}

impl Head {
    pub fn name(&self) -> &str {
        match self {
            Self::Branch(name) | Self::Detached(name) => name,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Git query timed out")]
    Timeout,
    #[error("Git is unavailable")]
    Unavailable,
    #[error("Git output exceeds the reader limit")]
    TooLarge,
    #[error("The file changed during the read; refresh and try again")]
    Changed,
    #[error("Git: {0}")]
    Failed(String),
}

/// Resolve a working tree, including linked worktrees and cwd below its root.
/// `None` means outside a working tree; execution failures remain errors.
pub async fn discover(cwd: &Path) -> Result<Option<Repository>, Error> {
    let output = run(cwd, &["rev-parse", "--is-inside-work-tree"]).await?;
    if !output.status.success() {
        if String::from_utf8_lossy(&output.stderr).contains("not a git repository") {
            return Ok(None);
        }
        return Err(failure(output));
    }
    if value(&output.stdout)? != "true" {
        return Ok(None);
    }
    let mut paths = Vec::new();
    for arg in ["--show-toplevel", "--absolute-git-dir", "--git-common-dir"] {
        let output = checked(cwd, &["rev-parse", "--path-format=absolute", arg]).await?;
        // Git for Windows emits slash-separated DOS paths, while filesystem
        // identity uses canonical paths (including the verbatim Windows prefix).
        paths.push(path(&output.stdout)?.canonicalize()?);
    }
    Ok(Some(Repository {
        root: paths.remove(0),
        git_dir: paths.remove(0),
        common_dir: paths.remove(0),
    }))
}

async fn checked(cwd: &Path, args: &[&str]) -> Result<Output, Error> {
    let output = run(cwd, args).await?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(failure(output))
    }
}

async fn run(cwd: &Path, args: &[&str]) -> Result<Output, Error> {
    run_program(cwd, args, "git").await
}

async fn run_program(cwd: &Path, args: &[&str], program: &str) -> Result<Output, Error> {
    run_bounded(cwd, args, program, 2 * 1024 * 1024).await
}

async fn run_bounded(
    cwd: &Path,
    args: &[&str],
    program: &str,
    limit: usize,
) -> Result<Output, Error> {
    let mut command = Command::new(program);
    command
        .current_dir(cwd)
        .args([
            "--no-pager",
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "color.ui=false",
        ])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("LC_ALL", "C");
    // The caller's Git shell environment must not redirect a session cwd query.
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
        "GIT_DIFF_OPTS",
    ] {
        command.env_remove(key);
    }
    smol::future::race(async {
        let mut child = command.spawn().map_err(|error| {
            if error.kind()==std::io::ErrorKind::NotFound && cwd.is_dir() { Error::Unavailable }else{Error::Io(error)}
        })?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let read = |stream: Box<dyn smol::io::AsyncRead + Unpin + Send>, limit: usize| async move {
            let mut bytes = Vec::new();
            stream.take(limit as u64 + 1).read_to_end(&mut bytes).await?;
            if bytes.len() > limit { return Err(Error::TooLarge); }
            Ok(bytes)
        };
        let (stdout, stderr) = smol::future::try_zip(
            read(Box::new(stdout), limit), read(Box::new(stderr), 64 * 1024)
        ).await?;
        Ok(Output { status: child.status().await?, stdout, stderr })
    }, async {
        smol::Timer::after(Duration::from_secs(5)).await;
        Err(Error::Timeout)
    }).await
}

fn failure(output: Output) -> Error {
    Error::Failed(
        String::from_utf8_lossy(&output.stderr)
            .trim_end()
            .to_owned(),
    )
}

fn protocol_line(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    #[cfg(windows)]
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    bytes
}

fn value(bytes: &[u8]) -> Result<String, Error> {
    String::from_utf8(protocol_line(bytes).to_vec())
        .map_err(|error| Error::Failed(error.to_string()))
}

fn path(bytes: &[u8]) -> Result<PathBuf, Error> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(protocol_line(
            bytes,
        ))))
    }
    #[cfg(not(unix))]
    {
        value(bytes).map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .current_dir(root)
            .arg("-c")
            .arg(format!(
                "core.hooksPath={}",
                root.join("no-hooks").display()
            ))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        value(&output.stdout).unwrap()
    }

    #[test]
    fn real_repository_heads_subdirectories_and_linked_worktrees() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo with spaces");
        std::fs::create_dir(&root).unwrap();
        assert!(smol::block_on(discover(&root)).unwrap().is_none());
        git(&root, &["init", "--initial-branch=fixture"]);
        let repository = smol::block_on(discover(&root)).unwrap().unwrap();
        assert_eq!(repository.root(), std::fs::canonicalize(&root).unwrap());
        assert_eq!(
            repository.git_dir(),
            std::fs::canonicalize(root.join(".git")).unwrap()
        );
        assert_eq!(repository.common_dir(), repository.git_dir());
        assert_eq!(
            smol::block_on(repository.head()).unwrap(),
            Head::Branch("fixture".into())
        );
        std::fs::create_dir(root.join("child")).unwrap();
        assert_eq!(
            smol::block_on(discover(&root.join("child"))).unwrap(),
            Some(repository.clone())
        );
        git(
            &root,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
                "--no-verify",
            ],
        );
        let sha = git(&root, &["rev-parse", "--short", "HEAD"]);
        let linked = dir.path().join("linked");
        git(
            &root,
            &["worktree", "add", "--detach", linked.to_str().unwrap()],
        );
        let worktree = smol::block_on(discover(&linked)).unwrap().unwrap();
        assert_eq!(worktree.root(), std::fs::canonicalize(&linked).unwrap());
        assert_eq!(
            worktree.git_dir(),
            std::fs::canonicalize(worktree.git_dir()).unwrap()
        );
        assert_ne!(worktree.git_dir(), worktree.common_dir());
        assert_eq!(worktree.common_dir(), repository.common_dir());
        assert_eq!(
            smol::block_on(worktree.head()).unwrap(),
            Head::Detached(sha)
        );
        git(
            &root,
            &["symbolic-ref", "HEAD", "refs/heads/external-change"],
        );
        assert_eq!(
            smol::block_on(repository.head()).unwrap(),
            Head::Branch("external-change".into())
        );
        assert!(smol::block_on(discover(&root.join("missing"))).is_err());
    }

    #[test]
    fn protocol_preserves_path_spaces() {
        assert_eq!(
            path(b"/repo with spaces /\n").unwrap(),
            PathBuf::from("/repo with spaces /")
        );
    }

    #[test]
    fn unavailable_git_is_an_error_not_a_non_repository() {
        assert!(matches!(
            smol::block_on(run_program(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                &["rev-parse"],
                "gupi-nonexistent-git-fixture"
            )),
            Err(Error::Unavailable)
        ));
    }
}
