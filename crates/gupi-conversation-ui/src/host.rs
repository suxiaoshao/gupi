//! Native window and notification presentation is supplied by the application.
use gpui_kit::{App, Entity, Global, Window};
use gupi_conversation::conversation::ConversationState;
pub struct Host {
    paste_answer: fn(String, &mut Window, &mut App),
    hide: fn(&mut Window, &mut App),
    present: fn(&Entity<ConversationState>, &Window, bool, &mut App),
}
impl Global for Host {}
impl Host {
    pub fn new(
        paste_answer: fn(String, &mut Window, &mut App),
        hide: fn(&mut Window, &mut App),
        present: fn(&Entity<ConversationState>, &Window, bool, &mut App),
    ) -> Self {
        Self {
            paste_answer,
            hide,
            present,
        }
    }
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
        paste_answer: |_, _, _| {},
        hide: |_, _| {},
        present: |_, _, _, _| {},
    });
    gupi_conversation::host::install_headless(cx);
    cx.set_global(gupi_settings::host::Host::headless());
}
