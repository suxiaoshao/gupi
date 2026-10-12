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
            Slot::Set(value) | Slot::Unreadable(value) => {
                match serde_json::from_value(value.clone()) {
                    Ok(value) => Self::Set(value),
                    Err(_) => Self::Unreadable(value),
                }
            }
            Slot::Invalid(text) => Self::Invalid(text),
        }
    }
    fn encode(&self) -> Slot<Value> {
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
    pub(super) http_proxy: Slot<String>,
    pub(super) retry: Slot<bool>,
    pub(super) max_retries: Slot<u64>,
    pub(super) base_delay: Slot<u64>,
    pub(super) max_delay: Slot<u64>,
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
    value: Value,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct Snapshot {
    pub(super) conversation: ConversationDraft,
    pub(super) network: NetworkDraft,
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
        cx: &App,
    ) -> Self {
        Self {
            conversation: ConversationDraft::ROOT.get(conversation, cx),
            network: NetworkDraft::ROOT.get(network, cx),
        }
    }
    pub(super) fn value(&self, field: Field) -> Vec<Slot<Value>> {
        match field {
            Field::Model => vec![
                self.conversation.default_provider.encode(),
                self.conversation.default_model.encode(),
            ],
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
        let mut values = values.into_iter();
        match field {
            Field::Model => {
                self.conversation.default_provider =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
                self.conversation.default_model =
                    Slot::decode(values.next().unwrap_or(Slot::Unset));
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
    pub(super) fn pending(&self, field: Field, baseline: &Self) -> bool {
        self.value(field) != baseline.value(field)
    }
    pub(super) fn invalid(&self, field: Field, baseline: &Self) -> bool {
        self.pending(field, baseline)
            && (self
                .value(field)
                .iter()
                .any(|v| matches!(v, Slot::Invalid(_)))
                || field == Field::ModelScope
                    && matches!(&self.conversation.enabled_models, ModelScopeSlot::Set(s) if s.patterns.iter().any(|p| p.text.trim().is_empty())))
    }
    pub(super) fn merge(current: &Self, next: &Self, since: &Self) -> Self {
        let mut merged = next.clone();
        for field in Field::ALL {
            if current.value(field) != since.value(field) {
                merged.set(field, current.value(field));
            }
        }
        merged
    }
    pub(super) fn install_field(
        &self,
        field: Field,
        conversation: &Entity<Form<ConversationDraft>>,
        network: &Entity<Form<NetworkDraft>>,
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
