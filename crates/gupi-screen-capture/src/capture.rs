use crate::{Error, overlay::Overlay};
use gpui_kit::component::Root;
use gpui_kit::*;
use image::RgbaImage;
use smol::channel::{Sender, bounded};
use std::sync::Arc;
use window_ext::WindowExt;
use window_ext::WindowLevel;

type CaptureResult = Result<Option<Vec<u8>>, Error>;
#[derive(Default)]
struct Capture {
    generation: u64,
    sender: Option<Sender<CaptureResult>>,
    task: Option<Task<()>>,
    window: Option<WindowHandle<Root>>,
}
impl Global for Capture {}

pub fn is_active(cx: &App) -> bool {
    cx.try_global::<Capture>()
        .is_some_and(|c| c.sender.is_some())
}

/// Cancel the active request, close its selection window and invalidate pending work.
pub fn cancel(cx: &mut App) {
    finish(Ok(None), cx);
}

pub(crate) fn finish_request(generation: u64, result: CaptureResult, cx: &mut App) {
    if cx
        .try_global::<Capture>()
        .is_some_and(|owner| owner.generation == generation)
    {
        finish(result, cx);
    }
}

pub(crate) fn finish(result: CaptureResult, cx: &mut App) {
    if !cx.has_global::<Capture>() {
        return;
    }
    let owner = cx.global_mut::<Capture>();
    owner.generation = owner.generation.wrapping_add(1);
    owner.task = None;
    let sender = owner.sender.take();
    let window = owner.window.take();
    if let Some(window) = window {
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        });
    }
    if let Some(sender) = sender {
        let _ = sender.try_send(result);
    }
}

/// Freeze the display under the pointer before opening a selection window.
/// The host must hide any obstructing window before calling this method.
/// On macOS, missing screen recording access triggers the system permission request.
/// `None` means cancellation; no clipboard or file destination is changed.
/// The host calls `cancel` when abandoning the returned task or shutting down.
pub fn start(cx: &mut App) -> Result<Task<CaptureResult>, Error> {
    if is_active(cx) {
        return Err(Error::Busy);
    }
    #[cfg(target_os = "macos")]
    if !objc2_core_graphics::CGPreflightScreenCaptureAccess()
        && !objc2_core_graphics::CGRequestScreenCaptureAccess()
    {
        // macOS owns the first-use prompt and remembers denial. A denied request
        // requires enabling access in System Settings before retrying capture.
        return Err(Error::PermissionDenied);
    }
    let id = platform_ext::app::current_mouse_display_id().ok_or(Error::DisplayChanged)?;
    let display = cx
        .displays()
        .into_iter()
        .find(|d| u64::from(d.id()) == id)
        .ok_or(Error::DisplayChanged)?;
    let display_id = display.id();
    let display_bounds = display.bounds();
    if !cx.has_global::<Capture>() {
        cx.set_global(Capture::default());
    }
    let (sender, receiver) = bounded(1);
    let owner = cx.global_mut::<Capture>();
    owner.generation = owner.generation.wrapping_add(1);
    let generation = owner.generation;
    owner.sender = Some(sender);
    let task = cx.spawn(async move |cx| {
        let captured = smol::unblock(move || freeze(id)).await;
        cx.update(|cx| {
            if cx.global::<Capture>().generation != generation {
                return;
            }
            let result = captured.and_then(|(frame, preview)| {
                if !cx
                    .displays()
                    .iter()
                    .any(|d| d.id() == display_id && d.bounds() == display_bounds)
                {
                    return Err(Error::DisplayChanged);
                }
                let handle = gpui_kit::open_window(
                    WindowOptions {
                        display_id: Some(display_id),
                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                            Point::default(),
                            display_bounds.size,
                        ))),
                        kind: WindowKind::PopUp,
                        titlebar: None,
                        window_background: WindowBackgroundAppearance::Opaque,
                        is_movable: false,
                        is_resizable: false,
                        focus: true,
                        show: true,
                        ..Default::default()
                    },
                    cx,
                    move |window, cx| {
                        // These bounds describe a physical display, not application spacing.
                        let _ = window.set_window_level(WindowLevel::PopUpMenu);
                        if let Ok(native) = window.native_window_handle() {
                            #[cfg(target_os = "macos")]
                            let origin = Point::default();
                            #[cfg(target_os = "windows")]
                            let origin = display_bounds.origin;
                            let _ = native.move_and_resize(
                                Bounds::new(origin, display_bounds.size),
                                Some(display_id),
                            );
                        }
                        window.on_window_should_close(cx, move |_, cx| {
                            finish_request(generation, Ok(None), cx);
                            false
                        });
                        cx.new(|cx| {
                            Overlay::new(
                                frame,
                                preview,
                                (display_id, display_bounds),
                                generation,
                                window,
                                cx,
                            )
                        })
                    },
                )
                .map_err(|e| Error::Capture(e.to_string()))?;
                cx.global_mut::<Capture>().window =
                    Some(handle.0.downcast::<Root>().expect("capture root"));
                Ok(())
            });
            if let Err(error) = result {
                finish(Err(error), cx);
            }
        });
    });
    cx.global_mut::<Capture>().task = Some(task);
    Ok(cx.spawn(async move |_| receiver.recv().await.unwrap_or(Ok(None))))
}

