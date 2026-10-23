//! Recorded editing operations, grouped lexically without inspecting the filesystem.
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::diff::DiffFile;
use gupi_conversation::tool_presentation::read_path;

use super::activity::{Tool, ToolStatus};
use super::composition;
use super::tools::ToolIndex;

#[derive(Clone)]
pub(in crate::home) enum ChangeBody {
    Patch {
        source: String,
        files: Vec<DiffFile>,
    },
    Written {
        content: String,
    },
    Text {
        content: String,
    },
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::home) struct LineStats {
    added: usize,
    removed: usize,
}
impl LineStats {
    pub fn added(&self) -> usize {
        self.added
    }
    pub fn removed(&self) -> usize {
        self.removed
    }
    fn add(&mut self, next: Self) {
        self.added += next.added;
        self.removed += next.removed;
    }
    fn from_body(body: &ChangeBody) -> Option<Self> {
        match body {
            ChangeBody::Patch { files, .. } => {
                Some(files.iter().fold(Self::default(), |mut sum, file| {
                    sum.add(Self {
                        added: file.additions(),
                        removed: file.deletions(),
                    });
                    sum
                }))
            }
            _ => None,
        }
    }
}
fn add_stats(total: &mut Option<LineStats>, stats: Option<LineStats>) {
    if let Some(stats) = stats {
        total.get_or_insert_with(LineStats::default).add(stats);
    }
}

#[derive(Clone)]
pub(in crate::home) struct EditOperation {
    id: String,
    status: ToolStatus,
    stats: Option<LineStats>,
    body: Rc<ChangeBody>,
}
impl EditOperation {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn failed(&self) -> bool {
        self.status == ToolStatus::Failed
    }
    pub fn status(&self) -> ToolStatus {
        self.status
    }
    pub fn stats(&self) -> Option<LineStats> {
        self.stats
    }
    pub fn body(&self) -> &ChangeBody {
        &self.body
    }
    /// Stable payload identity for retained renderers; sync reuses unchanged bodies.
    pub(in crate::home) fn shared_body(&self) -> &Rc<ChangeBody> {
        &self.body
    }
}

