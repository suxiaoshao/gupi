#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) enum Event {
    #[cfg(target_os = "macos")]
    Ready,
    Finished,
    Failed,
    Skipped,
    #[cfg(target_os = "windows")]
    Installer(tempfile::TempDir),
}

#[cfg(target_os = "macos")]
pub(crate) use macos::Driver;
#[cfg(target_os = "windows")]
pub(crate) use windows::Driver;

pub(crate) fn feed_url(release_url: &str, name: &str) -> String {
    format!(
        "{}/{}",
        release_url.replacen("/releases/tag/", "/releases/download/", 1),
        name
    )
}

#[cfg(target_os = "windows")]
pub(crate) use windows::prepare_install;
#[cfg(target_os = "windows")]
mod plan;
