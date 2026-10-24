//! Shared pan/zoom image canvas; only PreviewHost owns the modal lifecycle.
use crate::camera::Camera;
use gpui_kit::component::Disableable;
use gpui_kit::component::Icon;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::h_flex;
use gpui_kit::component::label::Label;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_conversation::attachments::ImageAttachment;
use gupi_settings::assets::IconName;
use gupi_settings::i18n::t;
use std::sync::Arc;

const MAX_ZOOM_PERCENT: f32 = 800.;
const ZOOM_EPSILON: f32 = 0.01;
const ZOOM_RAMP_PERCENT: [f32; 10] = [25., 50., 75., 100., 125., 150., 200., 300., 400., 800.];

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewSize {
    width: f32,
    height: f32,
}
#[derive(Clone, Copy)]
enum ZoomStep {
    In,
    Out,
}

struct ImagePreview {
    host: WeakEntity<PreviewHost>,
    canvas: Entity<ImageCanvas>,
    name: Option<String>,
    _subscription: Subscription,
}

enum PreviewImage {
    Attachment(Arc<ImageAttachment>),
    Message {
        image: Arc<Image>,
        dimensions: (u32, u32),
    },
}
impl PreviewImage {
    fn source(&self) -> ImageSource {
        match self {
            Self::Attachment(image) => image.path().into(),
            Self::Message { image, .. } => image.clone().into(),
        }
    }
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Attachment(image) => image.dimensions(),
            Self::Message { dimensions, .. } => *dimensions,
        }
    }
}

/// Per-page modal owner; the base dialog supplies the focus trap and dismissal.
pub(super) struct PreviewHost {
    preview: Option<Entity<ImagePreview>>,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
}

impl PreviewHost {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            preview: None,
            focus: cx.focus_handle(),
            previous_focus: None,
        }
    }

    pub(super) fn is_open(&self) -> bool {
        self.preview.is_some()
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preview = None;
        if let Some(focus) = self.previous_focus.take() {
            focus.focus(window, cx);
        }
        cx.notify();
    }
}

impl Render for PreviewHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(preview) = self.preview.clone() else {
            return div().into_any_element();
        };
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.focus.clone())
            .on_ok(|_, _, _| false)
            .on_close(cx.listener(|this, _, window, cx| this.close(window, cx)))
            .popup(preview)
            .into_any_element()
    }
}

pub(super) fn open(
    host: &Entity<PreviewHost>,
    image: Arc<ImageAttachment>,
    name: String,
    window: &mut Window,
    cx: &mut App,
) {
    open_source(
        host,
        PreviewImage::Attachment(image),
        Some(name),
        window,
        cx,
    );
}

pub(super) fn open_message(
    host: &Entity<PreviewHost>,
    image: Arc<Image>,
    dimensions: (u32, u32),
    window: &mut Window,
    cx: &mut App,
) {
    open_source(
        host,
        PreviewImage::Message { image, dimensions },
        None,
        window,
        cx,
    );
}

fn open_source(
    host: &Entity<PreviewHost>,
    image: PreviewImage,
    name: Option<String>,
    window: &mut Window,
    cx: &mut App,
) {
    host.update(cx, |host, cx| {
        if host.preview.is_none() {
            host.previous_focus = window.focused(cx);
        }
        let owner = cx.weak_entity();
        host.preview = Some(cx.new(|cx| {
            let canvas = cx.new(|_| ImageCanvas::new(CanvasImage::Preview(image)));
            let subscription = cx.observe(&canvas, |_, _, cx| cx.notify());
            ImagePreview {
                host: owner,
                canvas,
                name,
                _subscription: subscription,
            }
        }));
        host.focus.focus(window, cx);
        cx.notify();
    });
}