#[derive(Clone)]
pub(in crate::home) struct EditedFile {
    path: String,
    operations: Vec<EditOperation>,
    combined: Option<Rc<ChangeBody>>,
    stats: Option<LineStats>,
    written: bool,
    failed: bool,
}
impl EditedFile {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn stats(&self) -> Option<LineStats> {
        self.stats
    }
    pub fn written(&self) -> bool {
        self.written
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub(in crate::home) fn combined(&self) -> Option<&Rc<ChangeBody>> {
        self.combined.as_ref()
    }
    pub fn operations(&self) -> &[EditOperation] {
        &self.operations
    }
}

#[derive(Clone, Default)]
pub(in crate::home) struct EditedFiles {
    runs: HashMap<String, Vec<EditedFile>>,
    cached: HashMap<String, CachedBody>,
    stats: HashMap<String, LineStats>,
    compositions: HashMap<(String, String), ComposedFile>,
    #[cfg(test)]
    recompositions: usize,
}

#[derive(Clone)]
struct ComposedFile {
    operations: Vec<EditOperation>,
    cwd: String,
    body: Option<Rc<ChangeBody>>,
    unchanged: bool,
}
impl ComposedFile {
    fn matches(&self, operations: &[EditOperation], cwd: &str) -> bool {
        self.cwd == cwd
            && self.operations.len() == operations.len()
            && self
                .operations
                .iter()
                .zip(operations)
                .all(|(a, b)| a.id == b.id && Rc::ptr_eq(&a.body, &b.body))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyKind {
    Patch,
    Written,
    Text,
}
#[derive(Clone)]
struct CachedBody {
    kind: BodyKind,
    source: Rc<str>,
    body: Rc<ChangeBody>,
    tool: Rc<Tool>,
    stats: Option<LineStats>,
}

impl EditedFiles {
    /// Call during message synchronization, never during rendering. Unchanged
    /// source reuses the same parsed payload, including cached parse failures.
    pub fn sync(&mut self, tools: &ToolIndex, cwd: &str) {
        let mut runs: HashMap<String, Vec<EditedFile>> = HashMap::new();
        let mut cached = HashMap::new();
        let mut compositions = HashMap::new();
        for (run, tool) in tools.iter_shared() {
            let old = self.cached.remove(&tool.id);
            let body = if let Some(old) = old.as_ref().filter(|old| Rc::ptr_eq(&old.tool, tool)) {
                old.clone()
            } else {
                let Some((kind, source)) = recorded_body(tool) else {
                    continue;
                };
                let mut body = old
                    .filter(|old| old.kind == kind && old.source.as_ref() == source)
                    .unwrap_or_else(|| {
                        let mut cached = CachedBody {
                            stats: None,
                            tool: tool.clone(),
                            kind,
                            source: source.into(),
                            body: Rc::new(match kind {
                                BodyKind::Written => ChangeBody::Written {
                                    content: source.into(),
                                },
                                BodyKind::Text => ChangeBody::Text {
                                    content: source.into(),
                                },
                                BodyKind::Patch => match DiffFile::parse(source) {
                                    Ok(files) if !files.is_empty() => ChangeBody::Patch {
                                        source: source.into(),
                                        files,
                                    },
                                    _ => ChangeBody::Text {
                                        content: source.into(),
                                    },
                                },
                            }),
                        };
                        cached.stats = LineStats::from_body(&cached.body);
                        cached
                    });
                body.tool = tool.clone();
                body
            };
            let path = tool
                .args
                .as_ref()
                .and_then(read_path)
                .filter(|path| !path.is_empty())
                .or_else(|| match body.body.as_ref() {
                    // Built-in edit produces one file. An ambiguous orphan
                    // cannot be assigned to one file without inventing scope.
                    ChangeBody::Patch { files, .. } if files.len() == 1 => {
                        Some(files[0].path().as_ref())
                    }
                    _ => None,
                });
            if let Some(path) = path {
                let path = group_path(path, cwd);
                if !path.is_empty() {
                    let files = runs.entry(run.to_owned()).or_default();
                    let position = files
                        .iter()
                        .position(|file| file.path == path)
                        .unwrap_or_else(|| {
                            files.push(EditedFile {
                                path,
                                operations: Vec::new(),
                                combined: None,
                                stats: None,
                                written: false,
                                failed: false,
                            });
                            files.len() - 1
                        });
                    files[position].written |=
                        matches!(body.body.as_ref(), ChangeBody::Written { .. });
                    files[position].failed |= tool.status == ToolStatus::Failed;
                    files[position].operations.push(EditOperation {
                        id: tool.id.clone(),
                        status: tool.status,
                        stats: body.stats,
                        body: body.body.clone(),
                    });
                }
            }
            cached.insert(tool.id.clone(), body);
        }
        for files in runs.values_mut() {
            files.retain_mut(|file| {
                let key = (file.operations[0].id.clone(), file.path.clone());
                let old = self.compositions.remove(&key);
                let composed = if let Some(old) = old
                    .as_ref()
                    .filter(|old| old.matches(&file.operations, cwd))
                {
                    old.clone()
                } else {
                    #[cfg(test)]
                    {
                        self.recompositions += 1;
                    }
                    let outcome = combine(file, cwd);
                    let (body, unchanged) = match outcome {
                        Ok(None) => (None, true),
                        Ok(Some(body)) => {
                            let body = old
                                .as_ref()
                                .and_then(|old| old.body.as_ref())
                                .filter(|previous| same_body(previous, &body))
                                .cloned()
                                .unwrap_or_else(|| Rc::new(body));
                            (Some(body), false)
                        }
                        Err(()) => (None, false),
                    };
                    ComposedFile {
                        operations: file.operations.clone(),
                        cwd: cwd.to_owned(),
                        body,
                        unchanged,
                    }
                };
                file.combined = composed.body.clone();
                file.stats = file.combined.as_deref().and_then(LineStats::from_body);
                let visible = !composed.unchanged;
                compositions.insert(key, composed);
                visible
            });
        }
        runs.retain(|_, files| !files.is_empty());
        self.compositions = compositions;
        self.stats = runs
            .iter()
            .filter_map(|(run, files)| {
                let mut stats = None;
                for file in files {
                    add_stats(&mut stats, file.stats);
                }
                stats.map(|stats| (run.clone(), stats))
            })
            .collect();
        self.runs = runs;
        self.cached = cached;
    }

    pub fn stats(&self, run: &str) -> Option<LineStats> {
        self.stats.get(run).copied()
    }
    pub(super) fn runs(&self) -> impl Iterator<Item = (&str, &[EditedFile])> {
        self.runs
            .iter()
            .map(|(run, files)| (run.as_str(), files.as_slice()))
    }
    pub fn files(&self, run: &str) -> &[EditedFile] {
        self.runs.get(run).map(Vec::as_slice).unwrap_or_default()
    }
}

fn same_body(a: &ChangeBody, b: &ChangeBody) -> bool {
    match (a, b) {
        (ChangeBody::Patch { source: a, .. }, ChangeBody::Patch { source: b, .. }) => a == b,
        (ChangeBody::Written { content: a }, ChangeBody::Written { content: b }) => a == b,
        _ => false,
    }
}
fn combine(file: &EditedFile, cwd: &str) -> Result<Option<ChangeBody>, ()> {
    let last_write = file
        .operations
        .iter()
        .rposition(|operation| matches!(operation.body(), ChangeBody::Written { .. }));
    let suffix = last_write.map_or(file.operations.as_slice(), |ix| &file.operations[ix + 1..]);
    let patches = suffix
        .iter()
        .map(|operation| match operation.body() {
            ChangeBody::Patch { source, .. } => Ok(source.as_str()),
            _ => Err(()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(ix) = last_write {
        let ChangeBody::Written { content } = file.operations[ix].body() else {
            unreachable!()
        };
        return composition::apply(&file.path, cwd, content, &patches)
            .map(|content| Some(ChangeBody::Written { content }));
    }
    composition::compose(&file.path, cwd, &patches)?
        .map(|source| {
            let files = DiffFile::parse(&source).map_err(|_| ())?;
            if files.len() != 1 {
                return Err(());
            }
            Ok(ChangeBody::Patch { source, files })
        })
        .transpose()
}

fn recorded_body(tool: &Tool) -> Option<(BodyKind, &str)> {
    let terminal = matches!(tool.status, ToolStatus::Complete | ToolStatus::Failed);
    match tool.name.as_str() {
        "edit" if terminal => {
            if let Some(patch) = tool.result["details"]["patch"]
                .as_str()
                .filter(|s| !s.is_empty())
            {
                return Some((BodyKind::Patch, patch));
            }
            (tool.status == ToolStatus::Complete).then(|| {
                let source = tool.result["details"]["diff"]
                    .as_str()
                    .or_else(|| {
                        tool.result["content"]
                            .as_array()?
                            .iter()
                            .find_map(|part| part["text"].as_str())
                    })
                    .unwrap_or("");
                (BodyKind::Text, source)
            })
        }
        "write" if tool.status == ToolStatus::Complete => tool
            .args
            .as_ref()?
            .get("content")?
            .as_str()
            .map(|content| (BodyKind::Written, content)),
        _ => None,
    }
}

fn windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
        || path.starts_with("\\\\")
}

pub(super) fn group_path(path: &str, cwd: &str) -> String {
    // Recognize Windows syntax even when reading a Windows session on macOS.
    // Relative Windows paths inherit their interpretation from the session cwd.
    let windows = windows_absolute(path) || windows_absolute(cwd);
    let path = if windows {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    let cwd = if windows {
        cwd.replace('\\', "/")
    } else {
        cwd.to_owned()
    };
    let mut path = path.as_str();
    while let Some(rest) = path.strip_prefix("./") {
        path = rest;
    }
    let absolute = path.starts_with('/') || windows_absolute(path);
    if absolute && !cwd.is_empty() {
        let cwd = cwd.trim_end_matches('/');
        // An empty trimmed cwd denotes the POSIX root, not a missing cwd.
        if let Some(rest) = path
            .strip_prefix(cwd)
            .and_then(|rest| rest.strip_prefix('/'))
        {
            path = rest;
        }
    }
    while let Some(rest) = path.strip_prefix("./") {
        path = rest;
    }
    path.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gupi_conversation::history::DisplayMessage;
    use serde_json::{Value, json};

    const PATCH: &str = "--- a.rs\n+++ a.rs\n@@ -1 +1 @@\n-old\n+new\n";
    fn message(id: &str, value: Value) -> DisplayMessage {
        DisplayMessage::new(id.into(), value)
    }
    fn operation(
        id: &str,
        name: &str,
        args: Value,
        details: Value,
        error: bool,
    ) -> Vec<DisplayMessage> {
        vec![
            message(
                id,
                json!({"role":"assistant","content":[{"type":"toolCall","id":id,"name":name,"arguments":args}]}),
            ),
            message(
                &format!("result-{id}"),
                json!({"role":"toolResult","toolCallId":id,"toolName":name,"details":details,"isError":error}),
            ),
        ]
    }
    fn sync(messages: &[DisplayMessage]) -> EditedFiles {
        let mut files = EditedFiles::default();
        files.sync(&ToolIndex::local(messages, &[], false), "/repo");
        files
    }

    #[test]
    fn net_stats_cancel_and_writes_or_fallback_do_not_count() {
        let mut messages = operation(
            "a",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            false,
        );
        let second = PATCH.replace("old", "new").replace("+new", "+last");
        messages.extend(operation(
            "b",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":second}),
            true,
        ));
        messages.extend(operation(
            "write",
            "write",
            json!({"path":"b.rs","content":"a whole file"}),
            json!({}),
            false,
        ));
        messages.extend(operation(
            "bad",
            "edit",
            json!({"path":"c.rs"}),
            json!({"patch":"malformed"}),
            false,
        ));
        let files = sync(&messages);
        assert_eq!(
            files.stats("local"),
            Some(LineStats {
                added: 1,
                removed: 1
            })
        );
        assert!(files.files("local")[0].failed());
        assert!(files.files("local")[0].combined().is_some());
        assert!(files.files("local")[1].stats().is_none());
        assert!(files.files("local")[2].combined().is_none());
        assert!(files.files("local")[2].stats().is_none());
        let reverse = PATCH.replace("-old", "-new").replace("+new", "+old");
        let mut cancelled = operation(
            "a",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            false,
        );
        cancelled.extend(operation(
            "b",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":reverse}),
            false,
        ));
        assert!(sync(&cancelled).files("local").is_empty());
    }

    #[test]
    fn last_write_supersedes_bad_prefix_but_not_unknown_suffix() {
        let mut messages = operation(
            "bad",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":"bad"}),
            false,
        );
        messages.extend(operation(
            "old-write",
            "write",
            json!({"path":"a.rs","content":"superseded"}),
            json!({}),
            false,
        ));
        messages.extend(operation(
            "write",
            "write",
            json!({"path":"a.rs","content":"\u{feff}old\r\n"}),
            json!({}),
            false,
        ));
        messages.extend(operation(
            "edit",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            false,
        ));
        let files = sync(&messages);
        assert!(
            matches!(files.files("local")[0].combined().unwrap().as_ref(), ChangeBody::Written { content } if content == "\u{feff}new\r\n")
        );
        assert!(files.stats("local").is_none());
        messages.extend(operation(
            "missing",
            "edit",
            json!({"path":"a.rs"}),
            json!({}),
            false,
        ));
        assert!(sync(&messages).files("local")[0].combined().is_none());
    }

    #[test]
    fn file_cache_recomposes_only_changed_files_and_retains_identical_output() {
        let mut messages = operation(
            "a",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            false,
        );
        messages.extend(operation(
            "other",
            "edit",
            json!({"path":"other"}),
            json!({"patch":PATCH.replace("a.rs", "other")}),
            false,
        ));
        let mut files = sync(&messages);
        let original = files.files("local")[0].combined().unwrap().clone();
        let other = files.files("local")[1].combined().unwrap().clone();
        assert_eq!(files.recompositions, 2);
        let index = ToolIndex::local(&messages, &[], false);
        files.sync(&index, "/repo");
        assert_eq!(files.recompositions, 2);
        let forward = PATCH.replace("old", "new").replace("+new", "+temporary");
        let reverse = PATCH.replace("-old", "-temporary");
        messages.extend(operation(
            "b",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":forward}),
            false,
        ));
        messages.extend(operation(
            "c",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":reverse}),
            false,
        ));
        files.sync(&ToolIndex::local(&messages, &[], false), "/repo");
        assert_eq!(files.recompositions, 3);
        assert!(Rc::ptr_eq(
            &original,
            files.files("local")[0].combined().unwrap()
        ));
        assert!(Rc::ptr_eq(
            &other,
            files.files("local")[1].combined().unwrap()
        ));
        let bad = operation(
            "bad",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":"bad"}),
            false,
        );
        messages.extend(bad);
        let index = ToolIndex::local(&messages, &[], false);
        files.sync(&index, "/repo");
        assert!(files.files("local")[0].combined().is_none());
        let count = files.recompositions;
        files.sync(&index, "/repo");
        assert_eq!(files.recompositions, count);
    }

    #[test]
    fn repeated_edits_group_by_path_and_keep_independent_patches() {
        let mut messages = operation(
            "a",
            "edit",
            json!({"path":"./a.rs"}),
            json!({"patch":PATCH}),
            false,
        );
        let second = PATCH.replace("old", "new").replace("+new", "+last");
        messages.extend(operation(
            "b",
            "edit",
            json!({"path":"/repo/a.rs"}),
            json!({"patch":second}),
            false,
        ));
        let projection = sync(&messages);
        let files = projection.files("local");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path(), "a.rs");
        let ops = files[0].operations();
        assert_eq!(
            ops.iter().map(EditOperation::id).collect::<Vec<_>>(),
            ["tool-a", "tool-b"]
        );
        let ChangeBody::Patch { source, files } = ops[1].body() else {
            panic!("expected patch")
        };
        assert_eq!(source, &second);
        assert_eq!(files[0].additions(), 1);
        assert_eq!(files[0].deletions(), 1);
    }

    #[test]
    fn failures_require_returned_patch_and_empty_write_is_valid() {
        let mut messages = operation(
            "failed",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            true,
        );
        messages.extend(operation(
            "no-patch",
            "edit",
            json!({"path":"b.rs","oldText":"a","newText":"b"}),
            Value::Null,
            true,
        ));
        messages.extend(operation(
            "write-error",
            "write",
            json!({"path":"c.rs","content":"text"}),
            Value::Null,
            true,
        ));
        messages.extend(operation(
            "empty",
            "write",
            json!({"path":"empty.rs","content":""}),
            Value::Null,
            false,
        ));
        messages.push(message("pending", json!({"role":"assistant","content":[{"type":"toolCall","id":"pending","name":"write","arguments":{"path":"pending.rs","content":"text"}}]})));
        let projection = sync(&messages);
        let files = projection.files("local");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].operations()[0].status(), ToolStatus::Failed);
        assert!(
            matches!(files[1].operations()[0].body(), ChangeBody::Written { content } if content.is_empty())
        );
    }

    #[test]
    fn legacy_and_malformed_patch_remain_text_and_nested_tools_are_ignored() {
        let mut messages = operation(
            "legacy",
            "edit",
            json!({"path":"a.rs"}),
            json!({"diff":"+1 new"}),
            false,
        );
        messages.extend(operation(
            "broken",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":"not a patch"}),
            false,
        ));
        messages.extend(operation(
            "shell",
            "bash",
            json!({"path":"b.rs"}),
            json!({"patch":PATCH}),
            false,
        ));
        messages.extend(operation(
            "extension",
            "plugin",
            json!({}),
            json!({"nestedToolCalls":[{"name":"edit","details":{"patch":PATCH}}]}),
            false,
        ));
        let projection = sync(&messages);
        let files = projection.files("local");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].operations().len(), 2);
        assert!(
            matches!(files[0].operations()[0].body(), ChangeBody::Text { content } if content == "+1 new")
        );
        assert!(
            matches!(files[0].operations()[1].body(), ChangeBody::Text { content } if content == "not a patch")
        );
    }

