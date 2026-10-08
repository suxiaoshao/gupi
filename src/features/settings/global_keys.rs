use super::*;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::setting::SettingField;
use gpui_kit::component::setting::SettingGroup;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::prelude::FluentBuilder;
use gupi_settings::shortcuts::InputSource;
use gupi_settings::shortcuts::ShortcutTask;
use gupi_settings::shortcuts::Shortcuts;
mod editor;

pub(super) struct GlobalKeys {
    controller: Entity<ConfigController>,
    inputs: std::collections::BTreeMap<String, Entity<BindingInput>>,
}
impl GlobalKeys {
    pub fn new(controller: Entity<ConfigController>) -> Self {
        Self {
            controller,
            inputs: Default::default(),
        }
    }
    /// The system-wide section: the temporary window launcher and template tasks.
    pub fn groups(owner: &Entity<Self>, filter: &keys::KeysView, cx: &App) -> Vec<SettingGroup> {
        let config = owner.read(cx).controller.read(cx).preferences(cx).shortcuts;
        let row = |id: String, label: String, description: String| {
            let owner = owner.clone();
            SettingItem::new(
                label.clone(),
                SettingField::render(move |_, window, cx| {
                    owner.update(cx, |this, cx| this.render_binding(&id, window, cx))
                }),
            )
            .description(description)
            .keywords(vec![
                label,
                "global system-wide shortcut 全局 系统级 快捷键".into(),
            ])
        };
        let matches = |label: &str, id: &str, binding: &str| {
            filter.matches_binding(
                binding,
                !binding.is_empty(),
                &[
                    label.to_owned(),
                    id.to_owned(),
                    t(cx, "settings-key-group-system"),
                    "global system-wide shortcut 全局 系统级 快捷键".into(),
                ],
                cx,
            )
        };
        let launcher_label = t(cx, "shortcut-launcher");
        let launcher = matches(&launcher_label, "launcher", &config.launcher)
            .then(|| row(String::new(), launcher_label, String::new()));
        let tasks = config
            .tasks
            .iter()
            .filter(|task| matches(&task.name, &task.id, &task.binding))
            .map(|task| {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set(
                    "template",
                    task.template
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
                args.set("source", t(cx, source_key(task.source)));
                row(
                    task.id.clone(),
                    task.name.clone(),
                    gupi_settings::i18n::t_with_args(cx, "settings-key-task-description", &args),
                )
            });
        let items: Vec<_> = launcher.into_iter().chain(tasks).collect();
        if items.is_empty() {
            return vec![];
        }
        vec![
            SettingGroup::new()
                .title(t(cx, "settings-key-group-system"))
                .description(t(cx, "settings-key-system-help"))
                .items(items),
        ]
    }
    pub(super) fn launcher_control(
        owner: &Entity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        owner.update(cx, |this, cx| this.render_binding("", window, cx))
    }
    pub(super) fn add_button(owner: &Entity<Self>, cx: &App) -> Button {
        let disabled = owner.read(cx).controller.read(cx).busy(cx);
        let owner = owner.clone();
        Button::new("add-shortcut-task")
            .small()
            .icon(IconName::Plus)
            .label(t(cx, "shortcut-add"))
            .disabled(disabled)
            .on_click(move |_, window, cx| owner.update(cx, |this, cx| this.edit(None, window, cx)))
    }
    fn render_binding(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input = self
            .inputs
            .entry(id.to_owned())
            .or_insert_with(|| {
                cx.new(|cx| BindingInput::new(id.to_owned(), self.controller.clone(), window, cx))
            })
            .clone();
        let row = h_flex().gap_2().items_center().child(input);
        if id.is_empty() {
            return row.into_any_element();
        }
        let toggle = id.to_owned();
        let enabled = self
            .controller
            .read(cx)
            .preferences(cx)
            .shortcuts
            .tasks
            .iter()
            .find(|task| task.id == id)
            .is_some_and(|task| task.enabled);
        let busy = self.controller.read(cx).busy(cx);
        let owner = cx.entity().downgrade();
        let task = id.to_owned();
        row.child(
            gpui_kit::component::switch::Switch::new(format!("enable-task-{id}"))
                .small()
                .checked(enabled)
                .disabled(busy)
                .accessibility_label(t(cx, "settings-key-task-enabled"))
                .on_click(cx.listener(move |this, checked, _, cx| {
                    let mut config = this.controller.read(cx).preferences(cx).shortcuts;
                    if let Some(task) = config.tasks.iter_mut().find(|task| task.id == toggle) {
                        task.enabled = *checked;
                    }
                    this.controller.update(cx, |c, cx| {
                        c.set_preference(PreferenceChange::Shortcuts(config), cx)
                    });
                })),
        )
        .child(
            Button::new(format!("task-actions-{id}"))
                .ghost()
                .small()
                .icon(IconName::Ellipsis)
                .tooltip(t(cx, "settings-key-task-actions"))
                .accessibility_label(t(cx, "settings-key-task-actions"))
                .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                    let edit = owner.clone();
                    let remove = owner.clone();
                    let edit_id = task.clone();
                    let remove_id = task.clone();
                    menu.item(
                        PopupMenuItem::new(t(cx, "shortcut-edit"))
                            .icon(IconName::SquarePen)
                            .disabled(busy)
                            .on_click(move |_, window, cx| {
                                edit.update(cx, |this, cx| {
                                    this.edit(Some(edit_id.clone()), window, cx)
                                })
                                .ok();
                            }),
                    )
                    .separator()
                    .item(
                        PopupMenuItem::new(t(cx, "shortcut-delete-ellipsis"))
                            .icon(IconName::Trash)
                            .disabled(busy)
                            .on_click(move |_, window, cx| {
                                remove
                                    .update(cx, |this, cx| {
                                        this.confirm_delete(remove_id.clone(), window, cx)
                                    })
                                    .ok();
                            }),
                    )
                }),
        )
        .into_any_element()
    }
    fn confirm_delete(&mut self, target: String, window: &mut Window, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let target = target.clone();
            let controller = controller.clone();
            dialog
                .title(t(cx, "shortcut-delete"))
                .footer(dialog_buttons("shortcut-delete", false, false, cx))
                .on_ok(move |_, _, cx| {
                    let mut config = controller.read(cx).preferences(cx).shortcuts;
                    config.tasks.retain(|task| task.id != target);
                    controller.update(cx, |c, cx| {
                        c.set_preference(PreferenceChange::Shortcuts(config), cx)
                    });
                    true
                })
        });
    }
    fn edit(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let definition = id
            .and_then(|id| {
                self.controller
                    .read(cx)
                    .preferences(cx)
                    .shortcuts
                    .tasks
                    .into_iter()
                    .find(|t| t.id == id)
            })
            .unwrap_or_else(|| {
                let mut record = ShortcutTask::default();
                record.id = uuid::Uuid::new_v4().to_string();
                record.enabled = true;
                record
            });
        editor::open(definition, self.controller.clone(), window, cx);
    }
}

