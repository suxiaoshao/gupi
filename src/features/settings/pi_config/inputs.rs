//! Scope-local form bindings project effective inherited values into native inputs.
use super::fields::{Kind, display};
use super::*;
use gpui_form::{ControlBinding, ControlProjection, FormSchema, IntoTotalPath};
use gpui_kit::component::input::{InputEvent, InputState};
use serde::{Serialize, de::DeserializeOwned};

pub(super) struct Adapter {
    pub(super) input: Entity<InputState>,
    window: AnyWindowHandle,
    _binding: ControlBinding,
    _subscription: Subscription,
}
impl PiConfig {
    pub(super) fn refresh_inputs(&self, cx: &mut Context<Self>) {
        self.refresh_map_inputs(cx);
        let config = cx.entity().downgrade();
        for (field, adapter) in &self.inputs {
            let field = *field;
            let window = adapter.window;
            let input = adapter.input.clone();
            let config = config.clone();
            cx.defer(move |cx| {
                let Some(config) = config.upgrade() else {
                    return;
                };
                if !matches!(config.read(cx).files, Load::Ready(_)) {
                    return;
                }
                let shown = config.read(cx).shown(field, cx);
                let _ = window.update(cx, |_, window, cx| {
                    input.update(cx, |input, cx| {
                        if input.value().as_ref() != shown {
                            input.set_value(shown, window, cx);
                        }
                    })
                });
            });
        }
    }

    pub(super) fn shown(&self, field: Field, cx: &App) -> String {
        if let Some(Slot::Invalid(text)) = self.snapshot(cx).value(field).first() {
            return text.clone();
        }
        let resolved = self.resolved(field, cx);
        if matches!(field.kind(), Kind::Timeout)
            && resolved.values[0]
                .as_ref()
                .and_then(super::advanced::timeout)
                == Some(0.)
        {
            return t(cx, "pi-timeout-off");
        }
        if field == Field::NpmCommand {
            return npm_display(resolved.values[0].as_ref());
        }
        display(resolved.values[0].as_ref())
    }

    pub(super) fn input(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(adapter) = self.inputs.get(&field) {
            return adapter.input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx).default_value(self.shown(field, cx)));
        let adapter = match field {
            Field::NpmCommand => bind(
                &self.tools,
                ToolsDraft::NPM_COMMAND,
                field,
                &input,
                window,
                cx,
            ),
            Field::CodemodeBudget => bind(
                &self.tools,
                ToolsDraft::CODEMODE_BUDGET,
                field,
                &input,
                window,
                cx,
            ),
            Field::ShellPath => bind(
                &self.tools,
                ToolsDraft::SHELL_PATH,
                field,
                &input,
                window,
                cx,
            ),
            Field::ShellPrefix => bind(
                &self.tools,
                ToolsDraft::SHELL_PREFIX,
                field,
                &input,
                window,
                cx,
            ),
            Field::BudgetMinimal => bind(
                &self.conversation,
                ConversationDraft::BUDGET_MINIMAL,
                field,
                &input,
                window,
                cx,
            ),
            Field::BudgetLow => bind(
                &self.conversation,
                ConversationDraft::BUDGET_LOW,
                field,
                &input,
                window,
                cx,
            ),
            Field::BudgetMedium => bind(
                &self.conversation,
                ConversationDraft::BUDGET_MEDIUM,
                field,
                &input,
                window,
                cx,
            ),
            Field::BudgetHigh => bind(
                &self.conversation,
                ConversationDraft::BUDGET_HIGH,
                field,
                &input,
                window,
                cx,
            ),
            Field::HttpTimeout => bind(
                &self.network,
                NetworkDraft::HTTP_TIMEOUT,
                field,
                &input,
                window,
                cx,
            ),
            Field::WebsocketTimeout => bind(
                &self.network,
                NetworkDraft::WEBSOCKET_TIMEOUT,
                field,
                &input,
                window,
                cx,
            ),
            Field::ProviderTimeout => bind(
                &self.network,
                NetworkDraft::PROVIDER_TIMEOUT,
                field,
                &input,
                window,
                cx,
            ),
            Field::ProviderRetries => bind(
                &self.network,
                NetworkDraft::PROVIDER_RETRIES,
                field,
                &input,
                window,
                cx,
            ),
            Field::ProviderDelay => bind(
                &self.network,
                NetworkDraft::PROVIDER_DELAY,
                field,
                &input,
                window,
                cx,
            ),
            Field::ReserveTokens => bind(
                &self.conversation,
                ConversationDraft::RESERVE_TOKENS,
                field,
                &input,
                window,
                cx,
            ),
            Field::KeepRecentTokens => bind(
                &self.conversation,
                ConversationDraft::KEEP_RECENT_TOKENS,
                field,
                &input,
                window,
                cx,
            ),
            Field::Proxy => bind(
                &self.network,
                NetworkDraft::HTTP_PROXY,
                field,
                &input,
                window,
                cx,
            ),
            Field::MaxRetries => bind(
                &self.network,
                NetworkDraft::MAX_RETRIES,
                field,
                &input,
                window,
                cx,
            ),
            Field::BaseDelay => bind(
                &self.network,
                NetworkDraft::BASE_DELAY,
                field,
                &input,
                window,
                cx,
            ),
            Field::MaxDelay => bind(
                &self.network,
                NetworkDraft::MAX_DELAY,
                field,
                &input,
                window,
                cx,
            ),
            _ => unreachable!("only text/count fields have inputs"),
        };
        self.inputs.insert(field, adapter);
        input
    }
}

