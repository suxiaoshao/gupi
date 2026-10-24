//! A single hidden temporary conversation; never a second process manager.
use super::*;

#[derive(Default)]
pub(super) struct Prewarm {
    enabled: bool,
    requested: bool,
    pub slot: Option<String>,
}

impl Session {
    pub fn is_internal(&self) -> bool {
        self.internal
    }
    fn standby_ready(&self) -> bool {
        self.state.is_some()
            && matches!(self.transcript, Transcript::Ready(_))
            && !self.core_read.running()
            && !self.settings_busy()
            && !self.model_change.unconfirmed()
            && self.pending_ui.is_empty()
            && self.empty_conversation()
            && self.draft.text().is_empty()
            && self.error.is_none()
            && self.core_read.error().is_none()
    }
}
impl ConversationState {
    pub fn set_prewarm(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let enabled = enabled && self.temporary && !self.draining;
        if self.prewarm.enabled != enabled {
            self.prewarm.requested = enabled;
        }
        self.prewarm.enabled = enabled;
        if !enabled {
            self.discard_standby(cx);
        }
        self.start_prewarm(cx);
    }
    pub(super) fn discard_standby(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.prewarm.slot.take() {
            self.remove_temporary(&key, true, cx);
        }
    }
    pub(super) fn start_prewarm(&mut self, cx: &mut Context<Self>) {
        if self.draining
            || !self.prewarm.enabled
            || !self.prewarm.requested
            || self.prewarm.slot.is_some()
            || self.sessions.values().any(|session| session.internal)
        {
            return;
        }
        self.prewarm.requested = false;
        let Ok(key) = self.create_draft(None) else {
            tracing::warn!("Could not create standby conversation");
            return;
        };
        self.sessions.get_mut(&key).unwrap().internal = true;
        self.prewarm.slot = Some(key.clone());
        self.connect(&key, cx);
    }
    pub(super) fn maintain_prewarm(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(key) else {
            return;
        };
        if session.internal {
            if session.error.is_some()
                || session.core_read.error().is_some()
                || !session.pending_ui.is_empty()
                || !session.empty_conversation()
                || !session.draft.text().is_empty()
            {
                tracing::warn!("Discarding unusable standby conversation");
                self.discard_standby(cx);
            }
        } else if session.replenish
            && !session.preparing
            && session.state.is_some()
            && matches!(session.transcript, Transcript::Ready(_))
            && !session.core_read.running()
            && session.error.is_none()
            && session.core_read.error().is_none()
        {
            self.sessions.get_mut(key).unwrap().replenish = false;
            self.prewarm.requested = self.prewarm.enabled;
            self.start_prewarm(cx);
        }
    }
    /// Readiness and ownership are checked in the same update that adopts.
    pub fn new_temporary(
        &mut self,
        preparing: bool,
        cx: &mut Context<Self>,
    ) -> Result<String, String> {
        if !self.temporary || self.draining {
            return Err("Temporary conversations are unavailable".into());
        }
        let candidate = self
            .prewarm
            .slot
            .as_ref()
            .filter(|key| {
                self.prewarm.enabled
                    && self.sessions.get(*key).is_some_and(Session::standby_ready)
                    && self
                        .client(key, cx)
                        .is_some_and(|client| matches!(client.state(), ConnectionState::Ready(_)))
            })
            .cloned();
        if let Some(key) = candidate {
            tracing::info!("Adopted prewarmed temporary conversation");
            self.prewarm.slot = None;
            self.activate_temporary(&key, preparing, cx);
            Ok(key)
        } else {
            self.cold_temporary(preparing, cx)
        }
    }
    fn cold_temporary(
        &mut self,
        preparing: bool,
        cx: &mut Context<Self>,
    ) -> Result<String, String> {
        let key = self.create_draft(None)?;
        self.activate_temporary(&key, preparing, cx);
        self.connect(&key, cx);
        Ok(key)
    }
    fn activate_temporary(&mut self, key: &str, preparing: bool, cx: &mut Context<Self>) {
        let session = self.sessions.get_mut(key).unwrap();
        session.internal = false;
        session.replenish = true;
        session.preparing = preparing;
        self.select_existing(key, cx);
        notify_session(key, cx);
        self.maintain_prewarm(key, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversationState, Transcript};
    use gpui_kit::{AppContext as _, TestAppContext};
    use std::prelude::v1::test;
    #[gpui_kit::test]
    fn nonselected_creation_and_adoption_preserve_loaded_conversation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            gupi_pi_runtime::init(cx);
        });
        let owner = cx.new(|cx| ConversationState::temporary("unused-pi".into(), cx));
        owner.update(cx, |state, cx| {
            let key = state.create_draft(None).unwrap();
            assert!(state.selected.is_none(), "background creation never changes foreground selection");
            let session = state.sessions.get_mut(&key).unwrap();
            session.internal = true;
            session.state = Some(serde_json::from_value(serde_json::json!({"sessionId":"ready", "isStreaming":false, "isCompacting":false})).unwrap());
            session.transcript.replace(serde_json::from_value(serde_json::json!({"entries":[], "leafId":null})).unwrap());
            assert!(state.infos().is_empty());
            state.select_existing(&key, cx);
            assert!(state.selected.is_none(), "internal keys cannot be navigated to");
            state.activate_temporary(&key, true, cx);
            assert_eq!(state.selected.as_deref(), Some(key.as_str()));
            let session = &state.sessions[&key];
            assert!(!session.internal);
            assert!(session.preparing);
            assert_eq!(session.state.as_ref().unwrap().session_id, "ready");
            assert!(matches!(session.transcript, Transcript::Ready(_)));
            assert_eq!(state.infos().len(), 1);
        });
    }
}
