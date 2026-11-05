use super::*;
use crate::features::settings::composition::{SettingsRow, SettingsSection, control};
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
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
    pub fn groups(owner: &Entity<Self>, filter: &keys::KeysView, cx: &App) -> Vec<SettingsSection> {
        let config = owner.read(cx).controller.read(cx).preferences(cx).shortcuts;
        let row = |id: String, label: String, description: String| {
            let owner = owner.clone();
            let remove = owner.clone();
            let delete_id = id.clone();
            let delete_label = label.clone();
            let row = SettingsRow::new(
                id.clone(),
                label.clone(),
                control(move |_, window, cx| {
                    owner.update(cx, |this, cx| this.render_binding(&id, window, cx))
                }),
            )
            .framed(false)
            .description(description)
            .keywords(vec![
                label,
                "global system-wide shortcut 全局 系统级 快捷键".into(),
            ]);
            if delete_id.is_empty() {
                return row;
            }
            row.leading(control(move |_, _, cx| {
                let remove = remove.clone();
                let target = delete_id.clone();
                div().flex_shrink_0().child(
                    Button::new(format!("delete-task-{delete_id}"))
                        .ghost()
                        .small()
                        .icon(IconName::Trash)
                        .tooltip(format!(
                            "{}: {}",
                            t(cx, "shortcut-delete-ellipsis"),
                            delete_label
                        ))
                        .accessibility_label(format!(
                            "{}: {}",
                            t(cx, "shortcut-delete-ellipsis"),
                            delete_label
                        ))
                        .disabled(remove.read(cx).controller.read(cx).busy(cx))
                        .on_click(move |_, window, cx| {
                            remove.update(cx, |this, cx| {
                                this.confirm_delete(target.clone(), window, cx)
                            })
                        }),
                )
            }))
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
        let launcher = matches(&launcher_label, "launcher", &config.launcher).then(|| {
            row(
                String::new(),
                launcher_label,
                t(cx, "settings-key-launcher-help"),
            )
        });
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
        let add = owner.clone();
        vec![
            SettingsSection::new()
                .bindings()
                .title(t(cx, "settings-key-group-system"))
                .description(t(cx, "settings-key-system-help"))
                .tools(move |_, _, cx| Self::add_button(&add, cx).into_any_element())
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
            .outline()
            .flex_shrink_0()
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
        let row = h_flex()
            .w_full()
            .min_w_0()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(input);
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
            Button::new(format!("edit-task-{id}"))
                .ghost()
                .small()
                .icon(IconName::SquarePen)
                .tooltip(t(cx, "shortcut-edit"))
                .accessibility_label(t(cx, "shortcut-edit"))
                .disabled(busy)
                .on_click(move |_, window, cx| {
                    owner
                        .update(cx, |this, cx| this.edit(Some(task.clone()), window, cx))
                        .ok();
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
                .title({
                    let name = controller
                        .read(cx)
                        .preferences(cx)
                        .shortcuts
                        .tasks
                        .iter()
                        .find(|task| task.id == target)
                        .map(|task| task.name.clone())
                        .unwrap_or_default();
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("name", name);
                    gupi_settings::i18n::t_with_args(cx, "shortcut-delete-title", &args)
                })
                .footer(dialog_buttons(
                    "shortcut-delete",
                    false,
                    false,
                    DialogCommit::Danger,
                    cx,
                ))
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
        let creating = id.is_none();
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
        editor::open(definition, creating, self.controller.clone(), window, cx);
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
/// confirmation validates registration before writing preferences.
struct BindingInput {
    id: String,
    controller: Entity<ConfigController>,
    focus: FocusHandle,
    region: FocusHandle,
    draft: String,
    saved: String,
    error: Option<String>,
    capture: Option<Subscription>,
    editing: bool,
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
        let region = cx.focus_handle();
        let focus_in = cx.on_focus_in(&region, window, |_, _, cx| cx.notify());
        let focus_out = cx.on_focus_out(&region, window, |_, _, _, cx| cx.notify());
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
            region,
            draft: saved.clone(),
            saved,
            error: None,
            capture: None,
            editing: false,
            _subscriptions: vec![blur, config, focus_in, focus_out],
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
        self.editing = false;
        if onboarding {
            self.saved = binding;
        }
        cx.notify();
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.editing = false;
        self.capture = None;
        self.error = None;
        self.draft = self.saved.clone();
        cx.notify();
    }
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        self.editing = true;
        self.capture = None;
        self.error = None;
        self.draft.clear();
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn record(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        self.editing = true;
        self.focus.focus(window, cx);
        self.error = None;
        let listener = cx.listener(|this, event: &KeystrokeEvent, _, cx| {
            cx.stop_propagation();
            // Escape ends recording without becoming the binding.
            if event.keystroke.key != "escape" {
                this.draft = event.keystroke.unparse(cx.platform_info().os);
            }
            this.capture = None;
            cx.notify();
        });
        self.capture = Some(cx.intercept_keystrokes(listener));
        cx.notify();
    }
}
impl Render for BindingInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.controller.read(cx).busy(cx);
        let recording = self.capture.is_some();
        let dirty = self.draft != self.saved;
        let has_value = !self.draft.is_empty();
        let command = if self.id.is_empty() {
            t(cx, "shortcut-launcher")
        } else {
            self.controller
                .read(cx)
                .preferences(cx)
                .shortcuts
                .tasks
                .iter()
                .find(|task| task.id == self.id)
                .map(|task| task.name.clone())
                .unwrap_or_else(|| t(cx, "shortcut-name"))
        };
        let area = keys::key_area(
            "binding-area",
            &self.focus,
            &self.draft,
            &command,
            recording,
            cx,
        )
        .disabled(busy)
        .on_click(cx.listener(|this, event, window, cx| {
            if this.capture.is_some() {
                return;
            }
            if matches!(event, ClickEvent::Keyboard(event) if event.button == KeyboardButton::Enter)
                && this.draft != this.saved
            {
                this.save(cx);
            } else {
                this.record(window, cx);
            }
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" && this.capture.is_none() && this.editing {
                this.cancel(cx);
                cx.stop_propagation();
            }
        }));
        let clear = (has_value && !recording).then(|| {
            Button::new("binding-clear")
                .small()
                .ghost()
                .icon(IconName::X)
                .tooltip(format!("{}: {}", t(cx, "settings-key-clear"), command))
                .accessibility_label(format!("{}: {}", t(cx, "settings-key-clear"), command))
                .disabled(busy)
                .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx)))
        });
        let area = keys::binding_capsule(
            "binding-region",
            "binding-region".into(),
            &self.region,
            area,
            clear,
            window,
            cx,
        );
        v_flex()
            .max_w_full()
            .gap_1()
            .items_start()
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .flex_wrap()
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
                    .when(self.editing || dirty, |row| {
                        row.child(
                            Button::new("cancel")
                                .small()
                                .ghost()
                                .icon(IconName::X)
                                .label(t(cx, "action-cancel"))
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                        )
                        .child(
                            Button::new("confirm")
                                .primary()
                                .small()
                                .icon(IconName::Check)
                                .label(t(cx, "settings-key-confirm"))
                                .disabled(busy || recording || !dirty)
                                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                        )
                    }),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error))
            }))
    }
}
