//! Pair top-level tools across the selected branch without moving transcript messages.
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gupi_conversation::conversation::{ToolActivity, ToolExecution};
use gupi_conversation::history::DisplayMessage;
use serde_json::Value;

use super::activity::{Tool, ToolStatus};
use super::{ChatRow, RowKind};

#[derive(Clone, Default)]
pub(in crate::home) struct ToolIndex {
    records: Vec<Record>,
    ids: HashMap<String, usize>,
    runs: HashMap<String, Vec<Rc<Tool>>>,
}

#[derive(Clone)]
struct Record {
    owner: String,
    message: String,
    part: Option<usize>,
    tool: Rc<Tool>,
    call_source: Option<MessageSource>,
    result_source: Option<MessageSource>,
    live: Option<Arc<ToolActivity>>,
}

#[derive(Clone)]
struct MessageSource {
    id: String,
    entry: Option<String>,
    live_revision: u64,
}
impl MessageSource {
    fn new(message: &DisplayMessage) -> Self {
        Self {
            id: message.id.clone(),
            entry: message.entry.clone(),
            live_revision: message.live_revision(),
        }
    }
    fn matches(&self, message: &DisplayMessage) -> bool {
        // Live overlays retain persisted entry IDs. Their token takes precedence,
        // including transitions from/to a purely persisted value.
        if self.live_revision != 0 || message.live_revision() != 0 {
            self.live_revision == message.live_revision() && self.id == message.id
        } else {
            message.entry.is_some() && self.entry == message.entry
        }
    }
}

