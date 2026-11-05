//! Scope-local resource arrays. Values are JSON strings, not resolved tool names.
use super::lists::{ListItem, ListRows, ListSlot};
use super::*;
use gpui_form::DynamicItemsPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gupi_settings::assets::IconName;

pub(super) struct EntryInput {
    input: Entity<InputState>,
    rows: std::collections::HashMap<gpui_form::PathKey, gpui_form_gpui_component::FormInput>,
    _subscription: Subscription,
}

fn resource_text(
    id: &'static str,
    shown: String,
    full: String,
    cx: &App,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .min_w_0()
        .text_sm()
        .font_family(cx.theme().mono_font_family.clone())
        .truncate()
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(full.clone()).build(window, cx)
        })
        .child(shown)
}

impl PiConfig {
    fn resource_list(&self, field: Field, cx: &App) -> ListSlot {
        let baseline = ListSlot::decode(self.baseline(cx).value(field).remove(0));
        if matches!(baseline, ListSlot::Unreadable(_)) {
            return baseline;
        }
        ListSlot::decode(self.snapshot(cx).value(field).remove(0))
    }
    fn resource_items(
        &self,
        field: Field,
        cx: &App,
    ) -> Option<DynamicItemsPath<ToolsDraft, ListItem>> {
        let path = match field {
            Field::Extensions => ToolsDraft::EXTENSIONS,
            Field::Skills => ToolsDraft::SKILLS,
            Field::Prompts => ToolsDraft::PROMPTS,
            Field::Themes => ToolsDraft::THEMES,
            _ => unreachable!("resource array field"),
        };
        path.case(ListSlot::SET)
            .resolve(&self.tools, cx)
            .ok()
            .flatten()
            .map(|p| p.then(ListRows::ITEMS))
    }
    pub(super) fn append_resource_path(
        &mut self,
        field: Field,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() || matches!(self.resource_list(field, cx), ListSlot::Unreadable(_)) {
            return;
        }
        if self.resource_items(field, cx).is_none() {
            self.set_values(field, vec![Slot::Set(serde_json::json!([]))], cx);
        }
        if let Some(items) = self.resource_items(field, cx) {
            let _ = items.append(&self.tools, ListItem { text }, cx);
            cx.notify();
        }
    }
    fn resource_editor(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(editor) = self.resource_inputs.get(&field) {
            return editor.input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscription = cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
            if !matches!(event, InputEvent::PressEnter { .. }) {
                return;
            }
            let input = input.clone();
            cx.defer_in(window, move |this, window, cx| {
                if this
                    .resource_inputs
                    .get(&field)
                    .is_some_and(|editor| editor.input == input)
                {
                    this.add_resource_entry(field, window, cx);
                }
            });
        });
        self.resource_inputs.insert(
            field,
            EntryInput {
                input: input.clone(),
                rows: Default::default(),
                _subscription: subscription,
            },
        );
        input
    }

    fn add_resource_entry(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        let Some(input) = self
            .resource_inputs
            .get(&field)
            .map(|editor| editor.input.clone())
        else {
            return;
        };
        let value = input.read(cx).value().to_string();
        if value.is_empty() {
            return;
        }
        self.append_resource_path(field, value, cx);
        input.update(cx, |input, cx| input.set_value("", window, cx));
        input.focus_handle(cx).focus(window, cx);
    }

    fn change_resource_entry(
        &mut self,
        field: Field,
        key: &gpui_form::PathKey,
        action: i8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        let Some(items) = self.resource_items(field, cx) else {
            return;
        };
        let Ok(paths) = items.try_items(&self.tools, cx) else {
            return;
        };
        let Some(index) = paths.iter().position(|path| &path.key() == key) else {
            return;
        };
        let path = &paths[index];
        if action == 0 {
            let next = paths
                .get(index + 1)
                .or_else(|| index.checked_sub(1).and_then(|ix| paths.get(ix)))
                .and_then(|path| {
                    self.resource_inputs
                        .get(&field)?
                        .rows
                        .get(&path.key())
                        .map(|input| input.focus_handle(cx))
                })
                .or_else(|| {
                    self.resource_inputs
                        .get(&field)
                        .map(|editor| editor.input.focus_handle(cx))
                });
            let _ = items.remove(&self.tools, path.clone(), cx);
            if let Some(next) = next {
                next.focus(window, cx);
            }
        } else if action > 0 && index + 1 < paths.len() {
            let _ = items.move_before(&self.tools, &paths[index + 1], path, cx);
        } else if action < 0 && index > 0 {
            let _ = items.move_before(&self.tools, path, &paths[index - 1], cx);
        }
        cx.notify();
    }

    pub(super) fn render_resource_paths(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let list = self.resource_list(field, cx);
        let input = self.resource_editor(field, window, cx);
        let mut content = v_flex()
            .id(SharedString::from(format!("resource-box-{}", field.id())))
            .w_full()
            .min_w_0()
            .gap_2();
        if let ListSlot::Unreadable(raw) = list {
            self.resource_inputs.get_mut(&field).unwrap().rows.clear();
            return content
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "pi-resource-unsupported")),
                )
                .child(resource_text(
                    "raw",
                    raw.value.to_string(),
                    raw.value.to_string(),
                    cx,
                ))
                .into_any_element();
        }
        let paths = self
            .resource_items(field, cx)
            .and_then(|items| items.try_items(&self.tools, cx).ok())
            .unwrap_or_default();
        let active = paths
            .iter()
            .map(|path| path.key())
            .collect::<std::collections::HashSet<_>>();
        self.resource_inputs
            .get_mut(&field)
            .unwrap()
            .rows
            .retain(|key, _| active.contains(key));
        let count = paths.len();
        for (index, path) in paths.into_iter().enumerate() {
            let key = path.key();
            if !self.resource_inputs[&field].rows.contains_key(&key) {
                let input = gpui_form_gpui_component::FormInput::try_new(
                    &self.tools,
                    path.then(ListItem::TEXT),
                    InputState::new,
                    window,
                    cx,
                )
                .expect("active resource row");
                self.resource_inputs
                    .get_mut(&field)
                    .unwrap()
                    .rows
                    .insert(key.clone(), input);
            }
            let bound = &self.resource_inputs[&field].rows[&key];
            let mut row = h_flex()
                .id(SharedString::from(format!(
                    "resource-{}-{key:?}",
                    field.id()
                )))
                .w_full()
                .min_w_0()
                .gap_1()
                .child(
                    div().flex_1().min_w_0().child(
                        Input::new(bound)
                            .small()
                            .disabled(self.is_saving())
                            .aria_label(t(cx, field.label())),
                    ),
                );
            for (action, label, icon, disabled) in [
                (-1, "pi-resource-up", IconName::ArrowUp, index == 0),
                (
                    1,
                    "pi-resource-down",
                    IconName::ArrowDown,
                    index + 1 == count,
                ),
                (0, "pi-resource-remove", IconName::Trash, false),
            ] {
                let key = key.clone();
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("field", t(cx, field.label()));
                args.set("index", index as i64 + 1);
                args.set("action", t(cx, label));
                let title = gupi_settings::i18n::t_with_args(cx, "pi-list-item-action", &args);
                row = row.child(
                    Button::new(SharedString::from(format!("resource-{action}-{key:?}")))
                        .ghost()
                        .small()
                        .icon(icon)
                        .tooltip(title.clone())
                        .accessibility_label(title)
                        .disabled(disabled || self.is_saving())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.change_resource_entry(field, &key, action, window, cx)
                        })),
                );
            }
            content = content.child(row);
        }
        content
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&input)
                                .small()
                                .aria_label(t(cx, field.label()))
                                .disabled(self.is_saving()),
                        ),
                    )
                    .child(
                        Button::new(SharedString::from(format!("resource-add-{}", field.id())))
                            .small()
                            .outline()
                            .icon(IconName::Plus)
                            .tooltip(t(cx, "pi-resource-add"))
                            .accessibility_label(t(cx, "pi-resource-add"))
                            .disabled(self.is_saving())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.add_resource_entry(field, window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }
}
