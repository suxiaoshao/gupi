//! Immutable displayed selections owned by a session draft, never source-file handles.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Origin {
    Message {
        role: String,
    },
    File {
        path: PathBuf,
        start: usize,
        end: usize,
        language: String,
        kind: FileKind,
        original: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FileKind {
    Source,
    Turn,
    Staged,
    Unstaged,
    Untracked,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Selection {
    pub origin: Origin,
    pub text: String,
}
impl Selection {
    pub fn new(origin: Origin, text: String) -> Self {
        Self { origin, text }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QuoteAttachment {
    pub id: String,
    pub selection: Selection,
    /// Localized once at capture; restoration never consults the source or locale.
    pub title: String,
    pub short_title: String,
    pub comment_label: String,
    pub comment: String,
}
impl QuoteAttachment {
    pub fn new(
        selection: Selection,
        title: String,
        short_title: String,
        comment_label: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            selection,
            title,
            short_title,
            comment_label,
            comment: String::new(),
        }
    }
    pub fn formatted(&self) -> String {
        let text = &self.selection.text;
        let mut result = match &self.selection.origin {
            Origin::Message { .. } => format!(
                "{}\n{}",
                self.title,
                text.split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Origin::File { language, .. } => {
                let fence = code_fence(text);
                let language = if matches!(language.as_str(), "text" | "plaintext") {
                    ""
                } else {
                    language
                };
                format!(
                    "{}\n{fence}{language}\n{text}{}{fence}",
                    self.title,
                    if text.ends_with('\n') { "" } else { "\n" }
                )
            }
        };
        if !self.comment.is_empty() {
            result.push_str("\n\n");
            result.push_str(&self.comment_label);
            result.push_str(&self.comment);
        }
        result
    }
}
/// One submission boundary for ordinary, queued and temporary drafts. Call only
/// after preparing the ordinary body; explicit non-draft commands never use it.
pub fn compose(quotes: &[std::sync::Arc<QuoteAttachment>], body: &str) -> String {
    if quotes.is_empty() {
        return body.to_owned();
    }
    let mut text = String::new();
    for part in quotes
        .iter()
        .map(|quote| quote.formatted())
        .chain(std::iter::once(body.to_owned()))
    {
        if part.is_empty() {
            continue;
        }
        if !text.is_empty() {
            let existing = text.chars().rev().take_while(|c| *c == '\n').count()
                + part.chars().take_while(|c| *c == '\n').count();
            text.extend(std::iter::repeat_n('\n', 2_usize.saturating_sub(existing)));
        }
        text.push_str(&part);
    }
    text
}
pub fn code_fence(text: &str) -> String {
    "`".repeat(
        text.split(|c| c != '`')
            .map(str::len)
            .max()
            .unwrap_or(0)
            .max(2)
            + 1,
    )
}