impl ToolIndex {
    /// `rows` must belong to one selected branch. Pass no live tools for an
    /// inactive branch preview. Calls retain their first location and order.
    #[cfg(test)]
    pub fn project<'a>(rows: impl IntoIterator<Item = &'a ChatRow>, live: &[ToolActivity]) -> Self {
        let live: Vec<_> = live.iter().cloned().map(Arc::new).collect();
        Self::project_reusing(rows, &live, None)
    }

    /// Reuse immutable persisted payloads only within the same history revision.
    pub fn project_reusing<'a>(
        rows: impl IntoIterator<Item = &'a ChatRow>,
        live: &[Arc<ToolActivity>],
        previous: Option<&Self>,
    ) -> Self {
        Self::from_runs_reusing(
            rows.into_iter().filter_map(|row| match &row.kind {
                RowKind::Run {
                    messages, active, ..
                } => Some((row.id.as_str(), messages.as_slice(), *active)),
                _ => None,
            }),
            live,
            previous,
        )
    }

    pub(super) fn local(messages: &[DisplayMessage], live: &[ToolActivity], active: bool) -> Self {
        let live: Vec<_> = live.iter().cloned().map(Arc::new).collect();
        Self::from_runs_reusing(
            std::iter::once(("local", messages, active)),
            if active { &live } else { &[] },
            None,
        )
    }

    #[cfg(test)]
    fn from_runs<'a>(
        runs: impl IntoIterator<Item = (&'a str, &'a [DisplayMessage], bool)>,
        live: &[ToolActivity],
    ) -> Self {
        let live: Vec<_> = live.iter().cloned().map(Arc::new).collect();
        Self::from_runs_reusing(runs, &live, None)
    }

    fn from_runs_reusing<'a>(
        runs: impl IntoIterator<Item = (&'a str, &'a [DisplayMessage], bool)>,
        live: &[Arc<ToolActivity>],
        previous: Option<&Self>,
    ) -> Self {
        let runs: Vec<_> = runs.into_iter().collect();
        let results: HashMap<_, _> = runs
            .iter()
            .flat_map(|(_, messages, _)| messages.iter())
            .filter(|m| m.role() == "toolResult")
            .map(|m| (result_id(m), m))
            .collect();
        let live: HashMap<_, _> = live.iter().map(|tool| (tool.id.as_str(), tool)).collect();
        let mut index = Self::default();
        // Calls are indexed before orphan results, so a result across a user or
        // compaction boundary cannot steal ownership from its call.
        for (owner, messages, active) in &runs {
            for message in *messages {
                if message.role() != "assistant" {
                    continue;
                }
                let Some(parts) = message.value["content"].as_array() else {
                    continue;
                };
                for (part_ix, part) in parts.iter().enumerate() {
                    if part["type"] != "toolCall" {
                        continue;
                    }
                    let id = call_id(message, part_ix);
                    if index.ids.contains_key(&id) {
                        continue;
                    }
                    // A visible boundary can leave the call in an inactive Run.
                    // The caller scopes live snapshots to the selected branch.
                    let update = part["id"].as_str().and_then(|id| live.get(id)).copied();
                    let result = results.get(&id).copied();
                    let status = result.map(result_status).unwrap_or_else(|| {
                        match update.map(|tool| &tool.execution) {
                            Some(ToolExecution::Running(_)) => ToolStatus::Running,
                            Some(ToolExecution::Complete(_)) => ToolStatus::Complete,
                            Some(ToolExecution::Failed(_)) => ToolStatus::Failed,
                            None if *active => ToolStatus::Running,
                            None => ToolStatus::Unfinished,
                        }
                    });
                    let reused = previous
                        .and_then(|previous| previous.record(&id))
                        .filter(|old| {
                            let call_unchanged = old
                                .call_source
                                .as_ref()
                                .is_some_and(|source| source.matches(message));
                            let result_unchanged = match (old.result_source.as_ref(), result) {
                                (None, None) => true,
                                (Some(source), Some(result)) => source.matches(result),
                                _ => false,
                            };
                            let live_unchanged = match (update, old.live.as_ref()) {
                                (Some(next), Some(old)) => Arc::ptr_eq(next, old),
                                (None, None) => true,
                                _ => false,
                            };
                            call_unchanged
                                && result_unchanged
                                && live_unchanged
                                && old.tool.status == status
                        })
                        .map(|old| old.tool.clone());
                    let tool = reused.unwrap_or_else(|| {
                        let (output, status) = if let Some(result) = result {
                            (result.value.clone(), result_status(result))
                        } else if let Some(update) = update {
                            match &update.execution {
                                ToolExecution::Running(value) => {
                                    (value.clone(), ToolStatus::Running)
                                }
                                ToolExecution::Complete(value) => {
                                    (value.clone(), ToolStatus::Complete)
                                }
                                ToolExecution::Failed(value) => (value.clone(), ToolStatus::Failed),
                            }
                        } else {
                            (
                                Value::Null,
                                if *active {
                                    ToolStatus::Running
                                } else {
                                    ToolStatus::Unfinished
                                },
                            )
                        };
                        let mut args = part.get("arguments").filter(|v| !v.is_null()).cloned();
                        if let Some(update) = update {
                            match (&mut args, &update.args) {
                                (Some(Value::Object(args)), Value::Object(extra)) => {
                                    for (key, value) in extra {
                                        args.entry(key).or_insert_with(|| value.clone());
                                    }
                                }
                                (None, value) if !value.is_null() => args = Some(value.clone()),
                                _ => {}
                            }
                        }
                        let mut entries: Vec<_> = message.entry.iter().cloned().collect();
                        if let Some(entry) = result.and_then(|m| m.entry.as_ref())
                            && !entries.contains(entry)
                        {
                            entries.push(entry.clone());
                        }
                        let name = part["name"]
                            .as_str()
                            .or_else(|| update.map(|t| t.name.as_str()))
                            .unwrap_or("tool")
                            .to_owned();
                        Rc::new(Tool {
                            id: id.clone(),
                            entries,
                            name,
                            args,
                            result: output,
                            status,
                        })
                    });
                    index.insert(Record {
                        owner: (*owner).into(),
                        message: message.id.clone(),
                        part: Some(part_ix),
                        tool,
                        call_source: Some(MessageSource::new(message)),
                        result_source: result.map(MessageSource::new),
                        live: update.cloned(),
                    });
                }
            }
        }
        for (owner, messages, _) in &runs {
            for message in *messages {
                if message.role() != "toolResult" {
                    continue;
                }
                let id = result_id(message);
                if index.ids.contains_key(&id) {
                    continue;
                }
                let result = results[&id];
                index.insert(Record {
                    owner: (*owner).into(),
                    message: message.id.clone(),
                    part: None,
                    tool: previous
                        .and_then(|previous| previous.record(&id))
                        .filter(|old| {
                            old.part.is_none()
                                && old
                                    .result_source
                                    .as_ref()
                                    .is_some_and(|source| source.matches(result))
                        })
                        .map(|old| old.tool.clone())
                        .unwrap_or_else(|| {
                            Rc::new(Tool {
                                id,
                                entries: result.entry.iter().cloned().collect(),
                                name: result.value["toolName"].as_str().unwrap_or("tool").into(),
                                args: None,
                                result: result.value.clone(),
                                status: result_status(result),
                            })
                        }),
                    call_source: None,
                    result_source: Some(MessageSource::new(result)),
                    live: None,
                });
            }
        }
        // Preserve transcript order even when a genuine orphan precedes a call.
        let mut order = HashMap::new();
        for (position, message) in runs
            .iter()
            .flat_map(|(_, messages, _)| messages.iter())
            .enumerate()
        {
            order.entry(message.id.as_str()).or_insert(position);
        }
        index
            .records
            .sort_by_key(|r| (order[r.message.as_str()], r.part.unwrap_or(0)));
        for record in &index.records {
            index
                .runs
                .entry(record.owner.clone())
                .or_default()
                .push(record.tool.clone());
        }
        index.ids = index
            .records
            .iter()
            .enumerate()
            .map(|(i, r)| (r.tool.id.clone(), i))
            .collect();
        index
    }

    fn insert(&mut self, record: Record) {
        self.ids.insert(record.tool.id.clone(), self.records.len());
        self.records.push(record);
    }

    fn record(&self, id: &str) -> Option<&Record> {
        self.ids.get(id).map(|&i| &self.records[i])
    }

    #[cfg(test)]
    pub fn tool(&self, id: &str) -> Option<&Tool> {
        self.record(id).map(|record| record.tool.as_ref())
    }

    pub(super) fn shared_tool(&self, id: &str) -> Option<&Rc<Tool>> {
        self.record(id).map(|record| &record.tool)
    }

    pub fn same_run(&self, next: &Self, run: &str) -> bool {
        let old = self.runs.get(run).map(Vec::as_slice).unwrap_or_default();
        let new = next.runs.get(run).map(Vec::as_slice).unwrap_or_default();
        old.len() == new.len() && old.iter().zip(new).all(|(a, b)| Rc::ptr_eq(a, b) || a == b)
    }

    pub fn owner(&self, id: &str) -> Option<&str> {
        self.ids.get(id).map(|&i| self.records[i].owner.as_str())
    }

    #[cfg(test)]
    pub fn in_run<'a>(&'a self, run: &'a str) -> impl Iterator<Item = &'a Tool> {
        self.runs.get(run).into_iter().flatten().map(Rc::as_ref)
    }

    pub(super) fn iter_shared(&self) -> impl Iterator<Item = (&str, &Rc<Tool>)> {
        self.records.iter().map(|r| (r.owner.as_str(), &r.tool))
    }

    /// Returns a tool only at its first call, or its first genuinely orphaned result.
    pub(super) fn at(&self, message: &DisplayMessage, part: Option<usize>) -> Option<&Tool> {
        let id = part
            .map(|i| call_id(message, i))
            .unwrap_or_else(|| result_id(message));
        let record = &self.records[*self.ids.get(&id)?];
        (record.message == message.id && record.part == part).then_some(record.tool.as_ref())
    }
}

