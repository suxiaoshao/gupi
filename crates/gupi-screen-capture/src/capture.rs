use crate::{
    CaptureResult, Error,
    display::{self, Snapshot},
    overlay::Overlay,
};
use gpui_kit::component::Root;
use gpui_kit::*;
use image::RgbaImage;
use smol::channel::{Sender, bounded};
use std::{sync::Arc, time::Duration};
use window_ext::{NativeWindowHandle, WindowExt};

#[derive(Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Opening,
    Selecting,
    Finishing,
}
struct OverlayWindow {
    window: WindowHandle<Root>,
    native: Option<NativeWindowHandle>,
}
struct HiddenOverlay {
    id: u64,
    bounds: Bounds<Pixels>,
    native: NativeWindowHandle,
}
#[derive(Default)]
struct Capture {
    generation: u64,
    sender: Option<Sender<CaptureResult>>,
    task: Option<Task<()>>,
    activation_check: Option<Task<()>>,
    windows: Vec<OverlayWindow>,
    closing_windows: usize,
    pending_result: Option<CaptureResult>,
    snapshot: Snapshot,
    phase: Phase,
}
impl Global for Capture {}

pub fn is_active(cx: &App) -> bool {
    cx.try_global::<Capture>()
        .is_some_and(|c| c.sender.is_some())
}
pub fn cancel(cx: &mut App) {
    finish(Ok(None), cx);
}
fn current(generation: u64, cx: &App) -> bool {
    cx.try_global::<Capture>()
        .is_some_and(|c| c.generation == generation && c.sender.is_some())
}
pub(crate) fn is_selecting(generation: u64, cx: &App) -> bool {
    current(generation, cx) && cx.global::<Capture>().phase == Phase::Selecting
}
pub(crate) fn finish_request(generation: u64, result: CaptureResult, cx: &mut App) {
    if current(generation, cx) && cx.global::<Capture>().phase != Phase::Finishing {
        finish(result, cx);
    }
}
fn close_windows(cx: &mut App) {
    let owner = cx.global_mut::<Capture>();
    let windows = std::mem::take(&mut owner.windows);
    owner.closing_windows += windows.len();
    if windows.is_empty() {
        deliver_result(cx);
        return;
    }
    // Native cursor calls are outside every Window/App update. Cursor
    // restoration precedes removal; the request stays busy until cleanup ends.
    cx.spawn(async move |cx| {
        let restore = cx.update(|cx| {
            cx.try_global::<Capture>()
                .is_some_and(|owner| owner.phase == Phase::Finishing)
        });
        if restore {
            for entry in &windows {
                if let Some(native) = &entry.native {
                    let _ = native.restore_capture_cursor();
                }
            }
        }
        cx.update(|cx| {
            let count = windows.len();
            for entry in windows {
                let _ = entry
                    .window
                    .update(cx, |_, window, _| window.remove_window());
            }
            cx.global_mut::<Capture>().closing_windows -= count;
            deliver_result(cx);
        });
    })
    .detach();
}
pub(crate) fn finish(result: CaptureResult, cx: &mut App) {
    if !cx.has_global::<Capture>() {
        return;
    }
    let owner = cx.global_mut::<Capture>();
    owner.phase = Phase::Finishing;
    owner.generation = owner.generation.wrapping_add(1);
    owner.task = None;
    owner.activation_check = None;
    owner.snapshot.clear();
    owner.pending_result = Some(result);
    close_windows(cx);
}
fn deliver_result(cx: &mut App) {
    let owner = cx.global_mut::<Capture>();
    if owner.closing_windows != 0 {
        return;
    }
    if let Some(result) = owner.pending_result.take()
        && let Some(sender) = owner.sender.take()
    {
        let _ = sender.try_send(result);
    }
}

