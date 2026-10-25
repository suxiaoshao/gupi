//! Failure metadata only. Pi owns conversation content and its persistence.
use super::*;

impl Session {
    pub(super) fn log_failure(&self, stage: &'static str, category: &'static str) {
        // info.key() is a filesystem path; even before a handshake it must not
        // become a diagnostic identifier. Instance + binding identify that gap.
        tracing::warn!(
            session_id = self.state.as_ref().map(|state| state.session_id.as_str()),
            instance = ?self.instance,
            binding = self.binding,
            stage,
            category,
            failed = true,
            "Pi failure"
        );
    }

    pub(super) fn log_exit(&self, report: &pi_rpc::CloseReport) {
        tracing::warn!(
            session_id = self.state.as_ref().map(|state| state.session_id.as_str()),
            instance = ?self.instance,
            binding = self.binding,
            stage = "connection_exit",
            category = report.reason.as_ref().map_or("process_exit", pi_rpc::Error::category),
            exit_status = ?report.status,
            failed = true,
            "Pi failure"
        );
    }

    pub(super) fn reject_submission(
        &mut self,
        stage: &'static str,
        category: &'static str,
        error: String,
        cx: &mut Context<ConversationState>,
    ) {
        if category != "closed" {
            self.log_failure(stage, category);
        }
        self.error = Some(SessionError::Runtime(error.clone()));
        self.fail_submission(error, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::test;
    use std::sync::Mutex;
    use tracing::{
        Event, Metadata, Subscriber,
        field::{Field, Visit},
        span::{Attributes, Id, Record},
    };

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<String>>);
    impl Subscriber for Capture {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            struct Fields<'a>(&'a mut String);
            impl Visit for Fields<'_> {
                fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                    use std::fmt::Write as _;
                    write!(self.0, "{}={value:?} ", field.name()).unwrap();
                }
            }
            event.record(&mut Fields(&mut self.0.lock().unwrap()));
        }
    }
    // A rejected request must retain its UI error without copying sensitive
    // response text or the internal session-file key into local diagnostics.
    #[gpui_kit::test]
    fn rejected_input_logs_safe_metadata_and_keeps_the_ui_error(cx: &mut TestAppContext) {
        cx.update(gupi_pi_runtime::init);
        cx.update(crate::host::install_headless);
        let owner = cx.new(|cx| ConversationState::new("unused".into(), cx));
        let capture = Capture::default();
        let error = pi_rpc::Error::Rejected {
            command: "PRIVATE_COMMAND".into(),
            message: "SECRET_KEY=https://private.example/auth?token=secret PROMPT_BODY".into(),
        };
        let category = error.category();
        let display = error.to_string();
        tracing::subscriber::with_default(capture.clone(), || {
            owner.update(cx, |_, cx| {
                let mut session = Session::new(
                    SessionInfo::new(
                        "/PRIVATE_SESSION_PATH".into(),
                        "draft-unconfirmed".into(),
                        Default::default(),
                    ),
                    String::new(),
                );
                session.binding = 7;
                session.draft = "unsent draft".into();
                let (reply, mut result) = tokio::sync::oneshot::channel();
                session.submission = Submission::Sending {
                    _task: Task::ready(()),
                    result: reply,
                };
                session.reject_submission("input_submit", category, display.clone(), cx);
                assert!(matches!(session.error,
                    Some(SessionError::Runtime(ref text)) if text == &display));
                assert_eq!(session.draft.text(), "unsent draft");
                assert!(!session.submitting());
                assert_eq!(result.try_recv(), Ok(false));
                session.state = Some(
                    serde_json::from_value(serde_json::json!({
                        "sessionId":"pi-session-42", "isStreaming":false, "isCompacting":false
                    }))
                    .unwrap(),
                );
                session.reject_submission("input_submit", category, display, cx);
            })
        });
        let recorded = capture.0.lock().unwrap();
        for expected in [
            "input_submit",
            "rejected",
            "binding=7",
            "instance=",
            "pi-session-42",
            "failed=true",
        ] {
            assert!(
                recorded.contains(expected),
                "missing safe field {expected}: {recorded}"
            );
        }
        for excluded in [
            "PRIVATE_SESSION_PATH",
            "draft-unconfirmed",
            "PRIVATE_COMMAND",
            "SECRET_KEY",
            "private.example",
            "PROMPT_BODY",
            "unsent draft",
        ] {
            assert!(!recorded.contains(excluded), "sensitive field leaked");
        }
    }
}
