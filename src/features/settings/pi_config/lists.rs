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
use gpui_form::DynamicItemsPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex};
use gupi_settings::assets::IconName;

impl PiConfig {
    pub(super) fn list_paths(
        &self,
        field: Field,
        cx: &App,
    ) -> Option<DynamicItemsPath<ToolsDraft, ListItem>> {
        assert_eq!(field, Field::DefaultTools);
        ToolsDraft::DEFAULT_TOOLS
            .case(ListSlot::SET)
            .resolve(&self.tools, cx)
            .ok()
            .flatten()
            .map(|p| p.then(ListRows::ITEMS))
    }
    fn tool_values(&self, cx: &App) -> Option<Vec<String>> {
        match self.resolved(Field::DefaultTools, cx).values[0].clone() {
            None => Some(Vec::new()),
            Some(value) => serde_json::from_value(value).ok(),
        }
    }
    fn materialize_tools(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<DynamicItemsPath<ToolsDraft, ListItem>> {
        if self.is_saving() {
            return None;
        }
        if self.list_paths(Field::DefaultTools, cx).is_none() {
            let items = self
                .tool_values(cx)?
                .into_iter()
                .map(|text| ListItem { text })
                .collect();
            ToolsDraft::DEFAULT_TOOLS.set(&self.tools, ListSlot::Set(ListRows { items }), cx);
        }
        self.list_paths(Field::DefaultTools, cx)
    }
    pub(super) fn append_list(
        &mut self,
        field: Field,
        text: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        assert_eq!(field, Field::DefaultTools);
        if let Some(items) = self.materialize_tools(cx) {
            let _ = items.append(&self.tools, ListItem { text }, cx);
            cx.notify();
        }
    }
    pub(super) fn change_list(
        &mut self,
        field: Field,
        key: &RowKey,
        _action: i8,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        assert_eq!(field, Field::DefaultTools);
        let Some(items) = self.materialize_tools(cx) else {
            return;
        };
        let paths = items.try_items(&self.tools, cx).unwrap();
        let index = match key {
            RowKey::Local(key) => paths.iter().position(|p| p.key() == *key),
            RowKey::Inherited(g, i) if *g == self.generation => Some(*i),
            _ => None,
        };
        if let Some(path) = index.and_then(|i| paths.get(i)) {
            let _ = items.remove(&self.tools, path.clone(), cx);
            cx.notify();
        }
    }
    pub(super) fn tools_preview(&self, cx: &App) -> String {
        let Load::Ready(files) = &self.files else {
            return String::new();
        };
        let local = self.snapshot(cx).value(Field::DefaultTools)[0].raw();
        let tools = if files.project.is_some() {
            super::advanced::startup_tools(files.global.get(&["defaultTools"]), local.as_ref())
        } else {
            super::advanced::startup_tools(local.as_ref(), None)
        };
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("tools", tools.join(" · "));
        gupi_settings::i18n::t_with_args(cx, "pi-tools-preview", &args)
    }
    pub(super) fn render_tools_chips(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let field = Field::DefaultTools;
        let Some(values) = self.tool_values(cx) else {
            return div()
                .text_sm()
                .child(fields::display(self.resolved(field, cx).values[0].as_ref()))
                .into_any_element();
        };
        let keys = if let Some(items) = self.list_paths(field, cx) {
            items
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
        let mut chips = h_flex()
            .w_full()
            .min_w_0()
            .gap_1()
            .p_1()
            .flex_wrap()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .bg(cx.theme().group_box);
        let count = values.len();
        for (index, (key, text)) in keys.into_iter().zip(values).enumerate() {
            let to_add = count - index;
            let delete = key.clone();
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("tool", text.clone());
            let remove = gupi_settings::i18n::t_with_args(cx, "pi-tool-remove", &args);
            chips = chips.child(
                h_flex()
                    .id(SharedString::from(format!("tool-chip-{key:?}")))
                    .gap_1()
                    .pl_2()
                    .rounded_sm()
                    .bg(cx.theme().muted)
                    .child(div().text_sm().max_w_48().truncate().child(text))
                    .child(
                        Button::new("delete")
                            .xsmall()
                            .ghost()
                            .icon(IconName::X)
                            .accessibility_label(remove.clone())
                            .tooltip(remove)
                            .disabled(self.is_saving())
                            .on_click(cx.listener(
                                move |this, event: &gpui_kit::ClickEvent, window, cx| {
                                    if event.is_keyboard() {
                                        // The Add trigger survives inherited -> local materialization;
                                        // sibling chip occurrences may acquire new Form identities.
                                        for _ in 0..to_add {
                                            window.focus_next(cx);
                                        }
                                    }
                                    this.change_list(field, &delete, 0, window, cx)
                                },
                            )),
                    ),
            );
        }
        let entity = cx.entity();
        chips = chips.child(
            Button::new("pi-tools-add")
                .small()
                .ghost()
                .label(t(cx, "pi-tools-add"))
                .dropdown_caret(true)
                .disabled(self.is_saving())
                .dropdown_menu(move |menu, _, cx| {
                    let menu = super::advanced::KNOWN_TOOLS
                        .iter()
                        .fold(menu, |menu, name| {
                            let (entity, name) = (entity.clone(), (*name).to_owned());
                            menu.item(PopupMenuItem::new(name.clone()).on_click(
                                move |_, window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.append_list(field, name.clone(), window, cx)
                                    })
                                },
                            ))
                        });
                    let entity = entity.clone();
                    menu.separator()
                        .item(PopupMenuItem::new(t(cx, "pi-tools-custom")).on_click(
                            move |_, window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.custom_tool_open = true;
                                    this.custom_tool
                                        .update(cx, |input, cx| input.set_value("", window, cx));
                                    window.focus(&this.custom_tool.read(cx).focus_handle(cx), cx);
                                    cx.notify();
                                })
                            },
                        ))
                }),
        );
        if self.custom_tool_open {
            chips = chips
                .child(
                    Input::new(&self.custom_tool)
                        .small()
                        .w_32()
                        .aria_label(t(cx, "pi-tools-custom"))
                        .disabled(self.is_saving()),
                )
                .child(
                    Button::new("pi-tool-confirm")
                        .small()
                        .ghost()
                        .icon(IconName::Check)
                        .accessibility_label(t(cx, "pi-tools-add"))
                        .disabled(self.is_saving())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.finish_custom_tool(window, cx)),
                        ),
                );
        }
        let _ = window;
        chips.into_any_element()
    }
    pub(super) fn finish_custom_tool(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        let text = self.custom_tool.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        self.append_list(Field::DefaultTools, text, window, cx);
        self.custom_tool
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }
}