/// Freeze every display before opening any selection overlay.
/// The host hides obstructing windows and cancels abandoned requests.
pub fn start(cx: &mut App) -> Result<Task<CaptureResult>, Error> {
    if is_active(cx) {
        return Err(Error::Busy);
    }
    #[cfg(target_os = "macos")]
    if !objc2_core_graphics::CGPreflightScreenCaptureAccess()
        && !objc2_core_graphics::CGRequestScreenCaptureAccess()
    {
        return Err(Error::PermissionDenied);
    }
    let pointer = platform_ext::app::current_mouse_display_id().ok_or(Error::DisplayChanged)?;
    let (snapshot, monitors) = display::read(cx)?;
    if !snapshot.contains_key(&pointer) {
        return Err(Error::DisplayChanged);
    }
    if !cx.has_global::<Capture>() {
        cx.set_global(Capture::default());
    }
    let (sender, receiver) = bounded(1);
    let owner = cx.global_mut::<Capture>();
    owner.generation = owner.generation.wrapping_add(1);
    let generation = owner.generation;
    owner.sender = Some(sender);
    owner.snapshot = snapshot;
    owner.phase = Phase::Opening;
    let task = cx.spawn(async move |cx| {
        let captured = smol::unblock(move || freeze(monitors)).await;
        let opened = cx.update(|cx| open_captured(generation, captured, cx));
        let result = match opened {
            Ok(handles) => {
                let mut result = Ok(());
                for panel in &handles {
                    #[cfg(target_os = "macos")]
                    let origin = Point::default();
                    #[cfg(target_os = "windows")]
                    let origin = panel.bounds.origin;
                    if let Err(e) = panel.native.configure_capture(
                        Bounds::new(origin, panel.bounds.size),
                        DisplayId::new(panel.id),
                    ) {
                        result = Err(capture_window_error(e));
                        break;
                    }
                }
                if result.is_ok() {
                    // Non-key panels first; establish final key status only once.
                    for panel in &handles {
                        if panel.id != pointer
                            && let Err(e) = panel.native.show_capture(false)
                        {
                            result = Err(Error::Capture(e.to_string()));
                            break;
                        }
                    }
                }
                if result.is_ok()
                    && let Some(panel) = handles.iter().find(|panel| panel.id == pointer)
                {
                    result = panel
                        .native
                        .show_capture(true)
                        .and_then(|_| panel.native.set_capture_cursor())
                        .map_err(|e| Error::Capture(e.to_string()));
                }
                result
            }
            Err(e) => Err(e),
        };
        cx.update(|cx| {
            complete_opening(generation, result, cx);
        });
    });
    cx.global_mut::<Capture>().task = Some(task);
    Ok(cx.spawn(async move |_| receiver.recv().await.unwrap_or(Ok(None))))
}
fn open_captured(
    generation: u64,
    captured: Result<Vec<(u64, Arc<RenderImage>)>, Error>,
    cx: &mut App,
) -> Result<Vec<HiddenOverlay>, Error> {
    if !current(generation, cx) {
        return Ok(Vec::new());
    }
    let frames = captured?;
    validate_snapshot(cx)?;
    let mut handles = Vec::new();
    for (id, preview) in frames {
        let bounds = cx.global::<Capture>().snapshot[&id].bounds;
        let native = open_overlay(preview, id, bounds, generation, cx)?;
        handles.push(HiddenOverlay { id, bounds, native });
    }
    Ok::<_, Error>(handles)
}
fn capture_window_error(error: window_ext::WindowExtError) -> Error {
    match error {
        window_ext::WindowExtError::DisplayNotFound
        | window_ext::WindowExtError::DisplayDpi(_)
        | window_ext::WindowExtError::FailedSetBounds
        | window_ext::WindowExtError::WindowClosed => Error::DisplayChanged,
        error => Error::Capture(error.to_string()),
    }
}
fn complete_opening(generation: u64, result: Result<(), Error>, cx: &mut App) {
    if !current(generation, cx) {
        return;
    }
    match result {
        Ok(()) => cx.global_mut::<Capture>().phase = Phase::Selecting,
        Err(e) => finish(Err(e), cx),
    }
}
fn register_overlay(window: WindowHandle<Root>, cx: &mut App) {
    cx.global_mut::<Capture>().windows.push(OverlayWindow {
        window,
        native: None,
    });
}
fn open_overlay(
    preview: Arc<RenderImage>,
    id: u64,
    bounds: Bounds<Pixels>,
    generation: u64,
    cx: &mut App,
) -> Result<NativeWindowHandle, Error> {
    let handle = gpui_kit::open_window(
        WindowOptions {
            display_id: Some(DisplayId::new(id)),
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                Point::default(),
                bounds.size,
            ))),
            kind: WindowKind::PopUp,
            titlebar: None,
            window_background: WindowBackgroundAppearance::Opaque,
            is_movable: false,
            is_resizable: false,
            focus: false,
            show: false,
            ..Default::default()
        },
        cx,
        |window, cx| {
            window.on_window_should_close(cx, move |_, cx| {
                finish_request(generation, Ok(None), cx);
                false
            });
            cx.new(|cx| Overlay::new(preview, generation, window, cx))
        },
    )
    .map_err(|e| Error::Capture(e.to_string()))?
    .0
    .downcast::<Root>()
    .expect("capture root");
    // Register before acquiring/configuring native handles so every failure cleans up.
    register_overlay(handle, cx);
    let native = handle
        .update(cx, |_, window, cx| window.native_window_handle(cx))
        .map_err(|_| Error::DisplayChanged)?
        .map_err(|e| Error::Capture(e.to_string()))?;
    cx.global_mut::<Capture>()
        .windows
        .last_mut()
        .unwrap()
        .native = Some(native.clone());
    Ok(native)
}
fn validate_snapshot(cx: &App) -> Result<(), Error> {
    let next = display::current(cx)?;
    if next != cx.global::<Capture>().snapshot {
        Err(Error::DisplayChanged)
    } else {
        Ok(())
    }
}
fn freeze(ids: Vec<u64>) -> Result<Vec<(u64, Arc<RenderImage>)>, Error> {
    std::thread::scope(|scope| {
        let tasks: Vec<_> = ids
            .into_iter()
            .map(|id| {
                scope.spawn(move || {
                    // Windows XCap monitors contain a non-Send HMONITOR. Construct the
                    // monitor on its worker instead of sending native handles across threads.
                    let monitors =
                        xcap::Monitor::all().map_err(|e| Error::Capture(e.to_string()))?;
                    let mut matching = monitors
                        .into_iter()
                        .filter(|m| m.id().ok() == Some(id as u32));
                    let monitor = matching.next().ok_or(Error::DisplayChanged)?;
                    if matching.next().is_some() {
                        return Err(Error::DisplayChanged);
                    }
                    let frame = monitor
                        .capture_image()
                        .map_err(|e| Error::Capture(e.to_string()))?;
                    Ok((id, preview(frame)?))
                })
            })
            .collect();
        tasks
            .into_iter()
            .map(|t| {
                t.join()
                    .unwrap_or_else(|_| Err(Error::Capture("Capture worker panicked".into())))
            })
            .collect()
    })
}
fn preview(mut frame: RgbaImage) -> Result<Arc<RenderImage>, Error> {
    if frame.width() == 0 || frame.height() == 0 {
        return Err(Error::Capture("Empty image".into()));
    }
    for pixel in frame.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(frame)])))
}