impl ImagePreview {
    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.host.update(cx, |host, cx| host.close(window, cx));
    }
    fn step(&mut self, direction: ZoomStep, _: &mut Window, cx: &mut Context<Self>) {
        self.canvas
            .update(cx, |canvas, cx| canvas.step(direction, cx));
    }

    fn render_image(&self) -> impl IntoElement {
        div()
            .absolute()
            .top(rems(3.25))
            .bottom(rems(5.5))
            .left(rems(3.5))
            .right(rems(3.5))
            .child(self.canvas.clone())
    }

    fn render_controls(
        &self,
        natural: PreviewSize,
        viewport: PreviewSize,
        zoom: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let fit = fit_zoom_percent(natural, viewport);
        let ready = self.canvas.read(cx).initialized;
        let zoom_out = t(cx, "image-preview-zoom-out");
        let zoom_in = t(cx, "image-preview-zoom-in");
        h_flex()
            .absolute()
            .bottom_6()
            .left_0()
            .right_0()
            .justify_center()
            .child(
                h_flex()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .rounded_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().tokens.popover.background)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        Button::new("image-preview-zoom-out")
                            .ghost()
                            .size(rems(2.5))
                            .p_0()
                            .rounded_full()
                            .icon(Icon::new(IconName::Minus).size_5())
                            .tooltip(zoom_out.clone())
                            .accessibility_label(zoom_out)
                            .disabled(
                                !ready
                                    || next_zoom_step(zoom, fit, ZoomStep::Out)
                                        >= zoom - ZOOM_EPSILON,
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.step(ZoomStep::Out, window, cx)
                            })),
                    )
                    .child(
                        Label::new(format!("{}%", format_zoom_percent(zoom)))
                            .text_sm()
                            .text_color(cx.theme().popover_foreground)
                            .min_w(rems(4.25))
                            .text_center(),
                    )
                    .child(
                        Button::new("image-preview-zoom-in")
                            .ghost()
                            .size(rems(2.5))
                            .p_0()
                            .rounded_full()
                            .icon(Icon::new(IconName::Plus).size_5())
                            .tooltip(zoom_in.clone())
                            .accessibility_label(zoom_in)
                            .disabled(
                                !ready
                                    || next_zoom_step(zoom, fit, ZoomStep::In)
                                        <= zoom + ZOOM_EPSILON,
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.step(ZoomStep::In, window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ImagePreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let canvas = self.canvas.read(cx);
        let viewport = canvas.viewport();
        let natural = canvas.natural.expect("modal image dimensions");
        let zoom = canvas.camera.zoom * 100.;
        let close = t(cx, "image-preview-close");
        div()
            .id("image-preview")
            .test_support()
            .size_full()
            .occlude()
            .relative()
            .overflow_hidden()
            .bg(gupi_settings::theme::image_preview_backdrop())
            .text_color(cx.theme().foreground)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.close(window, cx);
                }),
            )
            .child(self.render_image())
            .child(self.render_controls(natural, viewport, zoom, cx))
            .child(
                h_flex()
                    .absolute()
                    .top_4()
                    // Keep the native traffic lights clear, even for long filenames.
                    .left(rems(7.))
                    .right_4()
                    .justify_end()
                    .gap_4()
                    .when_some(self.name.clone(), |header, name| {
                        header.child(
                            div()
                                .min_w_0()
                                .max_w(rems(32.5))
                                .px_3()
                                .py_2()
                                .rounded_full()
                                .bg(cx.theme().tokens.popover.background)
                                .text_color(cx.theme().popover_foreground)
                                .border_1()
                                .border_color(cx.theme().border)
                                .shadow_lg()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(Label::new(name).text_sm().truncate()),
                        )
                    })
                    .child(
                        Button::new("image-preview-close")
                            .ghost()
                            .size(rems(2.5))
                            .p_0()
                            .rounded_full()
                            .bg(cx.theme().tokens.popover.background)
                            .text_color(cx.theme().popover_foreground)
                            .border_1()
                            .border_color(cx.theme().border)
                            .shadow_lg()
                            .icon(Icon::new(IconName::X).size(rems(1.375)))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .tooltip(close.clone())
                            .accessibility_label(close)
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            )
    }
}

enum CanvasImage {
    Preview(PreviewImage),
    File {
        path: std::path::PathBuf,
        cache: Entity<RetainAllImageCache>,
    },
}

struct Drag {
    button: MouseButton,
    start: Point<f32>,
    pan: Point<f32>,
}

struct ImageCanvas {
    image: CanvasImage,
    natural: Option<PreviewSize>,
    camera: Camera,
    bounds: Bounds<Pixels>,
    initialized: bool,
    fitting: bool,
    drag: Option<Drag>,
}

struct Frame {
    bitmap: Option<AnyElement>,
    mask: ContentMask<Pixels>,
}

