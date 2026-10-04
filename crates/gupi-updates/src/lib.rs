//! Gupi internal updates capability.
pub mod releases;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod updater;
pub mod updates;
