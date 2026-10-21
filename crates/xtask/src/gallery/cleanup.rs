use std::{
    fs, io,
    os::unix::fs::MetadataExt as _,
    path::{Component, Path, PathBuf},
};

pub(super) const PREFIX: &str = "gupi-ui-gallery-";
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
}
pub(super) struct Candidate {
    root: PathBuf,
    identity: Identity,
}
impl Candidate {
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
}

fn owned_directory(path: &Path) -> io::Result<fs::Metadata> {
    let info = fs::symlink_metadata(path)?;
    if !info.is_dir() || info.uid() != unsafe { libc::getuid() } || info.mode() & 0o077 != 0 {
        return Err(io::Error::other("not a private owned directory"));
    }
    Ok(info)
}
fn identity(info: &fs::Metadata) -> Identity {
    Identity {
        device: info.dev(),
        inode: info.ino(),
    }
}
fn candidate(parent: &Path, name: &str) -> io::Result<Candidate> {
    let mut parts = Path::new(name).components();
    if !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
        || !name.starts_with(PREFIX)
    {
        return Err(io::Error::other("not a direct gallery name"));
    }
    let root = parent.join(name);
    let info = owned_directory(&root)?;
    for name in ["config", "agent", "project-a"] {
        let info = fs::symlink_metadata(root.join(name))?;
        if !info.is_dir() {
            return Err(io::Error::other("fixture structure missing"));
        }
    }
    Ok(Candidate {
        root,
        identity: identity(&info),
    })
}
pub(super) fn candidates(parent: &Path) -> io::Result<Vec<Candidate>> {
    fs::read_dir(parent)?
        .filter_map(|entry| match entry {
            Ok(entry) => entry
                .file_name()
                .to_str()
                .and_then(|name| candidate(parent, name).ok())
                .map(Ok),
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn remove(candidate: &Candidate) -> io::Result<()> {
    let root = &candidate.root;
    // Unix remove_dir_all does not follow symlinks; recheck identity first.
    // This does not eliminate same-user replacement or launch-vs-delete races.
    if identity(&owned_directory(root)?) != candidate.identity {
        return Err(io::Error::other("fixture identity changed"));
    }
    fs::remove_dir_all(root)?;
    println!("Removed gallery: {}", root.display());
    Ok(())
}
pub(super) fn cleanup(parent: &Path, names: &[String], excluded: &[String]) -> io::Result<()> {
    for (index, name) in names.iter().enumerate() {
        if names[..index].contains(name) {
            continue;
        }
        if excluded.contains(name) {
            println!("Kept excluded gallery: {name}");
            continue;
        }
        match candidate(parent, name) {
            Ok(candidate) => {
                if has_running_executable(candidate.root()) {
                    println!("Kept running gallery: {}", candidate.root().display());
                    continue;
                }
                if remove(&candidate).is_err() {
                    println!("Kept gallery (removal failed): {name}");
                }
            }
            Err(_) => {
                println!("Kept/unavailable gallery (not a private owned direct fixture): {name}")
            }
        }
    }
    Ok(())
}
pub(super) fn cleanup_own(root: &Path) {
    if let (Some(parent), Some(name)) = (
        root.parent(),
        root.file_name().and_then(|name| name.to_str()),
    ) {
        match candidate(parent, name).and_then(|candidate| remove(&candidate)) {
            Ok(_) => {}
            Err(_) => println!("Kept gallery (removal failed): {}", root.display()),
        }
    }
}
fn has_running_executable(root: &Path) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    // Only executable/user fields are requested. No cwd, command or environment
    // refresh, permission-completeness policy, or process-control operations.
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_user(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );
    let uid = unsafe { libc::getuid() };
    system.processes().values().any(|process| {
        let owned = process.user_id().is_some_and(|id| **id == uid)
            || process.effective_user_id().is_some_and(|id| **id == uid);
        owned
            && process.exe().is_some_and(|exe| {
                exe.starts_with(root) || exe.canonicalize().is_ok_and(|exe| exe.starts_with(root))
            })
    })
}
