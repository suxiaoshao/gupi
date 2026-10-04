//! One frozen display, one selection, and one result. The host owns what happens next.
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod capture;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod overlay;
#[cfg(any(target_os = "macos", target_os = "windows", test))]
mod selection;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use capture::{cancel, is_active, start};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Screen capture is unavailable on this platform")]
    Unsupported,
    #[error("Screen recording permission is required")]
    PermissionDenied,
    #[error("A screen capture is already in progress")]
    Busy,
    #[error("The capture display changed or is unavailable")]
    DisplayChanged,
    #[error("Screen capture failed: {0}")]
    Capture(String),
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn is_active(_: &gpui_kit::App) -> bool {
    false
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn cancel(_: &mut gpui_kit::App) {}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn start(
    _: &mut gpui_kit::App,
) -> Result<gpui_kit::Task<Result<Option<Vec<u8>>, Error>>, Error> {
    Err(Error::Unsupported)
}