/// Dialog-owned projection. Text-only events advance the content watermark without
/// re-reading history; the next tool delta replaces only its owning live message.
#[derive(Default)]
pub(in crate::home) struct ToolProjection {
    rows: Vec<ChatRow>,
    tools: ToolIndex,
    content_revision: u64,
    revision: Option<(u64, u64, bool)>,
    preview: Option<String>,
}
impl ToolProjection {
    #[cfg(test)]
    pub(super) fn index(&self) -> &ToolIndex {
        &self.tools
    }

    pub fn sync(
        &mut self,
        session: &gupi_conversation::conversation::Session,
        preview: Option<&str>,
    ) -> &ToolIndex {
        let revision = session.tool_projection_revision();
        let same_history = self.revision.is_some_and(|old| old.0 == revision.0)
            && self.preview.as_deref() == preview;
        if self.revision == Some(revision) && self.preview.as_deref() == preview {
            self.content_revision = session.content_revision();
            return &self.tools;
        }
        let incremental = same_history
            && self.revision.is_some_and(|old| old.2 == revision.2)
            && (self.content_revision == session.content_revision()
                || (preview.is_none()
                    && session
                        .message_update_since(self.content_revision)
                        .is_some_and(|message| {
                            self.rows.iter_mut().any(|row| row.update_message(message))
                        })));
        if !incremental {
            self.rows = super::project(session, preview);
        }
        self.tools = ToolIndex::project_reusing(
            self.rows.iter(),
            if preview.is_none() {
                session.tools()
            } else {
                &[]
            },
            same_history.then_some(&self.tools),
        );
        self.revision = Some(revision);
        self.content_revision = session.content_revision();
        self.preview = preview.map(str::to_owned);
        &self.tools
    }
}

