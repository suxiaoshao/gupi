use crate::{WindowExtError, WindowLevel};
use gpui::{Bounds, DisplayId, Pixels, Window};
#[cfg(target_os = "macos")]
use objc2::{MainThreadMarker, rc::Retained};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSScreen, NSView, NSWindow, NSWindowStyleMask};
use raw_window_handle::HasRawWindowHandle;
#[cfg(not(target_os = "macos"))]
use raw_window_handle::RawWindowHandle;
#[cfg(target_os = "windows")]
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            EnumDisplayMonitors, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromWindow,
        },
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                HTCLIENT, IDC_ARROW, IDC_CROSS, IsWindow, LoadCursorW, SW_HIDE, SW_SHOW,
                SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SetCursor, SetWindowPos,
                ShowWindow, USER_DEFAULT_SCREEN_DPI, WM_NCDESTROY, WM_SETCURSOR,
            },
        },
    },
    core::BOOL,
};

#[derive(Clone, Debug)]
pub(super) struct Native {
    #[cfg(target_os = "macos")]
    window: Retained<NSWindow>,
    #[cfg(target_os = "macos")]
    view: Retained<NSView>,
    #[cfg(not(target_os = "macos"))]
    raw: RawWindowHandle,
}
impl Native {
    pub(super) fn for_window(window: &Window) -> Result<Self, WindowExtError> {
        let raw = window
            .raw_window_handle()
            .map_err(WindowExtError::FailedToGetHandle)?;
        #[cfg(target_os = "macos")]
        {
            let raw_window_handle::RawWindowHandle::AppKit(handle) = raw else {
                return Err(WindowExtError::FailedToGetNSWindow);
            };
            // Window proves that the view is live. Retain the NSWindow now, never
            // dereference this raw NSView again after releasing the Window borrow.
            let view = unsafe { Retained::<NSView>::retain(handle.ns_view.as_ptr().cast()) }
                .ok_or(WindowExtError::FailedToGetNSView)?;
            Ok(Self {
                window: view.window().ok_or(WindowExtError::FailedToGetNSWindow)?,
                view,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self { raw })
        }
    }
    #[cfg(target_os = "windows")]
    fn hwnd(&self) -> Result<HWND, WindowExtError> {
        let RawWindowHandle::Win32(handle) = self.raw else {
            return Err(WindowExtError::WindowClosed);
        };
        let hwnd = HWND(handle.hwnd.get() as _);
        if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Err(WindowExtError::WindowClosed);
        }
        Ok(hwnd)
    }
    pub(super) fn configure_capture(
        &self,
        bounds: Bounds<Pixels>,
        display: DisplayId,
    ) -> Result<(), WindowExtError> {
        #[cfg(target_os = "macos")]
        {
            let style = self.window.styleMask();
            self.window.setStyleMask(
                style & !(NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView),
            );
            self.window
                .setLevel(macos_window_level_value(WindowLevel::PopUpMenu) as _);
        }
        #[cfg(target_os = "windows")]
        unsafe {
            if !SetWindowSubclass(self.hwnd()?, Some(capture_subclass), CAPTURE_SUBCLASS_ID, 0)
                .as_bool()
            {
                return Err(WindowExtError::CaptureWindow(
                    "SetWindowSubclass failed".into(),
                ));
            }
        }
        self.place(bounds, Some(display), true)
    }
    pub(super) fn show_capture(&self, activate: bool) -> Result<(), WindowExtError> {
        if activate {
            // A nonactivating panel takes key input without activating the app;
            // asynchronous app activation would reset the cursor afterwards.
            #[cfg(target_os = "macos")]
            {
                self.window.makeKeyAndOrderFront(None);
                if !self.window.makeFirstResponder(Some(&self.view)) {
                    return Err(WindowExtError::CaptureWindow(
                        "Could not restore first responder".into(),
                    ));
                }
            }
            #[cfg(target_os = "windows")]
            self.show()?;
        } else {
            #[cfg(target_os = "macos")]
            self.window.orderFront(None);
            #[cfg(target_os = "windows")]
            unsafe {
                let _ = ShowWindow(self.hwnd()?, SW_SHOWNOACTIVATE);
            }
        }
        Ok(())
    }
    pub(super) fn set_capture_cursor(&self) -> Result<(), WindowExtError> {
        #[cfg(target_os = "windows")]
        unsafe {
            SetCursor(Some(
                LoadCursorW(None, IDC_CROSS)
                    .map_err(|e| WindowExtError::CaptureWindow(e.to_string()))?,
            ));
        }
        Ok(())
    }
    pub(super) fn restore_capture_cursor(&self) -> Result<(), WindowExtError> {
        #[cfg(target_os = "windows")]
        unsafe {
            SetCursor(Some(
                LoadCursorW(None, IDC_ARROW)
                    .map_err(|e| WindowExtError::CaptureWindow(e.to_string()))?,
            ));
        }
        Ok(())
    }
    pub(super) fn hide(&self) -> Result<(), WindowExtError> {
        #[cfg(target_os = "macos")]
        self.window.orderOut(None);
        #[cfg(target_os = "windows")]
        unsafe {
            let _ = ShowWindow(self.hwnd()?, SW_HIDE);
        }
        Ok(())
    }
    pub(super) fn show(&self) -> Result<(), WindowExtError> {
        #[cfg(target_os = "macos")]
        {
            let mtm = MainThreadMarker::new().ok_or(WindowExtError::FailedToGetNSApplication)?;
            self.window.makeKeyAndOrderFront(None);
            NSApplication::sharedApplication(mtm).activate();
        }
        #[cfg(target_os = "windows")]
        unsafe {
            let _ = ShowWindow(self.hwnd()?, SW_SHOW);
        }
        Ok(())
    }
    pub(super) fn set_window_level(&self, level: WindowLevel) -> Result<(), WindowExtError> {
        #[cfg(target_os = "macos")]
        self.window.setLevel(macos_window_level_value(level) as _);
        #[cfg(not(target_os = "macos"))]
        let _ = level;
        Ok(())
    }
    pub(super) fn move_and_resize(
        &self,
        bounds: Bounds<Pixels>,
        display: Option<DisplayId>,
    ) -> Result<(), WindowExtError> {
        self.place(bounds, display, false)
    }
    fn place(
        &self,
        bounds: Bounds<Pixels>,
        display: Option<DisplayId>,
        preserve_activation: bool,
    ) -> Result<(), WindowExtError> {
        #[cfg(not(target_os = "windows"))]
        let _ = preserve_activation;
        #[cfg(target_os = "macos")]
        {
            let screen = resolve_screen_frame(&self.window, display)?;
            let frame = objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(
                    screen.origin.x + f64::from(f32::from(bounds.origin.x)),
                    screen.origin.y + screen.size.height
                        - f64::from(f32::from(bounds.origin.y))
                        - f64::from(f32::from(bounds.size.height)),
                ),
                objc2_foundation::NSSize::new(
                    f64::from(f32::from(bounds.size.width)),
                    f64::from(f32::from(bounds.size.height)),
                ),
            );
            self.window.setFrame_display(frame, true);
        }
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.hwnd()?;
            let monitors = available_monitors();
            let target = match display {
                Some(display) => u64::from(display),
                None => unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }.0 as u64,
            };
            let scale = select_display_scale(
                target,
                &monitors,
                |m| m.0 as u64,
                get_scale_factor_for_monitor,
            )?;
            let (x, y, width, height) = logical_bounds_to_device_rect(bounds, scale);
            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    x,
                    y,
                    width,
                    height,
                    if preserve_activation {
                        SWP_NOZORDER | SWP_NOACTIVATE
                    } else {
                        SWP_NOZORDER
                    },
                )
            }
            .map_err(|_| WindowExtError::FailedSetBounds)?;
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = (bounds, display, self.raw);
        Ok(())
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_window_level_value(level: WindowLevel) -> i32 {
    match level {
        WindowLevel::Normal => 0,
        WindowLevel::Floating => 3,
        WindowLevel::ModalPanel => 8,
        WindowLevel::PopUpMenu => 101,
        WindowLevel::Custom(v) => v,
    }
}
#[cfg(any(target_os = "windows", test))]
fn logical_bounds_to_device_rect(bounds: Bounds<Pixels>, scale: f32) -> (i32, i32, i32, i32) {
    let b = bounds.to_device_pixels(scale);
    (b.origin.x.0, b.origin.y.0, b.size.width.0, b.size.height.0)
}
#[cfg(any(target_os = "windows", test))]
fn select_display_scale<M>(
    target: u64,
    monitors: &[M],
    id: impl Fn(&M) -> u64,
    query: impl FnOnce(&M) -> Result<f32, WindowExtError>,
) -> Result<f32, WindowExtError> {
    let monitor = monitors
        .iter()
        .find(|monitor| id(monitor) == target)
        .ok_or(WindowExtError::DisplayNotFound)?;
    query(monitor)
}
#[cfg(target_os = "windows")]
fn available_monitors() -> Vec<HMONITOR> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut monitors as *mut _ as _),
        );
    }
    monitors
}
#[cfg(target_os = "windows")]
unsafe extern "system" fn monitor_enum_proc(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    unsafe {
        (*(data.0 as *mut Vec<HMONITOR>)).push(monitor);
    }
    BOOL(1)
}
#[cfg(target_os = "windows")]
fn get_scale_factor_for_monitor(monitor: &HMONITOR) -> Result<f32, WindowExtError> {
    let mut x = 0;
    let mut y = 0;
    unsafe { GetDpiForMonitor(*monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) }
        .map_err(|e| WindowExtError::DisplayDpi(e.to_string()))?;
    Ok(x as f32 / USER_DEFAULT_SCREEN_DPI as f32)
}
#[cfg(target_os = "macos")]
fn resolve_screen_frame(
    window: &NSWindow,
    display: Option<DisplayId>,
) -> Result<objc2_foundation::NSRect, WindowExtError> {
    if let Some(display) = display {
        let mtm = MainThreadMarker::new().ok_or(WindowExtError::FailedToGetNSApplication)?;
        let screens = NSScreen::screens(mtm);
        return screens
            .iter()
            .find(|s| u64::from(s.CGDirectDisplayID()) == u64::from(display))
            .map(|s| s.frame())
            .ok_or(WindowExtError::DisplayNotFound);
    }
    Ok(window
        .screen()
        .ok_or(WindowExtError::DisplayNotFound)?
        .frame())
}

