//! Native window and notification presentation is supplied by the application.
use gpui_kit::{App, Entity, Global, Task, Window};
use gupi_conversation::conversation::ConversationState;
use std::{io, path::PathBuf};
pub struct Host {
    open_file: fn(PathBuf, &mut App) -> Task<io::Result<()>>,
    paste_answer: fn(String, &mut Window, &mut App),
    hide: fn(&mut Window, &mut App),
    present: fn(&Entity<ConversationState>, &Window, bool, &mut App),
}
impl Global for Host {}
impl Host {
    pub fn new(
        open_file: fn(PathBuf, &mut App) -> Task<io::Result<()>>,
        paste_answer: fn(String, &mut Window, &mut App),
        hide: fn(&mut Window, &mut App),
        present: fn(&Entity<ConversationState>, &Window, bool, &mut App),
    ) -> Self {
        Self {
            open_file,
            paste_answer,
            hide,
            present,
        }
    }
}
pub(crate) fn open_file(path: PathBuf, cx: &mut App) -> Task<io::Result<()>> {
    (cx.global::<Host>().open_file)(path, cx)
}
pub(crate) fn paste_answer(text: String, window: &mut Window, cx: &mut App) {
    (cx.global::<Host>().paste_answer)(text, window, cx);
}
pub(crate) fn hide(window: &mut Window, cx: &mut App) {
    (cx.global::<Host>().hide)(window, cx);
}
pub(crate) fn present(
    owner: &Entity<ConversationState>,
    window: &Window,
    visible: bool,
    cx: &mut App,
) {
    (cx.global::<Host>().present)(owner, window, visible, cx);
}

#[cfg(test)]
pub fn install_headless(cx: &mut App) {
    cx.set_global(Host {
        open_file: |_, _| Task::ready(Err(io::Error::other("headless opener"))),
        paste_answer: |_, _, _| {},
        hide: |_, _| {},
        present: |_, _, _, _| {},
    });
    gupi_conversation::host::install_headless(cx);
    cx.set_global(gupi_settings::host::Host::headless());
}
