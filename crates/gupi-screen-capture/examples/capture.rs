//! Native smoke test: a changing clock behind the production frozen selector.
//! Does not create conversations, read user configuration, or send images.
use gpui_kit::component::{ActiveTheme, button::Button};
use gpui_kit::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

struct CaptureDemo {
    started: Instant,
    status: String,
    result: Option<Arc<Image>>,
    task: Option<Task<()>>,
    _clock: Task<()>,
}
impl CaptureDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let clock = cx.spawn(async |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        Self {
            started: Instant::now(),
            status: "Drag a region containing the clock; Escape cancels.".into(),
            result: None,
            task: None,
            _clock: clock,
        }
    }
    fn capture(&mut self, cx: &mut Context<Self>) {
        match gupi_screen_capture::start(cx) {
            Ok(task) => {
                self.task = Some(cx.spawn(async move |this, cx| {
                    let result = task.await;
                    let _ = this.update(cx, |this, cx| {
                        match result {
                            Ok(Some(bytes)) => {
                                this.result =
                                    Some(Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                                this.status =
                                    "Captured: compare the frozen clock below with the live clock."
                                        .into();
                            }
                            Ok(None) => this.status = "Cancelled; no image sent.".into(),
                            Err(error) => this.status = error.to_string(),
                        }
                        this.task = None;
                        cx.notify();
                    });
                }))
            }
            Err(error) => {
                self.status = error.to_string();
                cx.notify();
            }
        }
    }
}
impl Render for CaptureDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(format!(
                "Live clock: {:.1} s",
                self.started.elapsed().as_secs_f32()
            ))
            .child(
                Button::new("capture")
                    .label("Capture screen area…")
                    .on_click(cx.listener(|this, _, _, cx| this.capture(cx))),
            )
            .child(self.status.clone())
            .children(self.result.clone().map(|image| {
                img(image)
                    .max_w_full()
                    .max_h_96()
                    .object_fit(ObjectFit::Contain)
            }))
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(CaptureDemo::new)
        })
        .unwrap();
        cx.activate(true);
    });
}
