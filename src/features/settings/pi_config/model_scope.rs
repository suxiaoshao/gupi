//! Ordered model patterns use form-owned occurrence paths.
use super::*;
use fields::Source;
use forms::{ModelScope, ModelScopeSlot, Pattern};
use gpui_form::{ControlBinding, ControlProjection, DynamicItemsPath, ItemPath, PathKey};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gupi_settings::assets::IconName;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum RowKey {
    Local(PathKey),
    Inherited(u64, usize),
}
impl RowKey {
    fn element_id(&self) -> ElementId {
        match self {
            Self::Local(key) => key.into(),
            Self::Inherited(generation, index) => {
                SharedString::from(format!("inherited-pattern-{generation}-{index}")).into()
            }
        }
    }
}
pub(super) struct Adapter {
    pub(super) input: Entity<InputState>,
    _binding: Option<ControlBinding>,
    _subscription: Subscription,
}

impl PiConfig {
    pub(super) fn model_scope_rows(&self, cx: &App) -> Option<Vec<String>> {
        if !matches!(self.files, Load::Ready(_)) {
            return None;
        }
        match self.resolved(Field::ModelScope, cx).values[0].as_ref() {
            None => Some(vec![]),
            Some(value) => serde_json::from_value(value.clone()).ok(),
        }
    }

    fn patterns(&self, cx: &App) -> Option<DynamicItemsPath<ConversationDraft, Pattern>> {
        ConversationDraft::ENABLED_MODELS
            .case(ModelScopeSlot::SET)
            .resolve(&self.conversation, cx)
            .ok()
            .flatten()
            .map(|scope| scope.then(ModelScope::PATTERNS))
    }

    pub(super) fn row_keys(&self, cx: &App) -> Vec<RowKey> {
        if let Some(patterns) = self.patterns(cx) {
            patterns
                .try_items(&self.conversation, cx)
                .expect("active collection")
                .into_iter()
                .map(|p| RowKey::Local(p.key()))
                .collect()
        } else {
            (0..self.model_scope_rows(cx).map_or(0, |rows| rows.len()))
                .map(|index| RowKey::Inherited(self.generation, index))
                .collect()
        }
    }