pub(crate) fn activation_changed(generation: u64, cx: &mut App) {
    if !is_selecting(generation, cx) || cx.global::<Capture>().activation_check.is_some() {
        return;
    }
    let task = cx.spawn(async move |cx| {
        // Let the native event dispatch (including both activation notifications)
        // finish. This also coalesces multiple panels resigning in one transition.
        cx.background_executor()
            .timer(Duration::from_millis(16))
            .await;
        cx.update(|cx| {
            if !is_selecting(generation, cx) {
                return;
            }
            cx.global_mut::<Capture>().activation_check = None;
            let windows: Vec<_> = cx
                .global::<Capture>()
                .windows
                .iter()
                .map(|w| w.window)
                .collect();
            let active = windows.into_iter().any(|handle| {
                handle
                    .update(cx, |_, window, _| window.is_window_active())
                    .unwrap_or(false)
            });
            if !active {
                finish_request(generation, Ok(None), cx);
            }
        });
    });
    cx.global_mut::<Capture>().activation_check = Some(task);
}
pub(crate) fn submit(
    generation: u64,
    frame: Arc<RenderImage>,
    rect: (u32, u32, u32, u32),
    cx: &mut App,
) {
    if !is_selecting(generation, cx) {
        return;
    }
    if let Err(e) = validate_snapshot(cx) {
        finish_request(generation, Err(e), cx);
        return;
    }
    begin_encoding(generation, frame, rect, cx);
}
fn begin_encoding(
    generation: u64,
    frame: Arc<RenderImage>,
    rect: (u32, u32, u32, u32),
    cx: &mut App,
) {
    cx.global_mut::<Capture>().phase = Phase::Finishing;
    cx.global_mut::<Capture>().activation_check = None;
    close_windows(cx);
    let task = cx.spawn(async move |cx| {
        let result = smol::unblock(move || encode_frame(&frame, rect).map(Some)).await;
        cx.update(|cx| {
            if current(generation, cx) {
                finish(result, cx);
            }
        });
    });
    cx.global_mut::<Capture>().task = Some(task);
}
fn encode_frame(frame: &RenderImage, rect: (u32, u32, u32, u32)) -> Result<Vec<u8>, Error> {
    let (x, y, w, h) = rect;
    let size = frame.size(0);
    let data = frame
        .as_bytes(0)
        .ok_or_else(|| Error::Capture("Missing preview pixels".into()))?;
    if w == 0
        || h == 0
        || x.saturating_add(w) > size.width.0 as u32
        || y.saturating_add(h) > size.height.0 as u32
    {
        return Err(Error::Capture("Invalid crop bounds".into()));
    }
    let mut crop = RgbaImage::new(w, h);
    let pixels: &mut [u8] = crop.as_mut();
    for (row, target) in pixels.chunks_exact_mut(w as usize * 4).enumerate() {
        let offset = ((y as usize + row) * size.width.0 as usize + x as usize) * 4;
        target.copy_from_slice(&data[offset..offset + w as usize * 4]);
        for pixel in target.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    crop.write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| Error::Capture(e.to_string()))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::SnapshotFixture;
    use core::prelude::v1::test;

    fn frame() -> Arc<RenderImage> {
        preview(RgbaImage::from_fn(300, 240, |x, y| {
            image::Rgba([x as u8, y as u8, 193, 117])
        }))
        .unwrap()
    }
    fn install(cx: &mut App) -> smol::channel::Receiver<CaptureResult> {
        gpui_kit::init(cx);
        cx.set_quit_mode(QuitMode::Explicit);
        let (sender, receiver) = bounded(1);
        cx.set_global(Capture {
            generation: 7,
            sender: Some(sender),
            ..Default::default()
        });
        cx.set_global(SnapshotFixture(Snapshot::new()));
        receiver
    }
    struct Empty;
    impl Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    fn add_empty(cx: &mut TestAppContext) -> WindowHandle<Root> {
        cx.add_window(|window, cx| Root::new(cx.new(|_| Empty), window, cx))
    }
    #[test]
    fn bgra_crop_encodes_original_channels_alpha_and_row_stride() {
        let bgra = frame();
        let png = encode_frame(&bgra, (2, 3, 4, 2)).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (4, 2));
        assert_eq!(decoded.get_pixel(0, 0).0, [2, 3, 193, 117]);
        assert_eq!(decoded.get_pixel(3, 1).0, [5, 4, 193, 117]);
        assert!(encode_frame(&bgra, (299, 0, 2, 1)).is_err());
    }
    #[gpui_kit::test]
    fn partial_opening_failure_removes_all_registered_windows(cx: &mut TestAppContext) {
        let receiver = cx.update(install);
        let first = add_empty(cx);
        let second = add_empty(cx);
        cx.update(|cx| {
            register_overlay(first, cx);
            register_overlay(second, cx);
            complete_opening(7, Err(Error::DisplayChanged), cx);
            assert!(
                is_active(cx),
                "request stays busy until all panels are removed"
            );
        });
        cx.run_until_parked();
        assert!(matches!(
            receiver.try_recv(),
            Ok(Err(Error::DisplayChanged))
        ));
        assert!(first.update(cx, |_, _, _| ()).is_err());
        assert!(second.update(cx, |_, _, _| ()).is_err());
    }
    #[gpui_kit::test]
    fn cancel_while_freezing_rejects_late_opening_completion(cx: &mut TestAppContext) {
        let receiver = cx.update(install);
        cx.update(cancel);
        cx.update(|cx| {
            assert!(
                open_captured(7, Ok(vec![(999, frame())]), cx)
                    .unwrap()
                    .is_empty()
            );
            complete_opening(7, Ok(()), cx);
        });
        assert!(matches!(receiver.try_recv(), Ok(Ok(None))));
        assert!(!cx.update(|cx| is_active(cx)));
        assert!(cx.update(|cx| cx.global::<Capture>().phase == Phase::Finishing));
    }
    #[gpui_kit::test]
    async fn closing_all_panels_during_encoding_does_not_cancel_output(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        let receiver = cx.update(install);
        let first = add_empty(cx);
        let second = add_empty(cx);
        cx.update(|cx| {
            register_overlay(first, cx);
            register_overlay(second, cx);
            complete_opening(7, Ok(()), cx);
            begin_encoding(7, frame(), (2, 3, 4, 2), cx);
            activation_changed(7, cx);
            finish_request(7, Ok(None), cx);
        });
        cx.run_until_parked();
        let png = smol::future::or(receiver.recv(), async {
            smol::Timer::after(Duration::from_secs(3)).await;
            panic!("production encoding did not complete");
        })
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 4);
        assert!(first.update(cx, |_, _, _| ()).is_err());
        assert!(second.update(cx, |_, _, _| ()).is_err());
    }
    #[gpui_kit::test]
    fn initialization_and_panel_activation_transfer_do_not_cancel(cx: &mut TestAppContext) {
        let receiver = cx.update(install);
        let first = add_empty(cx);
        let second = add_empty(cx);
        cx.update(|cx| {
            register_overlay(first, cx);
            register_overlay(second, cx);
            activation_changed(7, cx);
            assert!(cx.global::<Capture>().activation_check.is_none());
            complete_opening(7, Ok(()), cx);
            activation_changed(7, cx);
            activation_changed(7, cx);
        });
        second
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        assert!(receiver.try_recv().is_err());
        assert!(cx.update(|cx| is_selecting(7, cx)));
        let (_, visual) = cx.add_window_view(|_, _| Empty);
        visual.update(|window, _| window.activate_window());
        cx.update(|cx| activation_changed(7, cx));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        assert!(matches!(receiver.try_recv(), Ok(Ok(None))));
    }
    #[gpui_kit::test]
    async fn first_mouse_drag_uses_production_binding_and_captured_image_dimensions(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        let receiver = cx.update(install);
        let (_, visual) = cx.add_window_view(|window, cx| {
            Root::new(
                cx.new(|cx| Overlay::new(frame(), 7, window, cx)),
                window,
                cx,
            )
        });
        let expected = visual.update(|window, cx| {
            register_overlay(window.window_handle().downcast::<Root>().unwrap(), cx);
            complete_opening(7, Ok(()), cx);
            window.draw(cx).clear(cx);
            let start = point(px(20.), px(30.));
            window.simulate_mouse_move(start, cx);
            window.dispatch_event(
                MouseDownEvent {
                    position: start,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: true,
                }
                .to_platform_input(),
                cx,
            );
            let end = point(px(60.), px(80.));
            window.dispatch_event(
                MouseMoveEvent {
                    position: end,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
            let rect = crate::selection::bounds(start, end, window.viewport_size()).unwrap();
            let expected = crate::selection::crop_rect(rect, window.viewport_size(), 300, 240);
            window.dispatch_event(
                MouseUpEvent {
                    position: end,
                    button: MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            expected
        });
        visual.run_until_parked();
        let png = smol::future::or(receiver.recv(), async {
            smol::Timer::after(Duration::from_secs(3)).await;
            panic!("production encoding did not complete");
        })
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (expected.2, expected.3));
        assert_eq!(
            image.get_pixel(0, 0).0,
            [expected.0 as u8, expected.1 as u8, 193, 117]
        );
    }
}
