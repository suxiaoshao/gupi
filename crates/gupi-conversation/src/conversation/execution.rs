//! Event-owned execution facts. Cached RPC snapshots are not a second lifecycle.
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default)]
pub(super) enum RunState {
    #[default]
    Idle,
    Active {
        messages: HashSet<String>,
        started_at: i64,
    },
}
impl RunState {
    pub fn active_messages(&self) -> Option<&HashSet<String>> {
        match self {
            Self::Idle => None,
            Self::Active { messages, .. } => Some(messages),
        }
    }
    pub fn started_at(&self) -> Option<i64> {
        match self {
            Self::Idle => None,
            Self::Active { started_at, .. } => Some(*started_at),
        }
    }
    pub fn start(&mut self) {
        *self = Self::Active {
            messages: HashSet::new(),
            started_at: (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64,
        };
    }
    pub fn observe_streaming(&mut self, streaming: bool) {
        // agent_end may precede queued work/retries. Only agent_settled ends a run.
        if streaming && self.active_messages().is_none() {
            self.start();
        }
    }
    pub fn record_message(&mut self, signature: String) {
        if let Self::Active { messages, .. } = self {
            messages.insert(signature);
        }
    }
}

#[derive(Clone)]
pub enum ToolExecution {
    Running(Value),
    Complete(Value),
    Failed(Value),
}

/// Pi owns scheduling; this deadline only describes its announced waiting period.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RetryProgress {
    pub attempt: u64,
    pub max_attempts: u64,
    pub deadline: std::time::Instant,
    pub reason: String,
}
impl RetryProgress {
    pub fn from_event(event: &Value) -> Option<Self> {
        let delay = event["delayMs"].as_u64()?;
        Some(Self {
            attempt: event["attempt"].as_u64()?,
            max_attempts: event["maxAttempts"].as_u64()?,
            deadline: std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(delay))?,
            reason: event["errorMessage"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        })
    }
    pub fn remaining(&self) -> u64 {
        self.deadline
            .saturating_duration_since(std::time::Instant::now())
            .as_secs_f64()
            .ceil() as u64
    }
}

impl super::Session {
    pub(super) fn receive_tool(&mut self, kind: &str, raw: &Value) {
        use std::sync::Arc;
        let id = raw["toolCallId"].as_str().unwrap_or_default();
        if !self.tools.iter().any(|tool| tool.id == id) {
            self.tools.push(Arc::new(super::ToolActivity::new(
                id.into(),
                raw["toolName"].as_str().unwrap_or_default().into(),
                raw.get("args").cloned().unwrap_or(Value::Null),
                ToolExecution::Running(Value::Null),
            )));
        }
        let tool = Arc::make_mut(self.tools.iter_mut().find(|tool| tool.id == id).unwrap());
        if let Some(parent) = raw["parentToolCallId"].as_str() {
            tool.parent = Some(parent.into());
        }
        if let Some(duration) = raw["durationMs"].as_u64() {
            tool.duration_ms = Some(duration);
        }
        let previous = match &tool.execution {
            ToolExecution::Running(value)
            | ToolExecution::Complete(value)
            | ToolExecution::Failed(value) => value,
        };
        let output = raw
            .get("partialResult")
            .or_else(|| raw.get("result"))
            .unwrap_or(previous)
            .clone();
        tool.execution = match kind {
            "tool_execution_end" if raw["isError"].as_bool().unwrap_or(false) => {
                ToolExecution::Failed(output)
            }
            "tool_execution_end" => ToolExecution::Complete(output),
            _ => match &tool.execution {
                ToolExecution::Running(_) => ToolExecution::Running(output),
                ToolExecution::Complete(_) => ToolExecution::Complete(output),
                ToolExecution::Failed(_) => ToolExecution::Failed(output),
            },
        };
    }
}
