//! Separate controls for per-model thinking values and per-token compaction leaves.
use super::model_maps::*;
use super::*;
use fields::{Source, display, supported_levels};
use gpui_form::{ControlBinding, ControlProjection, DynamicItemsPath, ItemPath, PathKey};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gupi_settings::assets::IconName;
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum MapInputKey {
    Local(PathKey, bool),
    Inherited(u64, String, bool),
}
pub(super) struct MapAdapter {
    pub(super) input: Entity<InputState>,
    window: AnyWindowHandle,
    _binding: Option<ControlBinding>,
    _subscription: Subscription,
}
impl PiConfig {
    fn thinking_rows(&self, cx: &App) -> Option<DynamicItemsPath<ConversationDraft, ThinkingRow>> {
        ConversationDraft::MODEL_THINKING
            .case(ThinkingSlot::SET)
            .resolve(&self.conversation, cx)
            .ok()
            .flatten()
            .map(|p| p.then(ThinkingRows::ROWS))
    }
    fn override_rows(
        &self,
        cx: &App,
    ) -> Option<DynamicItemsPath<ConversationDraft, CompactionRow>> {
        ConversationDraft::MODEL_OVERRIDES
            .case(CompactionSlot::SET)
            .resolve(&self.conversation, cx)
            .ok()
            .flatten()
            .map(|p| p.then(CompactionRows::ROWS))
    }
    fn thinking_path(
        &self,
        key: &str,
        cx: &App,
    ) -> Option<ItemPath<ConversationDraft, ThinkingRow>> {
        self.thinking_rows(cx)?
            .try_items(&self.conversation, cx)
            .ok()?
            .into_iter()
            .find(|p| {
                p.clone()
                    .then(ThinkingRow::KEY)
                    .try_get(&self.conversation, cx)
                    .ok()
                    .as_deref()
                    == Some(key)
            })
    }
    fn override_path(
        &self,
        key: &str,
        cx: &App,
    ) -> Option<ItemPath<ConversationDraft, CompactionRow>> {
        self.override_rows(cx)?
            .try_items(&self.conversation, cx)
            .ok()?
            .into_iter()
            .find(|p| {
                p.clone()
                    .then(CompactionRow::KEY)
                    .try_get(&self.conversation, cx)
                    .ok()
                    .as_deref()
                    == Some(key)
            })
    }
    fn map_values(&self, field: Field, cx: &App) -> Option<serde_json::Map<String, Value>> {
        let Load::Ready(files) = &self.files else {
            return None;
        };
        let local = self.snapshot(cx).value(field)[0].raw();
        let inherited = if files.project.is_some() {
            files.global.get(field.leaves()[0])
        } else {
            None
        };
        let mut map = match inherited {
            Some(Value::Object(map)) => map.clone(),
            _ => Default::default(),
        };
        match local {
            Some(Value::Object(local)) => {
                for (key, value) in local {
                    if field == Field::ModelOverrides
                        && let (Some(Value::Object(old)), Value::Object(new)) =
                            (map.get_mut(&key), &value)
                    {
                        old.extend(new.clone());
                    } else {
                        map.insert(key, value);
                    }
                }
            }
            Some(_) => return None,
            None => {}
        }
        Some(map)
    }
    pub(super) fn ensure_thinking(
        &mut self,
        key: &str,
        cx: &mut Context<Self>,
    ) -> Option<ItemPath<ConversationDraft, ThinkingRow>> {
        if self.is_saving() {
            return None;
        }
        if let Some(path) = self.thinking_path(key, cx) {
            return Some(path);
        }
        if matches!(
            ConversationDraft::MODEL_THINKING.get(&self.conversation, cx),
            ThinkingSlot::Unreadable(_)
        ) {
            return None;
        }
        if self.thinking_rows(cx).is_none() {
            ConversationDraft::MODEL_THINKING.set(
                &self.conversation,
                ThinkingSlot::Set(ThinkingRows::default()),
                cx,
            );
        }
        self.thinking_rows(cx)?
            .append(
                &self.conversation,
                ThinkingRow {
                    key: key.into(),
                    level: Slot::Unset,
                },
                cx,
            )
            .ok()
    }
    pub(super) fn ensure_override(
        &mut self,
        key: &str,
        cx: &mut Context<Self>,
    ) -> Option<ItemPath<ConversationDraft, CompactionRow>> {
        if self.is_saving() {
            return None;
        }
        if let Some(path) = self.override_path(key, cx) {
            return Some(path);
        }
        if matches!(
            ConversationDraft::MODEL_OVERRIDES.get(&self.conversation, cx),
            CompactionSlot::Unreadable(_)
        ) {
            return None;
        }
        if self.override_rows(cx).is_none() {
            ConversationDraft::MODEL_OVERRIDES.set(
                &self.conversation,
                CompactionSlot::Set(CompactionRows::default()),
                cx,
            );
        }
        self.override_rows(cx)?
            .append(
                &self.conversation,
                CompactionRow {
                    key: key.into(),
                    reserve: Slot::Unset,
                    keep: Slot::Unset,
                    extra: Default::default(),
                    unreadable: None,
                },
                cx,
            )
            .ok()
    }
    pub(super) fn token_slot(&self, key: &str, keep: bool, cx: &App) -> Slot<u64> {
        let Some(path) = self.override_path(key, cx) else {
            return Slot::Unset;
        };
        let row = path.try_get(&self.conversation, cx).expect("active row");
        if keep { row.keep } else { row.reserve }
    }
    pub(super) fn set_token(
        &mut self,
        key: &str,
        keep: bool,
        value: Slot<u64>,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = self.ensure_override(key, cx) {
            if keep {
                let _ = path
                    .then(CompactionRow::KEEP)
                    .try_set(&self.conversation, value, cx);
            } else {
                let _ = path
                    .then(CompactionRow::RESERVE)
                    .try_set(&self.conversation, value, cx);
            }
        }
        cx.notify();
    }
    pub(super) fn token_value(&self, key: &str, keep: bool, cx: &App) -> (String, Source) {
        if let Slot::Invalid(text) = self.token_slot(key, keep, cx) {
            return (
                text,
                if matches!(self.scope, Scope::Global) {
                    Source::Global
                } else {
                    Source::Project
                },
            );
        }
        let token = if keep {
            "keepRecentTokens"
        } else {
            "reserveTokens"
        };
        let slot = self.token_slot(key, keep, cx).encode();
        if let Some(value) = slot.raw() {
            return (
                display(Some(&value)),
                if matches!(self.scope, Scope::Global) {
                    Source::Global
                } else {
                    Source::Project
                },
            );
        }
        let Load::Ready(files) = &self.files else {
            unreachable!()
        };
        if files.project.is_some()
            && let Some(value) = files
                .global
                .get(&["compaction", "modelOverrides", key, token])
        {
            return (display(Some(value)), Source::Global);
        }
        let resolved = self.resolved(
            if keep {
                Field::KeepRecentTokens
            } else {
                Field::ReserveTokens
            },
            cx,
        );
        (display(resolved.values[0].as_ref()), resolved.source)
    }
    fn bind_token(
        &self,
        input: Entity<InputState>,
        path: ItemPath<ConversationDraft, CompactionRow>,
        key: String,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> MapAdapter {
        let path = if keep {
            path.then(CompactionRow::KEEP)
        } else {
            path.then(CompactionRow::RESERVE)
        };
        let owner = cx.entity().downgrade();
        let shown_key = key.clone();
        let (binding, writer) = path
            .try_bind_control_in(
                &self.conversation,
                &input,
                move |input, projection, window, cx| {
                    if matches!(projection, ControlProjection::Value(_))
                        && let Some(owner) = owner.upgrade()
                    {
                        let text = owner.read(cx).token_value(&shown_key, keep, cx).0;
                        if input.value().as_ref() != text {
                            input.set_value(text, window, cx);
                        }
                    }
                },
                window,
                cx,
            )
            .expect("active token leaf");
        let subscription =
            cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                if matches!(event, InputEvent::Change) && !this.is_saving() {
                    let text = input.read(cx).value().to_string();
                    let value = if text == this.token_value(&key, keep, cx).0 {
                        this.token_slot(&key, keep, cx)
                    } else {
                        parse_token(text)
                    };
                    writer.defer_set(value, window, cx);
                }
            });
        MapAdapter {
            input,
            window: window.window_handle(),
            _binding: Some(binding),
            _subscription: subscription,
        }
    }
    pub(super) fn token_input(
        &mut self,
        key: &str,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let path = self.override_path(key, cx);
        let id = path
            .as_ref()
            .map(|p| MapInputKey::Local(p.key(), keep))
            .unwrap_or_else(|| MapInputKey::Inherited(self.generation, key.into(), keep));
        if let Some(adapter) = self.map_inputs.get(&id) {
            return adapter.input.clone();
        }
        let text = self.token_value(key, keep, cx).0;
        let input = cx.new(|cx| InputState::new(window, cx).default_value(text));
        let adapter = if let Some(path) = path {
            self.bind_token(input.clone(), path, key.into(), keep, window, cx)
        } else {
            let id = id.clone();
            let key = key.to_owned();
            let generation = self.generation;
            let subscription =
                cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
                    if !matches!(event, InputEvent::Change) {
                        return;
                    }
                    let (input, id, key) = (input.clone(), id.clone(), key.clone());
                    cx.defer_in(window, move |this, window, cx| {
                        if this.generation != generation
                            || this.is_saving()
                            || this.map_inputs.get(&id).is_none_or(|a| a.input != input)
                        {
                            return;
                        }
                        let text = input.read(cx).value().to_string();
                        if text == this.token_value(&key, keep, cx).0 {
                            return;
                        }
                        if let Some(path) = this.ensure_override(&key, cx) {
                            this.map_inputs.remove(&id);
                            this.set_token(&key, keep, parse_token(text), cx);
                            let adapter =
                                this.bind_token(input, path.clone(), key, keep, window, cx);
                            this.map_inputs
                                .insert(MapInputKey::Local(path.key(), keep), adapter);
                        }
                    });
                });
            MapAdapter {
                input: input.clone(),
                window: window.window_handle(),
                _binding: None,
                _subscription: subscription,
            }
        };
        self.map_inputs.insert(id, adapter);
        input
    }
    pub(super) fn remove_map_row(&mut self, field: Field, key: &str, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        if field == Field::ModelThinking {
            if let (Some(rows), Some(path)) = (self.thinking_rows(cx), self.thinking_path(key, cx))
            {
                let _ = rows.remove(&self.conversation, path, cx);
            }
        } else if let (Some(rows), Some(path)) =
            (self.override_rows(cx), self.override_path(key, cx))
        {
            let _ = rows.remove(&self.conversation, path, cx);
        }
        cx.notify();
    }
    pub(super) fn refresh_map_inputs(&self, cx: &mut Context<Self>) {
        let paths = self
            .override_rows(cx)
            .and_then(|p| p.try_items(&self.conversation, cx).ok())
            .unwrap_or_default();
        for (id, adapter) in &self.map_inputs {
            let (key, keep) = match id {
                MapInputKey::Inherited(_, key, keep) => (key.clone(), *keep),
                MapInputKey::Local(id, keep) => {
                    let Some(key) = paths.iter().find(|p| p.key() == *id).and_then(|p| {
                        p.clone()
                            .then(CompactionRow::KEY)
                            .try_get(&self.conversation, cx)
                            .ok()
                    }) else {
                        continue;
                    };
                    (key, *keep)
                }
            };
            let shown = self.token_value(&key, keep, cx).0;
            let (input, window) = (adapter.input.clone(), adapter.window);
            cx.defer(move |cx| {
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

    fn thinking_levels_for_key(&self, key: &str) -> Vec<&'static str> {
        match &self.models {
            Models::Ready(models) => models
                .iter()
                .find(|m| format!("{}/{}", m.provider, m.id) == key)
                .map(supported_levels)
                .unwrap_or_else(|| fields::THINKING_LEVELS.to_vec()),
            _ => fields::THINKING_LEVELS.to_vec(),
        }
    }

    pub(super) fn add_map_model(&mut self, field: Field, key: &str, cx: &mut Context<Self>) {
        if self.is_saving()
            || self
                .map_values(field, cx)
                .is_none_or(|values| values.contains_key(key))
        {
            return;
        }
        if field == Field::ModelThinking {
            let level = self
                .thinking_levels_for_key(key)
                .first()
                .copied()
                .unwrap_or("off")
                .to_owned();
            if let Some(path) = self.ensure_thinking(key, cx) {
                let _ =
                    path.then(ThinkingRow::LEVEL)
                        .try_set(&self.conversation, Slot::Set(level), cx);
            }
        } else {
            let reserve = self.resolved(Field::ReserveTokens, cx).values[0]
                .as_ref()
                .and_then(Value::as_u64)
                .unwrap_or(16384);
            self.set_token(key, false, Slot::Set(reserve), cx);
        }
        cx.notify();
    }

    pub(super) fn render_catalog_status(&self, cx: &mut Context<Self>) -> AnyElement {
        let status = match &self.models {
            Models::Loading | Models::Idle => Some(("pi-models-loading", false)),
            Models::Failed(_) => Some(("pi-models-unavailable", true)),
            Models::Ready(_) => None,
        };
        let error = match &self.models {
            Models::Failed(error) => Some(error.clone()),
            _ => None,
        };
        h_flex()
            .id("pi-model-catalog-status")
            .when_some(error, |this, error| {
                this.tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(error.clone()).build(window, cx)
                })
            })
            .gap_1()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when_some(status, |this, (key, retry)| {
                this.child(t(cx, key)).when(retry, |this| {
                    this.child(
                        Button::new("pi-models-retry")
                            .xsmall()
                            .ghost()
                            .label(t(cx, "pi-models-retry"))
                            .disabled(self.is_saving())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.load_models(window, cx)),
                            ),
                    )
                })
            })
            .into_any_element()
    }
    fn map_row_menu(
        &self,
        field: Field,
        key: &str,
        own: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity = cx.entity();
        let key = key.to_owned();
        let project = matches!(self.scope, Scope::Project(_));
        let value = self
            .map_values(field, cx)
            .and_then(|v| v.get(&key).cloned());
        let mut token_actions = Vec::new();
        if field == Field::ModelOverrides {
            for keep in [false, true] {
                let leaf = vec![
                    "compaction".into(),
                    "modelOverrides".into(),
                    key.clone(),
                    if keep {
                        "keepRecentTokens".into()
                    } else {
                        "reserveTokens".into()
                    },
                ];
                let current = self.token_slot(&key, keep, cx);
                let old = self
                    .baseline
                    .leaf_values(field)
                    .remove(&leaf)
                    .unwrap_or(Slot::Unset);
                if current.encode() != old {
                    token_actions.push((keep, "pi-menu-revert", Slot::<u64>::decode(old)));
                }
                if matches!(current, Slot::Unset) {
                    if let Some(n) = fields::parse_count(&self.token_value(&key, keep, cx).0)
                        .and_then(|v| v.as_u64())
                    {
                        token_actions.push((
                            keep,
                            if project {
                                "pi-menu-pin-project"
                            } else {
                                "pi-menu-pin-global"
                            },
                            Slot::Set(n),
                        ));
                    }
                } else {
                    token_actions.push((
                        keep,
                        if project {
                            "pi-menu-inherit"
                        } else {
                            "pi-menu-default"
                        },
                        Slot::Unset,
                    ));
                }
            }
        }
        Button::new(SharedString::from(format!("map-menu-{field:?}-{key}")))
            .ghost()
            .small()
            .w_6()
            .flex_shrink_0()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-field-actions"))
            .disabled(self.is_saving())
            .dropdown_menu(move |mut menu, _, cx| {
                if !own && project {
                    let (entity, key, value) = (entity.clone(), key.clone(), value.clone());
                    menu = menu.item(PopupMenuItem::new(t(cx, "pi-menu-pin-project")).on_click(
                        move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                if field == Field::ModelThinking {
                                    if let (Some(path), Some(value)) =
                                        (this.ensure_thinking(&key, cx), value.clone())
                                    {
                                        let _ = path.then(ThinkingRow::LEVEL).try_set(
                                            &this.conversation,
                                            Slot::decode(Slot::Set(value)),
                                            cx,
                                        );
                                    }
                                } else if let Some(value) = &value {
                                    for (keep, name) in
                                        [(false, "reserveTokens"), (true, "keepRecentTokens")]
                                    {
                                        if let Some(n) = value.get(name).and_then(Value::as_u64) {
                                            this.set_token(&key, keep, Slot::Set(n), cx);
                                        }
                                    }
                                }
                                cx.notify();
                            })
                        },
                    ));
                }
                for (keep, action, value) in &token_actions {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set(
                        "field",
                        t(
                            cx,
                            if *keep {
                                "pi-map-keep"
                            } else {
                                "pi-map-reserve"
                            },
                        ),
                    );
                    args.set("action", t(cx, action));
                    let label = gupi_settings::i18n::t_with_args(cx, "pi-map-token-action", &args);
                    let (entity, key, value, keep) =
                        (entity.clone(), key.clone(), value.clone(), *keep);
                    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                        entity.update(cx, |this, cx| this.set_token(&key, keep, value.clone(), cx))
                    }));
                }
                if own {
                    let (entity, key) = (entity.clone(), key.clone());
                    menu = menu.separator().item(
                        PopupMenuItem::new(t(cx, "pi-map-remove-local"))
                            .icon(IconName::Trash)
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |this, cx| this.remove_map_row(field, &key, cx))
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }
    pub(super) fn render_model_map(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let saving = self.is_saving();
        let values = self.map_values(field, cx);
        let keys = values
            .as_ref()
            .map(|v| v.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let items = match &self.models {
            Models::Ready(models) => models
                .iter()
                .filter(|m| !keys.contains(&format!("{}/{}", m.provider, m.id)))
                .map(|m| ModelItem {
                    key: (m.provider.clone(), m.id.clone()),
                    title: format!("{} · {}", m.name, m.provider).into(),
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let picker = self.map_pickers[&field].clone();
        if self.map_picker_items.get(&field) != Some(&items) {
            picker.update(cx, |picker, cx| {
                picker.set_items(SearchableVec::new(items.clone()), window, cx)
            });
            self.map_picker_items.insert(field, items);
        }
        let header = h_flex()
            .gap_2()
            .w_full()
            .child(div().flex_1().min_w_0().child(self.field_label(field, cx)))
            .child(
                div().w_40().h_6().flex_shrink_0().child(
                    Select::new(&picker)
                        .small()
                        .w_full()
                        .placeholder(t(cx, "pi-map-add-model"))
                        .search_placeholder(t(cx, "conversation-model-search"))
                        .accessibility_label(t(cx, "pi-map-add-model"))
                        .disabled(saving || !matches!(self.models, Models::Ready(_))),
                ),
            )
            .child(self.render_field_menu(field, self.pending(field, cx), saving, cx));
        let mut body = v_flex()
            .w_full()
            .max_w(rems(56.))
            .gap_2()
            .mb_2()
            .child(header)
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if field == Field::ModelThinking {
                            "pi-model-thinking-help"
                        } else {
                            "pi-model-overrides-help"
                        },
                    )),
            );
        if field == Field::ModelThinking {
            body = body.child(self.render_catalog_status(cx));
        }
        let mut table = v_flex()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .bg(cx.theme().group_box);
        let Some(values) = values else {
            return body
                .child(
                    table.child(
                        div()
                            .p_2()
                            .text_sm()
                            .child(display(self.resolved(field, cx).values[0].as_ref())),
                    ),
                )
                .into_any_element();
        };
        let current_leaves = self.snapshot(cx).leaf_values(field);
        let baseline_leaves = self.baseline.leaf_values(field);
        if field == Field::ModelOverrides {
            table = table.child(
                h_flex()
                    .px_2()
                    .py_1()
                    .gap_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(div().flex_1().min_w_0().child(t(cx, "pi-map-model")))
                    .child(
                        div()
                            .w(rems(9.))
                            .flex_shrink_0()
                            .child(t(cx, "pi-map-reserve")),
                    )
                    .child(
                        div()
                            .w(rems(9.))
                            .flex_shrink_0()
                            .child(t(cx, "pi-map-keep")),
                    )
                    .child(div().w_6().flex_shrink_0()),
            );
        }
        if values.is_empty() {
            table = table.child(
                div()
                    .p_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-value-unset")),
            );
        }
        for (index, (key, value)) in values.into_iter().enumerate() {
            let available = match &self.models {
                Models::Ready(models) => models
                    .iter()
                    .find(|m| format!("{}/{}", m.provider, m.id) == key),
                _ => None,
            };
            let own = if field == Field::ModelThinking {
                self.thinking_path(&key, cx).is_some()
            } else {
                self.override_path(&key, cx).is_some()
            };
            let title = h_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("map-name-{field:?}-{key}")))
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .tooltip({
                            let key = key.clone();
                            move |window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(key.clone())
                                    .build(window, cx)
                            }
                        })
                        .child(key.clone()),
                )
                .when(
                    available.is_none() && matches!(self.models, Models::Ready(_)),
                    |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "pi-model-unavailable-short")),
                        )
                    },
                );
            let mut row = h_flex()
                .id(SharedString::from(format!("map-row-{field:?}-{key}")))
                .p_2()
                .gap_2()
                .w_full()
                .when(index > 0 || field == Field::ModelOverrides, |this| {
                    this.border_t_1().border_color(cx.theme().border)
                })
                .child(title);
            if field == Field::ModelThinking {
                let mut levels: Vec<String> = available
                    .map(supported_levels)
                    .unwrap_or_else(|| fields::THINKING_LEVELS.to_vec())
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let current = value.as_str().unwrap_or("").to_owned();
                if !current.is_empty() && !levels.contains(&current) {
                    levels.push(current.clone());
                }
                let entity = cx.entity();
                let edit_key = key.clone();
                let selected = value
                    .as_str()
                    .map(|v| super::page::level_label(v, cx))
                    .unwrap_or_else(|| display(Some(&value)));
                row = row.child(
                    Button::new(SharedString::from(format!("thinking-{key}")))
                        .small()
                        .outline()
                        .w_32()
                        .label(selected)
                        .dropdown_caret(true)
                        .disabled(saving)
                        .accessibility_label(key.clone())
                        .dropdown_menu(move |menu, _, cx| {
                            levels.iter().fold(menu, |menu, level| {
                                let (entity, key, level) =
                                    (entity.clone(), edit_key.clone(), level.clone());
                                menu.item(
                                    PopupMenuItem::new(super::page::level_label(&level, cx))
                                        .checked(current == level)
                                        .on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                if let Some(path) = this.ensure_thinking(&key, cx) {
                                                    let _ = path.then(ThinkingRow::LEVEL).try_set(
                                                        &this.conversation,
                                                        Slot::Set(level.clone()),
                                                        cx,
                                                    );
                                                }
                                                cx.notify();
                                            })
                                        }),
                                )
                            })
                        }),
                );
                let leaf = vec!["modelThinkingLevels".into(), key.clone()];
                let origin = if own && matches!(self.scope, Scope::Project(_)) {
                    Source::Project
                } else {
                    Source::Global
                };
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(origin_label(
                            origin,
                            current_leaves.get(&leaf).unwrap_or(&Slot::Unset)
                                != baseline_leaves.get(&leaf).unwrap_or(&Slot::Unset),
                            cx,
                        )),
                );
                if self.conflict_leaves.contains(&leaf) {
                    row = row.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().danger)
                            .child(t(cx, "pi-changed-on-disk")),
                    );
                }
            } else if !value.is_object() {
                row = row.child(div().text_sm().child(display(Some(&value))));
            } else {
                for keep in [false, true] {
                    let input = self.token_input(&key, keep, window, cx);
                    let source = self.token_value(&key, keep, cx).1;
                    let leaf = vec![
                        "compaction".into(),
                        "modelOverrides".into(),
                        key.clone(),
                        if keep {
                            "keepRecentTokens".into()
                        } else {
                            "reserveTokens".into()
                        },
                    ];
                    let invalid = matches!(self.token_slot(&key, keep, cx), Slot::Invalid(_));
                    let conflict = self.conflict_leaves.contains(&leaf);
                    row = row.child(
                        v_flex()
                            .w(rems(9.))
                            .flex_shrink_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Input::new(&input)
                                            .small()
                                            .w_20()
                                            .aria_label(format!(
                                                "{key} {}",
                                                t(
                                                    cx,
                                                    if keep {
                                                        "pi-map-keep"
                                                    } else {
                                                        "pi-map-reserve"
                                                    }
                                                )
                                            ))
                                            .disabled(saving),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(origin_label(
                                                source,
                                                current_leaves.get(&leaf)
                                                    != baseline_leaves.get(&leaf),
                                                cx,
                                            )),
                                    ),
                            )
                            .when(invalid || conflict, |this| {
                                this.child(div().text_xs().text_color(cx.theme().danger).child(t(
                                    cx,
                                    if conflict {
                                        "pi-changed-on-disk"
                                    } else {
                                        "pi-invalid-count"
                                    },
                                )))
                            }),
                    );
                }
            }
            if field == Field::ModelThinking && own {
                let delete = key.clone();
                row = row.child(
                    Button::new(SharedString::from(format!("map-delete-{key}")))
                        .small()
                        .ghost()
                        .w_6()
                        .flex_shrink_0()
                        .icon(IconName::Trash)
                        .tooltip(t(cx, "pi-map-remove-local"))
                        .accessibility_label(t(cx, "pi-map-remove-local"))
                        .disabled(saving)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_map_row(field, &delete, cx)
                        })),
                );
            } else {
                row = row.child(self.map_row_menu(field, &key, own, cx));
            }
            table = table.child(row);
        }
        body.child(table).into_any_element()
    }
}
fn origin_label(source: Source, pending: bool, cx: &App) -> String {
    let mut label = t(
        cx,
        match source {
            Source::Default => "pi-source-default",
            Source::Global => "pi-source-global",
            Source::Project => "pi-source-project",
        },
    );
    if pending {
        label = format!("{label} · {}", t(cx, "pi-unsaved"));
    }
    label
}

fn parse_token(text: String) -> Slot<u64> {
    if text.is_empty() {
        Slot::Unset
    } else {
        match fields::parse_count(&text).and_then(|n| n.as_u64()) {
            Some(n) => Slot::Set(n),
            None => Slot::Invalid(text),
        }
    }
}
