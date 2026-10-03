use crate::releases;
use crate::releases::Cache;
use crate::releases::Problem;
use crate::releases::Release;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::EventEmitter;
use gpui_kit::Global;
use gpui_kit::Task;
use semver::Version;
use std::time::Duration;

pub enum Status {
    Idle,
    Checking {
        _task: Task<()>,
        manual: bool,
    },
    Current,
    Unpublished,
    Available(Release),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Installing(Release),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    InstallFailed(Release),
    Failed(Problem),
}

pub struct Updates {
    status: Status,
    cache: Option<Cache>,
    periodic: Option<Task<()>>,
    notified: Option<Version>,
    skipped: Option<String>,
    stopped: bool,
}

/// Emitted once per newer version in this application session, for quiet in-app feedback.
pub struct Available(pub Release);
impl EventEmitter<Available> for Updates {}
struct Service(Entity<Updates>);
impl Global for Service {}

pub fn get(cx: &mut App) -> Entity<Updates> {
    if let Some(service) = cx.try_global::<Service>() {
        return service.0.clone();
    }
    let owner = cx.new(|_| Updates {
        status: Status::Idle,
        cache: None,
        periodic: None,
        notified: None,
        skipped: None,
        stopped: false,
    });
    cx.set_global(Service(owner.clone()));
    owner
}

impl Updates {
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn set_skipped(&mut self, version: Option<String>) {
        self.skipped = version;
    }
    #[cfg(feature = "test-support")]
    pub fn set_status_for_test(&mut self, status: Status) {
        self.status = status;
    }

    pub fn is_installing(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        return matches!(self.status, Status::Installing(_));
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        false
    }

    pub fn installable_release(&self) -> Option<&Release> {
        match &self.status {
            Status::Available(release) => Some(release),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Status::InstallFailed(release) => Some(release),
            _ => None,
        }
    }
    pub fn is_checking(&self) -> bool {
        matches!(self.status, Status::Checking { .. })
    }

    pub fn configure(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.stopped || enabled == self.periodic.is_some() {
            return;
        }
        if !enabled {
            self.periodic = None;
            // A manual request is independent of the automatic-check preference.
            if matches!(self.status, Status::Checking { manual: false, .. }) {
                self.status = Status::Idle;
                cx.notify();
            }
            return;
        }
        self.periodic = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(10))
                .await;
            loop {
                if owner
                    .update(cx, |owner, cx| owner.check(false, cx))
                    .is_err()
                {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_secs(60 * 60))
                    .await;
            }
        }));
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.stopped = true;
        self.periodic = None;
        if self.is_checking() {
            self.status = Status::Idle;
        }
        cx.notify();
    }

    pub fn check(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.stopped || self.is_installing() {
            return;
        }
        if let Status::Checking {
            manual: existing, ..
        } = &mut self.status
        {
            *existing |= manual;
            return;
        }
        let cache = self.cache.clone();
        let request = gpui_tokio::Tokio::spawn(cx, releases::latest(cache));
        let task = cx.spawn(async move |owner, cx| {
            let result = request.await.unwrap_or(Err(Problem::Network));
            let _ = owner.update(cx, |owner, cx| owner.complete(result, cx));
        });
        self.status = Status::Checking {
            _task: task,
            manual,
        };
        cx.notify();
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn start_install(&mut self, cx: &mut Context<Self>) -> Option<Release> {
        if self.stopped {
            return None;
        }
        let release = match &self.status {
            Status::Available(release) | Status::InstallFailed(release) => release.clone(),
            _ => return None,
        };
        self.status = Status::Installing(release.clone());
        cx.notify();
        Some(release)
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn finish_install(&mut self, failed: bool, cx: &mut Context<Self>) {
        if self.stopped {
            return;
        }
        if let Status::Installing(release) = &self.status {
            self.status = if failed {
                Status::InstallFailed(release.clone())
            } else {
                Status::Available(release.clone())
            };
            cx.notify();
        }
    }

    fn complete(&mut self, result: Result<Cache, Problem>, cx: &mut Context<Self>) {
        let manual = matches!(self.status, Status::Checking { manual: true, .. });
        self.status = match result {
            Ok(cache) => {
                let status = match &cache.release {
                    None => Status::Unpublished,
                    Some(release)
                        if releases::is_newer(
                            release,
                            &Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                        ) =>
                    {
                        if !manual
                            && self.notified.as_ref() != Some(&release.version)
                            && self.skipped.as_deref() != Some(release.version.to_string().as_str())
                        {
                            cx.emit(Available(release.clone()));
                        }
                        self.notified = Some(release.version.clone());
                        Status::Available(release.clone())
                    }
                    Some(_) => Status::Current,
                };
                self.cache = Some(cache);
                status
            }
            Err(problem) => Status::Failed(problem),
        };
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
