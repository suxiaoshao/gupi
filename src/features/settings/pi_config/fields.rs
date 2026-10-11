//! The Pi settings shown in Gupi and how scoped values resolve.
use gupi_resources::pi_settings::Change;
use gupi_resources::pi_settings::Document;
use gupi_resources::pi_settings::Leaf;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in super::super) enum Page {
    Conversation,
    Network,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Field {
    Model,
    Thinking,
    Compaction,
    ReserveTokens,
    KeepRecentTokens,
    Steering,
    FollowUp,
    AutoResize,
    BlockImages,
    Proxy,
    Retry,
    MaxRetries,
    BaseDelay,
    MaxDelay,
}

/// How a field is edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Model,
    Choice(&'static [&'static str]),
    Bool,
    /// A non-negative integer.
    Count,
    Text,
}

/// Pi 1.1.0 thinking levels, in order.
pub(super) const THINKING_LEVELS: &[&str] =
    &["off", "minimal", "low", "medium", "high", "xhigh", "max"];
pub(super) const DELIVERY: &[&str] = &["one-at-a-time", "all"];

impl Field {
    pub(super) const ALL: [Self; 14] = [
        Self::Model,
        Self::Thinking,
        Self::Compaction,
        Self::ReserveTokens,
        Self::KeepRecentTokens,
        Self::Steering,
        Self::FollowUp,
        Self::AutoResize,
        Self::BlockImages,
        Self::Proxy,
        Self::Retry,
        Self::MaxRetries,
        Self::BaseDelay,
        Self::MaxDelay,
    ];

    /// The JSON leaves this field writes. The model writes provider and id.
    pub(super) fn leaves(self) -> &'static [Leaf] {
        match self {
            Self::Model => &[&["defaultProvider"], &["defaultModel"]],
            Self::Thinking => &[&["defaultThinkingLevel"]],
            Self::Compaction => &[&["compaction", "enabled"]],
            Self::ReserveTokens => &[&["compaction", "reserveTokens"]],
            Self::KeepRecentTokens => &[&["compaction", "keepRecentTokens"]],
            Self::Steering => &[&["steeringMode"]],
            Self::FollowUp => &[&["followUpMode"]],
            Self::AutoResize => &[&["images", "autoResize"]],
            Self::BlockImages => &[&["images", "blockImages"]],
            Self::Proxy => &[&["httpProxy"]],
            Self::Retry => &[&["retry", "enabled"]],
            Self::MaxRetries => &[&["retry", "maxRetries"]],
            Self::BaseDelay => &[&["retry", "baseDelayMs"]],
            Self::MaxDelay => &[&["retry", "maxAgentDelayMs"]],
        }
    }

    pub(super) fn page(self) -> Page {
        match self {
            Self::Proxy | Self::Retry | Self::MaxRetries | Self::BaseDelay | Self::MaxDelay => {
                Page::Network
            }
            _ => Page::Conversation,
        }
    }

    pub(super) fn kind(self) -> Kind {
        match self {
            Self::Model => Kind::Model,
            Self::Thinking => Kind::Choice(THINKING_LEVELS),
            Self::Steering | Self::FollowUp => Kind::Choice(DELIVERY),
            Self::Compaction | Self::AutoResize | Self::BlockImages | Self::Retry => Kind::Bool,
            Self::ReserveTokens
            | Self::KeepRecentTokens
            | Self::MaxRetries
            | Self::BaseDelay
            | Self::MaxDelay => Kind::Count,
            Self::Proxy => Kind::Text,
        }
    }

    /// Pi reads this field from the global file only.
    pub(super) fn is_global_only(self) -> bool {
        self == Self::Proxy
    }

    /// Pi 1.1.0 defaults, per leaf. `None` means unset (automatic).
    fn default(self) -> Option<Value> {
        Some(match self {
            Self::Model | Self::Proxy => return None,
            Self::Thinking => json!("medium"),
            Self::Compaction | Self::AutoResize | Self::Retry => json!(true),
            Self::BlockImages => json!(false),
            Self::ReserveTokens => json!(16384),
            Self::KeepRecentTokens => json!(20000),
            Self::Steering | Self::FollowUp => json!("one-at-a-time"),
            Self::MaxRetries => json!(3),
            Self::BaseDelay => json!(2000),
            Self::MaxDelay => json!(60000),
        })
    }
}