    fn bind_row(
        &self,
        input: Entity<InputState>,
        path: ItemPath<ConversationDraft, Pattern>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Adapter {
        let text = path.then(Pattern::TEXT);
        let (binding, writer) = text
            .try_bind_control_in(
                &self.conversation,
                &input,
                |input, projection, window, cx| {
                    if let ControlProjection::Value(value) = projection
                        && input.value().as_ref() != value
                    {
                        input.set_value(value, window, cx);
                    }
                },
                window,
                cx,
            )
            .expect("active row path");
        let subscription = cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
            if matches!(event, InputEvent::Change) {
                writer.defer_set(input.read(cx).value().to_string(), window, cx);
            }
        });
        Adapter {
            input,
            _binding: Some(binding),
            _subscription: subscription,
        }
    }

    pub(super) fn row_input(
        &mut self,
        key: &RowKey,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(adapter) = self.rows.get(key) {
            return adapter.input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx).default_value(text.to_owned()));
        let adapter = match key {
            RowKey::Local(key) => {
                let path = self
                    .patterns(cx)
                    .unwrap()
                    .try_items(&self.conversation, cx)
                    .expect("active collection")
                    .into_iter()
                    .find(|path| &path.key() == key)
                    .expect("active row");
                self.bind_row(input.clone(), path, window, cx)
            }
            RowKey::Inherited(generation, index) => {
                let generation = *generation;
                let index = *index;
                let key = key.clone();
                let subscription =
                    cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
                        if !matches!(event, InputEvent::Change) {
                            return;
                        }
                        let input = input.clone();
                        let key = key.clone();
                        cx.defer_in(window, move |this, window, cx| {
                            if this.generation != generation
                                || this.is_saving()
                                || this
                                    .rows
                                    .get(&key)
                                    .is_none_or(|adapter| adapter.input != input)
                            {
                                return;
                            }
                            let text = input.read(cx).value().to_string();
                            if this
                                .model_scope_rows(cx)
                                .and_then(|rows| rows.get(index).cloned())
                                .as_deref()
                                == Some(&text)
                            {
                                return;
                            }
                            if let Some(path) = this
                                .materialize(window, cx)
                                .and_then(|paths| paths.get(index).cloned())
                            {
                                let _ =
                                    path.then(Pattern::TEXT)
                                        .try_set(&this.conversation, text, cx);
                            }
                        });
                    });
                Adapter {
                    input: input.clone(),
                    _binding: None,
                    _subscription: subscription,
                }
            }
        };
        self.rows.insert(key.clone(), adapter);
        input
    }

    /// Pin inherited rows only for a real edit, transferring their native entities.
    fn materialize(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Vec<ItemPath<ConversationDraft, Pattern>>> {
        if self.is_saving() {
            return None;
        }
        if self.patterns(cx).is_none() {
            let rows = self.model_scope_rows(cx)?;
            ConversationDraft::ENABLED_MODELS.set(
                &self.conversation,
                ModelScopeSlot::Set(ModelScope {
                    patterns: rows.into_iter().map(|text| Pattern { text }).collect(),
                }),
                cx,
            );
            let paths = self
                .patterns(cx)?
                .try_items(&self.conversation, cx)
                .expect("active collection");
            for (index, path) in paths.iter().enumerate() {
                if let Some(adapter) = self.rows.remove(&RowKey::Inherited(self.generation, index))
                {
                    let input = adapter.input.clone();
                    drop(adapter);
                    let adapter = self.bind_row(input, path.clone(), window, cx);
                    self.rows.insert(RowKey::Local(path.key()), adapter);
                }
            }
        }
        Some(
            self.patterns(cx)?
                .try_items(&self.conversation, cx)
                .expect("active collection"),
        )
    }

    fn path_for(
        &mut self,
        key: &RowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<ItemPath<ConversationDraft, Pattern>> {
        match key {
            RowKey::Local(key) => self
                .patterns(cx)?
                .try_items(&self.conversation, cx)
                .expect("active collection")
                .into_iter()
                .find(|path| &path.key() == key),
            RowKey::Inherited(generation, index)
                if *generation == self.generation && self.patterns(cx).is_none() =>
            {
                self.materialize(window, cx)?.get(*index).cloned()
            }
            _ => None,
        }
    }

    pub(super) fn append_pattern(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.materialize(window, cx).is_some() {
            let _ = self
                .patterns(cx)
                .unwrap()
                .append(&self.conversation, Pattern { text }, cx);
            cx.notify();
        }
    }

    pub(super) fn move_pattern(
        &mut self,
        key: &RowKey,
        down: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        let Some(path) = self.path_for(key, window, cx) else {
            return;
        };
        let patterns = self.patterns(cx).unwrap();
        let paths = patterns
            .try_items(&self.conversation, cx)
            .expect("active collection");
        let Some(index) = paths.iter().position(|p| p.key() == path.key()) else {
            return;
        };
        if down && index + 1 < paths.len() {
            let _ = patterns.move_before(&self.conversation, &paths[index + 1], &path, cx);
        } else if !down && index > 0 {
            let _ = patterns.move_before(&self.conversation, &path, &paths[index - 1], cx);
        }
        cx.notify();
    }

    pub(super) fn remove_pattern(
        &mut self,
        key: &RowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        let Some(path) = self.path_for(key, window, cx) else {
            return;
        };
        let patterns = self.patterns(cx).unwrap();
        self.rows.remove(&RowKey::Local(path.key()));
        let _ = patterns.remove(&self.conversation, path, cx);
        if patterns
            .try_items(&self.conversation, cx)
            .expect("active collection")
            .is_empty()
            && matches!(self.scope, Scope::Global)
        {
            ConversationDraft::ENABLED_MODELS.set(&self.conversation, ModelScopeSlot::Unset, cx);
        }
        cx.notify();
    }

    pub(super) fn render_model_scope(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Load::Ready(_) = &self.files else {
            return div().into_any_element();
        };
        let field = Field::ModelScope;
        let pending = self.pending(field, cx);
        let resolved = self.resolved(field, cx);
        let Some(rows) = self.model_scope_rows(cx) else {
            return v_flex()
                .gap_1()
                .child(t(cx, "pi-model-scope"))
                .child(
                    div()
                        .text_sm()
                        .child(fields::display(resolved.values[0].as_ref())),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "pi-model-scope-malformed")),
                )
                .into_any_element();
        };
        let saving = self.is_saving();
        let count = rows.len();
        let mut source = t(
            cx,
            match resolved.source {
                Source::Default => "pi-source-default",
                Source::Global => "pi-source-global",
                Source::Project => "pi-source-project",
            },
        );
        if pending {
            source = format!("{source} · {}", t(cx, "pi-unsaved"));
        }
        let mut list = v_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(t(cx, "pi-model-scope")))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(source),
                    )
                    .child(self.render_field_menu(field, pending, saving, cx)),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-model-scope-help")),
            );
        let keys = self.row_keys(cx);
        self.rows.retain(|key, _| keys.contains(key));
        for (index, (row, text)) in keys.into_iter().zip(&rows).enumerate() {
            let input = self.row_input(&row, text, window, cx);
            let up = row.clone();
            let down = row.clone();
            let delete = row.clone();
            list = list.child(
                h_flex()
                    .id(row.element_id())
                    .gap_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&input).small().disabled(saving)),
                    )
                    .child(
                        Button::new("model-scope-up")
                            .small()
                            .ghost()
                            .icon(IconName::ChevronUp)
                            .accessibility_label(t(cx, "pi-model-scope-up"))
                            .tooltip(t(cx, "pi-model-scope-up"))
                            .disabled(saving || index == 0)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.move_pattern(&up, false, window, cx);
                            })),
                    )
                    .child(
                        Button::new("model-scope-down")
                            .small()
                            .ghost()
                            .icon(IconName::ChevronDown)
                            .accessibility_label(t(cx, "pi-model-scope-down"))
                            .tooltip(t(cx, "pi-model-scope-down"))
                            .disabled(saving || index + 1 == count)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.move_pattern(&down, true, window, cx);
                            })),
                    )
                    .child(
                        Button::new("model-scope-delete")
                            .small()
                            .ghost()
                            .icon(IconName::Trash)
                            .accessibility_label(t(cx, "pi-model-scope-delete"))
                            .tooltip(t(cx, "pi-model-scope-delete"))
                            .disabled(saving)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_pattern(&delete, window, cx);
                            })),
                    ),
            );
        }
        if rows.is_empty() {
            list = list.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if resolved.source == Source::Project {
                            "pi-model-scope-unrestricted-project"
                        } else {
                            "pi-model-scope-unrestricted"
                        },
                    )),
            );
        }
        list = list.child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .child(
                    Select::new(&self.scope_picker)
                        .small()
                        .w_64()
                        .placeholder(t(cx, "pi-model-scope-add-model"))
                        .search_placeholder(t(cx, "conversation-model-search"))
                        .accessibility_label(t(cx, "pi-model-scope-add-model"))
                        .disabled(saving || !matches!(self.models, Models::Ready(_))),
                )
                .child(
                    Button::new("model-scope-add-pattern")
                        .small()
                        .label(t(cx, "pi-model-scope-add-pattern"))
                        .disabled(saving)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.append_pattern(String::new(), window, cx)
                        })),
                ),
        );
        if self.invalid(field, cx) || self.conflicts.contains(&field) {
            list = list.child(div().text_sm().text_color(cx.theme().danger).child(t(
                cx,
                if self.conflicts.contains(&field) {
                    "pi-changed-on-disk"
                } else {
                    "pi-model-scope-blank"
                },
            )));
        }
        list.into_any_element()
    }
}
