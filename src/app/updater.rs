#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) use native::available;
pub(crate) use native::init;
pub(crate) use native::install;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn available(_: &gpui_kit::App) -> bool {
    false
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn init(_: &mut gpui_kit::App) {}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn install(_: &mut gpui_kit::App) {}
