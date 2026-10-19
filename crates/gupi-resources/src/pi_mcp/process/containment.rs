//! Operation-local containment. Browser launchers detached by Pi are excluded
//! from Unix groups. Windows login and all logout operations own only Pi.
use super::*;
#[cfg(windows)]
mod windows;

pub(super) struct Containment {
    #[cfg(unix)]
    group: Option<i32>,
    #[cfg(unix)]
    grouped: bool,
    #[cfg(windows)]
    job: Option<windows::Job>,
}
impl Containment {
    pub(super) fn configure(command: &mut Command, action: &Action) -> Result<Self, ()> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            let grouped = !matches!(action, Action::Logout(_));
            if grouped {
                command.as_std_mut().process_group(0);
            }
            Ok(Self {
                group: None,
                grouped,
            })
        }
        #[cfg(windows)]
        {
            let list = matches!(action, Action::Check);
            command.creation_flags(0x08000000 | if list { 0x00000004 } else { 0 });
            Ok(Self {
                job: if list {
                    Some(windows::Job::new()?)
                } else {
                    None
                },
            })
        }
    }
    pub(super) fn attach(&mut self, child: &tokio::process::Child) -> Result<(), ()> {
        #[cfg(unix)]
        {
            if self.grouped {
                self.group = Some(child.id().ok_or(())? as i32);
            }
        }
        #[cfg(windows)]
        {
            if let Some(job) = &self.job {
                job.attach_and_resume(child)?;
            }
        }
        Ok(())
    }
    pub(super) fn stop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.group.take() {
            // Only the fresh group created for our owned Pi operation.
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
        }
        #[cfg(windows)]
        self.job.take(); // kill-on-close job; contains list's descendants only.
    }
}
impl Drop for Containment {
    fn drop(&mut self) {
        self.stop();
    }
}
