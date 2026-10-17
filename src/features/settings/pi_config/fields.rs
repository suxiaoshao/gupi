//! The Pi settings shown in Gupi and how scoped values resolve.
use super::forms::{Slot, Snapshot};
use gupi_resources::pi_settings::Change;
use gupi_resources::pi_settings::Document;
use serde_json::Value;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in super::super) enum Page {
    Conversation,
    Network,
    Tools,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Field {
    Model,
    Thinking,
    ModelScope,
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
    Extensions,
    Skills,
    Prompts,
    Themes,
    DefaultTools,
    CodemodeMode,
    CodemodeBudget,
    ShellPath,
    ShellPrefix,
    NpmCommand,
    Telemetry,
    ProjectTrust,
    ModelThinking,
    ModelOverrides,
    BudgetMinimal,
    BudgetLow,
    BudgetMedium,
    BudgetHigh,
    CacheWarming,
    Transport,
    HttpTimeout,
    WebsocketTimeout,
    ProviderTimeout,
    ProviderRetries,
    ProviderDelay,
}

/// How a field is edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Model,
    ModelScope,
    Choice(&'static [&'static str]),
    Bool,
    /// A non-negative integer.
    Count,
    Text,
    List,
    RawList,
    Timeout,
    ThinkingMap,
    CompactionMap,
}

/// Pi 1.1.0 thinking levels, in order.
pub(super) const THINKING_LEVELS: &[&str] =
    &["off", "minimal", "low", "medium", "high", "xhigh", "max"];
pub(super) const DELIVERY: &[&str] = &["one-at-a-time", "all"];

impl Field {
    pub(super) const ALL: [Self; 40] = [
        Self::Model,
        Self::Thinking,
        Self::ModelScope,
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
        Self::Extensions,
        Self::Skills,
        Self::Prompts,
        Self::Themes,
        Self::DefaultTools,
        Self::CodemodeMode,
        Self::CodemodeBudget,
        Self::ShellPath,
        Self::ShellPrefix,
        Self::NpmCommand,
        Self::Telemetry,
        Self::ProjectTrust,
        Self::ModelThinking,
        Self::ModelOverrides,
        Self::BudgetMinimal,
        Self::BudgetLow,
        Self::BudgetMedium,
        Self::BudgetHigh,
        Self::CacheWarming,
        Self::Transport,
        Self::HttpTimeout,
        Self::WebsocketTimeout,
        Self::ProviderTimeout,
        Self::ProviderRetries,
        Self::ProviderDelay,
    ];