fn source_key(source: InputSource) -> &'static str {
    match source {
        InputSource::Screenshot => "shortcut-screenshot",
        InputSource::Selection => "shortcut-selection",
        InputSource::Clipboard => "shortcut-clipboard",
        InputSource::SelectionOrClipboard => "shortcut-fallback",
    }
}

/// One system-wide binding. Recording and clearing change only the draft;
/// Save validates registration before writing preferences.
struct BindingInput {
    id: String,
    controller: Entity<ConfigController>,
    focus: FocusHandle,
    draft: String,
    saved: String,
    error: Option<String>,
    capture: Option<Subscription>,
    _subscriptions: Vec<Subscription>,
}
fn value(config: &Shortcuts, id: &str) -> String {
    if id.is_empty() {
        config.launcher.clone()
    } else {
        config
            .tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.binding.clone())
            .unwrap_or_default()
    }
}
impl BindingInput {
    fn new(
        id: String,
        controller: Entity<ConfigController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let saved = value(&controller.read(cx).preferences(cx).shortcuts, &id);
        let focus = cx.focus_handle().tab_stop(true);
        let blur = cx.on_focus_out(&focus, window, |this, _, _, cx| {
            this.capture = None;
            cx.notify();
        });
        let store = controller.read(cx).configuration();
        let config = store.observe_in(cx, window, |this, op, _, cx| {
            if !op.is_running()
                && let Some(config) = op.data().and_then(|d| d.configured())
            {
                let next = value(&config.shortcuts, &this.id);
                if next != this.saved {
                    this.saved = next.clone();
                    this.draft = next;
                }
            }
            cx.notify();
        });
        Self {
            id,
            controller,
            focus,
            draft: saved.clone(),
            saved,
            error: None,
            capture: None,
            _subscriptions: vec![blur, config],
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) || self.capture.is_some() {
            return;
        }
        let mut candidate = self.controller.read(cx).preferences(cx);
        let binding = self.draft.trim().to_owned();
        if self.id.is_empty() {
            candidate.shortcuts.launcher = binding.clone();
        } else if let Some(task) = candidate
            .shortcuts
            .tasks
            .iter_mut()
            .find(|t| t.id == self.id)
        {
            task.binding = binding.clone();
        }
        self.error = crate::app::shortcuts::validate_registration(&candidate, cx).err();
        if self.error.is_some() {
            cx.notify();
            return;
        }
        let config = candidate.shortcuts;
        let onboarding = self.controller.read(cx).is_onboarding(cx);
        self.controller.update(cx, |c, cx| {
            c.set_preference(PreferenceChange::Shortcuts(config), cx)
        });
        if onboarding {
            self.saved = binding;
        }
        cx.notify();
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.capture = None;
        self.error = None;
        self.draft = self.saved.clone();
        cx.notify();
    }
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.capture = None;
        self.error = None;
        self.draft.clear();
        cx.notify();
    }
    fn record(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        self.focus.focus(window, cx);
        self.error = None;
        let listener = cx.listener(|this, event: &KeystrokeEvent, _, cx| {
            cx.stop_propagation();
            // Escape ends recording without becoming the binding.
            if event.keystroke.key != "escape" {
                this.draft = event.keystroke.unparse();
            }
            this.capture = None;
            cx.notify();
        });
        self.capture = Some(cx.intercept_keystrokes(listener));
        cx.notify();
    }
}
impl Render for BindingInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.controller.read(cx).busy(cx);
        let recording = self.capture.is_some();
        let dirty = self.draft != self.saved;
        let has_value = !self.draft.is_empty();
        let owner = cx.entity().downgrade();
        let area = keys::key_area("binding-area", &self.focus, &self.draft, recording, cx)
            .on_click(cx.listener(|this, _, window, cx| {
                if this.capture.is_none() {
                    this.record(window, cx);
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.capture.is_some() {
                    return;
                }
                let dirty = this.draft != this.saved;
                match event.keystroke.key.as_str() {
                    "enter" if dirty => this.save(cx),
                    "enter" | "space" => this.record(window, cx),
                    "escape" if dirty => this.cancel(cx),
                    _ => return,
                }
                cx.stop_propagation();
            }));
        v_flex()
            .gap_1()
            .items_end()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .when(dirty && !recording, |row| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "settings-key-unsaved")),
                        )
                    })
                    .child(area)
                    .when(dirty || recording, |row| {
                        row.child(
                            Button::new("confirm")
                                .small()
                                .label(t(cx, "settings-key-save"))
                                .disabled(busy || recording)
                                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                        )
                        .child(
                            Button::new("cancel")
                                .small()
                                .ghost()
                                .label(t(cx, "action-cancel"))
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                        )
                    })
                    .child(
                        Button::new("binding-actions")
                            .ghost()
                            .small()
                            .icon(IconName::Ellipsis)
                            .tooltip(t(cx, "settings-key-actions"))
                            .accessibility_label(t(cx, "settings-key-actions"))
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                                let record = owner.clone();
                                let clear = owner.clone();
                                menu.item(
                                    PopupMenuItem::new(t(cx, "settings-key-record"))
                                        .icon(IconName::Keyboard)
                                        .disabled(busy)
                                        .on_click(move |_, window, cx| {
                                            record
                                                .update(cx, |this, cx| this.record(window, cx))
                                                .ok();
                                        }),
                                )
                                .item(
                                    PopupMenuItem::new(t(cx, "settings-key-clear"))
                                        .icon(IconName::Eraser)
                                        .disabled(busy || !has_value)
                                        .on_click(move |_, _, cx| {
                                            clear.update(cx, |this, cx| this.clear(cx)).ok();
                                        }),
                                )
                            }),
                    ),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error))
            }))
    }
}
