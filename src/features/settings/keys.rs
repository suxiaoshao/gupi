use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::setting::RenderOptions;
use gpui_kit::component::setting::SettingField;
use gpui_kit::component::setting::SettingGroup;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::prelude::FluentBuilder;
use gupi_settings::commands::Kind;
use gupi_settings::keybindings;
use gupi_settings::keybindings::COMMANDS;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    Application,
    Conversation,
    Workspace,
    Files,
    Temporary,
}
impl Group {
    pub const ALL: [Self; 5] = [
        Self::Application,
        Self::Conversation,
        Self::Workspace,
        Self::Files,
        Self::Temporary,
    ];
    pub fn of(kind: Kind) -> Self {
        match kind {
            Kind::Palette
            | Kind::QuickOpen
            | Kind::New
            | Kind::Settings
            | Kind::Sidebar
            | Kind::Scan
            | Kind::ShowMain
            | Kind::Quit => Self::Application,
            Kind::FocusInput
            | Kind::Model
            | Kind::Stop
            | Kind::Find
            | Kind::CopyLastAnswer
            | Kind::Compact
            | Kind::Reconnect
            | Kind::Rename
            | Kind::Clone
            | Kind::Close
            | Kind::SessionInfo => Self::Conversation,
            Kind::History
            | Kind::OpenHistory
            | Kind::ProjectFiles
            | Kind::CloseSource
            | Kind::FocusSource
            | Kind::ShowConversation => Self::Workspace,
            Kind::Export | Kind::Reveal | Kind::CopyPath | Kind::Delete => Self::Files,
            Kind::TemporaryActions
            | Kind::PasteAnswer
            | Kind::CopyTemporaryAnswer
            | Kind::RevealWorkspace
            | Kind::HideTemporary
            | Kind::TrashTemporary
            | Kind::TemporarySession(_) => Self::Temporary,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Application => "settings-key-group-app",
            Self::Conversation => "settings-key-group-conversation",
            Self::Workspace => "settings-key-group-workspace",
            Self::Files => "settings-key-group-files",
            Self::Temporary => "temporary-title",
        }
    }
}

/// Where a command's binding is active, following the key contexts in
/// `gupi_settings::keybindings`.
pub(super) fn scope_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Settings | Kind::ShowMain | Kind::Quit => "settings-key-scope-any-window",
        Kind::Palette => "settings-key-scope-palette",
        Kind::Find => "settings-key-scope-messages",
        Kind::Stop => "settings-key-scope-stop",
        Kind::PasteAnswer => "settings-key-scope-temporary-paste",
        kind if kind.temporary_only() => "settings-key-scope-temporary",
        _ => "settings-key-scope-conversation",
    }
}

/// Keycaps for a stored binding; `None` when nothing is bound or it does not parse.
pub(super) fn keycaps(value: &str) -> Option<AnyElement> {
    let strokes = value
        .split_whitespace()
        .map(Keystroke::parse)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (!strokes.is_empty()).then(|| {
        h_flex()
            .gap_1()
            .children(strokes.into_iter().map(Kbd::new))
            .into_any_element()
    })
}

/// The focusable key area that shows the binding and receives recording.
pub(super) fn key_area(
    id: impl Into<ElementId>,
    focus: &FocusHandle,
    value: &str,
    recording: bool,
    cx: &App,
) -> Stateful<Div> {
    let focused_border = cx.theme().ring;
    h_flex()
        .id(id)
        .track_focus(focus)
        .h_7()
        .min_w(px(132.))
        .px_2()
        .gap_1()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(if recording {
            focused_border
        } else {
            cx.theme().input
        })
        .bg(cx.theme().background)
        .focus(move |style| style.border_color(focused_border))
        .child(if recording {
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(t(cx, "settings-key-recording"))
                .into_any_element()
        } else {
            keycaps(value).unwrap_or_else(|| {
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if value.is_empty() {
                        t(cx, "settings-key-unbound")
                    } else {
                        value.to_owned()
                    })
                    .into_any_element()
            })
        })
}

struct KeyError {
    message: String,
    /// Another application command that already uses the recorded key.
    conflict: Option<usize>,
}