fn parse(field: Field, text: String) -> Slot<Value> {
    if field == Field::NpmCommand {
        if text.is_empty() {
            return Slot::Unset;
        }
        return match shell_words::split(&text) {
            Ok(args) if !args.is_empty() => Slot::Set(serde_json::json!(args)),
            Ok(_) => Slot::Unset,
            Err(_) => Slot::Invalid(text),
        };
    }
    match field.kind() {
        Kind::Timeout => match super::advanced::timeout(&Value::String(text.clone())) {
            Some(value) => Slot::Set(Value::from(value)),
            None => Slot::Invalid(text),
        },
        Kind::Count => match super::fields::parse_count(&text) {
            Some(n) => Slot::Set(n),
            None => Slot::Invalid(text),
        },
        _ if text.is_empty()
            || (!matches!(field, Field::ShellPath | Field::ShellPrefix)
                && text.trim().is_empty()) =>
        {
            Slot::Unset
        }
        _ if matches!(field, Field::ShellPath | Field::ShellPrefix) => {
            Slot::Set(Value::String(text))
        }
        _ => Slot::Set(Value::String(text.trim().to_owned())),
    }
}

fn bind<M, T>(
    form: &Entity<Form<M>>,
    path: impl IntoTotalPath<M, Slot<T>>,
    field: Field,
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<PiConfig>,
) -> Adapter
where
    M: FormSchema,
    T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
    let projected_config = cx.entity().downgrade();
    let (binding, writer) = path.into_total_path().bind_control_in(
        form,
        input,
        move |input, projection, window, cx| {
            if let ControlProjection::Value(_) = projection
                && let Some(config) = projected_config.upgrade()
                && matches!(config.read(cx).files, Load::Ready(_))
            {
                let shown = config.read(cx).shown(field, cx);
                if input.value().as_ref() != shown {
                    input.set_value(shown, window, cx);
                }
            }
        },
        window,
        cx,
    );
    let subscription = cx.subscribe_in(input, window, move |this, input, event, window, cx| {
        if !matches!(event, InputEvent::Change) || !matches!(this.files, Load::Ready(_)) {
            return;
        }
        let text = input.read(cx).value().to_string();
        // Even an edit back to the authoritative display must enter the deferred
        // writer: an earlier native edit may still be queued. Preserve Unset/raw
        // values instead of parsing their effective display into an override.
        let current = this.snapshot(cx);
        let parsed = parse(field, text.clone());
        let Load::Ready(files) = &this.files else {
            return;
        };
        let baseline_values = files.resolve(field, None).values;
        let baseline_display = if field == Field::NpmCommand {
            npm_display(baseline_values[0].as_ref())
        } else {
            display(baseline_values[0].as_ref())
        };
        let value = if (matches!(field.kind(), Kind::Count | Kind::Timeout)
            || field == Field::NpmCommand)
            && (text == baseline_display
                || matches!(&parsed, Slot::Set(value) if (if field == Field::NpmCommand {npm_display(Some(value))} else {display(Some(value))}) == baseline_display))
        {
            // Correcting a typo back to the effective baseline restores inheritance
            // (or malformed raw data), rather than pinning a displayed default.
            this.baseline.value(field).remove(0)
        } else if this.shown(field, cx) == text {
            current.value(field).remove(0)
        } else {
            parsed
        };
        this.error = None;
        writer.defer_set(Slot::<T>::decode(value), window, cx);
        cx.notify();
    });
    Adapter {
        input: input.clone(),
        window: window.window_handle(),
        _binding: binding,
        _subscription: subscription,
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

#[cfg(test)]
mod command_tests {
    use super::{Field, Slot, parse};
    #[test]
    fn shell_command_round_trips_verbatim_arguments() {
        let samples = [
            "",
            " ",
            "two words",
            "'",
            "\"",
            "\\",
            "a\nb",
            "é中文",
            "$HOME; *",
        ];
        for args in [
            vec![],
            vec![""],
            vec![
                "",
                "with space",
                "'quoted'",
                "\"double\"",
                "\\",
                "line\nbreak",
            ],
        ] {
            assert_eq!(shell_words::split(&shell_words::join(&args)).unwrap(), args);
        }
        for a in samples {
            for b in samples {
                let args = vec!["npm".to_owned(), a.to_owned(), b.to_owned()];
                let shown = shell_words::join(&args);
                assert!(parse(Field::NpmCommand, shown) == Slot::Set(serde_json::json!(args)));
            }
        }
        assert!(matches!(
            parse(Field::NpmCommand, "npm 'oops".into()),
            Slot::Invalid(_)
        ));
        assert!(matches!(
            parse(Field::NpmCommand, "npm \"oops".into()),
            Slot::Invalid(_)
        ));
    }
}
