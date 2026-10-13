#![allow(deprecated)]
mod lifetime;
mod platform;

use gpui::{App, Bounds, DisplayId, Pixels, Window};
use lifetime::Lifetime;
use raw_window_handle::HandleError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WindowExtError {
    #[error("Failed to get native window: {0}")]
    FailedToGetHandle(HandleError),
    #[error("Failed to get NSView")]
    FailedToGetNSView,
    #[error("Failed to get NSWindow")]
    FailedToGetNSWindow,
    #[error("Failed to get NSApplication")]
    FailedToGetNSApplication,
    #[error("Failed to set window bounds")]
    FailedSetBounds,
    #[error("Failed to configure capture window: {0}")]
    CaptureWindow(String),
    #[error("Window is closed")]
    WindowClosed,
    #[error("Target display was not found")]
    DisplayNotFound,
    #[error("Failed to query display DPI: {0}")]
    DisplayDpi(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowLevel {
    Normal,
    Floating,
    ModalPanel,
    PopUpMenu,
    Custom(i32),
}

/// A main-thread handle whose lifetime gate follows the original GPUI window.
/// Native operations must remain outside any re-entrant window/App borrow.
#[derive(Clone, Debug)]
pub struct NativeWindowHandle {
    lifetime: Lifetime,
    native: platform::Native,
}
impl NativeWindowHandle {
    pub fn hide(&self) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.hide()
    }
    pub fn show(&self) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.show()
    }
    /// Configure a hidden capture overlay; call outside Window/App updates.
    pub fn configure_capture(
        &self,
        bounds: Bounds<Pixels>,
        display: DisplayId,
    ) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.configure_capture(bounds, display)
    }
    /// Present an overlay, preserving the key window for non-pointer displays.
    pub fn show_capture(&self, activate: bool) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.show_capture(activate)
    }
    pub fn set_capture_cursor(&self) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.set_capture_cursor()
    }
    pub fn restore_capture_cursor(&self) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.restore_capture_cursor()
    }
    pub fn set_window_level(&self, level: WindowLevel) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.set_window_level(level)
    }
    pub fn move_and_resize(
        &self,
        bounds: Bounds<Pixels>,
        display: Option<DisplayId>,
    ) -> Result<(), WindowExtError> {
        self.lifetime.check()?;
        self.native.move_and_resize(bounds, display)
    }
}

pub trait WindowExt {
    fn native_window_handle(&self, cx: &mut App) -> Result<NativeWindowHandle, WindowExtError>;
    fn set_window_level(&self, level: WindowLevel) -> Result<(), WindowExtError>;
    /// Immediate placement for callers already holding a live Window.
    fn move_and_resize(
        &self,
        bounds: Bounds<Pixels>,
        display: Option<DisplayId>,
    ) -> Result<(), WindowExtError>;
}
impl WindowExt for Window {
    fn native_window_handle(&self, cx: &mut App) -> Result<NativeWindowHandle, WindowExtError> {
        let lifetime = Lifetime::for_window(self.window_handle().window_id(), cx)?;
        Ok(NativeWindowHandle {
            lifetime,
            native: platform::Native::for_window(self)?,
        })
    }
    fn set_window_level(&self, level: WindowLevel) -> Result<(), WindowExtError> {
        platform::Native::for_window(self)?.set_window_level(level)
    }
    fn move_and_resize(
        &self,
        bounds: Bounds<Pixels>,
        display: Option<DisplayId>,
    ) -> Result<(), WindowExtError> {
        platform::Native::for_window(self)?.move_and_resize(bounds, display)
    }
}
