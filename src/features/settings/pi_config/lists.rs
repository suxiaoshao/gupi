//! Ordered tool names and argv: verbatim values with Form-owned row identities.
use super::forms::{RawValue, Slot};
use gpui_form::FormSchema;
use serde_json::Value;

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) enum ListSlot {
    #[default]
    Unset,
    Set(ListRows),
    Unreadable(RawValue),
}
#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct ListRows {
    #[form(items)]
    pub(super) items: Vec<ListItem>,
}
#[derive(Clone, PartialEq, FormSchema)]
pub(super) struct ListItem {
    pub(super) text: String,
}
impl ListSlot {
    pub(super) fn decode(value: Slot<Value>) -> Self {
        match value {
            Slot::Unset => Self::Unset,
            Slot::Set(value) | Slot::Unreadable(value) => {
                match serde_json::from_value::<Vec<String>>(value.clone()) {
                    Ok(items) => Self::Set(ListRows {
                        items: items.into_iter().map(|text| ListItem { text }).collect(),
                    }),
                    Err(_) => Self::Unreadable(RawValue { value }),
                }
            }
            Slot::Invalid(_) => unreachable!("list rows hold verbatim text"),
        }
    }
    pub(super) fn encode(&self) -> Slot<Value> {
        match self {
            Self::Unset => Slot::Unset,
            Self::Set(rows) => Slot::Set(serde_json::json!(
                rows.items.iter().map(|i| &i.text).collect::<Vec<_>>()
            )),
            Self::Unreadable(raw) => Slot::Unreadable(raw.value.clone()),
        }
    }
}

