//! Opt-in development tracing. The guard in main saves the complete process trace.
use super::logging::LogWriter;
use gpui_kit::*;
use std::{
    cell::RefCell,
    fs::OpenOptions,
    io::{self, Write},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing_subscriber::{filter::filter_fn, prelude::*};

static RECORDING: AtomicBool = AtomicBool::new(false);
static LOG_WRITER: Mutex<Option<LogWriter>> = Mutex::new(None);
thread_local! {
    static GUARD: RefCell<Option<tracing_chrome::FlushGuard>> = const { RefCell::new(None) };
}

pub(crate) struct Recording;
impl Drop for Recording {
    fn drop(&mut self) {
        finish();
    }
}

fn finish() {
    RECORDING.store(false, Ordering::Relaxed);
    GUARD.with(|guard| drop(guard.borrow_mut().take()));
}

pub(crate) fn install_quit_hook(cx: &gpui_kit::App) {
    // AppKit termination may exit without returning through main's stack.
    cx.on_app_quit(|_| {
        finish();
        async {}
    })
    .detach();
}

pub(crate) fn is_recording() -> bool {
    RECORDING.load(Ordering::Relaxed)
}

pub(crate) fn set_log_writer(writer: LogWriter) {
    *LOG_WRITER.lock().expect("diagnostic writer lock") = Some(writer);
}

struct Diagnostics;
impl Write for Diagnostics {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match LOG_WRITER.lock().expect("diagnostic writer lock").as_mut() {
            Some(writer) => writer.write(bytes),
            None => io::stderr().write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match LOG_WRITER.lock().expect("diagnostic writer lock").as_mut() {
            Some(writer) => writer.flush(),
            None => io::stderr().flush(),
        }
    }
}

fn trace_metadata(metadata: &tracing::Metadata<'_>) -> bool {
    metadata.target() == "gupi::performance"
        || (metadata.is_span()
            && (metadata.target().starts_with("gpui") || metadata.name() == "app.initialize"))
        || (metadata.target().starts_with("gpui")
            && metadata.fields().field("tracy.frame_mark").is_some())
}

pub(crate) fn diagnostic_metadata(metadata: &tracing::Metadata<'_>) -> bool {
    metadata.level() <= &tracing::Level::INFO
        && metadata.target() != "gupi::performance"
        && metadata.fields().field("tracy.frame_mark").is_none()
}

pub(crate) fn start() -> Option<Recording> {
    let path = std::env::var_os("GUPI_PERFORMANCE_TRACE")?;
    let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Gupi performance trace could not be created: {error}");
            return None;
        }
    };
    let (chrome, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .writer(file)
        .include_args(true)
        .include_locations(true)
        .build();
    let subscriber = tracing_subscriber::registry()
        .with(chrome.with_filter(filter_fn(trace_metadata)))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(|| Diagnostics)
                .with_filter(filter_fn(diagnostic_metadata)),
        );
    if let Err(error) = subscriber.try_init() {
        eprintln!("Gupi performance trace could not be started: {error}");
        return None;
    }
    RECORDING.store(true, Ordering::Relaxed);
    GUARD.with(|slot| *slot.borrow_mut() = Some(guard));
    eprintln!(
        "Gupi performance trace: {}",
        std::path::Path::new(&path).display()
    );
    Some(Recording)
}

/// Delegate without adding layout nodes, identity, invalidation or frame requests.
pub(crate) fn measure(
    region: &'static str,
    shimmer: bool,
    element: impl IntoElement,
) -> AnyElement {
    let element = element.into_any_element();
    if !is_recording() {
        return element;
    }
    DrawProbe {
        region,
        shimmer,
        element,
    }
    .into_any_element()
}

struct DrawProbe {
    region: &'static str,
    shimmer: bool,
    element: AnyElement,
}

impl IntoElement for DrawProbe {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for DrawProbe {
    type RequestLayoutState = ();
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.request_layout",
            region = self.region, shimmer = self.shimmer, reduce_motion = cx.reduce_motion(),
            view = ?window.current_view())
        .entered();
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        // Geometric visibility only: this does not detect occluding overlays.
        let visible = !bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&Bounds::new(Point::default(), window.viewport_size()))
            .is_empty();
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.prepaint",
            region = self.region, shimmer = self.shimmer, visible,
            view = ?window.current_view())
        .entered();
        self.element.prepaint(window, cx);
        visible
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        visible: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let _span = tracing::debug_span!(target: "gupi::performance", "ui.paint",
            region = self.region, shimmer = self.shimmer, visible = *visible,
            view = ?window.current_view())
        .entered();
        self.element.paint(window, cx);
    }
}
