#[cfg(any(target_os = "windows", test))]
mod cleanup;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub enum Event {
    #[cfg(target_os = "macos")]
    Ready,
    Finished,
    Failed,
    Skipped,
    #[cfg(target_os = "windows")]
    Installer(tempfile::TempDir),
}

#[cfg(target_os = "macos")]
pub use macos::Driver;
#[cfg(target_os = "windows")]
pub use windows::Driver;

pub fn feed_url(release_url: &str, name: &str) -> String {
    format!(
        "{}/{}",
        release_url.replacen("/releases/tag/", "/releases/download/", 1),
        name
    )
}

#[cfg(target_os = "windows")]
pub use cleanup::after_update;
#[cfg(target_os = "windows")]
pub use windows::prepare_install;
#[cfg(target_os = "windows")]
pub mod plan;
