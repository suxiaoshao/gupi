//! Parent-owned nested-call summaries. IDs are opaque; only live parent fields form edges.
use super::activity::ToolStatus;
use gupi_conversation::conversation::{ToolActivity, ToolExecution};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Default, PartialEq)]
pub(in crate::home) struct NestedCalls {
    pub calls: Vec<Call>,
    /// Only the persisted summary can attest to recording completeness.
    pub complete: Option<bool>,
}
#[derive(Clone, PartialEq)]
pub(in crate::home) struct Call {
    pub id: String,
    pub name: String,
    pub args: Option<Value>,
    pub arguments_omitted: bool,
    pub status: ToolStatus,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}
impl NestedCalls {
    pub(super) fn project(result: &Value, persisted: bool, live: &[Arc<ToolActivity>]) -> Self {
        if let Some(summary) = result.get("nestedCalls").filter(|value| value.is_object()) {
            return Self {
                complete: Some(summary["complete"] == true),
                calls: summary["calls"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|call| {
                        let args = call.get("arguments").cloned();
                        Some(Call {
                            id: call["id"].as_str()?.into(),
                            name: call["name"].as_str()?.into(),
                            arguments_omitted: args.is_none(),
                            args,
                            status: match call["status"].as_str() {
                                Some("ok") => ToolStatus::Complete,
                                Some("error") => ToolStatus::Failed,
                                _ => ToolStatus::Unfinished,
                            },
                            duration_ms: call["durationMs"].as_u64(),
                            error: call["error"].as_str().map(str::to_owned),
                        })
                    })
                    .collect(),
            };
        }
        if persisted {
            return Self::default();
        }
        Self {
            complete: None,
            calls: live
                .iter()
                .map(|call| Call {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    args: (!call.args.is_null()).then(|| call.args.clone()),
                    arguments_omitted: false,
                    status: match call.execution {
                        ToolExecution::Running(_) => ToolStatus::Running,
                        ToolExecution::Complete(_) => ToolStatus::Complete,
                        ToolExecution::Failed(_) => ToolStatus::Failed,
                    },
                    duration_ms: call.duration_ms,
                    error: None,
                })
                .collect(),
        }
    }
}

/// Preserve start-event order and resolve only explicit chains to a visible root.
pub(super) fn by_parent(
    roots: &HashSet<&str>,
    live: &[Arc<ToolActivity>],
) -> HashMap<String, Vec<Arc<ToolActivity>>> {
    let lookup: HashMap<_, _> = live.iter().map(|call| (call.id.as_str(), call)).collect();
    let mut groups: HashMap<String, Vec<Arc<ToolActivity>>> = HashMap::new();
    for call in live {
        let mut parent = call.parent.as_deref();
        let mut seen = HashSet::from([call.id.as_str()]);
        while let Some(id) = parent {
            if !seen.insert(id) {
                break;
            }
            if roots.contains(id) {
                groups.entry(id.to_owned()).or_default().push(call.clone());
                break;
            }
            parent = lookup.get(id).and_then(|call| call.parent.as_deref());
        }
    }
    groups
}