impl ImageCanvas {
    fn new(image: CanvasImage) -> Self {
        let natural = match &image {
            CanvasImage::Preview(image) => {
                let (width, height) = image.dimensions();
                Some(PreviewSize {
                    width: width as f32,
                    height: height as f32,
                })
            }
            CanvasImage::File { .. } => None,
        };
        Self {
            image,
            natural,
            camera: Camera::default(),
            bounds: Bounds::default(),
            initialized: false,
            fitting: true,
            drag: None,
        }
    }
    fn viewport(&self) -> PreviewSize {
        PreviewSize {
            width: self.bounds.size.width.into(),
            height: self.bounds.size.height.into(),
        }
    }
    fn center(&self) -> Point<f32> {
        point(
            f32::from(self.bounds.size.width),
            f32::from(self.bounds.size.height),
        ) / 2.
    }
    fn local(&self, position: Point<Pixels>) -> Point<f32> {
        (position - self.bounds.origin).map(f32::from)
    }
    fn fit(&mut self, cx: &mut Context<Self>) {
        if !self.initialized {
            return;
        }
        let natural = self.natural.expect("initialized image");
        self.camera.zoom = fit_zoom_percent(natural, self.viewport()) / 100.;
        self.camera.pan =
            self.center() - point(natural.width, natural.height) * (self.camera.zoom / 2.);
        self.fitting = true;
        self.drag = None;
        cx.notify();
    }
    fn set_zoom(&mut self, percent: f32, anchor: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        if !self.initialized {
            return;
        }
        let natural = self.natural.expect("initialized image");
        let zoom = clamp_zoom_percent(percent, fit_zoom_percent(natural, self.viewport())) / 100.;
        let anchor = anchor
            .map(|p| self.local(p))
            .unwrap_or_else(|| self.center());
        self.camera.zoom_at(anchor, zoom);
        self.fitting = false;
        self.drag = None;
        cx.notify();
    }
    fn step(&mut self, direction: ZoomStep, cx: &mut Context<Self>) {
        let Some(natural) = self.natural else {
            return;
        };
        self.set_zoom(
            next_zoom_step(
                self.camera.zoom * 100.,
                fit_zoom_percent(natural, self.viewport()),
                direction,
            ),
            None,
            cx,
        );
    }
    fn move_drag(&mut self, position: Point<Pixels>) {
        if let Some(drag) = &self.drag {
            self.camera.pan = drag.pan + self.local(position) - drag.start;
            self.fitting = false;
        }
    }
    fn prepare(
        &mut self,
        bounds: Bounds<Pixels>,
        natural: PreviewSize,
        source: ImageSource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Frame {
        let old_center = self.center();
        let resized = bounds.size != self.bounds.size;
        self.bounds = bounds;
        let mask = ContentMask {
            bounds: bounds.intersect(&window.content_mask().bounds),
        };
        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
            return Frame { bitmap: None, mask };
        }
        let initializing = !self.initialized;
        self.natural = Some(natural);
        if initializing || (resized && self.fitting) {
            self.camera.zoom = fit_zoom_percent(natural, self.viewport()) / 100.;
            self.camera.pan =
                self.center() - point(natural.width, natural.height) * (self.camera.zoom / 2.);
            self.initialized = true;
        } else if resized {
            self.camera.pan += self.center() - old_center;
        }
        if resized {
            self.drag = None;
        }
        let image_size = image_render_size(natural, self.camera.zoom * 100.);
        let mut bitmap = div()
            .id("image-preview-bitmap")
            .test_support()
            .w(px(image_size.width))
            .h(px(image_size.height))
            .child(img(source).size_full().object_fit(ObjectFit::Contain))
            .into_any_element();
        window.with_content_mask(Some(mask), |window| {
            bitmap.layout_as_root(
                size(
                    AvailableSpace::Definite(px(image_size.width)),
                    AvailableSpace::Definite(px(image_size.height)),
                ),
                window,
                cx,
            );
            bitmap.prepaint_at(bounds.origin + self.camera.pan.map(px), window, cx);
        });
        // The body uses this frame's geometry. Notify observers after prepaint,
        // when the outer controls can consume the initialized viewport.
        if initializing || resized {
            let owner = cx.weak_entity();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |_, cx| cx.notify());
            });
        }
        Frame {
            bitmap: Some(bitmap),
            mask,
        }
    }
}

