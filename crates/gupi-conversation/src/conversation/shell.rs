//! Direct Shell presentation belongs to its Pi connection, independently of agent runs.
use super::*;

const LIVE_OUTPUT_BYTES: usize = 64 * 1024;

fn append_tail(output: &mut String, delta: &str) -> bool {
    output.push_str(delta);
    if output.len() <= LIVE_OUTPUT_BYTES {
        return false;
    }
    let mut start = output.len() - LIVE_OUTPUT_BYTES;
    while !output.is_char_boundary(start) {
        start += 1;
    }
    output.drain(..start);
    true
}

struct ActiveShell {
    id: String,
    message: DisplayMessage,
    _task: Task<()>,
    stop_task: Option<Task<()>>,
}

impl ActiveShell {
    fn new(id: String, binding: u64, command: String, exclude: bool, task: Task<()>) -> Self {
        Self {
            message: DisplayMessage::new(
                format!("shell-live-{binding}-{id}"),
                serde_json::json!({
                    "role":"bashExecution", "command":command, "excludeFromContext":exclude,
                    "output":"", "shellRunning":true
                }),
            ),
            id,
            _task: task,
            stop_task: None,
        }
    }
}

#[derive(Default)]
pub(super) struct Shell {
    active: Option<ActiveShell>,
    completed: Vec<DisplayMessage>,
}
impl Shell {
    pub(super) fn messages(&self) -> impl Iterator<Item = &DisplayMessage> {
        self.completed
            .iter()
            .chain(self.active.iter().map(|active| &active.message))
    }
    pub(super) fn completed_count(&self) -> usize {
        self.completed.len()
    }
    pub(super) fn defer_history(&self, captured: usize, streaming: bool) -> bool {
        self.completed.len() > captured
            || streaming
                && self
                    .completed
                    .iter()
                    .any(|message| !message.value["shellError"].is_string())
    }
    /// Pi flushes deferred bash messages before its fresh state becomes idle.
    /// Only results present when this read began are covered by its entries.
    pub(super) fn refreshed(&mut self, count: usize, streaming: bool) -> bool {
        if streaming || count == 0 {
            return false;
        }
        let mut index = 0;
        let before = self.completed.len();
        self.completed.retain(|message| {
            index += 1;
            index > count || message.value["shellError"].is_string()
        });
        self.completed.len() != before
    }
    fn finish(
        &mut self,
        id: &str,
        result: Result<protocol::BashResult, pi_rpc::Error>,
        error: &mut Option<SessionError>,
    ) -> bool {
        if self.active.as_ref().is_none_or(|active| active.id != id) {
            return false;
        }
        let mut message = self.active.take().unwrap().message;
        message.value["shellRunning"] = false.into();
        message.value["shellStopping"] = false.into();
        match result {
            Ok(result) => {
                message.value["output"] = result.output.into();
                message.value["shellDisplayTruncated"] = false.into();
                message.value["exitCode"] = result.exit_code.into();
                message.value["cancelled"] = result.cancelled.into();
                message.value["truncated"] = result.truncated.into();
                message.value["fullOutputPath"] = result.full_output_path.into();
            }
            Err(failure) => {
                message.value["shellError"] = failure.to_string().into();
                *error = Some(SessionError::Runtime(failure.to_string()));
            }
        }
        self.completed.push(message);
        true
    }
    pub(super) fn update(&mut self, id: &str, delta: &str) -> bool {
        let Some(active) = self.active.as_mut().filter(|active| active.id == id) else {
            return false;
        };
        if let Value::String(output) = &mut active.message.value["output"]
            && append_tail(output, delta)
        {
            active.message.value["shellDisplayTruncated"] = true.into();
        }
        true
    }
}
impl Session {
    pub(super) fn disconnect_shell(&mut self) {
        let Some(id) = self.shell.active.as_ref().map(|active| active.id.clone()) else {
            return;
        };
        let error = self
            .runtime_error()
            .unwrap_or("Pi connection closed")
            .to_owned();
        let mut ignored = None;
        self.shell
            .finish(&id, Err(pi_rpc::Error::Closed), &mut ignored);
        self.shell.completed.last_mut().unwrap().value["shellError"] = error.into();
        self.content_revision += 1;
    }
    pub fn shell_running(&self) -> bool {
        self.shell.active.is_some()
    }
    pub fn shell_stopping(&self) -> bool {
        self.shell.active.as_ref().is_some_and(|active| {
            active.message.value["shellStopping"] == true || active.stop_task.is_some()
        })
    }
}
impl ConversationState {
    pub fn can_run_shell(&self, key: &str, cx: &App) -> bool {
        self.can_submit(key, cx) && self.sessions.get(key).is_some_and(|s| !s.shell_running())
    }
    /// Executes command text directly; callers own input UI and never expand it as a prompt.
    pub fn run_shell(
        &mut self,
        key: &str,
        command: String,
        exclude_from_context: bool,
        cx: &mut Context<Self>,
    ) {
        if command.trim().is_empty() || !self.can_run_shell(key, cx) {
            return;
        }
        let Some(client) = self.client(key, cx) else {
            return;
        };
        let request = match client.bash(command.clone(), exclude_from_context) {
            Ok(request) => request,
            Err(error) => {
                self.sessions.get_mut(key).unwrap().error =
                    Some(SessionError::Runtime(error.to_string()));
                notify_session(key, cx);
                return;
            }
        };
        let id = request.id().to_owned();
        let binding = self.sessions[key].binding;
        let target = key.to_owned();
        let request_id = id.clone();
        let task = cx.spawn(async move |owner, cx| {
            let result = request.result().await;
            let _ = owner.update(cx, |this, cx| {
                let Some(s) = this
                    .sessions
                    .get_mut(&target)
                    .filter(|s| s.binding == binding)
                else {
                    return;
                };
                if !s.shell.finish(&request_id, result, &mut s.error) {
                    return;
                }
                s.content_revision += 1;
                notify_session(&target, cx);
                this.read_session(&target, ReadScope::History, cx);
            });
        });
        let s = self.sessions.get_mut(key).unwrap();
        s.error = None;
        s.transcript.receive_message();
        s.shell.active = Some(ActiveShell::new(
            id,
            binding,
            command,
            exclude_from_context,
            task,
        ));
        s.content_revision += 1;
        notify_session(key, cx);
    }
    pub fn stop_shell(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(client) = self.client(key, cx) else {
            return;
        };
        let Some(s) = self.sessions.get_mut(key) else {
            return;
        };
        let Some(active) = s
            .shell
            .active
            .as_mut()
            .filter(|a| a.message.value["shellStopping"] != true && a.stop_task.is_none())
        else {
            return;
        };
        let id = active.id.clone();
        let binding = s.binding;
        let target = key.to_owned();
        active.stop_task = Some(cx.spawn(async move |owner, cx| {
            let result = client.abort_bash().await;
            let _ = owner.update(cx, |this, cx| {
                let Some(s) = this
                    .sessions
                    .get_mut(&target)
                    .filter(|s| s.binding == binding)
                else {
                    return;
                };
                let Some(active) = s.shell.active.as_mut().filter(|a| a.id == id) else {
                    return;
                };
                active.stop_task = None;
                match result {
                    Ok(_) => {
                        active.message.value["shellStopping"] = true.into();
                        s.content_revision += 1;
                    }
                    Err(error) => s.error = Some(SessionError::Runtime(error.to_string())),
                }
                notify_session(&target, cx);
            });
        }));
        notify_controls(key, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveShell, Shell};
    use gpui_kit::Task;
    use serde_json::json;

    fn active(id: &str) -> ActiveShell {
        ActiveShell::new(
            id.into(),
            0,
            "printf repeated".into(),
            false,
            Task::ready(()),
        )
    }
    fn result(cancelled: bool) -> pi_rpc::protocol::BashResult {
        serde_json::from_value(
            json!({"output":"final output", "cancelled":cancelled,"truncated":false}),
        )
        .unwrap()
    }
    #[test]
    fn shell_stream_identity_stop_result_and_captured_history_prefix() {
        let mut shell = Shell {
            active: Some(active("1")),
            ..Default::default()
        };
        assert!(!shell.update("other", "wrong session"));
        assert!(shell.update("1", "partial\n"));
        assert_eq!(
            shell.active.as_ref().unwrap().message.value["output"],
            "partial\n"
        );
        let mut error = None;
        assert!(!shell.finish("old", Ok(result(false)), &mut error));
        assert!(shell.active.is_some());
        // An abort acknowledgement does not finish execution.
        shell.active.as_mut().unwrap().message.value["shellStopping"] = true.into();
        assert!(shell.finish("1", Ok(result(true)), &mut error));
        assert!(!shell.update("1", "late"));
        assert!(shell.completed[0].entry.is_none());
        assert!(shell.completed[0].value["exitCode"].is_null());
        assert_eq!(shell.completed[0].value["cancelled"], true);
        let captured = shell.completed_count();
        assert!(shell.defer_history(captured, true));
        assert!(!shell.refreshed(captured, true));
        shell.active = Some(active("2"));
        assert!(shell.finish("2", Ok(result(false)), &mut error));
        assert_eq!(shell.completed.len(), 2);
        assert!(shell.defer_history(captured, false));
        assert!(!shell.defer_history(shell.completed_count(), false));
        assert!(shell.refreshed(captured, false));
        assert_eq!(shell.completed.len(), 1);
        assert_eq!(shell.completed[0].id, "shell-live-0-2");
        assert!(shell.refreshed(shell.completed_count(), false));
        assert!(shell.completed.is_empty());
    }
    #[test]
    fn connection_error_preserves_output_without_inventing_cancellation() {
        let mut shell = Shell {
            active: Some(active("1")),
            ..Default::default()
        };
        shell.update("1", "received before disconnect");
        let mut error = None;
        assert!(shell.finish("1", Err(pi_rpc::Error::Closed), &mut error));
        assert!(error.is_some());
        let message = &shell.completed[0];
        assert_eq!(message.value["output"], "received before disconnect");
        assert!(message.value["cancelled"].is_null());
        assert!(!shell.refreshed(1, false));
    }
    #[test]
    fn disconnected_binding_invalidates_running_shell_before_its_late_result() {
        let mut session = super::Session::from_rpc_messages(&[]);
        session.run = super::RunState::Idle;
        session.shell.active = Some(active("1"));
        session.shell.update("1", "partial output");
        session.error = Some(super::SessionError::Runtime("lost connection".into()));
        session.disconnect_shell();
        session.binding += 1;
        assert!(!session.shell_running());
        assert!(!session.settings_busy());
        assert_eq!(session.shell.completed[0].value["output"], "partial output");
        assert_eq!(
            session.shell.completed[0].value["shellError"],
            "lost connection"
        );
        let mut ignored = None;
        assert!(!session.shell.finish("1", Ok(result(false)), &mut ignored));
        assert_eq!(session.shell.completed.len(), 1);
    }
    #[test]
    fn live_shell_output_keeps_a_bounded_utf8_tail_and_restores_final_pi_output() {
        let mut shell = Shell {
            active: Some(active("1")),
            ..Default::default()
        };
        for _ in 0..200 {
            shell.update("1", &"界".repeat(1000));
        }
        shell.update("1", "latest output");
        let message = &shell.active.as_ref().unwrap().message;
        assert!(message.value["output"].as_str().unwrap().len() <= super::LIVE_OUTPUT_BYTES);
        assert!(
            message.value["output"]
                .as_str()
                .unwrap()
                .ends_with("latest output")
        );
        assert_eq!(message.value["shellDisplayTruncated"], true);
        let mut error = None;
        shell.finish("1", Ok(result(false)), &mut error);
        assert_eq!(shell.completed[0].value["output"], "final output");
        assert_eq!(shell.completed[0].value["shellDisplayTruncated"], false);
    }
    #[test]
    fn reconnect_keeps_error_output_and_namespaces_reused_request_ids() {
        let mut shell = Shell {
            active: Some(active("1")),
            ..Default::default()
        };
        shell.update("1", "old connection output");
        let mut error = None;
        shell.finish("1", Err(pi_rpc::Error::Closed), &mut error);
        let old_id = shell.completed[0].id.clone();
        shell.active = Some(ActiveShell::new(
            "1".into(),
            1,
            "printf again".into(),
            false,
            Task::ready(()),
        ));
        assert_ne!(shell.active.as_ref().unwrap().message.id, old_id);
        assert!(shell.update("1", "new connection output"));
        assert_eq!(shell.completed[0].value["output"], "old connection output");
        assert_eq!(
            shell.active.as_ref().unwrap().message.value["output"],
            "new connection output"
        );
        assert_eq!(
            shell
                .messages()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            ["shell-live-0-1", "shell-live-1-1"]
        );
    }

    #[test]
    fn current_path_previews_keep_shell_rows() {
        let mut session = crate::conversation::Session::new(
            crate::conversation::SessionInfo {
                path: std::path::PathBuf::new(),
                id: "preview".into(),
                cwd: std::path::PathBuf::new(),
                name: None,
                first_message: String::new(),
                activity: String::new(),
                parent_session: None,
            },
            String::new(),
        );
        session.transcript.replace(serde_json::from_value(json!({
            "leafId":"answer", "entries":[
                {"id":"user", "timestamp":"1", "type":"message", "message":{"role":"user", "content":"question"}},
                {"id":"answer", "timestamp":"2", "parentId":"user", "type":"message", "message":{"role":"assistant", "content":"current"}},
                {"id":"other", "timestamp":"3", "parentId":"user", "type":"message", "message":{"role":"assistant", "content":"other branch"}}
            ]
        })).unwrap());
        session.shell.active = Some(active("1"));
        let shell_rows = |preview| {
            session
                .messages(preview)
                .iter()
                .filter(|m| m.id.starts_with("shell-live-"))
                .count()
        };
        assert_eq!(shell_rows(None), 1);
        assert_eq!(shell_rows(Some("user")), 1);
        assert_eq!(shell_rows(Some("other")), 0);
    }
}
