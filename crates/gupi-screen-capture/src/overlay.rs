use crate::{capture, selection};
use gpui_kit::*;
use std::sync::Arc;

pub(crate) struct Overlay {
    preview: Arc<RenderImage>,
    generation: u64,
    start: Option<Point<Pixels>>,
    end: Option<Point<Pixels>>,
    focus: FocusHandle,
    _activation: Subscription,
}
impl Overlay {
    pub(crate) fn new(
        preview: Arc<RenderImage>,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                // GPUI applies the cursor style while drawing a key window; without
                // a redraw the panel keeps its initial arrow until the mouse moves.
                window.refresh();
            } else {
                capture::activation_changed(this.generation, cx);
            }
        });
        Self {
            preview,
            generation,
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
        if !capture::is_selecting(self.generation, cx) {
            return;
        }
        let Some(start) = self.start.take() else {
            return;
        };
        let Some(rect) = selection::bounds(start, event.position, window.viewport_size()) else {
            capture::finish_request(self.generation, Ok(None), cx);
            return;
        };
        let size = self.preview.size(0);
        let rect = selection::crop_rect(
            rect,
            window.viewport_size(),
            size.width.0 as u32,
            size.height.0 as u32,
        );
        capture::submit(self.generation, self.preview.clone(), rect, cx);
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
                    if !capture::is_selecting(this.generation, cx) {
                        return;
                    }
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
        let mask = black().opacity(0.45);
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
                    .border_color(white())
                    .child(
                        div()
                            .absolute()
                            // Absolute insets start inside the parent border.
                            .left(px(-2.))
                            .top(px(-2.))
                            .w(rect.size.width + px(2.))
                            .h(rect.size.height + px(2.))
                            .border_1()
                            .border_color(black()),
                    ),
            );
        }
        if selection.is_some() {
            let image = self.preview.size(0);
            let (_, _, width, height) =
                selection::crop_rect(rect, viewport, image.width.0 as u32, image.height.0 as u32);
            let label = selection::label_bounds(rect, viewport);
            root = root.child(
                div()
                    .absolute()
                    .left(label.origin.x)
                    .top(label.origin.y)
                    .w(label.size.width)
                    .h(label.size.height)
                    .text_size(px(12.))
                    .line_height(px(24.))
                    .text_color(white())
                    .bg(black().opacity(0.85))
                    .text_center()
                    .child(format!("{width} × {height}")),
            );
        }
        root
    }
}