impl Render for ImageCanvas {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (id, image) = match &self.image {
            CanvasImage::Preview(image) => (
                "image-preview-canvas",
                Some(Ok((image.source(), self.natural.unwrap()))),
            ),
            CanvasImage::File { path, cache } => {
                let image = cache.update(cx, |cache, cx| {
                    cache.load(&Resource::from(path.clone()), window, cx)
                });
                (
                    "file-image-canvas",
                    image.map(|result| {
                        result.map(|image| {
                            let pixels = image.size(0);
                            let natural = PreviewSize {
                                width: u32::from(pixels.width) as f32,
                                height: u32::from(pixels.height) as f32,
                            };
                            (ImageSource::Render(image), natural)
                        })
                    }),
                )
            }
        };
        let content = match image {
            Some(Ok((source, natural))) => {
                let owner = cx.entity().clone();
                canvas(
                    move |bounds, window, cx| {
                        owner.update(cx, |this, cx| {
                            this.prepare(bounds, natural, source, window, cx)
                        })
                    },
                    |_, mut frame: Frame, window, cx| {
                        window.with_content_mask(Some(frame.mask), |window| {
                            if let Some(bitmap) = &mut frame.bitmap {
                                bitmap.paint(window, cx);
                            }
                        })
                    },
                )
                .size_full()
                .into_any_element()
            }
            Some(Err(_)) => {
                let CanvasImage::File { path, .. } = &self.image else {
                    unreachable!()
                };
                super::source::unavailable("files-unsupported", path.clone(), cx).into_any_element()
            }
            None => div().p_3().child(t(cx, "files-loading")).into_any_element(),
        };
        let mut surface = div()
            .id(id)
            .test_support()
            .size_full()
            .relative()
            .overflow_hidden()
            .cursor(if self.drag.is_some() {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            });
        for button in [MouseButton::Left, MouseButton::Middle] {
            surface = surface
                .on_mouse_down(
                    button,
                    cx.listener(|this, event: &MouseDownEvent, _, cx| {
                        if this.initialized {
                            this.drag = Some(Drag {
                                button: event.button,
                                start: this.local(event.position),
                                pan: this.camera.pan,
                            });
                            cx.notify();
                        }
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_up(
                    button,
                    cx.listener(|this, event: &MouseUpEvent, _, cx| {
                        if this
                            .drag
                            .as_ref()
                            .is_some_and(|drag| drag.button == event.button)
                        {
                            this.move_drag(event.position);
                            this.drag = None;
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }),
                )
                .on_mouse_up_out(
                    button,
                    cx.listener(|this, event: &MouseUpEvent, _, cx| {
                        if this
                            .drag
                            .as_ref()
                            .is_some_and(|drag| drag.button == event.button)
                        {
                            this.move_drag(event.position);
                            this.drag = None;
                            cx.notify();
                        }
                    }),
                );
        }
        surface
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some(drag) = &this.drag {
                    if event.pressed_button == Some(drag.button) {
                        this.move_drag(event.position);
                    } else {
                        this.drag = None;
                    }
                    cx.notify();
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                let delta = event.delta.pixel_delta(window.line_height()).map(f32::from);
                if event.modifiers.control || event.modifiers.platform {
                    this.set_zoom(
                        this.camera.zoom * 100. * (delta.y * 0.01).exp(),
                        Some(event.position),
                        cx,
                    );
                } else {
                    this.camera.pan += delta;
                    this.drag = None;
                    this.fitting = false;
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .on_pinch(cx.listener(|this, event: &PinchEvent, _, cx| {
                this.set_zoom(
                    this.camera.zoom * 100. * (1. + event.delta).max(0.01),
                    Some(event.position),
                    cx,
                );
                cx.stop_propagation();
            }))
            .child(content)
    }
}

/// Files owns the same canvas as the attachment popup, plus its compact controls.
pub(super) struct FileImage {
    canvas: Entity<ImageCanvas>,
    _subscription: Subscription,
}
impl FileImage {
    pub(super) fn new(path: std::path::PathBuf, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|cx| {
            ImageCanvas::new(CanvasImage::File {
                path,
                cache: RetainAllImageCache::new(cx),
            })
        });
        let subscription = cx.observe(&canvas, |_, _, cx| cx.notify());
        Self {
            canvas,
            _subscription: subscription,
        }
    }
}
impl Render for FileImage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = self.canvas.read(cx).initialized;
        gpui_kit::component::v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(
                h_flex()
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .child(
                        Button::new("file-image-zoom-out")
                            .small()
                            .ghost()
                            .icon(IconName::Minus)
                            .disabled(!ready)
                            .accessibility_label(t(cx, "image-preview-zoom-out"))
                            .tooltip(t(cx, "image-preview-zoom-out"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.canvas
                                    .update(cx, |canvas, cx| canvas.step(ZoomStep::Out, cx))
                            })),
                    )
                    .child(
                        Button::new("file-image-fit")
                            .small()
                            .ghost()
                            .label(t(cx, "image-preview-fit"))
                            .disabled(!ready)
                            .accessibility_label(t(cx, "image-preview-fit"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.canvas.update(cx, |canvas, cx| canvas.fit(cx))
                            })),
                    )
                    .child(
                        Button::new("file-image-original")
                            .small()
                            .ghost()
                            .label("100%")
                            .disabled(!ready)
                            .accessibility_label(t(cx, "image-preview-actual-size"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.canvas
                                    .update(cx, |canvas, cx| canvas.set_zoom(100., None, cx))
                            })),
                    )
                    .child(
                        Button::new("file-image-zoom-in")
                            .small()
                            .ghost()
                            .icon(IconName::Plus)
                            .disabled(!ready)
                            .accessibility_label(t(cx, "image-preview-zoom-in"))
                            .tooltip(t(cx, "image-preview-zoom-in"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.canvas
                                    .update(cx, |canvas, cx| canvas.step(ZoomStep::In, cx))
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .child(self.canvas.clone()),
            )
    }
}

fn fit_zoom_percent(natural_size: PreviewSize, viewport_size: PreviewSize) -> f32 {
    if natural_size.width <= 0. || natural_size.height <= 0. {
        return 100.;
    }

    let width_ratio = viewport_size.width / natural_size.width;
    let height_ratio = viewport_size.height / natural_size.height;
    (width_ratio.min(height_ratio).min(1.) * 100.).max(ZOOM_EPSILON)
}

fn clamp_zoom_percent(zoom_percent: f32, fit_percent: f32) -> f32 {
    if !zoom_percent.is_finite() {
        return fit_percent;
    }
    zoom_percent.clamp(fit_percent, MAX_ZOOM_PERCENT)
}

fn zoom_ramp(fit_percent: f32) -> Vec<f32> {
    let fit_percent = fit_percent.clamp(ZOOM_EPSILON, 100.);
    let mut ramp = Vec::with_capacity(ZOOM_RAMP_PERCENT.len() + 1);
    ramp.push(fit_percent);
    ramp.extend(
        ZOOM_RAMP_PERCENT
            .iter()
            .copied()
            .filter(|zoom| *zoom >= fit_percent - ZOOM_EPSILON),
    );
    ramp.sort_by(|a, b| a.total_cmp(b));
    ramp.dedup_by(|a, b| (*a - *b).abs() < ZOOM_EPSILON);
    ramp
}

fn next_zoom_step(current_zoom: f32, fit_percent: f32, direction: ZoomStep) -> f32 {
    let ramp = zoom_ramp(fit_percent);
    let current_zoom = clamp_zoom_percent(current_zoom, fit_percent);

    match direction {
        ZoomStep::In => ramp
            .iter()
            .copied()
            .find(|zoom| *zoom > current_zoom + ZOOM_EPSILON)
            .unwrap_or(MAX_ZOOM_PERCENT),
        ZoomStep::Out => ramp
            .iter()
            .rev()
            .copied()
            .find(|zoom| *zoom < current_zoom - ZOOM_EPSILON)
            .unwrap_or_else(|| ramp[0]),
    }
}

fn image_render_size(natural_size: PreviewSize, zoom_percent: f32) -> PreviewSize {
    let scale = zoom_percent / 100.;
    PreviewSize {
        width: (natural_size.width * scale).max(1.),
        height: (natural_size.height * scale).max(1.),
    }
}

fn format_zoom_percent(zoom_percent: f32) -> String {
    format!("{:.0}", zoom_percent.round())
}

#[cfg(test)]
mod tests {
    use super::PreviewSize;
    use super::ZoomStep;
    use super::fit_zoom_percent;
    use super::next_zoom_step;

    #[test]
    fn fit_zoom_caps_at_full_size_and_handles_image_aspect_ratios() {
        let viewport = PreviewSize {
            width: 1000.,
            height: 800.,
        };
        for (width, height, expected) in [
            (400., 300., 100.),
            (2000., 500., 50.),
            (500., 2000., 40.),
            (4000., 4000., 20.),
        ] {
            assert_eq!(
                fit_zoom_percent(PreviewSize { width, height }, viewport),
                expected
            );
        }
    }

    #[test]
    fn zoom_step_respects_bounds() {
        assert_eq!(next_zoom_step(50., 50., ZoomStep::In), 75.);
        assert_eq!(next_zoom_step(50., 50., ZoomStep::Out), 50.);
        assert_eq!(next_zoom_step(400., 50., ZoomStep::In), 800.);
        assert_eq!(next_zoom_step(800., 50., ZoomStep::In), 800.);
        assert_eq!(next_zoom_step(125., 50., ZoomStep::Out), 100.);
        // An irregular fit joins the ramp, with no duplicate full-size step.
        assert_eq!(next_zoom_step(62., 62., ZoomStep::In), 75.);
        assert_eq!(next_zoom_step(75., 62., ZoomStep::Out), 62.);
        assert_eq!(next_zoom_step(100., 100., ZoomStep::In), 125.);
        assert_eq!(next_zoom_step(100., 100., ZoomStep::Out), 100.);
    }
}
