//! Typed page drafts; only scope-local values are stored in the forms.
use super::fields::Field;
use gpui_form::{Form, FormSchema};
use gpui_kit::{App, Entity};
use gupi_resources::pi_settings::Document;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Slot<T> {
    #[default]
    Unset,
    Set(T),
    Unreadable(Value),
    Invalid(String),
}
impl<T: Serialize + DeserializeOwned> Slot<T> {
    pub(super) fn decode(value: Slot<Value>) -> Self {
        match value {
            Slot::Unset => Self::Unset,
            Slot::Unreadable(value) => Self::Unreadable(value),
            Slot::Set(value) => match serde_json::from_value(value.clone()) {
                Ok(value) => Self::Set(value),
                Err(_) => Self::Unreadable(value),
            },
            Slot::Invalid(text) => Self::Invalid(text),
        }
    }
    pub(super) fn encode(&self) -> Slot<Value> {
        match self {
            Self::Unset => Slot::Unset,
            Self::Set(value) => {
                Slot::Set(serde_json::to_value(value).expect("settings value serializes"))
            }
            Self::Unreadable(value) => Slot::Unreadable(value.clone()),
            Self::Invalid(text) => Slot::Invalid(text.clone()),
        }
    }
}
impl Slot<Value> {
    pub(super) fn raw(&self) -> Option<Value> {
        match self {
            Self::Set(v) | Self::Unreadable(v) => Some(v.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct ConversationDraft {
    #[form(child)]
    pub(super) model_thinking: super::model_maps::ThinkingSlot,
    #[form(child)]
    pub(super) model_overrides: super::model_maps::CompactionSlot,
    pub(super) budget_minimal: Slot<u64>,
    pub(super) budget_low: Slot<u64>,
    pub(super) budget_medium: Slot<u64>,
    pub(super) budget_high: Slot<u64>,
    pub(super) cache_warming: Slot<String>,

    pub(super) default_provider: Slot<String>,
    pub(super) default_model: Slot<String>,
    pub(super) thinking: Slot<String>,
    pub(super) compaction: Slot<bool>,
    pub(super) reserve_tokens: Slot<u64>,
    pub(super) keep_recent_tokens: Slot<u64>,
    pub(super) steering: Slot<String>,
    pub(super) follow_up: Slot<String>,
    pub(super) auto_resize: Slot<bool>,
    pub(super) block_images: Slot<bool>,
    #[form(child)]
    pub(super) enabled_models: ModelScopeSlot,
}

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct NetworkDraft {
    pub(super) transport: Slot<String>,
    pub(super) http_timeout: Slot<Value>,
    pub(super) websocket_timeout: Slot<Value>,
    pub(super) provider_timeout: Slot<u64>,
    pub(super) provider_retries: Slot<u64>,
    pub(super) provider_delay: Slot<u64>,

    pub(super) http_proxy: Slot<String>,
    pub(super) retry: Slot<bool>,
    pub(super) max_retries: Slot<u64>,
    pub(super) base_delay: Slot<u64>,
    pub(super) max_delay: Slot<u64>,
}

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct ToolsDraft {
    #[form(child)]
    pub(super) default_tools: super::lists::ListSlot,
    pub(super) codemode_mode: Slot<String>,
    pub(super) codemode_budget: Slot<u64>,
    pub(super) shell_path: Slot<String>,
    pub(super) shell_prefix: Slot<String>,
    pub(super) npm_command: Slot<Vec<String>>,
    pub(super) telemetry: Slot<bool>,
    pub(super) project_trust: Slot<String>,
}

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) enum ModelScopeSlot {
    #[default]
    Unset,
    Set(ModelScope),
    Unreadable(RawValue),
}
#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct ModelScope {
    #[form(items)]
    pub(super) patterns: Vec<Pattern>,
}
#[derive(Clone, PartialEq, FormSchema)]
pub(super) struct Pattern {
    pub(super) text: String,
}
#[derive(Clone, PartialEq, FormSchema)]
pub(super) struct RawValue {
    pub(super) value: Value,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct Snapshot {
    pub(super) conversation: ConversationDraft,
    pub(super) network: NetworkDraft,
    pub(super) tools: ToolsDraft,
}
impl Snapshot {
    pub(super) fn project(document: &Document) -> Self {
        let mut result = Self::default();
        for field in Field::ALL {
            result.set(
                field,
                field
                    .leaves()
                    .iter()
                    .map(|leaf| match document.get(leaf) {
                        Some(value) => Slot::Set(value.clone()),
                        None => Slot::Unset,
                    })
                    .collect(),
            );
        }
        result
    }
    pub(super) fn read(
        conversation: &Entity<Form<ConversationDraft>>,
        network: &Entity<Form<NetworkDraft>>,
        tools: &Entity<Form<ToolsDraft>>,
        cx: &App,
    ) -> Self {
        Self {
            conversation: ConversationDraft::ROOT.get(conversation, cx),
            network: NetworkDraft::ROOT.get(network, cx),
            tools: ToolsDraft::ROOT.get(tools, cx),
        }
    }
    pub(super) fn value(&self, field: Field) -> Vec<Slot<Value>> {
        match field {
            Field::Model => vec![
                self.conversation.default_provider.encode(),
                self.conversation.default_model.encode(),
            ],
            Field::DefaultTools => vec![self.tools.default_tools.encode()],
            Field::CodemodeMode => vec![self.tools.codemode_mode.encode()],
            Field::CodemodeBudget => vec![self.tools.codemode_budget.encode()],
            Field::ShellPath => vec![self.tools.shell_path.encode()],
            Field::ShellPrefix => vec![self.tools.shell_prefix.encode()],
            Field::NpmCommand => vec![self.tools.npm_command.encode()],
            Field::Telemetry => vec![self.tools.telemetry.encode()],
            Field::ProjectTrust => vec![self.tools.project_trust.encode()],
            Field::ModelThinking => vec![self.conversation.model_thinking.encode()],
            Field::ModelOverrides => vec![self.conversation.model_overrides.encode()],
            Field::BudgetMinimal => vec![self.conversation.budget_minimal.encode()],
            Field::BudgetLow => vec![self.conversation.budget_low.encode()],
            Field::BudgetMedium => vec![self.conversation.budget_medium.encode()],
            Field::BudgetHigh => vec![self.conversation.budget_high.encode()],
            Field::CacheWarming => vec![self.conversation.cache_warming.encode()],
            Field::Transport => vec![self.network.transport.encode()],
            Field::HttpTimeout => vec![self.network.http_timeout.encode()],
            Field::WebsocketTimeout => vec![self.network.websocket_timeout.encode()],
            Field::ProviderTimeout => vec![self.network.provider_timeout.encode()],
            Field::ProviderRetries => vec![self.network.provider_retries.encode()],
            Field::ProviderDelay => vec![self.network.provider_delay.encode()],
            Field::Thinking => vec![self.conversation.thinking.encode()],
            Field::Compaction => vec![self.conversation.compaction.encode()],
            Field::ReserveTokens => vec![self.conversation.reserve_tokens.encode()],
            Field::KeepRecentTokens => vec![self.conversation.keep_recent_tokens.encode()],
            Field::Steering => vec![self.conversation.steering.encode()],
            Field::FollowUp => vec![self.conversation.follow_up.encode()],
            Field::AutoResize => vec![self.conversation.auto_resize.encode()],
            Field::BlockImages => vec![self.conversation.block_images.encode()],
            Field::Proxy => vec![self.network.http_proxy.encode()],
            Field::Retry => vec![self.network.retry.encode()],
            Field::MaxRetries => vec![self.network.max_retries.encode()],
            Field::BaseDelay => vec![self.network.base_delay.encode()],
            Field::MaxDelay => vec![self.network.max_delay.encode()],
            Field::ModelScope => vec![match &self.conversation.enabled_models {
                ModelScopeSlot::Unset => Slot::Unset,
                ModelScopeSlot::Set(scope) => Slot::Set(serde_json::json!(
                    scope.patterns.iter().map(|p| &p.text).collect::<Vec<_>>()
                )),
                ModelScopeSlot::Unreadable(raw) => Slot::Unreadable(raw.value.clone()),
            }],
        }
    }
    pub(super) fn set(&mut self, field: Field, values: Vec<Slot<Value>>) {
        let values = values.into_iter().map(|slot| match slot {
            Slot::Set(value)
                if matches!(field.kind(), super::fields::Kind::Count)
                    && !value
                        .as_u64()
                        .is_some_and(|n| n <= super::fields::MAX_SAFE_INTEGER) =>
            {
                Slot::Unreadable(value)
            }
            Slot::Set(value)
                if matches!(field.kind(), super::fields::Kind::Timeout)
                    && super::advanced::timeout(&value).is_none() =>
            {
                Slot::Unreadable(value)
            }
            slot => slot,
        });
        let mut values = values;
        match field {
            Field::Model => {
                self.conversation.default_provider =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
                self.conversation.default_model =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::DefaultTools => {
                self.tools.default_tools =
                    super::lists::ListSlot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::CodemodeMode => {
                self.tools.codemode_mode = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::CodemodeBudget => {
                self.tools.codemode_budget = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ShellPath => {
                self.tools.shell_path = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ShellPrefix => {
                self.tools.shell_prefix = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::NpmCommand => {
                self.tools.npm_command = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Telemetry => {
                self.tools.telemetry = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ProjectTrust => {
                self.tools.project_trust = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ModelThinking => {
                self.conversation.model_thinking =
                    super::model_maps::ThinkingSlot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ModelOverrides => {
                self.conversation.model_overrides =
                    super::model_maps::CompactionSlot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BudgetMinimal => {
                self.conversation.budget_minimal =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BudgetLow => {
                self.conversation.budget_low = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BudgetMedium => {
                self.conversation.budget_medium =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BudgetHigh => {
                self.conversation.budget_high = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::CacheWarming => {
                self.conversation.cache_warming =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Transport => {
                self.network.transport = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::HttpTimeout => {
                self.network.http_timeout = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::WebsocketTimeout => {
                self.network.websocket_timeout = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ProviderTimeout => {
                self.network.provider_timeout = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ProviderRetries => {
                self.network.provider_retries = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ProviderDelay => {
                self.network.provider_delay = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Thinking => {
                self.conversation.thinking = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Compaction => {
                self.conversation.compaction = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ReserveTokens => {
                self.conversation.reserve_tokens =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::KeepRecentTokens => {
                self.conversation.keep_recent_tokens =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Steering => {
                self.conversation.steering = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::FollowUp => {
                self.conversation.follow_up = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::AutoResize => {
                self.conversation.auto_resize = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BlockImages => {
                self.conversation.block_images = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Proxy => {
                self.network.http_proxy = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::Retry => {
                self.network.retry = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::MaxRetries => {
                self.network.max_retries = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::BaseDelay => {
                self.network.base_delay = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::MaxDelay => {
                self.network.max_delay = Slot::decode(values.next().unwrap_or(Slot::Unset));
            }
            Field::ModelScope => {
                self.conversation.enabled_models = match values.next().unwrap_or(Slot::Unset) {
                    Slot::Unset => ModelScopeSlot::Unset,
                    Slot::Set(value) | Slot::Unreadable(value) => {
                        match serde_json::from_value::<Vec<String>>(value.clone()) {
                            Ok(rows) => ModelScopeSlot::Set(ModelScope {
                                patterns: rows.into_iter().map(|text| Pattern { text }).collect(),
                            }),
                            Err(_) => ModelScopeSlot::Unreadable(RawValue { value }),
                        }
                    }
                    Slot::Invalid(_) => {
                        unreachable!("model scope stores invalid patterns as strings")
                    }
                }
            }
        }
    }
    pub(super) fn leaf_values(
        &self,
        field: Field,
    ) -> std::collections::BTreeMap<Vec<String>, Slot<Value>> {
        match field {
            Field::ModelThinking => self.conversation.model_thinking.leaves(),
            Field::ModelOverrides => self.conversation.model_overrides.leaves(),
            _ => field
                .leaves()
                .iter()
                .zip(self.value(field))
                .map(|(leaf, value)| (leaf.iter().map(|s| (*s).to_owned()).collect(), value))
                .collect(),
        }
    }

    pub(super) fn pending(&self, field: Field, baseline: &Self) -> bool {
        self.leaf_values(field) != baseline.leaf_values(field)
    }
    pub(super) fn invalid(&self, field: Field, baseline: &Self) -> bool {
        let previous = baseline.leaf_values(field);
        self.leaf_values(field).iter().any(|(leaf, value)| {
            previous.get(leaf).unwrap_or(&Slot::Unset) != value && matches!(value, Slot::Invalid(_))
        }) || self.pending(field, baseline)
            && (field == Field::ModelScope
                && matches!(&self.conversation.enabled_models, ModelScopeSlot::Set(s) if s.patterns.iter().any(|p| p.text.trim().is_empty()))
                || field == Field::NpmCommand
                    && matches!(&self.tools.npm_command, Slot::Set(s) if s.first().is_some_and(|i| i.is_empty())))
    }
    pub(super) fn merge(current: &Self, next: &Self, since: &Self) -> Self {
        let mut merged = next.clone();
        for field in Field::ALL {
            if matches!(field, Field::ModelThinking | Field::ModelOverrides) {
                continue;
            }
            if current.value(field) != since.value(field) {
                merged.set(field, current.value(field));
            }
        }
        merged.conversation.model_thinking = super::model_maps::ThinkingSlot::merge(
            &current.conversation.model_thinking,
            &next.conversation.model_thinking,
            &since.conversation.model_thinking,
        );
        merged.conversation.model_overrides = super::model_maps::CompactionSlot::merge(
            &current.conversation.model_overrides,
            &next.conversation.model_overrides,
            &since.conversation.model_overrides,
        );
        merged
    }
    pub(super) fn install_field(
        &self,
        field: Field,
        conversation: &Entity<Form<ConversationDraft>>,
        network: &Entity<Form<NetworkDraft>>,
        tools: &Entity<Form<ToolsDraft>>,
        cx: &mut App,
    ) {
        match field {
            Field::Model => {
                ConversationDraft::DEFAULT_PROVIDER.set(
                    conversation,
                    self.conversation.default_provider.clone(),
                    cx,
                );
                ConversationDraft::DEFAULT_MODEL.set(
                    conversation,
                    self.conversation.default_model.clone(),
                    cx,
                );
            }
            Field::DefaultTools => {
                ToolsDraft::DEFAULT_TOOLS.set(tools, self.tools.default_tools.clone(), cx);
            }
            Field::CodemodeMode => {
                ToolsDraft::CODEMODE_MODE.set(tools, self.tools.codemode_mode.clone(), cx);
            }
            Field::CodemodeBudget => {
                ToolsDraft::CODEMODE_BUDGET.set(tools, self.tools.codemode_budget.clone(), cx);
            }
            Field::ShellPath => {
                ToolsDraft::SHELL_PATH.set(tools, self.tools.shell_path.clone(), cx);
            }
            Field::ShellPrefix => {
                ToolsDraft::SHELL_PREFIX.set(tools, self.tools.shell_prefix.clone(), cx);
            }
            Field::NpmCommand => {
                ToolsDraft::NPM_COMMAND.set(tools, self.tools.npm_command.clone(), cx);
            }
            Field::Telemetry => {
                ToolsDraft::TELEMETRY.set(tools, self.tools.telemetry.clone(), cx);
            }
            Field::ProjectTrust => {
                ToolsDraft::PROJECT_TRUST.set(tools, self.tools.project_trust.clone(), cx);
            }
            Field::ModelThinking => {
                ConversationDraft::MODEL_THINKING.set(
                    conversation,
                    self.conversation.model_thinking.clone(),
                    cx,
                );
            }
            Field::ModelOverrides => {
                ConversationDraft::MODEL_OVERRIDES.set(
                    conversation,
                    self.conversation.model_overrides.clone(),
                    cx,
                );
            }
            Field::BudgetMinimal => {
                ConversationDraft::BUDGET_MINIMAL.set(
                    conversation,
                    self.conversation.budget_minimal.clone(),
                    cx,
                );
            }
            Field::BudgetLow => {
                ConversationDraft::BUDGET_LOW.set(
                    conversation,
                    self.conversation.budget_low.clone(),
                    cx,
                );
            }
            Field::BudgetMedium => {
                ConversationDraft::BUDGET_MEDIUM.set(
                    conversation,
                    self.conversation.budget_medium.clone(),
                    cx,
                );
            }
            Field::BudgetHigh => {
                ConversationDraft::BUDGET_HIGH.set(
                    conversation,
                    self.conversation.budget_high.clone(),
                    cx,
                );
            }
            Field::CacheWarming => {
                ConversationDraft::CACHE_WARMING.set(
                    conversation,
                    self.conversation.cache_warming.clone(),
                    cx,
                );
            }
            Field::Transport => {
                NetworkDraft::TRANSPORT.set(network, self.network.transport.clone(), cx);
            }
            Field::HttpTimeout => {
                NetworkDraft::HTTP_TIMEOUT.set(network, self.network.http_timeout.clone(), cx);
            }
            Field::WebsocketTimeout => {
                NetworkDraft::WEBSOCKET_TIMEOUT.set(
                    network,
                    self.network.websocket_timeout.clone(),
                    cx,
                );
            }
            Field::ProviderTimeout => {
                NetworkDraft::PROVIDER_TIMEOUT.set(
                    network,
                    self.network.provider_timeout.clone(),
                    cx,
                );
            }
            Field::ProviderRetries => {
                NetworkDraft::PROVIDER_RETRIES.set(
                    network,
                    self.network.provider_retries.clone(),
                    cx,
                );
            }
            Field::ProviderDelay => {
                NetworkDraft::PROVIDER_DELAY.set(network, self.network.provider_delay.clone(), cx);
            }
            Field::Thinking => {
                ConversationDraft::THINKING.set(
                    conversation,
                    self.conversation.thinking.clone(),
                    cx,
                );
            }
            Field::Compaction => {
                ConversationDraft::COMPACTION.set(
                    conversation,
                    self.conversation.compaction.clone(),
                    cx,
                );
            }
            Field::ReserveTokens => {
                ConversationDraft::RESERVE_TOKENS.set(
                    conversation,
                    self.conversation.reserve_tokens.clone(),
                    cx,
                );
            }
            Field::KeepRecentTokens => {
                ConversationDraft::KEEP_RECENT_TOKENS.set(
                    conversation,
                    self.conversation.keep_recent_tokens.clone(),
                    cx,
                );
            }
            Field::Steering => {
                ConversationDraft::STEERING.set(
                    conversation,
                    self.conversation.steering.clone(),
                    cx,
                );
            }
            Field::FollowUp => {
                ConversationDraft::FOLLOW_UP.set(
                    conversation,
                    self.conversation.follow_up.clone(),
                    cx,
                );
            }
            Field::AutoResize => {
                ConversationDraft::AUTO_RESIZE.set(
                    conversation,
                    self.conversation.auto_resize.clone(),
                    cx,
                );
            }
            Field::BlockImages => {
                ConversationDraft::BLOCK_IMAGES.set(
                    conversation,
                    self.conversation.block_images.clone(),
                    cx,
                );
            }
            Field::Proxy => {
                NetworkDraft::HTTP_PROXY.set(network, self.network.http_proxy.clone(), cx);
            }
            Field::Retry => {
                NetworkDraft::RETRY.set(network, self.network.retry.clone(), cx);
            }
            Field::MaxRetries => {
                NetworkDraft::MAX_RETRIES.set(network, self.network.max_retries.clone(), cx);
            }
            Field::BaseDelay => {
                NetworkDraft::BASE_DELAY.set(network, self.network.base_delay.clone(), cx);
            }
            Field::MaxDelay => {
                NetworkDraft::MAX_DELAY.set(network, self.network.max_delay.clone(), cx);
            }
            Field::ModelScope => {
                ConversationDraft::ENABLED_MODELS.set(
                    conversation,
                    self.conversation.enabled_models.clone(),
                    cx,
                );
            }
        }
    }
}
