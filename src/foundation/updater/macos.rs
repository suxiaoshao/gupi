use super::Event;
use std::{ffi::CString, sync::OnceLock};

static EVENTS: OnceLock<smol::channel::Sender<Event>> = OnceLock::new();
unsafe extern "C" {
    fn gupi_updater_init(callback: extern "C" fn(i32)) -> i32;
    fn gupi_updater_install(feed: *const std::ffi::c_char) -> i32;
    fn gupi_updater_resume();
    fn gupi_updater_show();
}
extern "C" fn event(value: i32) {
    if let Some(sender) = EVENTS.get() {
        let _ = sender.try_send(match value {
            1 => Event::Ready,
            2 => Event::Finished,
            4 => Event::Skipped,
            _ => Event::Failed,
        });
    }
}
pub(crate) struct Driver;
impl Driver {
    pub fn new(sender: smol::channel::Sender<Event>) -> Result<Self, String> {
        if !cfg!(feature = "bundled") {
            return Err("source build".into());
        }
        let _ = EVENTS.set(sender);
        // All bridge calls occur on the application main thread. Native callbacks
        // only queue events; they never re-enter GPUI while it is borrowed.
        if unsafe { gupi_updater_init(event) } == 0 {
            return Err("Sparkle is not configured in this bundle".into());
        }
        Ok(Self)
    }
    pub fn install(&self, feed: &str) -> Result<(), String> {
        let feed = CString::new(feed).map_err(|e| e.to_string())?;
        if unsafe { gupi_updater_install(feed.as_ptr()) } == 0 {
            return Err("Sparkle cannot start an update now".into());
        }
        Ok(())
    }
    pub fn resume(&self) {
        unsafe { gupi_updater_resume() };
    }
    pub fn show(&self) {
        unsafe { gupi_updater_show() };
    }
    pub fn feed_name(&self) -> String {
        format!("appcast-macos-{}.xml", std::env::consts::ARCH)
    }
}