    #[test]
    fn orphan_edit_uses_patch_path_and_orphan_write_is_not_invented() {
        let messages = [
            message(
                "edit",
                json!({"role":"toolResult","toolCallId":"edit","toolName":"edit","details":{"patch":PATCH}}),
            ),
            message(
                "write",
                json!({"role":"toolResult","toolCallId":"write","toolName":"write","content":"Successfully wrote a.rs"}),
            ),
            message(
                "invalid",
                json!({"role":"toolResult","toolCallId":"invalid","toolName":"edit","details":{"patch":"invalid"}}),
            ),
        ];
        let projection = sync(&messages);
        assert_eq!(projection.files("local").len(), 1);
        assert_eq!(projection.files("local")[0].path(), "a.rs");
    }

    #[test]
    fn lexical_paths_do_not_guess_filesystem_equivalence() {
        for (path, cwd, expected) in [
            ("././a b.rs", "/repo", "a b.rs"),
            ("/repo/a b.rs", "/repo/", "a b.rs"),
            ("/repo2/a.rs", "/repo", "/repo2/a.rs"),
            ("../a.rs", "/repo", "../a.rs"),
            ("A.rs", "/repo", "A.rs"),
            ("a\\b.rs", "/repo", "a\\b.rs"),
            ("C:\\work\\a b.rs", "C:\\work", "a b.rs"),
            (".\\src\\a.rs", "C:\\work", "src/a.rs"),
            ("D:\\work\\a.rs", "C:\\work", "D:/work/a.rs"),
            ("C:\\worker\\a.rs", "C:\\work", "C:/worker/a.rs"),
            ("\\\\host\\share\\a.rs", "\\\\host\\share", "a.rs"),
            ("/a.rs", "/", "a.rs"),
        ] {
            assert_eq!(group_path(path, cwd), expected, "{path}");
        }
    }

    #[test]
    fn unchanged_patch_and_parse_failure_reuse_payload_and_removed_tools_are_evicted() {
        let messages = operation(
            "a",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":PATCH}),
            false,
        );
        let index = ToolIndex::local(&messages, &[], false);
        let mut projection = EditedFiles::default();
        projection.sync(&index, "/repo");
        let original = projection.files("local")[0].operations[0].body.clone();
        projection.sync(&index, "/other");
        assert!(Rc::ptr_eq(
            &original,
            &projection.files("local")[0].operations[0].body
        ));
        let messages = operation(
            "a",
            "edit",
            json!({"path":"a.rs"}),
            json!({"patch":"invalid"}),
            false,
        );
        let index = ToolIndex::local(&messages, &[], false);
        projection.sync(&index, "/repo");
        let invalid = projection.files("local")[0].operations[0].body.clone();
        assert!(!Rc::ptr_eq(&original, &invalid));
        projection.sync(&index, "/repo");
        assert!(Rc::ptr_eq(
            &invalid,
            &projection.files("local")[0].operations[0].body
        ));
        projection.sync(&ToolIndex::default(), "/repo");
        assert!(projection.files("local").is_empty());
        assert!(projection.cached.is_empty());
    }
}
