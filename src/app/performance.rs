//! Opt-in development tracing. The guard in main saves the complete process trace.
use super::logging::LogWriter;
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
