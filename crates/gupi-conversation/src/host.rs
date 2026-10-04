//! Application lifetime integrations; conversation state owns no native windows.
use crate::conversation::ConversationState;
use gpui_kit::{App, Entity, Global};
pub struct Host {
    attach: fn(&Entity<ConversationState>, &mut App),
    cancel_preparation: fn(&str, &mut App),
}
impl Global for Host {}
impl Host {
    pub fn new(
        attach: fn(&Entity<ConversationState>, &mut App),
        cancel_preparation: fn(&str, &mut App),
    ) -> Self {
        Self {
            attach,
            cancel_preparation,
        }
    }
}
pub(crate) fn attach(owner: &Entity<ConversationState>, cx: &mut App) {
    (cx.global::<Host>().attach)(owner, cx);
}
pub(crate) fn cancel_preparation(key: &str, cx: &mut App) {
    (cx.global::<Host>().cancel_preparation)(key, cx);
}

#[cfg(any(test, feature = "test-support"))]
pub fn install_headless(cx: &mut App) {
    cx.set_global(Host {
        attach: |_, _| {},
        cancel_preparation: |_, _| {},
    });
}
