//! Scope-local resource arrays. Values are JSON strings, not resolved tool names.
use super::lists::{ListItem, ListRows, ListSlot};
use super::*;
use gpui_form::DynamicItemsPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gupi_settings::assets::IconName;

impl PiConfig {
    fn resource_list(&self, field: Field, cx: &App) -> ListSlot {
        let baseline = ListSlot::decode(self.baseline.value(field).remove(0));
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
    pub(super) fn render_resource_paths(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let list = self.resource_list(field, cx);
        let input = self
            .resource_inputs
            .entry(field)
            .or_insert_with(|| cx.new(|cx| InputState::new(window, cx)))
            .clone();
        let mut content = v_flex().w_full().gap_1();
        match list {
            ListSlot::Unreadable(raw) => {
                return content
                    .child(div().text_sm().child(t(cx, "pi-resource-unsupported")))
                    .child(div().text_xs().child(raw.value.to_string()))
                    .into_any_element();
            }
            ListSlot::Unset => {
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-resource-absent")),
                )
            }
            ListSlot::Set(rows) => {
                if rows.items.is_empty() {
                    content = content.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "pi-resource-empty")),
                    );
                }
                let items = self.resource_items(field, cx).unwrap();
                let paths = items.try_items(&self.tools, cx).unwrap();
                let count = paths.len();
                for (index, (path, row)) in paths.into_iter().zip(rows.items).enumerate() {
                    let mut entry = h_flex()
                        .id(SharedString::from(format!(
                            "resource-{}-{:?}",
                            field.id(),
                            path.key()
                        )))
                        .w_full()
                        .gap_1()
                        .px_2()
                        .rounded_sm()
                        .bg(cx.theme().muted)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_sm()
                                .truncate()
                                .child(serde_json::to_string(&row.text).unwrap()),
                        );
                    for (action, icon, label, disabled) in [
                        (-1, IconName::ArrowUp, "pi-resource-up", index == 0),
                        (
                            1,
                            IconName::ArrowDown,
                            "pi-resource-down",
                            index + 1 == count,
                        ),
                        (0, IconName::X, "pi-resource-remove", false),
                    ] {
                        let (items, path) = (items.clone(), path.clone());
                        let focus = input.clone();
                        entry = entry.child(
                            Button::new(SharedString::from(format!("action-{action}")))
                                .xsmall()
                                .ghost()
                                .icon(icon)
                                .tooltip(t(cx, label))
                                .accessibility_label(t(cx, label))
                                .disabled(disabled || self.is_saving())
                                .on_click(cx.listener(
                                    move |this, event: &ClickEvent, window, cx| {
                                        if this.is_saving() {
                                            return;
                                        }
                                        if action == 0 {
                                            if event.is_keyboard() {
                                                window.focus(&focus.read(cx).focus_handle(cx), cx);
                                            }
                                            let _ = items.remove(&this.tools, path.clone(), cx);
                                        } else if let Ok(paths) = items.try_items(&this.tools, cx) {
                                            let Some(index) =
                                                paths.iter().position(|p| p.key() == path.key())
                                            else {
                                                return;
                                            };
                                            if action > 0 && index + 1 < paths.len() {
                                                let _ = items.move_before(
                                                    &this.tools,
                                                    &paths[index + 1],
                                                    &path,
                                                    cx,
                                                );
                                            } else if action < 0 && index > 0 {
                                                let _ = items.move_before(
                                                    &this.tools,
                                                    &path,
                                                    &paths[index - 1],
                                                    cx,
                                                );
                                            }
                                        }
                                        cx.notify();
                                    },
                                )),
                        );
                    }
                    content = content.child(entry);
                }
            }
        }
        content
            .child(
                h_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        Input::new(&input)
                            .small()
                            .aria_label(t(cx, field.label()))
                            .disabled(self.is_saving()),
                    )
                    .child(
                        Button::new(SharedString::from(format!("resource-add-{}", field.id())))
                            .small()
                            .ghost()
                            .icon(IconName::Plus)
                            .tooltip(t(cx, "pi-resource-add"))
                            .accessibility_label(t(cx, "pi-resource-add"))
                            .disabled(self.is_saving())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let text = input.read(cx).value().to_string();
                                this.append_resource_path(field, text, cx);
                                input.update(cx, |input, cx| input.set_value("", window, cx));
                            })),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn render_resource_paths_menu(
        &self,
        field: Field,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let unsupported = matches!(self.resource_list(field, cx), ListSlot::Unreadable(_));
        let pending = self.pending(field, cx);
        let entity = cx.entity();
        Button::new(SharedString::from(format!("resource-menu-{}", field.id())))
            .w_6()
            .flex_shrink_0()
            .ghost()
            .xsmall()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-field-actions"))
            .disabled(self.is_saving() || unsupported)
            .dropdown_menu(move |menu, _, cx| {
                let reset = entity.clone();
                let empty = entity.clone();
                let remove = entity.clone();
                menu.item(
                    PopupMenuItem::new(t(cx, "pi-menu-revert"))
                        .disabled(!pending)
                        .on_click(move |_, _, cx| {
                            reset.update(cx, |this, cx| this.clear_draft(field, cx))
                        }),
                )
                .item(
                    PopupMenuItem::new(t(cx, "pi-resource-clear")).on_click(move |_, _, cx| {
                        empty.update(cx, |this, cx| {
                            this.set_values(field, vec![Slot::Set(serde_json::json!([]))], cx)
                        })
                    }),
                )
                .item(
                    PopupMenuItem::new(t(cx, "pi-resource-remove-key")).on_click(
                        move |_, _, cx| {
                            remove.update(cx, |this, cx| {
                                this.set_values(field, vec![Slot::Unset], cx)
                            })
                        },
                    ),
                )
            })
            .into_any_element()
    }
}