fn freeze(id: u64) -> Result<(RgbaImage, Arc<RenderImage>), Error> {
    let monitors = xcap::Monitor::all().map_err(|e| Error::Capture(e.to_string()))?;
    // XCap represents Windows HMONITOR as u32; GPUI preserves the native handle.
    let monitor = monitors
        .into_iter()
        .find(|m| m.id().ok() == Some(id as u32))
        .ok_or(Error::DisplayChanged)?;
    let frame = monitor
        .capture_image()
        .map_err(|e| Error::Capture(e.to_string()))?;
    if frame.width() == 0 || frame.height() == 0 {
        return Err(Error::Capture("Empty image".into()));
    }
    let mut preview = frame.clone();
    // GPUI uploads BGRA; retain unmodified RGBA for the actual PNG.
    for pixel in preview.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    let preview = Arc::new(RenderImage::new(vec![image::Frame::new(preview)]));
    Ok((frame, preview))
}

pub(crate) fn encode(generation: u64, frame: RgbaImage, rect: (u32, u32, u32, u32), cx: &mut App) {
    if cx.global::<Capture>().generation != generation || !is_active(cx) {
        return;
    }
    let task = cx.spawn(async move |cx| {
        let result = smol::unblock(move || encode_frame(frame, rect).map(Some)).await;
        cx.update(|cx| {
            if cx.global::<Capture>().generation == generation {
                finish(result, cx);
            }
        });
    });
    cx.global_mut::<Capture>().task = Some(task);
}

fn encode_frame(frame: RgbaImage, rect: (u32, u32, u32, u32)) -> Result<Vec<u8>, Error> {
    let (x, y, w, h) = rect;
    let cropped = image::imageops::crop_imm(&frame, x, y, w, h).to_image();
    let mut bytes = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| Error::Capture(e.to_string()))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::encode_frame;
    use image::RgbaImage;
    #[test]
    fn output_preserves_frozen_pixels_and_rgba_channels() {
        let frame = RgbaImage::from_fn(8, 6, |x, y| image::Rgba([x as u8, y as u8, 193, 255]));
        let expected = frame.get_pixel(2, 3).0;
        let png = encode_frame(frame, (2, 3, 4, 2)).unwrap();
        let result = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(result.dimensions(), (4, 2));
        assert_eq!(result.get_pixel(0, 0).0, expected);
        assert_eq!(result.get_pixel(3, 1).0, [5, 4, 193, 255]);
    }
}
