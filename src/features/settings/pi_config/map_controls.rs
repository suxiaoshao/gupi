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

    fn token_menu(&self, key: &str, keep: bool, cx: &mut Context<Self>) -> AnyElement {
        let leaf = vec![
            "compaction".into(),
            "modelOverrides".into(),
            key.to_owned(),
            if keep {
                "keepRecentTokens".into()
            } else {
                "reserveTokens".into()
            },
        ];
        let old = self
            .baseline
            .leaf_values(Field::ModelOverrides)
            .remove(&leaf)
            .unwrap_or(Slot::Unset);
        let current = self.token_slot(key, keep, cx);
        let pending = current.encode() != old;
        let unset = matches!(current, Slot::Unset);
        let inherited = matches!(self.scope, Scope::Project(_));
        let pin = fields::parse_count(&self.token_value(key, keep, cx).0).and_then(|v| v.as_u64());
        let entity = cx.entity();
        let key = key.to_owned();
        Button::new(SharedString::from(format!("token-menu-{key}-{keep}")))
            .ghost()
            .small()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-field-actions"))
            .disabled(self.is_saving())
            .dropdown_menu(move |mut menu, _, cx| {
                if pending {
                    let (entity, key, old) = (entity.clone(), key.clone(), old.clone());
                    menu = menu.item(PopupMenuItem::new(t(cx, "pi-menu-revert")).on_click(
                        move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.set_token(&key, keep, Slot::decode(old.clone()), cx)
                            })
                        },
                    ));
                }
                let (entity, key) = (entity.clone(), key.clone());
                if unset {
                    if let Some(pin) = pin {
                        menu = menu.item(
                            PopupMenuItem::new(t(
                                cx,
                                if inherited {
                                    "pi-menu-pin-project"
                                } else {
                                    "pi-menu-pin-global"
                                },
                            ))
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.set_token(&key, keep, Slot::Set(pin), cx)
                                })
                            }),
                        );
                    }
                } else {
                    menu = menu.item(
                        PopupMenuItem::new(t(
                            cx,
                            if inherited {
                                "pi-menu-inherit"
                            } else {
                                "pi-menu-default"
                            },
                        ))
                        .on_click(move |_, _, cx| {
                            entity
                                .update(cx, |this, cx| this.set_token(&key, keep, Slot::Unset, cx))
                        }),
                    );
                }
                menu
            })
            .into_any_element()
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

    pub(super) fn render_model_map(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let saving = self.is_saving();
        let Some(values) = self.map_values(field, cx) else {
            return v_flex()
                .gap_2()
                .child(t(cx, field.label()))
                .child(display(self.resolved(field, cx).values[0].as_ref()))
                .child(self.render_field_menu(field, self.pending(field, cx), saving, cx))
                .into_any_element();
        };
        let current_leaves = self.snapshot(cx).leaf_values(field);
        let baseline_leaves = self.baseline.leaf_values(field);
        let keys = values.keys().cloned().collect::<BTreeSet<_>>();
        let mut body = v_flex().gap_3().w_full().child(
            h_flex()
                .justify_between()
                .child(t(cx, field.label()))
                .child(self.render_field_menu(field, self.pending(field, cx), saving, cx)),
        );
        for (key, value) in values {
            let available = match &self.models {
                Models::Ready(models) => models
                    .iter()
                    .find(|m| format!("{}/{}", m.provider, m.id) == key),
                _ => None,
            };
            let mut levels: Vec<String> = available
                .map(supported_levels)
                .unwrap_or_else(|| fields::THINKING_LEVELS.to_vec())
                .into_iter()
                .map(str::to_owned)
                .collect();
            if field == Field::ModelThinking
                && let Some(current) = value.as_str()
                && !levels.iter().any(|v| v == current)
            {
                levels.push(current.to_owned());
            }
            let mut title = h_flex()
                .gap_2()
                .items_center()
                .child(div().flex_1().min_w_0().truncate().child(key.clone()));
            if available.is_none() && matches!(self.models, Models::Ready(_)) {
                title = title.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-field-model-unavailable")),
                );
            }
            let own = if field == Field::ModelThinking {
                self.thinking_path(&key, cx).is_some()
            } else {
                self.override_path(&key, cx).is_some()
            };
            let mut row = v_flex().gap_2().child(title);
            if field == Field::ModelThinking {
                let current = value.as_str().unwrap_or("").to_owned();
                let entity = cx.entity();
                let next_key = key.clone();
                let selected = value
                    .as_str()
                    .map(|v| super::page::level_label(v, cx))
                    .unwrap_or_else(|| display(Some(&value)));
                row = row.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Button::new(SharedString::from(format!("thinking-{key}")))
                                .small()
                                .outline()
                                .label(selected)
                                .disabled(saving)
                                .dropdown_menu(move |menu, _, cx| {
                                    levels.iter().fold(menu, |menu, level| {
                                        let label = super::page::level_label(level, cx);
                                        let (entity, key, level) =
                                            (entity.clone(), next_key.clone(), (*level).to_owned());
                                        menu.item(
                                            PopupMenuItem::new(label)
                                                .checked(current == level)
                                                .on_click(move |_, _, cx| {
                                                    entity.update(cx, |this, cx| {
                                                        if let Some(path) =
                                                            this.ensure_thinking(&key, cx)
                                                        {
                                                            let _ = path
                                                                .then(ThinkingRow::LEVEL)
                                                                .try_set(
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
                        )
                        .child({
                            let mut label = t(
                                cx,
                                if own && matches!(self.scope, Scope::Project(_)) {
                                    "pi-source-project"
                                } else {
                                    "pi-source-global"
                                },
                            );
                            let leaf = vec!["modelThinkingLevels".to_owned(), key.clone()];
                            if current_leaves.get(&leaf) != baseline_leaves.get(&leaf) {
                                label = format!("{label} · {}", t(cx, "pi-unsaved"));
                            }
                            label
                        }),
                );
            } else if !value.is_object() {
                row = row.child(display(Some(&value)));
            } else {
                for keep in [false, true] {
                    let input = self.token_input(&key, keep, window, cx);
                    let source = self.token_value(&key, keep, cx).1;
                    row = row.child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(t(
                                cx,
                                if keep {
                                    "pi-field-keep"
                                } else {
                                    "pi-field-reserve"
                                },
                            ))
                            .child(Input::new(&input).small().w_40().disabled(saving))
                            .child({
                                let mut label = t(
                                    cx,
                                    match source {
                                        Source::Default => "pi-source-default",
                                        Source::Global => "pi-source-global",
                                        Source::Project => "pi-source-project",
                                    },
                                );
                                let leaf = vec![
                                    "compaction".to_owned(),
                                    "modelOverrides".to_owned(),
                                    key.clone(),
                                    if keep {
                                        "keepRecentTokens".to_owned()
                                    } else {
                                        "reserveTokens".to_owned()
                                    },
                                ];
                                if current_leaves.get(&leaf) != baseline_leaves.get(&leaf) {
                                    label = format!("{label} · {}", t(cx, "pi-unsaved"));
                                }
                                label
                            })
                            .child(self.token_menu(&key, keep, cx)),
                    );
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
                    let conflict = self.conflict_leaves.contains(&leaf);
                    let current = self.token_slot(&key, keep, cx);
                    if conflict || matches!(current, Slot::Invalid(_)) {
                        row = row.child(div().text_xs().text_color(cx.theme().danger).child(t(
                            cx,
                            if conflict {
                                "pi-changed-on-disk"
                            } else {
                                "pi-invalid-count"
                            },
                        )));
                    }
                }
            }
            if field == Field::ModelThinking
                && self
                    .conflict_leaves
                    .contains(&vec!["modelThinkingLevels".into(), key.clone()])
            {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "pi-changed-on-disk")),
                );
            }
            let key_for_delete = key.clone();
            row = row.child(
                Button::new(SharedString::from(format!("map-remove-{field:?}-{key}")))
                    .small()
                    .ghost()
                    .icon(IconName::Trash)
                    .label(t(cx, "pi-map-remove-local"))
                    .disabled(saving || !own)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_map_row(field, &key_for_delete, cx)
                    })),
            );
            body = body.child(row);
        }
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
        body = body.child(
            Select::new(&picker)
                .small()
                .w_64()
                .placeholder(t(cx, "pi-map-add-model"))
                .search_placeholder(t(cx, "conversation-model-search"))
                .accessibility_label(t(cx, "pi-map-add-model"))
                .disabled(saving || !matches!(self.models, Models::Ready(_))),
        );
        body = body.child(
            Button::new(SharedString::from(format!("map-models-reload-{field:?}")))
                .small()
                .label(t(cx, "pi-models-retry"))
                .disabled(saving)
                .on_click(cx.listener(|this, _, window, cx| this.load_models(window, cx))),
        );
        body = body.child(
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
        body.into_any_element()
    }
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
