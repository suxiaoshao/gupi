//! Native window and notification presentation is supplied by the application.
use gpui_kit::{App, Entity, Global, Window};
use gupi_conversation::conversation::ConversationState;
pub struct Host {
    pub paste_answer: fn(String, &mut Window, &mut App),
    pub hide: fn(&mut Window, &mut App),
    pub present: fn(&Entity<ConversationState>, &Window, bool, &mut App),
}
impl Global for Host {}
pub fn paste_answer(text: String, window: &mut Window, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.paste_answer)(text, window, cx);
    }
}
pub fn hide(window: &mut Window, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.hide)(window, cx);
    }
}
pub fn present(owner: &Entity<ConversationState>, window: &Window, visible: bool, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.present)(owner, window, visible, cx);
    }
}
