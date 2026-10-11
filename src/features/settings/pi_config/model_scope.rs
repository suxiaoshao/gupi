//! Ordered Pi model patterns. Preserve syntax and ordering; Pi resolves matches.
use super::*;
use fields::Source;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gupi_settings::assets::IconName;

fn rows(value: Option<&Value>) -> Option<Vec<String>> {
    match value {
        None => Some(vec![]),
        Some(Value::Array(values)) => values
            .iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    }
}

fn edit(rows: Vec<String>, project: bool) -> Edit {
    if rows.iter().any(|s| s.trim().is_empty()) {
        Edit::Invalid(serde_json::to_string(&rows).expect("strings serialize"))
    } else if rows.is_empty() && !project {
        Edit::Remove
    } else {
        Edit::Set(vec![serde_json::json!(rows)])
    }
}

impl PiConfig {
    fn model_scope_rows(&self) -> Option<Vec<String>> {
        let Load::Ready(files) = &self.files else {
            return None;
        };
        let draft = self.drafts.get(&Field::ModelScope);
        if let Some(Edit::Invalid(text)) = draft {
            return serde_json::from_str(text).ok();
        }
        rows(files.resolve(Field::ModelScope, draft).values[0].as_ref())
    }

    /// Row identities for the shown patterns. Any change made outside the
    /// editor, such as a reload or a discarded draft, assigns new identities so
    /// retained row state never pairs with another pattern.
    pub(super) fn model_scope_ids(&mut self, rows: &[String]) -> Vec<u64> {
        if !self.scope_rows.iter().map(|(_, text)| text).eq(rows) {
            self.scope_rows = rows
                .iter()
                .map(|text| (self.next_model_scope_id(), text.clone()))
                .collect();
        }
        self.scope_rows.iter().map(|(id, _)| *id).collect()
    }

    fn next_model_scope_id(&mut self) -> u64 {
        self.next_scope_row += 1;
        self.next_scope_row
    }

    /// Edits the rows with their identities; `change` draws new identities
    /// from the counter it is given.
    fn edit_model_rows(
        &mut self,
        change: impl FnOnce(&mut Vec<(u64, String)>, &mut u64),
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        let Some(rows) = self.model_scope_rows() else {
            return;
        };
        let ids = self.model_scope_ids(&rows);
        let mut pairs: Vec<_> = ids.into_iter().zip(rows).collect();
        change(&mut pairs, &mut self.next_scope_row);
        let rows = pairs.iter().map(|(_, text)| text.clone()).collect();
        self.scope_rows = pairs;
        self.set_draft(
            Field::ModelScope,
            edit(rows, matches!(self.scope, Scope::Project(_))),
            cx,
        );
    }

    /// Edits the patterns by position; rows past the old length get new
    /// identities.
    pub(super) fn edit_model_scope(
        &mut self,
        change: impl FnOnce(&mut Vec<String>),
        cx: &mut Context<Self>,
    ) {
        self.edit_model_rows(
            |pairs, next| {
                let (ids, mut rows): (Vec<_>, Vec<_>) = pairs.drain(..).unzip();
                change(&mut rows);
                let fresh = std::iter::repeat_with(|| {
                    *next += 1;
                    *next
                });
                pairs.extend(ids.into_iter().chain(fresh).zip(rows));
            },
            cx,
        );
    }

    /// Applies `change` at the current position of row `id`.
    pub(super) fn edit_model_row(
        &mut self,
        id: u64,
        change: impl FnOnce(&mut Vec<(u64, String)>, usize),
        cx: &mut Context<Self>,
    ) {
        self.edit_model_rows(
            |pairs, _| {
                if let Some(index) = pairs.iter().position(|(row, _)| *row == id) {
                    change(pairs, index);
                }
            },
            cx,
        );
    }