use super::model_scope::RowKey;
use super::*;
use gpui_form::{
    ControlBinding, ControlProjection, DynamicItemsPath, IntoTotalPath, ItemPath, TotalPath,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gupi_settings::assets::IconName;

pub(super) struct ListAdapter {
    pub(super) input: Entity<InputState>,
    _binding: Option<ControlBinding>,
    _subscription: Subscription,
}
fn slot(field: Field) -> TotalPath<ToolsDraft, ListSlot> {
    match field {
        Field::DefaultTools => ToolsDraft::DEFAULT_TOOLS.into_total_path(),
        Field::NpmCommand => ToolsDraft::NPM_COMMAND.into_total_path(),
        _ => unreachable!("ordered tool/argv field"),
    }
}
impl PiConfig {
    pub(super) fn list_paths(
        &self,
        field: Field,
        cx: &App,
    ) -> Option<DynamicItemsPath<ToolsDraft, ListItem>> {
        slot(field)
            .case(ListSlot::SET)
            .resolve(&self.tools, cx)
            .ok()
            .flatten()
            .map(|p| p.then(ListRows::ITEMS))
    }
    fn list_values(&self, field: Field, cx: &App) -> Option<Vec<String>> {
        match self.resolved(field, cx).values[0].clone() {
            None => Some(Vec::new()),
            Some(value) => serde_json::from_value(value).ok(),
        }
    }
    fn bind_list(
        &self,
        input: Entity<InputState>,
        path: ItemPath<ToolsDraft, ListItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ListAdapter {
        let (binding, writer) = path
            .then(ListItem::TEXT)
            .try_bind_control_in(
                &self.tools,
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
            .expect("active list row");
        let subscription =
            cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                if matches!(event, InputEvent::Change) && !this.is_saving() {
                    writer.defer_set(input.read(cx).value().to_string(), window, cx);
                }
            });
        ListAdapter {
            input,
            _binding: Some(binding),
            _subscription: subscription,
        }
    }
    fn materialize_list(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<DynamicItemsPath<ToolsDraft, ListItem>> {
        if self.is_saving() {
            return None;
        }
        if self.list_paths(field, cx).is_none() {
            let items = self
                .list_values(field, cx)?
                .into_iter()
                .map(|text| ListItem { text })
                .collect();
            slot(field).set(&self.tools, ListSlot::Set(ListRows { items }), cx);
            let paths = self
                .list_paths(field, cx)?
                .try_items(&self.tools, cx)
                .ok()?;
            for (index, path) in paths.into_iter().enumerate() {
                if let Some(adapter) = self
                    .list_inputs
                    .remove(&(field, RowKey::Inherited(self.generation, index)))
                {
                    let input = adapter.input.clone();
                    drop(adapter);
                    let adapter = self.bind_list(input, path.clone(), window, cx);
                    self.list_inputs
                        .insert((field, RowKey::Local(path.key())), adapter);
                }
            }
        }
        self.list_paths(field, cx)
    }
    fn list_input(
        &mut self,
        field: Field,
        key: &RowKey,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(adapter) = self.list_inputs.get(&(field, key.clone())) {
            return adapter.input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx).default_value(text.to_owned()));
        let adapter = match key {
            RowKey::Local(key) => {
                let path = self
                    .list_paths(field, cx)
                    .unwrap()
                    .try_items(&self.tools, cx)
                    .unwrap()
                    .into_iter()
                    .find(|p| p.key() == *key)
                    .unwrap();
                self.bind_list(input.clone(), path, window, cx)
            }
            RowKey::Inherited(generation, index) => {
                let (generation, index, key) = (*generation, *index, key.clone());
                let subscription =
                    cx.subscribe_in(&input, window, move |_, input, event, window, cx| {
                        if !matches!(event, InputEvent::Change) {
                            return;
                        }
                        let input = input.clone();
                        let key = key.clone();
                        cx.defer_in(window, move |this, window, cx| {
                            if this.generation != generation
                                || this
                                    .list_inputs
                                    .get(&(field, key))
                                    .is_none_or(|a| a.input != input)
                                || this.is_saving()
                            {
                                return;
                            }
                            let text = input.read(cx).value().to_string();
                            if this
                                .list_values(field, cx)
                                .and_then(|v| v.get(index).cloned())
                                .as_deref()
                                == Some(&text)
                            {
                                return;
                            }
                            if let Some(paths) = this
                                .materialize_list(field, window, cx)
                                .and_then(|p| p.try_items(&this.tools, cx).ok())
                                && let Some(path) = paths.get(index)
                            {
                                let _ = path.clone().then(ListItem::TEXT).try_set(
                                    &this.tools,
                                    text,
                                    cx,
                                );
                            }
                        });
                    });
                ListAdapter {
                    input: input.clone(),
                    _binding: None,
                    _subscription: subscription,
                }
            }
        };
        self.list_inputs.insert((field, key.clone()), adapter);
        input
    }
    pub(super) fn append_list(
        &mut self,
        field: Field,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(items) = self.materialize_list(field, window, cx) {
            let _ = items.append(&self.tools, ListItem { text }, cx);
            cx.notify();
        }
    }
    pub(super) fn change_list(
        &mut self,
        field: Field,
        key: &RowKey,
        action: i8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(items) = self.materialize_list(field, window, cx) else {
            return;
        };
        let paths = items.try_items(&self.tools, cx).unwrap();
        let index = match key {
            RowKey::Local(key) => paths.iter().position(|p| p.key() == *key),
            RowKey::Inherited(g, i) if *g == self.generation => Some(*i),
            _ => None,
        };
        let Some(index) = index.filter(|i| *i < paths.len()) else {
            return;
        };
        if action == 0 {
            self.list_inputs
                .remove(&(field, RowKey::Local(paths[index].key())));
            let _ = items.remove(&self.tools, paths[index].clone(), cx);
            if paths.len() == 1 && field == Field::NpmCommand {
                slot(field).set(&self.tools, ListSlot::Unset, cx);
            }
        } else if action < 0 && index > 0 {
            let _ = items.move_before(&self.tools, &paths[index], &paths[index - 1], cx);
        } else if action > 0 && index + 1 < paths.len() {
            let _ = items.move_before(&self.tools, &paths[index + 1], &paths[index], cx);
        }
        cx.notify();
    }
    pub(super) fn render_list(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let saving = self.is_saving();
        let Some(values) = self.list_values(field, cx) else {
            return v_flex()
                .gap_2()
                .child(t(cx, field.label()))
                .child(
                    serde_json::to_string(&self.resolved(field, cx).values[0]).unwrap_or_default(),
                )
                .child(self.render_field_menu(field, self.pending(field, cx), saving, cx))
                .into_any_element();
        };
        let keys = if let Some(paths) = self.list_paths(field, cx) {
            paths
                .try_items(&self.tools, cx)
                .unwrap()
                .into_iter()
                .map(|p| RowKey::Local(p.key()))
                .collect::<Vec<_>>()
        } else {
            (0..values.len())
                .map(|i| RowKey::Inherited(self.generation, i))
                .collect()
        };
        let mut origin = t(
            cx,
            match self.resolved(field, cx).source {
                fields::Source::Default => "pi-source-default",
                fields::Source::Global => "pi-source-global",
                fields::Source::Project => "pi-source-project",
            },
        );
        if self.pending(field, cx) {
            origin = format!("{origin} · {}", t(cx, "pi-unsaved"));
        }
        let mut body = v_flex().gap_2().w_full().child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().flex_1().child(t(cx, field.label())))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(origin),
                )
                .child(self.render_field_menu(field, self.pending(field, cx), saving, cx)),
        );
        for (index, (key, text)) in keys.into_iter().zip(&values).enumerate() {
            let input = self.list_input(field, &key, text, window, cx);
            let mut row = h_flex()
                .gap_2()
                .items_center()
                .child(Input::new(&input).small().disabled(saving).flex_1());
            for (action, icon, label) in [
                (-1, IconName::ArrowUp, "pi-model-scope-up"),
                (1, IconName::ArrowDown, "pi-model-scope-down"),
                (0, IconName::Trash, "pi-list-remove"),
            ] {
                let key = key.clone();
                row = row.child(
                    Button::new(SharedString::from(format!(
                        "list-{field:?}-{index}-{action}"
                    )))
                    .ghost()
                    .small()
                    .icon(icon)
                    .accessibility_label(t(cx, label))
                    .disabled(
                        saving
                            || action < 0 && index == 0
                            || action > 0 && index + 1 == values.len(),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.change_list(field, &key, action, window, cx)
                    })),
                );
            }
            body = body.child(row);
        }
        let entity = cx.entity();
        let add = Button::new(SharedString::from(format!("list-add-{field:?}")))
            .small()
            .label(t(cx, "pi-list-add"))
            .disabled(saving)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.append_list(field, String::new(), window, cx)
            }));
        if field == Field::DefaultTools {
            body = body.child(
                Button::new("pi-tools-known")
                    .small()
                    .label(t(cx, "pi-tools-known"))
                    .disabled(saving)
                    .dropdown_menu(move |menu, _, _cx| {
                        super::advanced::KNOWN_TOOLS
                            .iter()
                            .fold(menu, |menu, name| {
                                let entity = entity.clone();
                                let name = (*name).to_owned();
                                menu.item(PopupMenuItem::new(name.clone()).on_click(
                                    move |_, window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.append_list(field, name.clone(), window, cx)
                                        })
                                    },
                                ))
                            })
                    }),
            );
        }
        body = body.child(add);
        if field == Field::DefaultTools {
            let Load::Ready(files) = &self.files else {
                unreachable!()
            };
            let local = self.snapshot(cx).value(field)[0].raw();
            let tools = if files.project.is_some() {
                super::advanced::startup_tools(files.global.get(&["defaultTools"]), local.as_ref())
            } else {
                super::advanced::startup_tools(local.as_ref(), None)
            };
            body = body
                .child(div().text_sm().child(t(cx, "pi-tools-startup")))
                .child(div().text_sm().child(tools.join(", ")))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-tools-startup-help")),
                );
        } else if values.is_empty() {
            body = body.child(t(cx, "pi-npm-default"));
        }
        if self.invalid(field, cx) {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, "pi-npm-command-required")),
            );
        }
        body.into_any_element()
    }
}
