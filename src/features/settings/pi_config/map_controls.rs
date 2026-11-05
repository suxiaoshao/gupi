//! Separate controls for per-model thinking values and per-token compaction leaves.
use super::model_maps::*;
use super::*;
use fields::{Source, display, supported_levels};
use gpui_form::{ControlMapping, DynamicItemsPath, EditProblem, EditResult, ItemPath};
use gpui_form_gpui_component::FormMappedInput;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gupi_settings::assets::IconName;
use std::collections::BTreeSet;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct MapInputKey(pub(super) String, pub(super) bool);
pub(super) struct MapAdapter {
    pub(super) input: FormMappedInput<TokenContext>,
}
#[derive(Clone)]
pub(super) struct TokenContext {
    inherited: String,
}
struct TokenMapping {
    key: String,
    keep: bool,
}
fn token_editor_key(key: &str, keep: bool) -> gpui_form::EditorKey {
    gpui_form::EditorKey::new(format!("model-token-{key}-{keep}"))
}
fn token_slot(stored: &CompactionSlot, key: &str, keep: bool) -> Slot<u64> {
    match stored {
        CompactionSlot::Set(rows) => rows
            .rows
            .iter()
            .find(|r| r.key == key)
            .map(|r| {
                if keep {
                    r.keep.clone()
                } else {
                    r.reserve.clone()
                }
            })
            .unwrap_or(Slot::Unset),
        _ => Slot::Unset,
    }
}
impl ControlMapping<CompactionSlot, String> for TokenMapping {
    type Context = TokenContext;
    fn format(&self, stored: &CompactionSlot, context: &TokenContext) -> String {
        token_slot(stored, &self.key, self.keep)
            .encode()
            .raw()
            .as_ref()
            .map(|v| display(Some(v)))
            .unwrap_or_else(|| context.inherited.clone())
    }
    fn parse(
        &self,
        text: String,
        current: &CompactionSlot,
        baseline: Option<&CompactionSlot>,
        context: &TokenContext,
    ) -> EditResult<CompactionSlot, String> {
        let parsed = if text.is_empty() {
            Some(Slot::Unset)
        } else {
            fields::parse_count(&text)
                .and_then(|n| n.as_u64())
                .map(Slot::Set)
        };
        let Some(base) = baseline else {
            return EditResult::draft(
                text,
                "baseline-unavailable",
                gpui_form::ValidationMessage::key("pi-settings-refresh-rejected"),
            );
        };
        let base_slot = token_slot(base, &self.key, self.keep);
        let normalized = parsed
            .as_ref()
            .and_then(|target| target.encode().raw().as_ref().map(|v| display(Some(v))));
        let base_shown = self.format(base, context);
        let target = if text == base_shown || normalized.as_ref() == Some(&base_shown) {
            base_slot
        } else if text == self.format(current, context) {
            token_slot(current, &self.key, self.keep)
        } else if let Some(parsed) = parsed {
            parsed
        } else {
            return EditResult::draft(
                text,
                "pi-invalid-count",
                gpui_form::ValidationMessage::key("pi-invalid-count"),
            );
        };
        let base_has_row =
            matches!(base,CompactionSlot::Set(rows) if rows.rows.iter().any(|r|r.key==self.key));
        let (key, keep) = (self.key.clone(), self.keep);
        EditResult::update(move |latest: &CompactionSlot| {
            update_token(latest, &key, keep, target, !base_has_row)
        })
    }
}
fn update_token(
    latest: &CompactionSlot,
    key: &str,
    keep: bool,
    value: Slot<u64>,
    remove_empty: bool,
) -> Result<CompactionSlot, EditProblem> {
    let mut next = latest.clone();
    if matches!(next, CompactionSlot::Unreadable(_)) {
        return Err(EditProblem::new(
            "unreadable-map",
            gpui_form::ValidationMessage::key("pi-resource-unsupported"),
        ));
    }
    if matches!(next, CompactionSlot::Unset) && value == Slot::Unset {
        return Ok(next);
    }
    if matches!(next, CompactionSlot::Unset) {
        next = CompactionSlot::Set(CompactionRows::default());
    }
    let CompactionSlot::Set(rows) = &mut next else {
        unreachable!()
    };
    if !rows.rows.iter().any(|r| r.key == key) {
        if value == Slot::Unset {
            return Ok(next);
        }
        rows.rows.push(CompactionRow {
            key: key.into(),
            reserve: Slot::Unset,
            keep: Slot::Unset,
            extra: Default::default(),
            unreadable: None,
        });
    }
    let row = rows
        .rows
        .iter_mut()
        .find(|r| r.key == key)
        .expect("inserted row");
    if row.unreadable.is_some() {
        return Err(EditProblem::new(
            "unreadable-row",
            gpui_form::ValidationMessage::key("pi-resource-unsupported"),
        ));
    }
    if keep {
        row.keep = value;
    } else {
        row.reserve = value;
    }
    if remove_empty {
        rows.rows.retain(|r| {
            r.key != key || r.reserve != Slot::Unset || r.keep != Slot::Unset || !r.extra.is_empty()
        });
    }
    if rows.rows.is_empty() {
        next = CompactionSlot::Unset;
    }
    Ok(next)
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
        if self.thinking_rows(cx).is_none()
            && let Err(error) = ConversationDraft::MODEL_THINKING.set(
                &self.conversation,
                ThinkingSlot::Set(ThinkingRows::default()),
                cx,
            )
        {
            self.error = Some(error.to_string());
            cx.notify();
            return None;
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
    pub(super) fn token_slot(&self, key: &str, keep: bool, cx: &App) -> Slot<u64> {
        let Some(path) = self.override_path(key, cx) else {
            return Slot::Unset;
        };
        let row = path.try_get(&self.conversation, cx).expect("active row");
        if keep { row.keep } else { row.reserve }
    }
    pub(super) fn token_status(&self, key: &str, keep: bool, cx: &App) -> gpui_form::EditorStatus {
        self.conversation
            .read(cx)
            .editor_status(
                ConversationDraft::MODEL_OVERRIDES,
                token_editor_key(key, keep),
            )
            .expect("total map editor belongs to this session")
    }
    fn discard_token_draft(
        &self,
        key: &str,
        keep: bool,
        cx: &mut App,
    ) -> Result<(), gpui_form::LifecycleError> {
        if let Some(adapter) = self.map_inputs.get(&MapInputKey(key.into(), keep)) {
            adapter.input.discard_draft(cx)?;
        } else {
            self.conversation.update(cx, |form, cx| {
                form.discard_editor_draft(
                    ConversationDraft::MODEL_OVERRIDES,
                    token_editor_key(key, keep),
                    cx,
                )
            })?;
        }
        Ok(())
    }
    pub(super) fn set_token(
        &mut self,
        key: &str,
        keep: bool,
        value: Slot<u64>,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        if let Err(error) = self.discard_token_draft(key, keep, cx) {
            tracing::warn!(%error,"Pi token discard rejected");
            self.error = Some(t(cx, "pi-settings-refresh-rejected"));
            cx.notify();
            return;
        }
        let current = ConversationDraft::MODEL_OVERRIDES.get(&self.conversation, cx);
        let baseline = ConversationDraft::MODEL_OVERRIDES.baseline(&self.conversation, cx);
        let remove_empty =
            !matches!(&baseline,CompactionSlot::Set(rows) if rows.rows.iter().any(|r|r.key==key));
        let result = update_token(&current, key, keep, value, remove_empty)
            .map_err(|error| error.code().to_owned())
            .and_then(|next| {
                ConversationDraft::MODEL_OVERRIDES
                    .set(&self.conversation, next, cx)
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = result {
            tracing::warn!(%error,"Pi token update rejected");
            self.error = Some(t(cx, "pi-settings-refresh-rejected"));
        } else {
            self.error = None;
            if !self.pending(Field::ModelOverrides, cx) {
                self.refresh_conflicts
                    .retain(|f| *f != Field::ModelOverrides);
            }
        }
        cx.notify();
    }
    pub(super) fn token_value(&self, key: &str, keep: bool, cx: &App) -> (String, Source) {
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
    fn token_context(&self, key: &str, keep: bool, cx: &App) -> TokenContext {
        let Load::Ready(files) = &self.files else {
            unreachable!()
        };
        let token = if keep {
            "keepRecentTokens"
        } else {
            "reserveTokens"
        };
        let global = files.project.as_ref().and_then(|_| {
            files
                .global
                .get(&["compaction", "modelOverrides", key, token])
        });
        let fallback = self.resolved(
            if keep {
                Field::KeepRecentTokens
            } else {
                Field::ReserveTokens
            },
            cx,
        );
        TokenContext {
            inherited: display(global.or(fallback.values[0].as_ref())),
        }
    }
    pub(super) fn token_input(
        &mut self,
        key: &str,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let id = MapInputKey(key.into(), keep);
        if let Some(adapter) = self.map_inputs.get(&id) {
            return (*adapter.input).clone();
        }
        let input = FormMappedInput::new(
            &self.conversation,
            ConversationDraft::MODEL_OVERRIDES,
            token_editor_key(key, keep),
            TokenMapping {
                key: key.into(),
                keep,
            },
            self.token_context(key, keep, cx),
            InputState::new,
            window,
            cx,
        )
        .expect("stable token mapping type");
        let state = (*input).clone();
        self.map_inputs.insert(id, MapAdapter { input });
        state
    }
    pub(super) fn remove_map_row(&mut self, field: Field, key: &str, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        if field == Field::ModelOverrides {
            for keep in [false, true] {
                if let Err(error) = self.discard_token_draft(key, keep, cx) {
                    tracing::warn!(%error,"Pi row discard rejected");
                    self.error = Some(t(cx, "pi-settings-refresh-rejected"));
                    cx.notify();
                    return;
                }
            }
        }
        let result = if field == Field::ModelThinking {
            if let (Some(rows), Some(path)) = (self.thinking_rows(cx), self.thinking_path(key, cx))
            {
                rows.remove(&self.conversation, path, cx).map(|_| ())
            } else {
                Ok(())
            }
        } else if let (Some(rows), Some(path)) =
            (self.override_rows(cx), self.override_path(key, cx))
        {
            rows.remove(&self.conversation, path, cx).map(|_| ())
        } else {
            Ok(())
        };
        match result {
            Ok(()) => {
                for keep in [false, true] {
                    self.map_inputs.remove(&MapInputKey(key.into(), keep));
                }
                if !self.pending(field, cx) {
                    self.refresh_conflicts.retain(|f| *f != field);
                }
                self.error = None;
            }
            Err(error) => {
                tracing::warn!(%error,"Pi map row removal rejected");
                self.error = Some(t(cx, "pi-settings-refresh-rejected"));
            }
        }
        cx.notify();
    }
    pub(super) fn refresh_map_inputs(&self, cx: &mut Context<Self>) {
        for (MapInputKey(key, keep), adapter) in &self.map_inputs {
            adapter
                .input
                .refresh(self.token_context(key, *keep, cx), cx);
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
        let key = key.to_owned();
        div()
            .w_6()
            .flex_shrink_0()
            .when(own, |slot| {
                slot.child(
                    Button::new(SharedString::from(format!("map-remove-{field:?}-{key}")))
                        .ghost()
                        .small()
                        .icon(IconName::Trash)
                        .tooltip(format!("{}: {key}", t(cx, "pi-map-remove-local")))
                        .accessibility_label(format!("{}: {key}", t(cx, "pi-map-remove-local")))
                        .disabled(self.is_saving())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.remove_map_row(field, &key, cx)),
                        ),
                )
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
        let mut values = self.map_values(field, cx);
        // Keep Form-owned drafts reachable if an inherited key disappears externally.
        if field == Field::ModelOverrides
            && let Some(values) = &mut values
        {
            for MapInputKey(key, keep) in self.map_inputs.keys() {
                if self.token_status(key, *keep, cx).has_draft() {
                    values
                        .entry(key.clone())
                        .or_insert_with(|| Value::Object(Default::default()));
                }
            }
        }
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
            .flex_wrap()
            .gap_2()
            .w_full()
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t(cx, field.label())),
                    )
                    .child(self.render_field_source(field, cx)),
            )
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
            );
        let mut body = v_flex()
            .w_full()
            .min_w_0()
            .text_sm()
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
        if self.refresh_conflicts.contains(&field) {
            body = body.child(div().text_xs().text_color(cx.theme().danger).child(t(
                cx,
                if field == Field::ModelOverrides && self.field_errors(field, cx) {
                    "pi-map-refresh-draft"
                } else {
                    "pi-settings-refresh-conflict"
                },
            )));
        }
        if field == Field::ModelThinking {
            body = body.child(self.render_catalog_status(cx));
        }
        let mut table = v_flex().w_full().min_w_0();
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
        let baseline_leaves = self.baseline(cx).leaf_values(field);
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
                    let status = self.token_status(&key, keep, cx);
                    let invalid = status.has_draft() || !status.issues().is_empty();
                    let problem = status
                        .issues()
                        .first()
                        .map(|issue| {
                            t(
                                cx,
                                match issue.code() {
                                    "pi-invalid-count" => "pi-invalid-count",
                                    "unreadable-map" | "unreadable-row" => {
                                        "pi-resource-unsupported"
                                    }
                                    _ => "pi-settings-refresh-rejected",
                                },
                            )
                        })
                        .unwrap_or_else(|| t(cx, "pi-invalid-count"));
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
                                this.child(div().text_xs().text_color(cx.theme().danger).child(
                                    if conflict {
                                        t(cx, "pi-changed-on-disk")
                                    } else {
                                        problem
                                    },
                                ))
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
pub(super) fn origin_label(source: Source, pending: bool, cx: &App) -> String {
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