    /// The JSON leaves this field writes. The model writes provider and id.
    pub(super) fn leaves(self) -> &'static [&'static [&'static str]] {
        match self {
            Self::Extensions => &[&["extensions"]],
            Self::Skills => &[&["skills"]],
            Self::Prompts => &[&["prompts"]],
            Self::Themes => &[&["themes"]],
            Self::DefaultTools => &[&["defaultTools"]],
            Self::CodemodeMode => &[&["codemode", "mode"]],
            Self::CodemodeBudget => &[&["codemode", "inlineBudget"]],
            Self::ShellPath => &[&["shellPath"]],
            Self::ShellPrefix => &[&["shellCommandPrefix"]],
            Self::NpmCommand => &[&["npmCommand"]],
            Self::Telemetry => &[&["enableInstallTelemetry"]],
            Self::ProjectTrust => &[&["defaultProjectTrust"]],
            Self::ModelThinking => &[&["modelThinkingLevels"]],
            Self::ModelOverrides => &[&["compaction", "modelOverrides"]],
            Self::BudgetMinimal => &[&["thinkingBudgets", "minimal"]],
            Self::BudgetLow => &[&["thinkingBudgets", "low"]],
            Self::BudgetMedium => &[&["thinkingBudgets", "medium"]],
            Self::BudgetHigh => &[&["thinkingBudgets", "high"]],
            Self::CacheWarming => &[&["cacheWarming"]],
            Self::Transport => &[&["transport"]],
            Self::HttpTimeout => &[&["httpIdleTimeoutMs"]],
            Self::WebsocketTimeout => &[&["websocketConnectTimeoutMs"]],
            Self::ProviderTimeout => &[&["retry", "provider", "timeoutMs"]],
            Self::ProviderRetries => &[&["retry", "provider", "maxRetries"]],
            Self::ProviderDelay => &[&["retry", "provider", "maxRetryDelayMs"]],
            Self::Model => &[&["defaultProvider"], &["defaultModel"]],
            Self::Thinking => &[&["defaultThinkingLevel"]],
            Self::ModelScope => &[&["enabledModels"]],
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
            Self::Extensions | Self::Skills | Self::Prompts | Self::Themes => Page::Tools,
            Self::DefaultTools => Page::Tools,
            Self::CodemodeMode => Page::Tools,
            Self::CodemodeBudget => Page::Tools,
            Self::ShellPath => Page::Tools,
            Self::ShellPrefix => Page::Tools,
            Self::NpmCommand => Page::Tools,
            Self::Telemetry => Page::Tools,
            Self::ProjectTrust => Page::Tools,
            Self::ModelThinking => Page::Conversation,
            Self::ModelOverrides => Page::Conversation,
            Self::BudgetMinimal => Page::Conversation,
            Self::BudgetLow => Page::Conversation,
            Self::BudgetMedium => Page::Conversation,
            Self::BudgetHigh => Page::Conversation,
            Self::CacheWarming => Page::Conversation,
            Self::Transport => Page::Network,
            Self::HttpTimeout => Page::Network,
            Self::WebsocketTimeout => Page::Network,
            Self::ProviderTimeout => Page::Network,
            Self::ProviderRetries => Page::Network,
            Self::ProviderDelay => Page::Network,
            Self::Proxy | Self::Retry | Self::MaxRetries | Self::BaseDelay | Self::MaxDelay => {
                Page::Network
            }
            _ => Page::Conversation,
        }
    }