pub(super) struct KeysView {
    controller: Entity<ConfigController>,
    /// The shown value of each command: the saved binding, or the draft while editing.
    drafts: Vec<SharedString>,
    focus: Vec<FocusHandle>,
    editing: Option<usize>,
    error: Option<KeyError>,
    capture: Option<Subscription>,
    search: Entity<InputState>,
    customized_only: bool,
    key_query: Option<Keystroke>,
    key_capture: Option<Subscription>,
    sessions_open: bool,
    _subscriptions: Vec<Subscription>,
}
impl KeysView {
    pub fn actions(owner: &Entity<Self>, global: &Entity<global_keys::GlobalKeys>) -> SettingGroup {
        let global = global.clone();
        let owner = owner.clone();
        SettingGroup::new().border_0().p_0().item(
            SettingItem::render(move |_, _, cx| Self::render_actions(&owner, &global, cx))
                .keywords(["快捷键 keys keyboard shortcut 恢复默认 reset defaults global template task 全局快捷任务 模板 系统级 system-wide"]),
        )
    }

    fn render_actions(
        owner: &Entity<Self>,
        global: &Entity<global_keys::GlobalKeys>,
        cx: &mut App,
    ) -> AnyElement {
        let this = owner.read(cx);
        let preferences = this.controller.read(cx).preferences(cx);
        let busy = this.controller.read(cx).busy(cx);
        let disabled =
            busy || preferences.keybindings.is_empty() && !preferences.shortcuts.has_bindings();
        let customized_only = this.customized_only;
        let key_query = this.key_query.clone();
        let key_capturing = this.key_capture.is_some();
        let search = this.search.clone();
        let weak = owner.downgrade();
        let toggle = owner.downgrade();
        let capture = owner.downgrade();
        let actions = h_flex()
            .w_full()
            .flex_wrap()
            .gap_2()
            .child(
                div().flex_1().min_w(px(200.)).child(
                    Input::new(&search)
                        .prefix(gpui_kit::component::Icon::new(IconName::Search))
                        .when_some(key_query.as_ref(), |input, key| {
                            input.suffix(Kbd::new(key.clone()))
                        }),
                ),
            )
            .child(
                Button::new("key-search-keystroke")
                    .small()
                    .icon(IconName::Keyboard)
                    .selected(key_capturing || key_query.is_some())
                    .label(t(
                        cx,
                        if key_capturing {
                            "settings-key-recording"
                        } else if key_query.is_some() {
                            "settings-key-search-clear"
                        } else {
                            "settings-key-search-keystroke"
                        },
                    ))
                    .on_click(move |_, window, cx| {
                        capture
                            .update(cx, |this, cx| this.toggle_key_search(window, cx))
                            .ok();
                    }),
            )
            .child(
                gpui_kit::component::checkbox::Checkbox::new("key-customized-only")
                    .label(t(cx, "settings-key-customized-only"))
                    .checked(customized_only)
                    .on_click(move |checked, _, cx| {
                        toggle
                            .update(cx, |this, cx| {
                                this.customized_only = *checked;
                                cx.notify();
                            })
                            .ok();
                    }),
            )
            .child(div().flex_1())
            .child(global_keys::GlobalKeys::add_button(global, cx))
            .child(
                Button::new("key-reset-all")
                    .small()
                    .label(t(cx, "settings-key-reset-all-ellipsis"))
                    .disabled(disabled)
                    .debug_selector(|| "key-reset-all".into())
                    .on_click(move |_, window, cx| {
                        let owner = weak.clone();
                        window.open_dialog(cx, move |dialog, _, cx| {
                            let owner = owner.clone();
                            dialog
                                .title(t(cx, "settings-key-reset-all"))
                                .child(t(cx, "settings-key-reset-all-confirm"))
                                .footer(dialog_buttons("settings-key-reset-all", false, false, cx))
                                .on_ok(move |_, window, cx| {
                                    owner
                                        .update(cx, |this, cx| this.reset_all(window, cx))
                                        .unwrap_or(false)
                                })
                        });
                    }),
            );
        v_flex()
            .gap_2()
            .child(actions)
            .children(
                cx.try_global::<crate::app::shortcuts::ShortcutsRuntime>()
                    .and_then(|rt| rt.error.clone())
                    .map(|error| div().text_color(cx.theme().danger).child(error)),
            )
            .into_any_element()
    }

    fn reset_all(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.controller.read(cx).busy(cx) {
            return false;
        }
        self.cancel(window, cx);
        self.controller.update(cx, |controller, cx| {
            controller.set_preference(PreferenceChange::ResetKeybindings, cx);
        });
        self.sync(window, cx);
        true
    }