/// A pending edit to one field.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Edit {
    /// Values for each leaf, written to the edited scope.
    Set(Vec<Value>),
    /// Remove the field from the edited scope so it inherits again.
    Remove,
    /// Text that is not a valid value yet; blocks saving.
    Invalid(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Default,
    Global,
    Project,
}

/// The value a page shows for a field.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Resolved {
    /// Effective value per leaf, `None` when unset everywhere.
    pub values: Vec<Option<Value>>,
    pub source: Source,
    /// The edited scope has a value for at least one leaf (before drafts).
    pub overridden: bool,
}

/// Raw files for the edited scope and, for a project, the global file it
/// inherits from.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Files {
    pub global: Document,
    pub project: Option<Document>,
}

impl Files {
    /// The file a page writes.
    pub(super) fn edited(&self) -> &Document {
        self.project.as_ref().unwrap_or(&self.global)
    }

    fn scope_source(&self) -> Source {
        if self.project.is_some() {
            Source::Project
        } else {
            Source::Global
        }
    }

    /// The value one leaf inherits when the edited scope does not set it.
    fn inherited(&self, field: Field, leaf: Leaf) -> (Option<Value>, Source) {
        if self.project.is_some()
            && let Some(value) = self.global.get(leaf)
        {
            return (Some(value.clone()), Source::Global);
        }
        (field.default(), Source::Default)
    }

    pub(super) fn resolve(&self, field: Field, draft: Option<&Edit>) -> Resolved {
        let edited = self.edited();
        let overridden = field.leaves().iter().any(|leaf| edited.get(leaf).is_some());
        let (values, sources): (Vec<_>, Vec<_>) = field
            .leaves()
            .iter()
            .enumerate()
            .map(|(index, leaf)| match draft {
                Some(Edit::Set(values)) => (values.get(index).cloned(), self.scope_source()),
                Some(Edit::Remove) => self.inherited(field, leaf),
                Some(Edit::Invalid(_)) | None => match edited.get(leaf) {
                    Some(value) => (Some(value.clone()), self.scope_source()),
                    None => self.inherited(field, leaf),
                },
            })
            .unzip();
        // A partially overridden provider/model pair reports the nearest source.
        let source = [Source::Project, Source::Global]
            .into_iter()
            .find(|source| sources.contains(source))
            .unwrap_or(Source::Default);
        Resolved {
            values,
            source,
            overridden,
        }
    }

    /// Leaf changes for the given drafts, relative to the files as read.
    /// Invalid drafts and edits that would not change the file are skipped.
    pub(super) fn changes(&self, drafts: &BTreeMap<Field, Edit>) -> Vec<Change> {
        let edited = self.edited();
        let mut changes = Vec::new();
        for (field, edit) in drafts {
            for (index, leaf) in field.leaves().iter().enumerate() {
                let base = edited.get(leaf).cloned();
                let next = match edit {
                    Edit::Set(values) => values.get(index).cloned(),
                    Edit::Remove => None,
                    Edit::Invalid(_) => continue,
                };
                if base != next {
                    changes.push(Change::new(leaf, base, next));
                }
            }
        }
        changes
    }
}

/// Parses a non-negative integer as typed by the user.
pub(super) fn parse_count(text: &str) -> Option<Value> {
    text.trim().parse::<u64>().ok().map(Value::from)
}

/// Displays a JSON value in a text field.
pub(super) fn display(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(value) => value.to_string(),
    }
}

/// Levels Pi offers for a model, following Pi 1.1.0 `getSupportedThinkingLevels`:
/// non-reasoning models only support `off`; a `null` entry in
/// `thinkingLevelMap` removes a level; `xhigh` and `max` need an entry.
pub(super) fn supported_levels(model: &pi_rpc::protocol::Model) -> Vec<&'static str> {
    if !model.reasoning {
        return vec!["off"];
    }
    let map = model
        .extra
        .get("thinkingLevelMap")
        .and_then(Value::as_object);
    THINKING_LEVELS
        .iter()
        .copied()
        .filter(|level| match map.and_then(|map| map.get(*level)) {
            Some(Value::Null) => false,
            Some(_) => true,
            None => !matches!(*level, "xhigh" | "max"),
        })
        .collect()
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod tests;
