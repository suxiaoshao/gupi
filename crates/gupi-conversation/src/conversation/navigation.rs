use super::*;

impl Session {
    pub fn can_navigate(&self) -> bool {
        self.state.is_some()
            && !self.settings_busy()
            && !self.model_change.unconfirmed()
            && self.commands.data().is_some_and(|commands| {
                commands.iter().any(|command| {
                    command.name == gupi_pi_runtime::TREE_COMMAND && command.source == "extension"
                })
            })
    }
}

impl ConversationState {
    pub fn navigate(&mut self, key: &str, entry: String, cx: &mut Context<Self>) {
        if self.temporary || self.draining {
            return;
        }
        let Some(session) = self.sessions.get(key) else {
            return;
        };
        if !session.can_navigate() || session.history().entry(&entry).is_none() {
            return;
        }
        let Some(client) = self.client(key, cx) else {
            return;
        };
        let session = self.sessions.get_mut(key).unwrap();
        session.reset_reads();
        session.binding += 1;
        let binding = session.binding;
        let target = key.to_owned();
        let task_key = target.clone();
        let task = cx.spawn(async move |owner, cx| {
            let result = client
                .prompt(protocol::Prompt::new(format!(
                    "/{} {}",
                    gupi_pi_runtime::TREE_COMMAND,
                    entry
                )))
                .await;
            let _ = owner.update(cx, |this, cx| {
                let Some(session) = this
                    .sessions
                    .get_mut(&task_key)
                    .filter(|session| session.binding == binding)
                else {
                    return;
                };
                session.command.finish();
                if let Err(error) = result {
                    session.error = Some(SessionError::Runtime(error.to_string()));
                }
                // Read Pi's actual leaf even when another extension cancelled navigation.
                this.refresh(&task_key, cx);
                notify_session(&task_key, cx);
            });
        });
        self.sessions.get_mut(&target).unwrap().command =
            SessionCommand::Navigating { _task: task };
        notify_session(&target, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{ReadState, RunState, Session};

    #[test]
    fn navigation_requires_registered_command_and_an_idle_session() {
        let mut session = Session::from_rpc_messages(&[]);
        session.state = Some(
            serde_json::from_value(serde_json::json!({
                "sessionId": "test", "isStreaming": false, "isCompacting": false
            }))
            .unwrap(),
        );
        assert!(!session.can_navigate());
        session.commands = ReadState::Ready(vec![
            serde_json::from_value(serde_json::json!({
                "name": gupi_pi_runtime::TREE_COMMAND, "source": "extension", "sourceInfo": {}
            }))
            .unwrap(),
        ]);
        assert!(!session.can_navigate());
        session.run = RunState::Idle;
        assert!(session.can_navigate());
        session.pending_count = 1;
        assert!(!session.can_navigate());
        session.pending_count = 0;
        session.compacting = true;
        assert!(!session.can_navigate());
        session.compacting = false;
        session.commands = ReadState::Ready(Vec::new());
        assert!(!session.can_navigate());
    }
}