    pub(super) fn kind(self) -> Kind {
        match self {
            Self::Extensions | Self::Skills | Self::Prompts | Self::Themes => Kind::RawList,
            Self::DefaultTools => Kind::List,
            Self::CodemodeMode => Kind::Choice(&["on", "only"]),
            Self::CodemodeBudget => Kind::Count,
            Self::ShellPath => Kind::Text,
            Self::ShellPrefix => Kind::Text,
            Self::NpmCommand => Kind::Text,
            Self::Telemetry => Kind::Bool,
            Self::ProjectTrust => Kind::Choice(&["ask", "always", "never"]),
            Self::ModelThinking => Kind::ThinkingMap,
            Self::ModelOverrides => Kind::CompactionMap,
            Self::BudgetMinimal => Kind::Count,
            Self::BudgetLow => Kind::Count,
            Self::BudgetMedium => Kind::Count,
            Self::BudgetHigh => Kind::Count,
            Self::CacheWarming => Kind::Choice(&["off", "streaming", "idle"]),
            Self::Transport => Kind::Choice(&["auto", "sse", "websocket", "websocket-cached"]),
            Self::HttpTimeout => Kind::Timeout,
            Self::WebsocketTimeout => Kind::Timeout,
            Self::ProviderTimeout => Kind::Count,
            Self::ProviderRetries => Kind::Count,
            Self::ProviderDelay => Kind::Count,
            Self::Model => Kind::Model,
            Self::ModelScope => Kind::ModelScope,
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

    pub(super) const RESOURCE_PATHS: [Self; 4] =
        [Self::Extensions, Self::Skills, Self::Prompts, Self::Themes];
    pub(super) fn is_resource_paths(self) -> bool {
        Self::RESOURCE_PATHS.contains(&self)
    }

    /// Pi reads this field from the global file only.
    pub(super) fn is_global_only(self) -> bool {
        matches!(self, Self::Proxy | Self::ProjectTrust | Self::CacheWarming)
    }

    /// Pi 1.1.0 defaults, per leaf. `None` means unset (automatic).
    fn default(self) -> Option<Value> {
        Some(match self {
            Self::Extensions | Self::Skills | Self::Prompts | Self::Themes => return None,
            Self::DefaultTools => return None,
            Self::CodemodeMode => json!("on"),
            Self::CodemodeBudget => json!(3000),
            Self::ShellPath => return None,
            Self::ShellPrefix => return None,
            Self::NpmCommand => return None,
            Self::Telemetry => json!(true),
            Self::ProjectTrust => json!("ask"),
            Self::ModelThinking => return None,
            Self::ModelOverrides => return None,
            Self::BudgetMinimal => return None,
            Self::BudgetLow => return None,
            Self::BudgetMedium => return None,
            Self::BudgetHigh => return None,
            Self::CacheWarming => json!("streaming"),
            Self::Transport => json!("auto"),
            Self::HttpTimeout => json!(300000),
            Self::WebsocketTimeout => return None,
            Self::ProviderTimeout => return None,
            Self::ProviderRetries => return None,
            Self::ProviderDelay => json!(60000),
            Self::Model | Self::Proxy | Self::ModelScope => return None,
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
    fn inherited(&self, field: Field, leaf: &[&str]) -> (Option<Value>, Source) {
        if !field.is_resource_paths()
            && self.project.is_some()
            && let Some(value) = self.global.get(leaf)
        {
            return (Some(value.clone()), Source::Global);
        }
        (field.default(), Source::Default)
    }

    pub(super) fn resolve(&self, field: Field, draft: Option<&[Slot<Value>]>) -> Resolved {
        let edited = self.edited();
        let overridden = field.leaves().iter().any(|leaf| edited.get(leaf).is_some());
        let (values, sources): (Vec<_>, Vec<_>) = field
            .leaves()
            .iter()
            .enumerate()
            .map(
                |(index, leaf)| match draft.and_then(|values| values.get(index)) {
                    Some(Slot::Set(value) | Slot::Unreadable(value)) => {
                        (Some(value.clone()), self.scope_source())
                    }
                    Some(Slot::Unset) => self.inherited(field, leaf),
                    Some(Slot::Invalid(_)) | None => match edited.get(leaf) {
                        Some(value) => (Some(value.clone()), self.scope_source()),
                        None => self.inherited(field, leaf),
                    },
                },
            )
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

    /// A resource switch must not advance the conflict base of a dirty raw array.
    pub(super) fn preserve_dirty_resource_bases(
        &mut self,
        previous: &Self,
        current: &Snapshot,
        baseline: &Snapshot,
    ) {
        let mut root = self.edited().root().clone();
        for field in Field::RESOURCE_PATHS {
            if current.pending(field, baseline) {
                let key = field.leaves()[0][0];
                match previous.edited().get(&[key]) {
                    Some(value) => {
                        root.insert(key.to_owned(), value.clone());
                    }
                    None => {
                        root.remove(key);
                    }
                }
            }
        }
        let document = Document::from_object(root);
        match &mut self.project {
            Some(project) => *project = document,
            None => self.global = document,
        }
    }

    /// Changes compare typed scope values, but retain raw file values as the conflict base.
    pub(super) fn changes(
        &self,
        current: &Snapshot,
        baseline: &Snapshot,
        page: Option<Page>,
    ) -> Vec<Change> {
        let edited = self.edited();
        let mut changes = Vec::new();
        for field in Field::ALL {
            if page.is_some_and(|page| field.page() != page) || !current.pending(field, baseline) {
                continue;
            }
            if field.is_global_only() && self.project.is_some() {
                continue;
            }
            let values = current.leaf_values(field);
            let previous = baseline.leaf_values(field);
            for leaf in values
                .keys()
                .chain(previous.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                let value = values.get(leaf).unwrap_or(&Slot::Unset);
                if previous.get(leaf).unwrap_or(&Slot::Unset) == value
                    || matches!(value, Slot::Invalid(_))
                {
                    continue;
                }
                let base = edited.get(leaf).cloned();
                let next = value.raw();
                if base != next {
                    changes.push(Change::new(leaf, base, next));
                }
            }
        }
        changes
    }
}

/// Parses a non-negative integer as typed by the user.
pub(super) const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(super) fn parse_count(text: &str) -> Option<Value> {
    text.trim()
        .parse::<u64>()
        .ok()
        .filter(|n| *n <= MAX_SAFE_INTEGER)
        .map(Value::from)
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