    /// Page-level search, keystroke search and the customized filter.
    pub fn matches(&self, index: usize, cx: &App) -> bool {
        if self.editing == Some(index) {
            return true;
        }
        let command = &COMMANDS[index];
        let config = self.controller.read(cx).preferences(cx);
        if self.customized_only && !config.keybindings.contains_key(command.id) {
            return false;
        }
        let value = command.value(&config.keybindings);
        if let Some(key) = &self.key_query {
            let first = value
                .split_whitespace()
                .next()
                .and_then(|text| Keystroke::parse(text).ok());
            if first.as_ref() != Some(key) {
                return false;
            }
        }
        let query = self.search.read(cx).value().trim().to_lowercase();
        query.is_empty()
            || [
                t(cx, command.label),
                t(cx, scope_key(command.kind)),
                command.id.to_owned(),
                command.kind.search_terms().to_owned(),
            ]
            .iter()
            .any(|text| text.to_lowercase().contains(&query))
    }

    pub fn filtering(&self, cx: &App) -> bool {
        self.customized_only
            || self.key_query.is_some()
            || !self.search.read(cx).value().trim().is_empty()
    }

    pub fn customized_only(&self) -> bool {
        self.customized_only
    }

    pub fn sessions_open(&self) -> bool {
        self.sessions_open
    }