fn call_id(message: &DisplayMessage, part: usize) -> String {
    match message.value["content"][part]["id"].as_str() {
        Some(id) => format!("tool-{id}"),
        None => format!("tool-part-{}-{part}", message.id),
    }
}
fn result_id(message: &DisplayMessage) -> String {
    format!(
        "tool-{}",
        message.value["toolCallId"].as_str().unwrap_or(&message.id)
    )
}
fn result_status(message: &DisplayMessage) -> ToolStatus {
    if message.value["isError"] == true {
        ToolStatus::Failed
    } else {
        ToolStatus::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message(id: &str, value: Value) -> DisplayMessage {
        DisplayMessage::new(id.into(), value)
    }
    fn call(id: &str, name: &str) -> DisplayMessage {
        message(
            id,
            json!({"role":"assistant","content":[{"type":"toolCall","id":id,"name":name,"arguments":{"path":"a.rs","content":""}}]}),
        )
    }
    fn result(id: &str) -> DisplayMessage {
        message(
            &format!("result-{id}"),
            json!({"role":"toolResult","toolCallId":id,"toolName":"write","content":"done"}),
        )
    }

    #[test]
    fn immutable_history_payloads_are_shared_across_tool_argument_updates() {
        let mut history = vec![
            call("recorded", "write"),
            result("recorded"),
            result("orphan"),
        ];
        for message in &mut history {
            message.entry = Some(message.id.clone());
        }
        history[1].value["content"] = Value::String("large output".repeat(10000));
        history[2].value["content"] = Value::String("orphan output".repeat(10000));
        let mut live = vec![call("stream", "write")];
        let old = ToolIndex::from_runs_reusing(
            [
                ("history", history.as_slice(), false),
                ("live", live.as_slice(), true),
            ],
            &[],
            None,
        );
        live[0].value["content"][0]["arguments"]["content"] = "new arguments".into();
        let next = ToolIndex::from_runs_reusing(
            [
                ("history", history.as_slice(), false),
                ("live", live.as_slice(), true),
            ],
            &[],
            Some(&old),
        );
        for id in ["tool-recorded", "tool-orphan"] {
            assert!(Rc::ptr_eq(
                old.shared_tool(id).unwrap(),
                next.shared_tool(id).unwrap()
            ));
        }
        assert!(old.same_run(&next, "history"));
        assert!(!old.same_run(&next, "live"));
        assert_eq!(
            next.tool("tool-stream").unwrap().args.as_ref().unwrap()["content"],
            "new arguments"
        );
        // Authoritative history replacement invalidates reuse even for identical IDs.
        history[1].value["content"] = "changed history".into();
        let replaced =
            ToolIndex::from_runs_reusing([("history", history.as_slice(), false)], &[], None);
        assert!(!Rc::ptr_eq(
            next.shared_tool("tool-recorded").unwrap(),
            replaced.shared_tool("tool-recorded").unwrap()
        ));
        assert_eq!(
            replaced.tool("tool-recorded").unwrap().result["content"],
            "changed history"
        );
    }

    #[test]
    fn live_overlays_of_persisted_calls_results_and_orphans_invalidate_cached_payloads() {
        use gupi_conversation::conversation::Session;
        let call = json!({"role":"assistant","timestamp":1,"content":[{"type":"toolCall","id":"one","name":"write","arguments":{"path":"disk.txt","content":"disk"}}]});
        let result = json!({"role":"toolResult","timestamp":2,"toolCallId":"one","toolName":"write","content":"disk result"});
        let orphan = json!({"role":"toolResult","timestamp":3,"toolCallId":"orphan","toolName":"edit","content":"disk orphan"});
        let mut session = Session::from_rpc_messages(&[]);
        let mut parent = None;
        let entries = [call.clone(), result.clone(), orphan.clone()].into_iter().enumerate().map(|(i, message)| {
            let id = format!("entry-{i}");
            let entry = serde_json::from_value(json!({"id":id,"parentId":parent,"timestamp":"2026-10-14T00:00:00Z","type":"message","message":message})).unwrap();
            parent = Some(id); entry
        }).collect();
        session.replace_entries_for_test(pi_rpc::protocol::Entries {
            entries,
            leaf_id: parent,
        });
        let history_revision = session.history().revision;
        let send = |session: &mut Session, mut message: Value, content: &str| {
            message["content"] = content.into();
            session.receive_message_for_test("message_end", &json!({"message":message}));
            *session.content_revision_for_test() += 1;
        };
        let mut projection = ToolProjection::default();
        projection.sync(&session, None);
        let disk = projection.index().shared_tool("tool-one").unwrap().clone();
        send(&mut session, result.clone(), "live one");
        let first_live = projection
            .sync(&session, None)
            .shared_tool("tool-one")
            .unwrap()
            .clone();
        assert!(!Rc::ptr_eq(&disk, &first_live));
        send(&mut session, result, "live two");
        let next = projection
            .sync(&session, None)
            .shared_tool("tool-one")
            .unwrap();
        assert!(!Rc::ptr_eq(&first_live, next));
        assert_eq!(next.result["content"], "live two");
        let mut live_call = call;
        live_call["content"][0]["arguments"]["path"] = "live.txt".into();
        session.receive_message_for_test("message_end", &json!({"message":live_call}));
        *session.content_revision_for_test() += 1;
        assert_eq!(
            projection
                .sync(&session, None)
                .tool("tool-one")
                .unwrap()
                .args
                .as_ref()
                .unwrap()["path"],
            "live.txt"
        );
        let old_orphan = projection
            .index()
            .shared_tool("tool-orphan")
            .unwrap()
            .clone();
        send(&mut session, orphan, "live orphan");
        let next = projection
            .sync(&session, None)
            .shared_tool("tool-orphan")
            .unwrap();
        assert!(!Rc::ptr_eq(&old_orphan, next));
        assert_eq!(next.result["content"], "live orphan");
        // Dropping live overlays adopts the same persisted IDs with zero tokens.
        session.live_for_test().clear();
        *session.content_revision_for_test() += 1;
        let adopted = projection.sync(&session, None).tool("tool-one").unwrap();
        assert_eq!(adopted.args.as_ref().unwrap()["path"], "disk.txt");
        assert_eq!(adopted.result["content"], "disk result");
        assert_eq!(session.history().revision, history_revision);
    }

    #[test]
    fn completed_live_tools_keep_payload_identity_during_the_next_call_stream() {
        use gupi_conversation::conversation::Session;
        let args = json!({"path":"first.txt", "content":"written line\n".repeat(300)});
        let output = json!({"content":"large output".repeat(10000)});
        let mut session = Session::from_rpc_messages(&[
            json!({"type":"message_end","message":{"role":"assistant","timestamp":1,"content":[{"type":"toolCall","id":"first","name":"write","arguments":args}]}}),
            json!({"type":"message_end","message":{"role":"toolResult","timestamp":2,"toolCallId":"first","toolName":"write","content":output["content"]}}),
        ]);
        session.tools_for_test().push(Arc::new(ToolActivity::new(
            "first".into(),
            "write".into(),
            args,
            ToolExecution::Complete(output),
        )));
        let mut projection = ToolProjection::default();
        projection.sync(&session, None);
        let send = |session: &mut Session, kind: &str, raw: Value| {
            session.receive_message_for_test(kind, &raw);
            *session.content_revision_for_test() += 1;
        };
        send(
            &mut session,
            "message_start",
            json!({"message":{"role":"assistant","timestamp":3,"content":[]}}),
        );
        projection.sync(&session, None);
        send(
            &mut session,
            "message_update",
            json!({"assistantMessageEvent":{"type":"toolcall_start","contentIndex":0,"id":"second","toolName":"write"}}),
        );
        let first = projection
            .sync(&session, None)
            .shared_tool("tool-first")
            .unwrap()
            .clone();
        let snapshot = session.tools().clone();
        let second =
            json!({"path":"second.txt","content":"another line\n".repeat(300)}).to_string();
        for chunk in second.as_bytes().chunks(80) {
            send(
                &mut session,
                "message_update",
                json!({"assistantMessageEvent":{"type":"toolcall_delta","contentIndex":0,"delta":std::str::from_utf8(chunk).unwrap()}}),
            );
            assert!(Arc::ptr_eq(&snapshot[0], &session.tools()[0]));
            assert!(Rc::ptr_eq(
                &first,
                projection
                    .sync(&session, None)
                    .shared_tool("tool-first")
                    .unwrap()
            ));
        }
        assert_eq!(
            projection
                .index()
                .tool("tool-second")
                .unwrap()
                .args
                .as_ref()
                .unwrap()["path"],
            "second.txt"
        );
        // A replacement snapshot must invalidate live reuse, even when a recorded
        // result still wins over its newer terminal output.
        session.tools_for_test()[0] = Arc::new(ToolActivity::new(
            "first".into(),
            "write".into(),
            json!({"extra":"new"}),
            ToolExecution::Failed(json!({"content":"late failure"})),
        ));
        let changed = projection
            .sync(&session, None)
            .shared_tool("tool-first")
            .unwrap();
        assert!(!Rc::ptr_eq(&first, changed));
        assert_eq!(changed.status, ToolStatus::Complete);
        assert_eq!(changed.args.as_ref().unwrap()["extra"], "new");
        // A correction to the recorded call itself also invalidates its live token.
        send(
            &mut session,
            "message_end",
            json!({"message":{"role":"assistant","timestamp":3,"content":[{"type":"toolCall","id":"second","name":"write","arguments":{"path":"second.txt","content":"done"}}]}}),
        );
        send(
            &mut session,
            "message_end",
            json!({"message":{"role":"assistant","timestamp":1,"content":[{"type":"toolCall","id":"first","name":"write","arguments":{"path":"corrected.txt","content":"corrected"}}]}}),
        );
        assert_eq!(
            projection
                .sync(&session, None)
                .tool("tool-first")
                .unwrap()
                .args
                .as_ref()
                .unwrap()["path"],
            "corrected.txt"
        );
    }

    #[test]
    fn persisted_calls_still_receive_missing_live_arguments_and_new_results() {
        let mut call = call("one", "write");
        call.entry = Some("persisted-call".into());
        call.value["content"][0]["arguments"] = json!({"path":"recorded.rs"});
        let old =
            ToolIndex::from_runs_reusing([("run", std::slice::from_ref(&call), false)], &[], None);
        let live = [Arc::new(ToolActivity::new(
            "one".into(),
            "write".into(),
            json!({"path":"other.rs","content":"complete"}),
            ToolExecution::Complete(json!({"content":"done"})),
        ))];
        let next = ToolIndex::from_runs_reusing(
            [("run", std::slice::from_ref(&call), false)],
            &live,
            Some(&old),
        );
        assert_eq!(
            next.tool("tool-one").unwrap().args,
            Some(json!({"path":"recorded.rs","content":"complete"}))
        );
        assert_eq!(next.tool("tool-one").unwrap().status, ToolStatus::Complete);
        assert!(!old.same_run(&next, "run"));
    }

    #[test]
    fn cross_run_pairing_keeps_first_owner_and_complete_details() {
        for name in ["edit", "write"] {
            let first = [call("one", name)];
            let later = [result("one")];
            let index = ToolIndex::from_runs(
                [
                    ("first", first.as_slice(), false),
                    ("later", later.as_slice(), false),
                ],
                &[],
            );
            assert_eq!(index.owner("tool-one"), Some("first"));
            assert_eq!(index.in_run("first").count(), 1);
            assert_eq!(index.in_run("later").count(), 0);
            let tool = index.tool("tool-one").unwrap();
            assert_eq!(tool.status, ToolStatus::Complete);
            assert_eq!(tool.args.as_ref().unwrap()["path"], "a.rs");
            assert!(index.at(&later[0], None).is_none());
            assert!(index.at(&first[0], Some(0)).is_some());
        }
    }

    #[test]
    fn visible_boundaries_preserve_call_ownership_in_all_projections() {
        use super::super::activity::{Activity, RunContent};
        use super::super::edited_files::EditedFiles;
        use super::super::project_rows;

        for boundary in ["user", "compaction", "branch_summary"] {
            for name in ["edit", "write"] {
                let mut output = result("one");
                output.value["details"] =
                    json!({"patch":"--- a.rs\n+++ a.rs\n@@ -1 +1 @@\n-old\n+new\n"});
                let rows = project_rows(
                    vec![
                        call("one", name),
                        message("boundary", json!({"role":boundary})),
                        output,
                    ],
                    None,
                );
                let index = ToolIndex::project(rows.iter(), &[]);
                let mut files = EditedFiles::default();
                files.sync(&index, "/repo");
                assert_eq!(index.owner("tool-one"), Some(rows[0].id.as_str()));
                assert_eq!(files.files(&rows[0].id).len(), 1);
                assert!(files.files(&rows[2].id).is_empty());
                for (row, expected) in [(&rows[0], 1), (&rows[2], 0)] {
                    let RowKind::Run {
                        messages, active, ..
                    } = &row.kind
                    else {
                        panic!("expected run");
                    };
                    let content = RunContent::project_indexed(messages, *active, &index);
                    assert_eq!(
                        content
                            .activities
                            .iter()
                            .filter(|a| matches!(a, Activity::Tool(_)))
                            .count(),
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn inactive_owner_receives_live_terminal_result_across_visible_boundaries() {
        use super::super::edited_files::EditedFiles;
        use super::super::project_rows;

        for boundary in ["user", "compaction", "branch_summary"] {
            for name in ["edit", "write"] {
                let rows = project_rows(
                    vec![
                        call("one", name),
                        message("boundary", json!({"role":boundary})),
                        message("later", json!({"role":"assistant", "content":"continuing"})),
                    ],
                    None,
                );
                assert!(matches!(rows[0].kind, RowKind::Run { active: false, .. }));
                let output =
                    json!({"details":{"patch":"--- a.rs\n+++ a.rs\n@@ -1 +1 @@\n-old\n+new\n"}});
                let live = [ToolActivity::new(
                    "one".into(),
                    name.into(),
                    Value::Null,
                    ToolExecution::Complete(output.clone()),
                )];
                let before = ToolIndex::project(rows.iter(), &[]);
                let index = ToolIndex::project(rows.iter(), &live);
                assert!(!before.in_run(&rows[0].id).eq(index.in_run(&rows[0].id)));
                assert_eq!(index.owner("tool-one"), Some(rows[0].id.as_str()));
                let tool = index.tool("tool-one").unwrap();
                assert_eq!(tool.status, ToolStatus::Complete);
                assert_eq!(tool.result, output);
                assert_eq!(index.in_run(&rows[2].id).count(), 0);
                let mut files = EditedFiles::default();
                files.sync(&index, "/repo");
                assert_eq!(files.files(&rows[0].id).len(), 1);
                assert!(files.files(&rows[2].id).is_empty());
            }
        }
    }

    #[test]
    fn persisted_result_replaces_live_and_reload_matches() {
        let first = [call("one", "write")];
        let live = [ToolActivity::new(
            "one".into(),
            "write".into(),
            Value::Null,
            ToolExecution::Complete(json!({"content":"live"})),
        )];
        let running = ToolIndex::from_runs([("first", first.as_slice(), true)], &live);
        assert_eq!(
            running.tool("tool-one").unwrap().status,
            ToolStatus::Complete
        );
        let later = [result("one")];
        let settled = ToolIndex::from_runs(
            [
                ("first", first.as_slice(), true),
                ("later", later.as_slice(), false),
            ],
            &live,
        );
        let reloaded = ToolIndex::from_runs(
            [
                ("first", first.as_slice(), false),
                ("later", later.as_slice(), false),
            ],
            &[],
        );
        assert!(settled.in_run("first").eq(reloaded.in_run("first")));
        assert!(!running.in_run("first").eq(settled.in_run("first")));
        assert_eq!(settled.tool("tool-one").unwrap().result["content"], "done");
        assert!(settled.tool("tool-one").unwrap().args.is_some());
    }

    #[test]
    fn calls_are_ordered_and_deduped_independently_of_results() {
        for duplicate_message_id in ["a", "repeated-call"] {
            let first = [call("a", "edit"), call("b", "edit")];
            let mut duplicate = call("a", "edit");
            duplicate.id = duplicate_message_id.into();
            let later = [result("b"), result("a"), duplicate];
            let index = ToolIndex::from_runs(
                [
                    ("first", first.as_slice(), false),
                    ("later", later.as_slice(), false),
                ],
                &[],
            );
            assert_eq!(
                index
                    .in_run("first")
                    .map(|t| t.id.as_str())
                    .collect::<Vec<_>>(),
                ["tool-a", "tool-b"]
            );
            assert_eq!(index.in_run("later").count(), 0);
        }
    }

    #[test]
    fn live_arguments_only_fill_missing_recorded_fields() {
        let mut first = call("one", "write");
        first.value["content"][0]["arguments"] = json!({"path":"recorded.rs"});
        let live = [ToolActivity::new(
            "one".into(),
            "other-name".into(),
            json!({"path":"live.rs", "content":"written"}),
            ToolExecution::Complete(json!({"content":"done"})),
        )];
        let index = ToolIndex::local(&[first], &live, true);
        let tool = index.tool("tool-one").unwrap();
        assert_eq!(tool.name, "write");
        assert_eq!(
            tool.args,
            Some(json!({"path":"recorded.rs", "content":"written"}))
        );
    }

    #[test]
    fn orphan_and_inactive_local_projection_do_not_borrow_live_arguments() {
        let orphan = [result("one")];
        let live = [ToolActivity::new(
            "one".into(),
            "write".into(),
            json!({"path":"other.rs"}),
            ToolExecution::Complete(json!({"content":"other"})),
        )];
        let index = ToolIndex::from_runs([("orphan", orphan.as_slice(), false)], &live);
        assert!(index.tool("tool-one").unwrap().args.is_none());
        assert_eq!(index.owner("tool-one"), Some("orphan"));
        let first = [call("one", "write")];
        let preview = ToolIndex::local(&first, &live, false);
        assert_eq!(
            preview.tool("tool-one").unwrap().status,
            ToolStatus::Unfinished
        );
        assert_eq!(
            preview.tool("tool-one").unwrap().args.as_ref().unwrap()["path"],
            "a.rs"
        );
    }
}