    pub(super) fn render_model_scope(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Load::Ready(files) = &self.files else {
            return div().into_any_element();
        };
        let field = Field::ModelScope;
        let draft = self.drafts.get(&field).cloned();
        let resolved = files.resolve(field, draft.as_ref());
        let Some(rows) = self.model_scope_rows() else {
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
        if draft.is_some() {
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
                    .child(self.render_field_menu(
                        field,
                        resolved.overridden,
                        draft.as_ref(),
                        saving,
                        cx,
                    )),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-model-scope-help")),
            );
        let ids = self.model_scope_ids(&rows);
        for (index, (row, text)) in ids.into_iter().zip(&rows).enumerate() {
            let entity = cx.entity().downgrade();
            let id = format!("pi-model-pattern-{}-{row}", self.generation);
            let shown = text.clone();
            let state = window.use_keyed_state(SharedString::from(id), cx, move |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(shown));
                let sub = cx.subscribe(&input, move |_, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let text = input.read(cx).value().to_string();
                        let _ = entity.update(cx, |this, cx| {
                            this.edit_model_row(row, |rows, index| rows[index].1 = text, cx)
                        });
                    }
                });
                (input, sub)
            });
            let input = state.read(cx).0.clone();
            if input.read(cx).value().as_ref() != text {
                input.update(cx, |input, cx| input.set_value(text.clone(), window, cx));
            }
            list = list.child(
                h_flex()
                    .gap_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&input).small().disabled(saving)),
                    )
                    .child(
                        Button::new(("model-scope-up", row as usize))
                            .small()
                            .ghost()
                            .icon(IconName::ChevronUp)
                            .accessibility_label(t(cx, "pi-model-scope-up"))
                            .tooltip(t(cx, "pi-model-scope-up"))
                            .disabled(saving || index == 0)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit_model_row(
                                    row,
                                    |rows, index| {
                                        if index > 0 {
                                            rows.swap(index, index - 1);
                                        }
                                    },
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new(("model-scope-down", row as usize))
                            .small()
                            .ghost()
                            .icon(IconName::ChevronDown)
                            .accessibility_label(t(cx, "pi-model-scope-down"))
                            .tooltip(t(cx, "pi-model-scope-down"))
                            .disabled(saving || index + 1 == count)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit_model_row(
                                    row,
                                    |rows, index| {
                                        if index + 1 < rows.len() {
                                            rows.swap(index, index + 1);
                                        }
                                    },
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new(("model-scope-delete", row as usize))
                            .small()
                            .ghost()
                            .icon(IconName::Trash)
                            .accessibility_label(t(cx, "pi-model-scope-delete"))
                            .tooltip(t(cx, "pi-model-scope-delete"))
                            .disabled(saving)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit_model_row(
                                    row,
                                    |rows, index| {
                                        rows.remove(index);
                                    },
                                    cx,
                                )
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
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.edit_model_scope(|rows| rows.push(String::new()), cx)
                        })),
                ),
        );
        if matches!(draft, Some(Edit::Invalid(_))) || self.conflicts.contains(&field) {
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

#[cfg(test)]
mod tests {
    use super::{Edit, edit, rows};
    #[test]
    fn ordered_patterns_and_empty_scope_are_preserved() {
        let patterns = vec![
            "provider/model:exacto".into(),
            " *sonnet*:high ".into(),
            "provider/model:exacto".into(),
        ];
        assert_eq!(
            rows(Some(&serde_json::json!(patterns))),
            Some(patterns.clone())
        );
        assert_eq!(
            edit(patterns.clone(), false),
            Edit::Set(vec![serde_json::json!(patterns)])
        );
        assert_eq!(edit(vec![], false), Edit::Remove);
        assert_eq!(edit(vec![], true), Edit::Set(vec![serde_json::json!([])]));
        assert!(matches!(edit(vec!["  ".into()], true), Edit::Invalid(_)));
        assert_eq!(rows(Some(&serde_json::json!([42]))), None);
        assert_eq!(rows(Some(&serde_json::json!("broken"))), None);
    }
}