    fn toggle_key_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.key_capture.take().is_some() || self.key_query.take().is_some() {
            cx.notify();
            return;
        }
        self.cancel(window, cx);
        let listener = cx.listener(|this, event: &KeystrokeEvent, _, cx| {
            cx.stop_propagation();
            if event.keystroke.key != "escape" {
                this.key_query = Some(event.keystroke.clone());
            }
            this.key_capture = None;
            cx.notify();
        });
        self.key_capture = Some(cx.intercept_keystrokes(listener));
        cx.notify();
    }

    /// One row per command; the scope is the row description.
    pub fn item(owner: &Entity<Self>, index: usize, cx: &App) -> SettingItem {
        let command = &COMMANDS[index];
        let owner = owner.downgrade();
        SettingItem::new(
            t(cx, command.label),
            SettingField::render(move |options, _, cx| {
                owner
                    .update(cx, |this, cx| this.render_binding(index, options, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            }),
        )
        .description(t(cx, scope_key(command.kind)))
        .keywords([command.id, "快捷键 keys keyboard shortcut"])
    }

    /// Disclosure row for the nine temporary conversation shortcuts.
    pub fn sessions_item(owner: &Entity<Self>, cx: &App) -> SettingItem {
        let owner = owner.downgrade();
        SettingItem::new(
            t(cx, "settings-key-sessions"),
            SettingField::render(move |_, _, cx| {
                owner
                    .update(cx, |this, cx| this.render_sessions(cx))
                    .unwrap_or_else(|_| div().into_any_element())
            }),
        )
        .description(t(cx, "settings-key-scope-temporary"))
        .keywords(["temporary session switch 临时会话 切换 1 9"])
    }

    fn render_sessions(&self, cx: &mut Context<Self>) -> AnyElement {
        let config = self.controller.read(cx).preferences(cx);
        let sessions: Vec<_> = COMMANDS
            .iter()
            .filter(|command| matches!(command.kind, Kind::TemporarySession(_)))
            .collect();
        let first = sessions
            .first()
            .and_then(|c| keycaps(c.value(&config.keybindings)));
        let last = sessions
            .last()
            .and_then(|c| keycaps(c.value(&config.keybindings)));
        let open = self.sessions_open;
        h_flex()
            .gap_2()
            .items_center()
            .children(first)
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("…"),
            )
            .children(last)
            .child(
                Button::new("key-sessions-toggle")
                    .ghost()
                    .small()
                    .icon(if open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .label(t(
                        cx,
                        if open {
                            "settings-key-sessions-collapse"
                        } else {
                            "settings-key-sessions-expand"
                        },
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sessions_open = !this.sessions_open;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    pub fn new(
        controller: Entity<ConfigController>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let config = controller.read(cx).preferences(cx);
        let drafts = COMMANDS
            .iter()
            .map(|command| SharedString::from(command.value(&config.keybindings).to_owned()))
            .collect();
        let focus: Vec<_> = COMMANDS
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "settings-key-search")));
        let mut subscriptions = vec![cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify())];
        for (index, focus) in focus.iter().enumerate() {
            subscriptions.push(cx.on_focus_in(focus, window, move |this, window, cx| {
                this.activate(index, window, cx);
            }));
            subscriptions.push(cx.on_focus_out(focus, window, move |this, _, _, cx| {
                if this.editing == Some(index) {
                    this.capture = None;
                    cx.notify();
                }
            }));
        }
        let store = controller.read(cx).configuration();
        subscriptions.push(store.observe_in(cx, window, |this, _, window, cx| {
            this.sync(window, cx);
        }));
        subscriptions.push(cx.observe_global_in::<gupi_settings::i18n::I18n>(
            window,
            |this, window, cx| {
                let placeholder = t(cx, "settings-key-search");
                this.search.update(cx, |input, cx| {
                    input.set_placeholder(placeholder, window, cx)
                });
                cx.notify();
            },
        ));
        Self {
            controller,
            drafts,
            focus,
            editing: None,
            error: None,
            capture: None,
            search,
            customized_only: false,
            key_query: None,
            key_capture: None,
            sessions_open: false,
            _subscriptions: subscriptions,
        }
    }

    fn saved(&self, index: usize, cx: &App) -> String {
        let config = self.controller.read(cx).preferences(cx);
        COMMANDS[index].value(&config.keybindings).to_owned()
    }

    fn set_draft(&mut self, index: usize, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.drafts[index] = value.into();
        if self.editing == Some(index) {
            self.error = None;
        }
        cx.notify();
    }

    fn sync(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        for index in 0..COMMANDS.len() {
            if self.editing != Some(index) {
                let value = self.saved(index, cx);
                if self.drafts[index].as_ref() != value {
                    self.drafts[index] = value.into();
                }
            }
        }
        cx.notify();
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing != Some(index) {
            self.cancel(window, cx);
            self.editing = Some(index);
        }
    }

    fn cancel(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.capture = None;
        self.error = None;
        if let Some(index) = self.editing.take() {
            self.drafts[index] = self.saved(index, cx).into();
        }
        cx.notify();
    }

    /// Clears only the draft; saving it unbinds the command.
    fn clear(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        self.activate(index, window, cx);
        self.capture = None;
        self.set_draft(index, "", cx);
    }

    fn save(
        &mut self,
        index: usize,
        value: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.controller.read(cx).busy(cx) || self.capture.is_some() {
            return false;
        }
        self.activate(index, window, cx);
        let command = &COMMANDS[index];
        let config = self.controller.read(cx).preferences(cx);
        let text = value.as_deref().unwrap_or(command.default);
        match keybindings::validate(command.id, text, &config.keybindings, cx) {
            Ok(()) => {
                self.editing = None;
                self.error = None;
                self.controller.update(cx, |owner, cx| {
                    owner.set_preference(PreferenceChange::Keybinding(command.id.into(), value), cx)
                });
                self.sync(window, cx);
                true
            }
            Err(error) => {
                let conflict = Keystroke::parse(text).ok().and_then(|key| {
                    let other =
                        keybindings::conflicting_command(command.id, &key, &config.keybindings)?;
                    COMMANDS.iter().position(|c| c.id == other.id)
                });
                let message = if let Some(other) = conflict {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("command", t(cx, COMMANDS[other].label));
                    args.set("scope", t(cx, scope_key(COMMANDS[other].kind)));
                    gupi_settings::i18n::t_with_args(cx, "settings-key-used-by", &args)
                } else {
                    format!("{}: {}", t(cx, "settings-key-conflict"), t(cx, &error))
                };
                self.error = Some(KeyError { message, conflict });
                cx.notify();
                false
            }
        }
    }

    fn confirm(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.capture.is_some() || self.controller.read(cx).busy(cx) {
            return false;
        }
        let value = self.drafts[index].trim().to_owned();
        if value == self.saved(index, cx) {
            self.cancel(window, cx);
            return true;
        }
        self.save(index, Some(value), window, cx)
    }

    /// Leaves the current draft and shows the command that holds the key.
    fn show_conflict(&mut self, other: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel(window, cx);
        self.customized_only = false;
        self.key_query = None;
        let label = t(cx, COMMANDS[other].label);
        self.search
            .update(cx, |input, cx| input.set_value(label, window, cx));
        if matches!(COMMANDS[other].kind, Kind::TemporarySession(_)) {
            self.sessions_open = true;
        }
        self.focus[other].focus(window, cx);
        cx.notify();
    }

    fn start_recording(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.read(cx).busy(cx) {
            return;
        }
        self.activate(index, window, cx);
        self.focus[index].focus(window, cx);
        self.error = None;
        let listener = cx.listener(move |this, event: &KeystrokeEvent, window, cx| {
            if !this.focus[index].is_focused(window) || this.controller.read(cx).busy(cx) {
                this.capture = None;
                cx.notify();
                return;
            }
            cx.stop_propagation();
            // Escape ends recording without becoming the binding.
            if event.keystroke.key != "escape" {
                this.set_draft(index, event.keystroke.unparse(), cx);
            }
            this.capture = None;
            cx.notify();
        });
        self.capture = Some(cx.intercept_keystrokes(listener));
        cx.notify();
    }

    fn render_binding(
        &self,
        index: usize,
        options: &RenderOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let command = &COMMANDS[index];
        let config = self.controller.read(cx).preferences(cx);
        let busy = self.controller.read(cx).busy(cx);
        let editing = self.editing == Some(index);
        let recording = editing && self.capture.is_some();
        let value = self.drafts[index].clone();
        let dirty = value.as_ref() != command.value(&config.keybindings);
        let customized = config.keybindings.contains_key(command.id);
        let owner = cx.entity().downgrade();
        let menu = {
            let owner = owner.clone();
            let has_value = !value.is_empty();
            move |menu: gpui_kit::component::menu::PopupMenu,
                  _: &mut Window,
                  cx: &mut Context<gpui_kit::component::menu::PopupMenu>| {
                let record = owner.clone();
                let clear = owner.clone();
                let reset = owner.clone();
                menu.item(
                    PopupMenuItem::new(t(cx, "settings-key-record"))
                        .icon(IconName::Keyboard)
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            record
                                .update(cx, |this, cx| this.start_recording(index, window, cx))
                                .ok();
                        }),
                )
                .item(
                    PopupMenuItem::new(t(cx, "settings-key-clear"))
                        .icon(IconName::Eraser)
                        .disabled(busy || !has_value)
                        .on_click(move |_, window, cx| {
                            clear
                                .update(cx, |this, cx| this.clear(index, window, cx))
                                .ok();
                        }),
                )
                .when(customized, |menu| {
                    menu.separator().item(
                        PopupMenuItem::new(t(cx, "settings-key-reset"))
                            .icon(IconName::Undo2)
                            .disabled(busy || recording)
                            .on_click(move |_, window, cx| {
                                reset
                                    .update(cx, |this, cx| {
                                        this.save(index, None, window, cx);
                                    })
                                    .ok();
                            }),
                    )
                })
            }
        };
        let area = key_area(
            ("key-area", index),
            &self.focus[index],
            &value,
            recording,
            cx,
        )
        .debug_selector(move || format!("key-record-{}", command.id))
        .on_click(cx.listener(move |this, _, window, cx| {
            if this.capture.is_none() {
                this.start_recording(index, window, cx);
            }
        }))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if this.capture.is_some() {
                return;
            }
            match event.keystroke.key.as_str() {
                "enter" => {
                    if this.editing == Some(index) {
                        this.confirm(index, window, cx);
                    } else {
                        this.start_recording(index, window, cx);
                    }
                }
                "space" => this.start_recording(index, window, cx),
                "escape" if this.editing == Some(index) => this.cancel(window, cx),
                _ => return,
            }
            cx.stop_propagation();
        }));
        let row = h_flex()
            .gap_2()
            .items_center()
            .justify_end()
            .when(customized && !dirty, |row| {
                row.child(
                    Tag::secondary()
                        .small()
                        .outline()
                        .child(t(cx, "settings-key-customized")),
                )
            })
            .when(editing && dirty && !recording, |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "settings-key-unsaved")),
                )
            })
            .child(area)
            .when(editing && (dirty || recording), |row| {
                row.child(
                    Button::new(("key-confirm", index))
                        .small()
                        .label(t(cx, "settings-key-save"))
                        .disabled(busy || recording)
                        .debug_selector(move || format!("key-confirm-{}", command.id))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.confirm(index, window, cx);
                        })),
                )
                .child(
                    Button::new(("key-cancel", index))
                        .small()
                        .ghost()
                        .label(t(cx, "action-cancel"))
                        .disabled(busy)
                        .debug_selector(move || format!("key-cancel-{}", command.id))
                        .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                )
            })
            .child(
                Button::new(("key-actions", index))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .tooltip(t(cx, "settings-key-actions"))
                    .accessibility_label(t(cx, "settings-key-actions"))
                    .debug_selector(move || format!("key-actions-{}", command.id))
                    .dropdown_menu_with_anchor(Anchor::TopRight, menu),
            );
        v_flex()
            .id(("key-field", index))
            .debug_selector(move || format!("key-binding-{}", command.id))
            .gap_1()
            .items_end()
            .map(|field| {
                if options.layout() == Axis::Horizontal {
                    field.max_w(px(460.))
                } else {
                    field.w_full()
                }
            })
            .child(row)
            .when(editing, |field| {
                field.children(self.error.as_ref().map(|error| {
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(error.message.clone()),
                        )
                        .children(error.conflict.map(|other| {
                            Button::new(("key-show-conflict", index))
                                .xsmall()
                                .outline()
                                .label(t(cx, "settings-key-show-conflict"))
                                .debug_selector(move || format!("key-show-conflict-{}", command.id))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.show_conflict(other, window, cx)
                                }))
                        }))
                }))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use super::AppLanguage;
    use super::COMMANDS;
    use super::ConfigController;
    use super::KeysView;
    use crate::features::settings::global_keys::GlobalKeys;
    use gpui_form::Form;
    use gpui_kit::AppContext;
    use gpui_kit::Context;
    use gpui_kit::Entity;
    use gpui_kit::InteractiveElement;
    use gpui_kit::IntoElement;
    use gpui_kit::KeyBinding;
    use gpui_kit::Modifiers;
    use gpui_kit::ParentElement;
    use gpui_kit::Render;
    use gpui_kit::Styled;

    use gpui_kit::TestAppContext;
    use gpui_kit::VisualTestContext;
    use gpui_kit::Window;
    use gpui_kit::component::Root;
    use gpui_kit::component::WindowExt;
    use gpui_kit::component::group_box::GroupBoxVariant;
    use gpui_kit::component::setting::SettingGroup;
    use gpui_kit::component::setting::SettingPage;
    use gpui_kit::component::setting::Settings;
    use gpui_kit::div;
    use gpui_kit::point;
    use gpui_kit::px;

    use gupi_settings::config::ConfigContents;
    use gupi_settings::config::ConfigData;
    use std::cell::Cell;
    use std::rc::Rc;

    gpui_kit::actions!(keys_test, [UnrelatedAction]);

    fn command_index(id: &str) -> usize {
        COMMANDS
            .iter()
            .position(|command| command.id == id)
            .unwrap()
    }

    struct Fixture {
        global: Entity<GlobalKeys>,
        keys: Entity<KeysView>,
        _form: Entity<Form<AppConfig>>,
        dispatched: Rc<Cell<bool>>,
    }
    impl Render for Fixture {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let keys = self.keys.clone();
            div()
                .id("keys-fixture-root")
                .size_full()
                .on_action(cx.listener(|this, _: &UnrelatedAction, _, _| this.dispatched.set(true)))
                .child(
                    Settings::new("keys-fixture")
                        .with_group_variant(GroupBoxVariant::Normal)
                        .page(
                            SettingPage::new("快捷键")
                                .group(KeysView::actions(&keys, &self.global))
                                .group(
                                    SettingGroup::new()
                                        .title("应用")
                                        .item(KeysView::item(
                                            &self.keys,
                                            command_index("palette"),
                                            cx,
                                        ))
                                        .item(KeysView::item(
                                            &self.keys,
                                            command_index("quick_open"),
                                            cx,
                                        )),
                                ),
                        ),
                )
        }
    }
    fn setup(
        cx: &mut TestAppContext,
    ) -> (Entity<KeysView>, Rc<Cell<bool>>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::app::init_capability_hosts(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(AppLanguage::Chinese, cx);
            cx.bind_keys([KeyBinding::new("ctrl-alt-9", UnrelatedAction, None)]);
        });
        let mut keys = None;
        let dispatched = Rc::new(Cell::new(false));
        let flag = dispatched.clone();
        let (_, cx) = cx.add_window_view(|window, cx| {
            let form = cx.new(|_| Form::new(AppConfig::default()));
            let controller = cx.new(|cx| ConfigController::new(&form, cx));
            // Use the onboarding form as an in-memory preference store. No user files are written.
            controller.update(cx, |owner, cx| {
                owner.settle_for_test(
                    {
                        let mut record = ConfigData::new(
                            "/tmp/keys-fixture/config.toml".into(),
                            ConfigContents::Missing,
                        );
                        record.backup = None;
                        record
                    },
                    cx,
                )
            });
            let global = cx.new(|_| GlobalKeys::new(controller.clone()));
            let owner = cx.new(|cx| KeysView::new(controller, window, cx));
            keys = Some(owner.clone());
            let fixture = cx.new(|_| Fixture {
                global,
                keys: owner,
                _form: form,
                dispatched: flag,
            });
            Root::new(fixture, window, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1000.), px(640.)));
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (keys.unwrap(), dispatched, cx)
    }
    fn click(cx: &mut VisualTestContext, selector: &'static str) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {selector}"));
        cx.simulate_click(bounds.center(), Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn draft(cx: &mut VisualTestContext, keys: &Entity<KeysView>, index: usize) -> String {
        cx.update(|_, cx| keys.read(cx).drafts[index].to_string())
    }

    fn record(cx: &mut VisualTestContext, keys: &Entity<KeysView>, index: usize, value: &str) {
        cx.update(|window, cx| {
            keys.update(cx, |keys, cx| keys.start_recording(index, window, cx));
        });
        cx.simulate_keystrokes(value);
    }

    #[gpui_kit::test]
    fn reset_all_requires_confirmation_and_restores_every_input(cx: &mut TestAppContext) {
        let (keys, _, cx) = setup(cx);
        let button = cx.debug_bounds("key-reset-all").unwrap();
        assert!(
            button.left() > px(800.) && button.right() <= px(1000.),
            "{button:?}"
        );
        assert!(button.bottom() < px(120.));
        click(cx, "key-reset-all");
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
        cx.update(|window, cx| {
            keys.update(cx, |this, cx| {
                assert!(this.save(
                    command_index("palette"),
                    Some("ctrl-alt-7".into()),
                    window,
                    cx
                ));
                assert!(this.save(command_index("quick_open"), Some(String::new()), window, cx));
            });
        });
        record(cx, &keys, command_index("palette"), "ctrl-alt-8");
        click(cx, "key-reset-all");
        cx.update(|window, cx| {
            assert!(window.has_active_dialog(cx));
            assert_eq!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .len(),
                2
            );
        });
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            assert_eq!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .len(),
                2
            );
        });
        assert_eq!(draft(cx, &keys, command_index("palette")), "ctrl-alt-8");
        click(cx, "key-reset-all");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            let keys = keys.read(cx);
            assert!(
                keys.controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
            for (draft, command) in keys.drafts.iter().zip(super::COMMANDS) {
                assert_eq!(draft.as_ref(), command.default);
            }
        });
        click(cx, "key-reset-all");
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn clearing_only_changes_the_draft_until_saved(cx: &mut TestAppContext) {
        let (keys, _, cx) = setup(cx);
        let palette = command_index("palette");
        cx.update(|window, cx| {
            let focus = keys.read(cx).focus[palette].clone();
            focus.focus(window, cx)
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| {
            assert!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
        });
        cx.update(|window, cx| keys.update(cx, |this, cx| this.clear(palette, window, cx)));
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            assert!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
        });
        assert!(draft(cx, &keys, palette).is_empty());
        click(cx, "key-cancel-palette");
        assert_eq!(draft(cx, &keys, palette), "secondary-shift-p");
        cx.update(|window, cx| keys.update(cx, |this, cx| this.clear(palette, window, cx)));
        click(cx, "key-confirm-palette");
        cx.update(|_, cx| {
            assert_eq!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .get("palette")
                    .map(String::as_str),
                Some("")
            );
        });
        // Restore default saves immediately, as before.
        cx.update(|window, cx| {
            keys.update(cx, |this, cx| assert!(this.save(palette, None, window, cx)))
        });
        cx.update(|_, cx| {
            assert!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
        });
        assert_eq!(draft(cx, &keys, palette), "secondary-shift-p");
        record(cx, &keys, palette, "secondary-p");
        click(cx, "key-confirm-palette");
        cx.update(|_, cx| {
            let keys = keys.read(cx);
            let error = keys
                .error
                .as_ref()
                .expect("conflicting shortcut stays in the item");
            assert_eq!(error.conflict, Some(command_index("quick_open")));
            assert!(
                keys.controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
        });
        click(cx, "key-show-conflict-palette");
        cx.update(|window, cx| {
            let keys = keys.read(cx);
            assert_eq!(keys.editing, Some(command_index("quick_open")));
            assert!(keys.focus[command_index("quick_open")].is_focused(window));
            assert_eq!(keys.drafts[palette].as_ref(), "secondary-shift-p");
        });
        record(cx, &keys, palette, "ctrl-alt-7");
        click(cx, "key-confirm-palette");
        cx.update(|_, cx| {
            assert_eq!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .get("palette")
                    .map(String::as_str),
                Some("ctrl-alt-7")
            );
        });
    }

    #[gpui_kit::test]
    fn shortcut_field_rejects_typing_pasting_and_a_second_stroke(cx: &mut TestAppContext) {
        let (keys, _, cx) = setup(cx);
        let index = command_index("palette");
        cx.update(|window, cx| {
            let focus = keys.read(cx).focus[index].clone();
            focus.focus(window, cx)
        });
        cx.simulate_input("ctrl-alt-x y");
        cx.update(|_, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("ctrl-alt-x y".into()))
        });
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-a cmd-v backspace"
        } else {
            "ctrl-a ctrl-v backspace"
        });
        assert_eq!(draft(cx, &keys, index), "secondary-shift-p");
        record(cx, &keys, index, "ctrl-alt-7");
        cx.simulate_keystrokes("y");
        assert_eq!(draft(cx, &keys, index), "ctrl-alt-7");
        click(cx, "key-confirm-palette");
        cx.update(|_, cx| {
            assert_eq!(
                keys.read(cx)
                    .controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings["palette"],
                "ctrl-alt-7"
            )
        });
    }

    #[gpui_kit::test]
    fn inline_recording_intercepts_keys_and_search_filters_each_item(cx: &mut TestAppContext) {
        let (keys, dispatched, cx) = setup(cx);
        click(cx, "key-record-palette");
        cx.simulate_keystrokes("ctrl-alt-9");
        assert!(
            !dispatched.get(),
            "recording must not execute application shortcuts"
        );
        cx.update(|window, cx| {
            assert!(!window.has_active_dialog(cx));
            let keys = keys.read(cx);
            assert!(keys.capture.is_none());
            assert_eq!(keys.drafts[command_index("palette")].as_ref(), "ctrl-alt-9");
            assert!(
                keys.controller
                    .read(cx)
                    .preferences(cx)
                    .keybindings
                    .is_empty()
            );
        });
        click(cx, "key-record-palette");
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| assert!(keys.read(cx).capture.is_none()));
        assert_eq!(draft(cx, &keys, command_index("palette")), "ctrl-alt-9");
        cx.simulate_keystrokes("escape");
        assert_eq!(
            draft(cx, &keys, command_index("palette")),
            "secondary-shift-p"
        );
        click(cx, "key-record-palette");
        // Moving focus to another field must release the recorder before the next keystroke.
        cx.update(|window, cx| {
            let focus = keys.read(cx).focus[command_index("quick_open")].clone();
            focus.focus(window, cx)
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| {
            let keys = keys.read(cx);
            assert!(
                keys.capture.is_none(),
                "editing={:?}, first={}, second={}",
                keys.editing,
                keys.focus[command_index("palette")].is_focused(window),
                keys.focus[command_index("quick_open")].is_focused(window)
            );
        });
        cx.simulate_click(point(px(100.), px(24.)), Modifiers::default());
        cx.simulate_input("palette");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("key-binding-palette").is_some());
        assert!(cx.debug_bounds("key-binding-quick_open").is_none());
    }

    #[gpui_kit::test]
    fn page_filters_customized_and_keystroke_matches(cx: &mut TestAppContext) {
        let (keys, _, cx) = setup(cx);
        let palette = command_index("palette");
        let quick_open = command_index("quick_open");
        cx.update(|window, cx| {
            keys.update(cx, |this, cx| {
                assert!(this.save(palette, Some("ctrl-alt-7".into()), window, cx));
                this.customized_only = true;
                assert!(this.matches(palette, cx));
                assert!(!this.matches(quick_open, cx));
                this.customized_only = false;
                this.key_query = gpui_kit::Keystroke::parse("secondary-p").ok();
                assert!(this.matches(quick_open, cx));
                assert!(!this.matches(palette, cx));
                assert!(this.filtering(cx));
            })
        });
    }
}
