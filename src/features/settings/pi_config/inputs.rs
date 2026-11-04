//! Scope-local storage/display mappings. Form owns drafts and native admission.
use super::fields::{Kind, display};
use super::*;
use gpui_form::{ControlMapping, EditResult, FormSchema, IntoTotalPath};
use gpui_form_gpui_component::FormMappedInput;
use gpui_kit::component::input::InputState;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Clone)]
pub(super) struct MappingContext {
    field: Field,
    inherited: Option<Value>,
    off: String,
}
pub(super) struct Adapter {
    pub(super) input: FormMappedInput<MappingContext>,
}
impl PiConfig {
    fn mapping_context(&self, field: Field, cx: &App) -> MappingContext {
        let Load::Ready(files) = &self.files else {
            unreachable!("loaded mappings")
        };
        MappingContext {
            field,
            inherited: files.resolve(field, Some(&[Slot::Unset])).values.remove(0),
            off: t(cx, "pi-timeout-off"),
        }
    }
    pub(super) fn refresh_field_input(&self, field: Field, cx: &mut Context<Self>) {
        if let Some(adapter) = self.inputs.get(&field) {
            adapter.input.refresh(self.mapping_context(field, cx), cx);
        }
        if field == Field::ModelOverrides {
            self.refresh_map_inputs(cx);
        }
    }
    pub(super) fn refresh_inputs(&self, cx: &mut Context<Self>) {
        for (field, adapter) in &self.inputs {
            adapter.input.refresh(self.mapping_context(*field, cx), cx);
        }
        self.refresh_map_inputs(cx);
    }
    pub(super) fn input(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(adapter) = self.inputs.get(&field) {
            return (*adapter.input).clone();
        }
        let context = self.mapping_context(field, cx);
        let adapter = match field {
            Field::CodemodeBudget => bind(
                &self.tools,
                ToolsDraft::CODEMODE_BUDGET,
                context,
                window,
                cx,
            ),
            Field::ShellPath => bind(&self.tools, ToolsDraft::SHELL_PATH, context, window, cx),
            Field::ShellPrefix => bind(&self.tools, ToolsDraft::SHELL_PREFIX, context, window, cx),
            Field::NpmCommand => bind(&self.tools, ToolsDraft::NPM_COMMAND, context, window, cx),
            Field::BudgetMinimal => bind(
                &self.conversation,
                ConversationDraft::BUDGET_MINIMAL,
                context,
                window,
                cx,
            ),
            Field::BudgetLow => bind(
                &self.conversation,
                ConversationDraft::BUDGET_LOW,
                context,
                window,
                cx,
            ),
            Field::BudgetMedium => bind(
                &self.conversation,
                ConversationDraft::BUDGET_MEDIUM,
                context,
                window,
                cx,
            ),
            Field::BudgetHigh => bind(
                &self.conversation,
                ConversationDraft::BUDGET_HIGH,
                context,
                window,
                cx,
            ),
            Field::HttpTimeout => bind(
                &self.network,
                NetworkDraft::HTTP_TIMEOUT,
                context,
                window,
                cx,
            ),
            Field::WebsocketTimeout => bind(
                &self.network,
                NetworkDraft::WEBSOCKET_TIMEOUT,
                context,
                window,
                cx,
            ),
            Field::ProviderTimeout => bind(
                &self.network,
                NetworkDraft::PROVIDER_TIMEOUT,
                context,
                window,
                cx,
            ),
            Field::ProviderRetries => bind(
                &self.network,
                NetworkDraft::PROVIDER_RETRIES,
                context,
                window,
                cx,
            ),
            Field::ProviderDelay => bind(
                &self.network,
                NetworkDraft::PROVIDER_DELAY,
                context,
                window,
                cx,
            ),
            Field::ReserveTokens => bind(
                &self.conversation,
                ConversationDraft::RESERVE_TOKENS,
                context,
                window,
                cx,
            ),
            Field::KeepRecentTokens => bind(
                &self.conversation,
                ConversationDraft::KEEP_RECENT_TOKENS,
                context,
                window,
                cx,
            ),
            Field::Proxy => bind(&self.network, NetworkDraft::HTTP_PROXY, context, window, cx),
            Field::MaxRetries => bind(
                &self.network,
                NetworkDraft::MAX_RETRIES,
                context,
                window,
                cx,
            ),
            Field::BaseDelay => bind(&self.network, NetworkDraft::BASE_DELAY, context, window, cx),
            Field::MaxDelay => bind(&self.network, NetworkDraft::MAX_DELAY, context, window, cx),
            _ => unreachable!("only text/count fields have inputs"),
        };
        let input = (*adapter.input).clone();
        self.inputs.insert(field, adapter);
        input
    }
}
fn bind<M, T>(
    form: &Entity<Form<M>>,
    path: impl IntoTotalPath<M, Slot<T>>,
    context: MappingContext,
    window: &mut Window,
    cx: &mut Context<PiConfig>,
) -> Adapter
where
    M: FormSchema,
    T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
    let editor = gpui_form::EditorKey::new(format!("pi-{:?}", context.field));
    let input = FormMappedInput::new(
        form,
        path,
        editor,
        SlotMapping::<T>(std::marker::PhantomData),
        context,
        InputState::new,
        window,
        cx,
    )
    .expect("stable field mapping type");
    Adapter { input }
}
struct SlotMapping<T>(std::marker::PhantomData<T>);
impl<T: Clone + PartialEq + Serialize + DeserializeOwned + 'static> ControlMapping<Slot<T>, String>
    for SlotMapping<T>
{
    type Context = MappingContext;
    fn format(&self, stored: &Slot<T>, context: &MappingContext) -> String {
        let value = stored.encode().raw().or_else(|| context.inherited.clone());
        shown(context, value.as_ref())
    }
    fn parse(
        &self,
        text: String,
        current: &Slot<T>,
        baseline: Option<&Slot<T>>,
        context: &MappingContext,
    ) -> EditResult<Slot<T>, String> {
        let parsed = parse(context.field, &text);
        let normalized = parsed.as_ref().map(|v| shown(context, v.raw().as_ref()));
        if let Some(base) = baseline.filter(|base| {
            let display = self.format(base, context);
            text == display
                || (matches!(context.field.kind(), Kind::Count | Kind::Timeout)
                    || context.field == Field::NpmCommand)
                    && normalized.as_ref() == Some(&display)
        }) {
            return EditResult::Parsed(base.clone());
        }
        if text == self.format(current, context) {
            return EditResult::Parsed(current.clone());
        }
        match parsed {
            Some(value) => EditResult::Parsed(Slot::decode(value)),
            None => EditResult::draft(
                text,
                "pi-invalid-input",
                gpui_form::ValidationMessage::key(if context.field == Field::NpmCommand {
                    "pi-npm-invalid"
                } else if matches!(context.field.kind(), Kind::Count) {
                    "pi-invalid-count"
                } else {
                    "pi-settings-invalid"
                }),
            ),
        }
    }
}
fn shown(context: &MappingContext, value: Option<&Value>) -> String {
    if context.field == Field::NpmCommand {
        return npm_display(value);
    }
    if matches!(context.field.kind(), Kind::Timeout)
        && value.and_then(super::advanced::timeout) == Some(0.)
    {
        return context.off.clone();
    }
    display(value)
}
fn parse(field: Field, text: &str) -> Option<Slot<Value>> {
    if field == Field::NpmCommand {
        if text.is_empty() {
            return Some(Slot::Unset);
        }
        return match shell_words::split(text) {
            Ok(args) if !args.is_empty() => Some(Slot::Set(serde_json::json!(args))),
            Ok(_) => Some(Slot::Unset),
            Err(_) => None,
        };
    }
    match field.kind() {
        Kind::Timeout => {
            super::advanced::timeout(&Value::String(text.into())).map(|v| Slot::Set(Value::from(v)))
        }
        Kind::Count => super::fields::parse_count(text).map(Slot::Set),
        _ if text.is_empty()
            || (!matches!(field, Field::ShellPath | Field::ShellPrefix)
                && text.trim().is_empty()) =>
        {
            Some(Slot::Unset)
        }
        _ if matches!(field, Field::ShellPath | Field::ShellPrefix) => {
            Some(Slot::Set(Value::String(text.into())))
        }
        _ => Some(Slot::Set(Value::String(text.trim().into()))),
    }
}
fn npm_display(value: Option<&Value>) -> String {
    match value {
        None => "npm".into(),
        Some(Value::Array(args)) if args.is_empty() => "npm".into(),
        Some(value) => match serde_json::from_value::<Vec<String>>(value.clone()) {
            Ok(args) => shell_words::join(args),
            Err(_) => display(Some(value)),
        },
    }
}