#[cfg(target_os = "windows")]
const CAPTURE_SUBCLASS_ID: usize = 0x47555049;
#[cfg(target_os = "windows")]
unsafe extern "system" fn capture_subclass(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        if message == WM_SETCURSOR
            && (lparam.0 as u16) == HTCLIENT as u16
            && let Ok(cursor) = LoadCursorW(None, IDC_CROSS)
        {
            SetCursor(Some(cursor));
            return LRESULT(1);
        }
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(capture_subclass), id);
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

#[cfg(test)]
mod tests {
    use super::{logical_bounds_to_device_rect, macos_window_level_value, select_display_scale};
    use crate::{WindowExtError, WindowLevel};
    use gpui::{bounds, point, px, size};
    #[test]
    fn named_and_custom_levels_match_appkit() {
        for (level, expected) in [
            (WindowLevel::Normal, 0),
            (WindowLevel::Floating, 3),
            (WindowLevel::ModalPanel, 8),
            (WindowLevel::PopUpMenu, 101),
            (WindowLevel::Custom(9), 9),
        ] {
            assert_eq!(macos_window_level_value(level), expected);
        }
    }
    #[test]
    fn display_selection_uses_opaque_identity_and_preserves_query_errors() {
        let monitors = [(0x100000abcu64, 1.0), (0x200000def, 1.5)];
        for monitors in [monitors, [monitors[1], monitors[0]]] {
            assert_eq!(
                select_display_scale(0x200000def, &monitors, |m| m.0, |m| Ok(m.1)).unwrap(),
                1.5
            );
            assert!(matches!(
                select_display_scale(
                    1,
                    &monitors,
                    |m| m.0,
                    |_| panic!("missing monitor must not query DPI")
                ),
                Err(WindowExtError::DisplayNotFound)
            ));
            assert!(matches!(
                select_display_scale(
                    0x100000abc,
                    &monitors,
                    |m| m.0,
                    |_| Err(WindowExtError::DisplayDpi("failed".into()))
                ),
                Err(WindowExtError::DisplayDpi(_))
            ));
        }
    }
    #[test]
    fn device_coordinates_scale_offsets_and_dimensions() {
        for (x, expected) in [(1280., 1920), (-1280., -1920)] {
            assert_eq!(
                logical_bounds_to_device_rect(
                    bounds(point(px(x), px(120.)), size(px(800.), px(600.))),
                    1.5
                ),
                (expected, 180, 1200, 900)
            );
        }
    }
}
