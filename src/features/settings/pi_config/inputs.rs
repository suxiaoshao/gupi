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
        display(self.resolved(field, cx).values[0].as_ref())
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
    match field.kind() {
        Kind::Count => match super::fields::parse_count(&text) {
            Some(n) => Slot::Set(n),
            None => Slot::Invalid(text),
        },
        _ if text.trim().is_empty() => Slot::Unset,
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
        let baseline_display = display(files.resolve(field, None).values[0].as_ref());
        let value = if matches!(field.kind(), Kind::Count)
            && (text == baseline_display
                || matches!(&parsed, Slot::Set(value) if display(Some(value)) == baseline_display))
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
