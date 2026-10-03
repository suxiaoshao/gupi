//! Application lifetime integrations; conversation state owns no native windows.
use crate::conversation::ConversationState;
use gpui_kit::{App, Entity, Global};
pub struct Host {
    pub attach: fn(&Entity<ConversationState>, &mut App),
    pub cancel_preparation: fn(&str, &mut App),
}
impl Global for Host {}
pub fn attach(owner: &Entity<ConversationState>, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.attach)(owner, cx);
    }
}
pub fn cancel_preparation(key: &str, cx: &mut App) {
    if let Some(host) = cx.try_global::<Host>() {
        (host.cancel_preparation)(key, cx);
    }
}
