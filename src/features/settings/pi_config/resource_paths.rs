//! Scope-local resource arrays. Values are JSON strings, not resolved tool names.
use super::lists::{ListItem, ListRows, ListSlot};
use super::*;
use gpui_form::DynamicItemsPath;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gupi_settings::assets::IconName;

pub(super) struct EntryInput {
    input: Entity<InputState>,
    open: bool,
    epoch: u64,
    menu_trigger: Option<(gpui_form::PathKey, FocusHandle)>,
    row_focus: std::collections::HashMap<gpui_form::PathKey, EntryFocus>,
    _subscription: Subscription,
}

struct EntryFocus {
    scope: FocusHandle,
    _subscription: Subscription,
}

fn path_label(text: &str) -> String {
    if text.is_empty() || text.trim() != text {
        serde_json::to_string(text).unwrap()
    } else {
        text.to_owned()
    }
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
        let subscription =
            cx.subscribe_in(&input, window, move |this, input, event, window, cx| {
                if !matches!(event, InputEvent::PressEnter { .. }) {
                    return;
                }
                let Some(editor) = this.resource_inputs.get(&field) else {
                    return;
                };
                let epoch = editor.epoch;
                let input = input.clone();
                cx.defer_in(window, move |this, window, cx| {
                    if this.resource_inputs.get(&field).is_some_and(|editor| {
                        editor.open && editor.epoch == epoch && editor.input == input
                    }) {
                        this.finish_resource_entry(field, true, window, cx);
                    }
                });
            });
        self.resource_inputs.insert(
            field,
            EntryInput {
                input: input.clone(),
                open: false,
                epoch: 0,
                menu_trigger: None,
                row_focus: Default::default(),
                _subscription: subscription,
            },
        );
        input
    }

    fn resource_row_focus(
        &mut self,
        field: Field,
        key: &gpui_form::PathKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        if let Some(row) = self.resource_inputs[&field].row_focus.get(key) {
            return row.scope.clone();
        }
        let scope = cx.focus_handle();
        let (entity, key_for_focus) = (cx.entity().downgrade(), key.clone());
        let subscription = window.on_focus_in(&scope, cx, move |window, cx| {
            let focus = window.focused(cx);
            let _ = entity.update(cx, |this, _| {
                if let Some(editor) = this.resource_inputs.get_mut(&field) {
                    editor.menu_trigger = focus.map(|focus| (key_for_focus.clone(), focus));
                }
            });
        });
        self.resource_inputs
            .get_mut(&field)
            .unwrap()
            .row_focus
            .insert(
                key.clone(),
                EntryFocus {
                    scope: scope.clone(),
                    _subscription: subscription,
                },
            );
        scope
    }

    fn open_resource_entry(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_saving() {
            return;
        }
        let input = self.resource_editor(field, window, cx);
        let editor = self.resource_inputs.get_mut(&field).unwrap();
        editor.open = true;
        editor.epoch += 1;
        input.update(cx, |input, cx| input.set_value("", window, cx));
        window.focus(&input.read(cx).focus_handle(cx), cx);
        cx.notify();
    }

    fn finish_resource_entry(
        &mut self,
        field: Field,
        append: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            return;
        }
        let Some(editor) = self
            .resource_inputs
            .get(&field)
            .filter(|editor| editor.open)
        else {
            return;
        };
        let input = editor.input.clone();
        let text = input.read(cx).value().to_string();
        // The stable Add button follows the input and stays mounted when it closes.
        if input.read(cx).focus_handle(cx).is_focused(window) {
            window.focus_next(cx);
        }
        if append {
            self.append_resource_path(field, text, cx);
        }
        let editor = self.resource_inputs.get_mut(&field).unwrap();
        editor.open = false;
        editor.epoch += 1;
        input.update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    fn change_resource_entry(
        &mut self,
        field: Field,
        key: &gpui_form::PathKey,
        action: i8,
        trigger: Option<&FocusHandle>,
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
            if let Some(trigger) = trigger {
                // DropdownMenu restores its own non-tab-stop scope handle. Use the
                // actual button handle captured before keyboard activation.
                window.focus(trigger, cx);
                let input_open = self
                    .resource_inputs
                    .get(&field)
                    .is_some_and(|editor| editor.open);
                for _ in 0..(paths.len() - index + usize::from(input_open)) {
                    window.focus_next(cx);
                }
            }
            let _ = items.remove(&self.tools, path.clone(), cx);
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
            .gap_1()
            .p_1()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .bg(cx.theme().group_box);
        match list {
            ListSlot::Unreadable(raw) => {
                self.resource_inputs
                    .get_mut(&field)
                    .unwrap()
                    .row_focus
                    .clear();
                let text = raw.value.to_string();
                return content
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "pi-resource-unsupported")),
                    )
                    .child(
                        resource_text("raw", text.clone(), text, cx)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .into_any_element();
            }
            ListSlot::Unset => {
                self.resource_inputs
                    .get_mut(&field)
                    .unwrap()
                    .row_focus
                    .clear();
                content = content.child(
                    div()
                        .px_1()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-resource-absent")),
                )
            }
            ListSlot::Set(rows) => {
                if rows.items.is_empty() {
                    content = content.child(
                        div()
                            .px_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "pi-resource-empty")),
                    );
                }
                let items = self.resource_items(field, cx).unwrap();
                let paths = items.try_items(&self.tools, cx).unwrap();
                let count = paths.len();
                let active: std::collections::HashSet<_> =
                    paths.iter().map(|path| path.key()).collect();
                self.resource_inputs
                    .get_mut(&field)
                    .unwrap()
                    .row_focus
                    .retain(|key, _| active.contains(key));
                for (index, (path, row)) in paths.into_iter().zip(rows.items).enumerate() {
                    let key = path.key();
                    let scope = self.resource_row_focus(field, &key, window, cx);
                    let entity = cx.entity();
                    content = content.child(
                        h_flex()
                            .id(SharedString::from(format!(
                                "resource-{}-{key:?}",
                                field.id()
                            )))
                            .w_full()
                            .min_w_0()
                            .gap_1()
                            .track_focus(&scope)
                            .child(
                                resource_text("path", path_label(&row.text), row.text, cx).flex_1(),
                            )
                            .child(
                                Button::new("actions")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::Ellipsis)
                                    .accessibility_label(t(cx, "pi-resource-entry-actions"))
                                    .disabled(self.is_saving())
                                    .dropdown_menu(move |mut menu, _, cx| {
                                        let trigger = entity
                                            .read(cx)
                                            .resource_inputs
                                            .get(&field)
                                            .and_then(|editor| editor.menu_trigger.as_ref())
                                            .filter(|(row, _)| row == &key)
                                            .map(|(_, focus)| focus.clone());
                                        for (action, label, icon, applicable) in [
                                            (-1, "pi-resource-up", IconName::ChevronUp, index > 0),
                                            (
                                                1,
                                                "pi-resource-down",
                                                IconName::ChevronDown,
                                                index + 1 < count,
                                            ),
                                            (0, "pi-resource-remove", IconName::Trash, true),
                                        ] {
                                            if !applicable {
                                                continue;
                                            }
                                            let (entity, key, trigger) =
                                                (entity.clone(), key.clone(), trigger.clone());
                                            menu = menu.item(
                                                PopupMenuItem::new(t(cx, label))
                                                    .icon(icon)
                                                    .on_click(move |_, window, cx| {
                                                        let (entity, key, trigger) = (
                                                            entity.clone(),
                                                            key.clone(),
                                                            trigger.clone(),
                                                        );
                                                        // PopupMenu invokes callbacks before restoring trigger focus.
                                                        window.defer(cx, move |window, cx| {
                                                            entity.update(cx, |this, cx| {
                                                                this.change_resource_entry(
                                                                    field,
                                                                    &key,
                                                                    action,
                                                                    trigger.as_ref(),
                                                                    window,
                                                                    cx,
                                                                )
                                                            })
                                                        });
                                                    }),
                                            );
                                        }
                                        menu
                                    }),
                            ),
                    );
                }
            }
        }
        let open = self.resource_inputs[&field].open;
        let focus_input = input.clone();
        content
            .child(
                h_flex()
                    .id("add-line")
                    .w_full()
                    .min_w_0()
                    .gap_1()
                    .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape"
                            && focus_input.read(cx).focus_handle(cx).is_focused(window)
                        {
                            this.finish_resource_entry(field, false, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .when(open, |row| {
                        row.child(
                            div().flex_1().min_w_0().child(
                                Input::new(&input)
                                    .small()
                                    .aria_label(t(cx, field.label()))
                                    .disabled(self.is_saving()),
                            ),
                        )
                    })
                    .child(
                        Button::new(SharedString::from(format!("resource-add-{}", field.id())))
                            .small()
                            .ghost()
                            .icon(if open {
                                IconName::Check
                            } else {
                                IconName::Plus
                            })
                            .label(t(cx, "pi-resource-add"))
                            .disabled(self.is_saving())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if open {
                                    this.finish_resource_entry(field, true, window, cx);
                                } else {
                                    this.open_resource_entry(field, window, cx);
                                }
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
