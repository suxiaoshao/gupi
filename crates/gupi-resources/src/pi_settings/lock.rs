//! Directory locks compatible with Pi's `proper-lockfile` usage.
//!
//! Pi locks `<file>.lock` by creating that directory, refreshes its mtime
//! while held, and may remove (with a non-recursive `rmdir`) a lock whose
//! mtime is more than ten seconds old. Gupi never removes another process's
//! lock; it waits briefly, then reports the file as busy. Ownership is the
//! lock directory's identity plus the mtime Gupi last set, so a lock that was
//! taken over and recreated is never mistaken for ours.
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

const ATTEMPTS: u32 = 50;
const RETRY_DELAY: Duration = Duration::from_millis(20);
/// Pi's default stale window. Past it, Pi may already be taking the lock.
const STALE: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    modified: SystemTime,
    #[cfg(unix)]
    node: (u64, u64),
    #[cfg(windows)]
    created: u64,
}

impl Identity {
    fn read(dir: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(dir)?;
        if !metadata.is_dir() {
            return Err(io::Error::other("lock is not a directory"));
        }
        Ok(Self {
            modified: metadata.modified()?,
            #[cfg(unix)]
            node: {
                use std::os::unix::fs::MetadataExt as _;
                (metadata.dev(), metadata.ino())
            },
            #[cfg(windows)]
            created: {
                use std::os::windows::fs::MetadataExt as _;
                metadata.creation_time()
            },
        })
    }
}

pub(crate) struct Lock {
    dir: PathBuf,
    identity: Identity,
    /// When the mtime was last set by us.
    refreshed: Instant,
}

impl Lock {
    /// Stage and flush before renewing ownership immediately at the replace
    /// boundary. A slow flush must not allow an expired lock to publish data.
    pub(crate) fn write_atomic(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("missing parent"))?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        staged.write_all(bytes)?;
        staged.as_file().sync_all()?;
        self.renew()?;
        staged.persist(path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }

    /// Locks `file` by creating `file.lock`. A held lock is reported as
    /// [`io::ErrorKind::WouldBlock`].
    pub(crate) fn acquire(file: &Path) -> io::Result<Self> {
        let mut dir = file.as_os_str().to_owned();
        dir.push(".lock");
        let dir = PathBuf::from(dir);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        for attempt in 1..=ATTEMPTS {
            match fs::create_dir(&dir) {
                Ok(()) => {
                    return Ok(Self {
                        identity: Identity::read(&dir)?,
                        dir,
                        refreshed: Instant::now(),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if attempt < ATTEMPTS {
                        std::thread::sleep(RETRY_DELAY);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("{} is locked by another process", file.display()),
        ))
    }

    fn is_owned(&self) -> bool {
        Identity::read(&self.dir).is_ok_and(|identity| identity == self.identity)
    }

    /// Confirms the lock is still ours and renews it, like `proper-lockfile`
    /// does, so it stays fresh for the replace that immediately follows.
    pub(crate) fn renew(&mut self) -> io::Result<()> {
        let lost = || {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                format!("lost the lock {}", self.dir.display()),
            )
        };
        // Past the stale window another process may be taking over right now.
        if self.refreshed.elapsed() >= STALE || !self.is_owned() {
            return Err(lost());
        }
        touch(&self.dir)?;
        self.refreshed = Instant::now();
        self.identity = Identity::read(&self.dir)?;
        Ok(())
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // A lock replaced by another process is no longer ours to remove.
        if self.is_owned() {
            let _ = fs::remove_dir(&self.dir);
        }
    }
}

fn touch(dir: &Path) -> io::Result<()> {
    let now = SystemTime::now();
    #[cfg(windows)]
    let file = {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_WRITE_ATTRIBUTES on a directory handle.
        fs::OpenOptions::new()
            .access_mode(0x100)
            .custom_flags(0x0200_0000)
            .open(dir)?
    };
    #[cfg(not(windows))]
    let file = fs::File::open(dir)?;
    file.set_times(fs::FileTimes::new().set_modified(now).set_accessed(now))
}

#[cfg(test)]
pub(crate) fn replace_for_test(file: &Path, keep_mtime: bool) {
    let mut dir = file.as_os_str().to_owned();
    dir.push(".lock");
    let dir = PathBuf::from(dir);
    let modified = fs::metadata(&dir).unwrap().modified().unwrap();
    fs::remove_dir(&dir).unwrap();
    fs::create_dir(&dir).unwrap();
    if keep_mtime {
        fs::File::open(&dir)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_lock_cannot_publish_staged_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "original").unwrap();
        let mut lock = Lock::acquire(&path).unwrap();
        lock.refreshed = Instant::now() - STALE;
        assert_eq!(
            lock.write_atomic(&path, b"replacement").unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn replaced_lock_is_not_used_or_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "external").unwrap();
        let mut lock = Lock::acquire(&path).unwrap();
        let original = fs::File::open(&lock.dir).unwrap();
        replace_for_test(&path, true);
        assert_eq!(
            lock.write_atomic(&path, b"replacement").unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(lock);
        drop(original);
        assert!(dir.path().join("settings.json.lock").is_dir());
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
    }
}
