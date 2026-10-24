//! Shared bitmap/zoom composition; only PreviewHost owns the modal lifecycle.
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
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewPoint {
    x: f32,
    y: f32,
}
#[derive(Clone, Copy)]
enum ZoomStep {
    In,
    Out,
}

struct ImagePreview {
    host: WeakEntity<PreviewHost>,
    image: PreviewImage,
    name: Option<String>,
    natural_size: PreviewSize,
    zoom_percent: Option<f32>,
    scroll: ScrollHandle,
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
    let (width, height) = image.dimensions();
    host.update(cx, |host, cx| {
        if host.preview.is_none() {
            host.previous_focus = window.focused(cx);
        }
        let owner = cx.weak_entity();
        host.preview = Some(cx.new(|_| ImagePreview {
            host: owner,
            image,
            name,
            natural_size: PreviewSize {
                width: width as f32,
                height: height as f32,
            },
            zoom_percent: None,
            scroll: ScrollHandle::new(),
        }));
        host.focus.focus(window, cx);
        cx.notify();
    });
}

impl ImagePreview {
    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.host.update(cx, |host, cx| host.close(window, cx));
    }
    fn viewport(window: &Window) -> PreviewSize {
        let size = window.viewport_size();
        let rem: f32 = window.rem_size().into();
        PreviewSize {
            width: (f32::from(size.width) - rem * 7.).max(1.),
            height: (f32::from(size.height) - rem * 8.75).max(1.),
        }
    }

    fn zoom(&self, viewport: PreviewSize) -> f32 {
        let natural = self.natural_size;
        let fit = fit_zoom_percent(natural, viewport);
        self.zoom_percent
            .map(|zoom| clamp_zoom_percent(zoom, fit))
            .unwrap_or(fit)
    }

    fn set_zoom(
        &mut self,
        next: f32,
        anchor: Option<Point<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let natural = self.natural_size;
        let viewport = Self::viewport(window);
        let next = clamp_zoom_percent(next, fit_zoom_percent(natural, viewport));
        let bounds = self.scroll.bounds();
        let anchor = anchor
            .map(|p| preview_point_from_pixels(p - bounds.origin))
            .unwrap_or(PreviewPoint {
                x: viewport.width / 2.,
                y: viewport.height / 2.,
            });
        let offset = anchored_scroll_offset(
            preview_point_from_pixels(self.scroll.offset()),
            viewport,
            natural,
            self.zoom(viewport),
            next,
            anchor,
        );
        self.zoom_percent = Some(next);
        self.scroll.set_offset(point(px(offset.x), px(offset.y)));
        cx.notify();
    }

    fn step(&mut self, direction: ZoomStep, window: &mut Window, cx: &mut Context<Self>) {
        let natural = self.natural_size;
        let viewport = Self::viewport(window);
        self.set_zoom(
            next_zoom_step(
                self.zoom(viewport),
                fit_zoom_percent(natural, viewport),
                direction,
            ),
            None,
            window,
            cx,
        );
    }

    fn scroll_zoom(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !(event.modifiers.control || event.modifiers.platform) {
            return;
        }
        let delta: f32 = match event.delta {
            ScrollDelta::Pixels(p) => p.y.into(),
            ScrollDelta::Lines(lines) => lines.y * 20.,
        };
        let factor = if delta > 0. {
            1. + delta.abs() * 0.01
        } else {
            1. / (1. + delta.abs() * 0.01)
        };
        cx.stop_propagation();
        self.set_zoom(
            self.zoom(Self::viewport(window)) * factor,
            Some(event.position),
            window,
            cx,
        );
    }

    fn pinch_zoom(&mut self, event: &PinchEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_zoom(
            self.zoom(Self::viewport(window)) * (1. + event.delta).max(0.01),
            Some(event.position),
            window,
            cx,
        );
    }

    fn render_image(
        &self,
        natural: PreviewSize,
        viewport: PreviewSize,
        zoom: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("image-preview-scroll")
            .test_support()
            .absolute()
            .top(rems(3.25))
            .bottom(rems(5.5))
            .left(rems(3.5))
            .right(rems(3.5))
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .on_scroll_wheel(cx.listener(Self::scroll_zoom))
            .on_pinch(cx.listener(Self::pinch_zoom))
            .child(bitmap(self.image.source(), natural, viewport, zoom))
            .into_any_element()
    }

    fn render_controls(
        &self,
        natural: PreviewSize,
        viewport: PreviewSize,
        zoom: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let fit = fit_zoom_percent(natural, viewport);
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
                                next_zoom_step(zoom, fit, ZoomStep::Out) >= zoom - ZOOM_EPSILON,
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
                                next_zoom_step(zoom, fit, ZoomStep::In) <= zoom + ZOOM_EPSILON,
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = Self::viewport(window);
        let zoom = self.zoom(viewport);
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
            .child(self.render_image(self.natural_size, viewport, zoom, cx))
            .child(self.render_controls(self.natural_size, viewport, zoom, cx))
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

/// Shared bitmap composition. The modal and Files supply their own viewport and scroll owner.
fn bitmap(
    source: ImageSource,
    natural: PreviewSize,
    viewport: PreviewSize,
    zoom: f32,
) -> AnyElement {
    let image_size = image_render_size(natural, zoom);
    let content_size = scroll_content_size(viewport, image_size);
    bitmap_body(source, Some(image_size))
        .w(px(content_size.width))
        .h(px(content_size.height))
        .into_any_element()
}

fn bitmap_body(source: ImageSource, image_size: Option<PreviewSize>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .relative()
        .min_w_full()
        .min_h_full()
        .when(image_size.is_none(), |body| body.size_full())
        .child(
            div()
                .id("image-preview-bitmap")
                .test_support()
                .flex_none()
                .map(|image| {
                    if let Some(size) = image_size {
                        image.w(px(size.width)).h(px(size.height))
                    } else {
                        image.size_full()
                    }
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(img(source).size_full().object_fit(ObjectFit::Contain)),
        )
}

fn natural_size(image: &RenderImage) -> PreviewSize {
    let size = image.size(0);
    PreviewSize {
        width: u32::from(size.width) as f32,
        height: u32::from(size.height) as f32,
    }
}

/// A Files-owned image body; loading and decoding stay in GPUI's public image cache.
pub(super) struct FileImage {
    path: std::path::PathBuf,
    cache: Entity<RetainAllImageCache>,
    scroll: ScrollHandle,
    zoom: Option<f32>,
}
impl FileImage {
    pub(super) fn new(path: std::path::PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            path,
            cache: RetainAllImageCache::new(cx),
            scroll: ScrollHandle::new(),
            zoom: None,
        }
    }
    fn image(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        self.cache.update(cx, |cache, cx| {
            cache.load(&Resource::from(self.path.clone()), window, cx)
        })
    }
    fn viewport(&self) -> PreviewSize {
        let bounds = self.scroll.bounds();
        PreviewSize {
            width: f32::from(bounds.size.width).max(1.),
            height: f32::from(bounds.size.height).max(1.),
        }
    }
    fn set_zoom(
        &mut self,
        natural: PreviewSize,
        requested: Option<f32>,
        anchor: Option<Point<Pixels>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let viewport = self.viewport();
        let fit = fit_zoom_percent(natural, viewport);
        let old = self.zoom.map(|z| clamp_zoom_percent(z, fit)).unwrap_or(fit);
        let next = requested.map(|z| clamp_zoom_percent(z, fit)).unwrap_or(fit);
        let anchor = anchor
            .map(|p| preview_point_from_pixels(p - self.scroll.bounds().origin))
            .unwrap_or(PreviewPoint {
                x: viewport.width / 2.,
                y: viewport.height / 2.,
            });
        let offset = anchored_scroll_offset(
            preview_point_from_pixels(self.scroll.offset()),
            viewport,
            natural,
            old,
            next,
            anchor,
        );
        self.zoom = requested.map(|_| next);
        self.scroll.set_offset(point(px(offset.x), px(offset.y)));
        cx.notify();
    }
    fn step(
        &mut self,
        natural: PreviewSize,
        direction: ZoomStep,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fit = fit_zoom_percent(natural, self.viewport());
        self.set_zoom(
            natural,
            Some(next_zoom_step(self.zoom.unwrap_or(fit), fit, direction)),
            None,
            window,
            cx,
        );
    }
}
impl Render for FileImage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image = self.image(window, cx);
        let natural = image
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .map(|image| natural_size(image));
        let ready = natural.is_some();
        let body = match image {
            Some(Ok(image)) => {
                let natural = natural_size(&image);
                let fit = fit_zoom_percent(natural, self.viewport());
                let image_size = self
                    .zoom
                    .map(|zoom| image_render_size(natural, clamp_zoom_percent(zoom, fit)));
                let child = bitmap_body(ImageSource::Render(image), image_size);
                div()
                    .id("file-image-scroll")
                    .test_support()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_scroll()
                    .track_scroll(&self.scroll)
                    .on_scroll_wheel(cx.listener(
                        move |this, event: &ScrollWheelEvent, window, cx| {
                            if event.modifiers.control || event.modifiers.platform {
                                let delta = event.delta.pixel_delta(px(20.)).y;

                                let fit = fit_zoom_percent(natural, this.viewport());
                                let current = this.zoom.unwrap_or(fit);
                                let factor = (f32::from(delta) * 0.01).exp();
                                this.set_zoom(
                                    natural,
                                    Some(current * factor),
                                    Some(event.position),
                                    window,
                                    cx,
                                );
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .on_pinch(cx.listener(move |this, event: &PinchEvent, window, cx| {
                        let fit = fit_zoom_percent(natural, this.viewport());
                        this.set_zoom(
                            natural,
                            Some(this.zoom.unwrap_or(fit) * (1. + event.delta).max(0.01)),
                            Some(event.position),
                            window,
                            cx,
                        );
                        cx.stop_propagation();
                    }))
                    .child(child)
                    .into_any_element()
            }
            Some(Err(_)) => {
                crate::home::source::unavailable("files-unsupported", self.path.clone(), cx)
                    .into_any_element()
            }
            None => div().p_3().child(t(cx, "files-loading")).into_any_element(),
        };
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
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(natural) = natural {
                                    this.step(natural, ZoomStep::Out, window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("file-image-fit")
                            .small()
                            .ghost()
                            .label(t(cx, "image-preview-fit"))
                            .disabled(!ready)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(natural) = natural {
                                    this.set_zoom(natural, None, None, window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("file-image-original")
                            .small()
                            .ghost()
                            .label("100%")
                            .disabled(!ready)
                            .accessibility_label(t(cx, "image-preview-actual-size"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(natural) = natural {
                                    this.set_zoom(natural, Some(100.), None, window, cx);
                                }
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
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(natural) = natural {
                                    this.step(natural, ZoomStep::In, window, cx);
                                }
                            })),
                    ),
            )
            .child(body)
    }
}

fn preview_point_from_pixels(point: Point<Pixels>) -> PreviewPoint {
    PreviewPoint {
        x: point.x.into(),
        y: point.y.into(),
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

fn scroll_content_size(viewport_size: PreviewSize, image_size: PreviewSize) -> PreviewSize {
    PreviewSize {
        width: image_size.width.max(viewport_size.width),
        height: image_size.height.max(viewport_size.height),
    }
}

fn centered_image_origin(content_size: PreviewSize, image_size: PreviewSize) -> PreviewPoint {
    PreviewPoint {
        x: ((content_size.width - image_size.width) / 2.).max(0.),
        y: ((content_size.height - image_size.height) / 2.).max(0.),
    }
}

fn anchored_scroll_offset(
    old_offset: PreviewPoint,
    viewport_size: PreviewSize,
    natural_size: PreviewSize,
    old_zoom_percent: f32,
    new_zoom_percent: f32,
    anchor: PreviewPoint,
) -> PreviewPoint {
    let old_image_size = image_render_size(natural_size, old_zoom_percent);
    let new_image_size = image_render_size(natural_size, new_zoom_percent);
    let old_content_size = scroll_content_size(viewport_size, old_image_size);
    let new_content_size = scroll_content_size(viewport_size, new_image_size);
    let old_image_origin = centered_image_origin(old_content_size, old_image_size);
    let new_image_origin = centered_image_origin(new_content_size, new_image_size);

    let content_point = PreviewPoint {
        x: anchor.x - old_offset.x,
        y: anchor.y - old_offset.y,
    };
    let image_unit = PreviewPoint {
        x: (content_point.x - old_image_origin.x) / old_image_size.width,
        y: (content_point.y - old_image_origin.y) / old_image_size.height,
    };
    let next_content_point = PreviewPoint {
        x: new_image_origin.x + image_unit.x * new_image_size.width,
        y: new_image_origin.y + image_unit.y * new_image_size.height,
    };

    clamp_scroll_offset(
        PreviewPoint {
            x: anchor.x - next_content_point.x,
            y: anchor.y - next_content_point.y,
        },
        viewport_size,
        new_content_size,
    )
}

fn clamp_scroll_offset(
    offset: PreviewPoint,
    viewport_size: PreviewSize,
    content_size: PreviewSize,
) -> PreviewPoint {
    let min_x = (viewport_size.width - content_size.width).min(0.);
    let min_y = (viewport_size.height - content_size.height).min(0.);
    PreviewPoint {
        x: offset.x.clamp(min_x, 0.),
        y: offset.y.clamp(min_y, 0.),
    }
}

fn format_zoom_percent(zoom_percent: f32) -> String {
    format!("{:.0}", zoom_percent.round())
}

#[cfg(test)]
mod tests {
    use super::PreviewPoint;
    use super::PreviewSize;
    use super::ZoomStep;
    use super::anchored_scroll_offset;
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

    #[test]
    fn anchored_scroll_offset_keeps_focus_point_stable() {
        let old_offset = PreviewPoint { x: -100., y: -50. };
        let viewport_size = PreviewSize {
            width: 1000.,
            height: 800.,
        };
        let natural_size = PreviewSize {
            width: 2000.,
            height: 1600.,
        };
        let anchor = PreviewPoint { x: 500., y: 400. };

        let next =
            anchored_scroll_offset(old_offset, viewport_size, natural_size, 50., 100., anchor);

        assert_eq!(next, PreviewPoint { x: -700., y: -500. });
        let restored = anchored_scroll_offset(next, viewport_size, natural_size, 100., 50., anchor);
        assert_eq!(restored, PreviewPoint { x: 0., y: 0. });
    }
}
