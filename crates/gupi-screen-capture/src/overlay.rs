use crate::{Error, capture, selection};
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use image::RgbaImage;
use std::sync::Arc;

pub(crate) struct Overlay {
    frame: Option<RgbaImage>,
    preview: Arc<RenderImage>,
    generation: u64,
    display_id: DisplayId,
    display_bounds: Bounds<Pixels>,
    start: Option<Point<Pixels>>,
    end: Option<Point<Pixels>>,
    focus: FocusHandle,
    _activation: Subscription,
}
impl Overlay {
    pub(crate) fn new(
        frame: RgbaImage,
        preview: Arc<RenderImage>,
        display: (DisplayId, Bounds<Pixels>),
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (display_id, display_bounds) = display;
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.frame.is_some() {
                capture::finish_request(this.generation, Ok(None), cx);
            }
        });
        Self {
            frame: Some(frame),
            preview,
            generation,
            display_id,
            display_bounds,
            start: None,
            end: None,
            focus,
            _activation: activation,
        }
    }
    fn finish(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left {
            return;
        }
        let Some(frame) = self.frame.take() else {
            return;
        };
        if !cx
            .displays()
            .iter()
            .any(|d| d.id() == self.display_id && d.bounds() == self.display_bounds)
        {
            capture::finish_request(self.generation, Err(Error::DisplayChanged), cx);
            return;
        }
        let rect = self
            .start
            .and_then(|start| selection::bounds(start, event.position, window.viewport_size()));
        let Some(rect) = rect else {
            capture::finish_request(self.generation, Ok(None), cx);
            return;
        };
        let rect =
            selection::crop_rect(rect, window.viewport_size(), frame.width(), frame.height());
        window.remove_window();
        capture::encode(self.generation, frame, rect, cx);
    }
}
impl Render for Overlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let selection = self
            .start
            .zip(self.end)
            .and_then(|(start, end)| selection::bounds(start, end, viewport));
        let mut root = div()
            .id("screen-selection")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .overflow_hidden()
            .cursor(CursorStyle::Crosshair)
            .child(
                img(self.preview.clone())
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    capture::finish_request(this.generation, Ok(None), cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    capture::finish_request(this.generation, Ok(None), cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.start = Some(event.position);
                    this.end = Some(event.position);
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.start.is_some() {
                    this.end = Some(event.position);
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish));
        // Raster-relative geometry must not scale with application typography.
        let rect = selection.unwrap_or(Bounds::new(Point::default(), Size::default()));
        let mask = cx.theme().background.opacity(0.6);
        for bounds in [
            Bounds::new(Point::default(), size(viewport.width, rect.origin.y)),
            Bounds::new(
                point(px(0.), rect.origin.y),
                size(rect.origin.x, rect.size.height),
            ),
            Bounds::new(
                point(rect.right(), rect.origin.y),
                size(viewport.width - rect.right(), rect.size.height),
            ),
            Bounds::new(
                point(px(0.), rect.bottom()),
                size(viewport.width, viewport.height - rect.bottom()),
            ),
        ] {
            root = root.child(
                div()
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .bg(mask),
            );
        }
        if selection.is_some() {
            root = root.child(
                div()
                    .absolute()
                    .left(rect.origin.x)
                    .top(rect.origin.y)
                    .w(rect.size.width)
                    .h(rect.size.height)
                    .border_1()
                    .border_color(cx.theme().primary),
            );
        }
        root
    }
}
