use super::*;
use gpui_kit::component::button::{Toggle, ToggleGroup};
use gpui_kit::component::input::{self, Editor, EditorState};
use gpui_kit::component::toolbar::{Toolbar, ToolbarGroup};
use gpui_kit::component::tooltip::Tooltip;
use gupi_conversation::conversation::{BodyState, LoadStage};
use std::cell::Cell;
use std::ops::Range;

actions!(gupi, [Bang, EmptyShellBackspace, EscapeShell]);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("!", Bang, Some("GupiComposer > Input && !GupiQuoteComment")),
        KeyBinding::new("!", Bang, Some("GupiShell > Input")),
        KeyBinding::new("backspace", EmptyShellBackspace, Some("GupiShell > Input")),
        KeyBinding::new("escape", EscapeShell, Some("GupiShell")),
    ]);
}

#[derive(Clone, PartialEq)]
struct Command {
    text: String,
    exclude: bool,
}
struct Browse {
    ix: usize,
    current: Command,
    selection: Range<usize>,
}

pub(super) struct Input {
    pub editor: Entity<EditorState>,
    pub open: bool,
    pub second_bang: bool,
    exclude: bool,
    history: Vec<Command>,
    browse: Option<Browse>,
}
impl Input {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        let placeholder = t(cx, "shell-placeholder");
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx)
                .language("bash")
                .soft_wrap(true)
                .searchable(true)
                .submit_on_enter(true)
                .placeholder(placeholder);
            editor.set_line_number(false, window, cx);
            editor.set_folding(false, window, cx);
            editor.set_indent_guides(false, window, cx);
            editor
        });
        Self {
            editor,
            open: false,
            second_bang: false,
            exclude: false,
            history: vec![],
            browse: None,
        }
    }
}

impl HomeView {
    pub(super) fn shell_open(&self) -> bool {
        self.shown_key
            .as_ref()
            .and_then(|key| self.views.get(key))
            .is_some_and(|view| view.shell.open)
    }
    fn shell_entry_allowed(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.shown_key.as_ref() == self.state.read(cx).selected().as_ref()
            && self.input.focus_handle(cx).is_focused(window)
            && self.input.update(cx, |input, cx| {
                input.presentation().is_editable()
                    && input.selected_range() == (0..0)
                    && input.marked_text_range(window, cx).is_none()
            })
    }
    pub(super) fn shell_bang(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shell_open() {
            let view = self
                .views
                .get_mut(self.shown_key.as_ref().unwrap())
                .unwrap();
            if view.shell.second_bang
                && view.shell.editor.update(cx, |input, cx| {
                    input.value().is_empty()
                        && input.selected_range() == (0..0)
                        && input.marked_text_range(window, cx).is_none()
                })
            {
                view.shell.second_bang = false;
                view.shell.exclude = true;
                cx.notify();
                return;
            }
        } else if self.shell_entry_allowed(window, cx) {
            self.enter_shell("", false, true, window, cx);
            return;
        }
        cx.propagate();
    }
    pub(super) fn paste_shell_entry(
        &mut self,
        item: &ClipboardItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(text) = item.text() else {
            return false;
        };
        let Some(command) = text.strip_prefix('!') else {
            return false;
        };
        if !self.shell_entry_allowed(window, cx) {
            return false;
        }
        let (command, exclude) = command
            .strip_prefix('!')
            .map_or((command, false), |command| (command, true));
        self.enter_shell(
            command.trim_start_matches(char::is_whitespace),
            exclude,
            false,
            window,
            cx,
        );
        true
    }
    fn enter_shell(
        &mut self,
        command: &str,
        exclude: bool,
        second_bang: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        view.shell.open = true;
        view.shell.exclude = exclude;
        view.shell.browse = None;
        view.shell.editor.update(cx, |input, cx| {
            input.set_value(command, window, cx);
            input.set_selected_range(command.len()..command.len(), cx);
            input.focus(window, cx);
        });
        view.shell.second_bang = second_bang;
        cx.notify();
    }
    fn exit_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        view.shell.open = false;
        view.shell.second_bang = false;
        view.shell.browse = None;
        view.shell
            .editor
            .update(cx, |input, cx| input.set_value("", window, cx));
        view.input.focus_handle(cx).focus(window, cx);
        cx.notify();
    }
    fn can_run_shell_input(&self, key: &str, cx: &App) -> bool {
        self.views.get(key).is_some_and(|view| {
            view.preview.as_ref().is_none_or(|id| {
                self.state.read(cx).sessions()[key]
                    .history()
                    .on_current_path(id)
            })
        }) && self.state.read(cx).can_run_shell(key, cx)
    }
    pub(super) fn run_shell_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self
            .shown_key
            .clone()
            .filter(|key| self.state.read(cx).selected().as_ref() == Some(key))
        else {
            return;
        };
        if !self.can_run_shell_input(&key, cx) {
            return;
        }
        let Some(view) = self.views.get(&key).filter(|view| view.shell.open) else {
            return;
        };
        if view.shell.editor.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        }) {
            return;
        }
        let command = Command {
            text: view.shell.editor.read(cx).value().to_string(),
            exclude: view.shell.exclude,
        };
        let accepted = self.state.update(cx, |state, cx| {
            state.run_shell(&key, command.text.clone(), command.exclude, cx)
        });
        if !accepted {
            return;
        }
        let view = self.views.get_mut(&key).unwrap();
        if view.shell.history.last() != Some(&command) {
            view.shell.history.push(command);
            if view.shell.history.len() > 100 {
                view.shell.history.remove(0);
            }
        }
        view.shell.second_bang = false;
        view.shell.browse = None;
        view.shell
            .editor
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }
    fn escape_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.shown_key.clone() else {
            return;
        };
        let Some(view) = self.views.get(&key).filter(|view| view.shell.open) else {
            cx.propagate();
            return;
        };
        if view.shell.editor.read(cx).search_session().is_active() {
            view.shell
                .editor
                .update(cx, |input, cx| input.close_search(cx));
        } else if self
            .state
            .read(cx)
            .sessions()
            .get(&key)
            .is_some_and(|session| session.busy())
        {
            self.state.update(cx, |state, cx| state.abort(&key, cx));
        } else if self
            .state
            .read(cx)
            .sessions()
            .get(&key)
            .is_some_and(|session| session.shell_running())
        {
            self.state
                .update(cx, |state, cx| state.stop_shell(&key, cx));
        } else {
            self.exit_shell(window, cx);
        }
    }
    pub(super) fn shell_history_available(&self, next: bool) -> bool {
        self.shown_key
            .as_ref()
            .and_then(|key| self.views.get(key))
            .is_some_and(|view| {
                view.shell.open
                    && if next {
                        view.shell.browse.is_some()
                    } else {
                        !view.shell.history.is_empty()
                            && view
                                .shell
                                .browse
                                .as_ref()
                                .is_none_or(|browse| browse.ix > 0)
                    }
            })
    }
    pub(super) fn recall_shell(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shell_history_available(next) {
            return;
        }
        let shell = &mut self
            .views
            .get_mut(self.shown_key.as_ref().unwrap())
            .unwrap()
            .shell;
        let (command, selection) =
            if next && shell.browse.as_ref().unwrap().ix + 1 == shell.history.len() {
                let browse = shell.browse.take().unwrap();
                (browse.current, browse.selection)
            } else {
                if let Some(browse) = &mut shell.browse {
                    if next {
                        browse.ix += 1;
                    } else {
                        browse.ix -= 1;
                    }
                } else {
                    shell.browse = Some(Browse {
                        ix: shell.history.len() - 1,
                        current: Command {
                            text: shell.editor.read(cx).value().to_string(),
                            exclude: shell.exclude,
                        },
                        selection: shell.editor.read(cx).selected_range(),
                    });
                }
                let command = shell.history[shell.browse.as_ref().unwrap().ix].clone();
                let end = command.text.len();
                (command, end..end)
            };
        shell.second_bang = false;
        shell.exclude = command.exclude;
        shell.editor.update(cx, |input, cx| {
            input.set_value(command.text, window, cx);
            input.set_selected_range(selection, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }
    pub(super) fn render_shell_composer(&self, cx: &Context<Self>) -> AnyElement {
        let key = self.shown_key.as_ref().unwrap();
        let shell = &self.views[key].shell;
        let session = &self.state.read(cx).sessions()[key];
        let owner = cx.weak_entity();
        let composition = Rc::new(Cell::new(false));
        let capture_composition = composition.clone();
        let path = session.info().cwd.to_string_lossy().into_owned();
        let tooltip_path = path.clone();
        let can_run =
            !shell.editor.read(cx).value().trim().is_empty() && self.can_run_shell_input(key, cx);
        let panel = div()
            .id("shell-composer")
            .test_support()
            .key_context("GupiShell")
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap_2()
            .on_action(cx.listener(|home, _: &Bang, window, cx| home.shell_bang(window, cx)))
            // Kit emits PressEnter for submission and propagates the action.
            // Consume it here so the platform cannot also insert a newline.
            .on_action(|_: &input::Enter, _, _| {})
            .on_action(cx.listener(|home, _: &EmptyShellBackspace, window, cx| {
                let input = home.views[home.shown_key.as_ref().unwrap()]
                    .shell
                    .editor
                    .clone();
                if input.update(cx, |input, cx| {
                    input.value().is_empty()
                        && input.selected_range().is_empty()
                        && input.marked_text_range(window, cx).is_none()
                }) {
                    home.exit_shell(window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .on_action(
                cx.listener(|home, _: &EscapeShell, window, cx| home.escape_shell(window, cx)),
            )
            .capture_action(cx.listener(move |home, _: &input::Escape, window, cx| {
                let input = home.views[home.shown_key.as_ref().unwrap()]
                    .shell
                    .editor
                    .clone();
                capture_composition.set(input.update(cx, |input, cx| {
                    input.marked_text_range(window, cx).is_some()
                }));
            }))
            .on_action(cx.listener(move |home, _: &input::Escape, window, cx| {
                if !composition.replace(false) {
                    home.escape_shell(window, cx);
                }
            }))
            .child(
                div()
                    .id("shell-editor-pointer")
                    .capture_any_mouse_down(cx.listener(|home, _, _, _| {
                        if let Some(view) = home
                            .shown_key
                            .as_ref()
                            .and_then(|key| home.views.get_mut(key))
                        {
                            view.shell.second_bang = false;
                        }
                    }))
                    .child(
                        Editor::new(&shell.editor)
                            .h(rems(8.))
                            .aria_label(t(cx, "shell-command"))
                            .on_paste(move |_, _, cx| {
                                let _ = owner.update(cx, |home, _| {
                                    if let Some(view) = home
                                        .shown_key
                                        .as_ref()
                                        .and_then(|key| home.views.get_mut(key))
                                    {
                                        view.shell.second_bang = false;
                                    }
                                });
                                false
                            }),
                    ),
            )
            .child(
                Toolbar::new("shell-toolbar")
                    .w_full()
                    .flex_wrap()
                    .content(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(gpui_kit::component::Icon::new(IconName::SquareTerminal))
                            .child(t(cx, "shell-title"))
                            .child(
                                div()
                                    .id("shell-cwd")
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(path)
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(tooltip_path.clone()).build(window, cx)
                                    }),
                            ),
                    )
                    .child(
                        ToggleGroup::new("shell-context")
                            .segmented()
                            .child(
                                Toggle::new("shell-include")
                                    .label(t(cx, "shell-include"))
                                    .checked(!shell.exclude)
                                    .tooltip(t(cx, "shell-include-help")),
                            )
                            .child(
                                Toggle::new("shell-exclude")
                                    .label(t(cx, "shell-exclude"))
                                    .checked(shell.exclude)
                                    .tooltip(t(cx, "shell-exclude-help")),
                            )
                            .on_click(cx.listener(|home, checks: &Vec<bool>, _, cx| {
                                let shell = &mut home
                                    .views
                                    .get_mut(home.shown_key.as_ref().unwrap())
                                    .unwrap()
                                    .shell;
                                shell.exclude = if shell.exclude && checks[0] {
                                    false
                                } else if !shell.exclude && checks[1] {
                                    true
                                } else {
                                    shell.exclude
                                };
                                cx.notify();
                            })),
                    )
                    .content(
                        ToolbarGroup::new("shell-actions")
                            .child(
                                Button::new("shell-return")
                                    .ghost()
                                    .label(t(cx, "shell-return"))
                                    .on_click(cx.listener(|home, _, window, cx| {
                                        home.exit_shell(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("shell-run")
                                    .primary()
                                    .label(t(cx, "shell-run"))
                                    .disabled(!can_run)
                                    .on_click(cx.listener(|home, _, window, cx| {
                                        home.run_shell_input(window, cx)
                                    })),
                            ),
                    ),
            )
            .when(session.shell_running(), |panel| {
                panel.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "shell-input-running")),
                )
            })
            .when(
                matches!(
                    session.body_state(),
                    BodyState::Loading(LoadStage::Connecting)
                        | BodyState::Refreshing(LoadStage::Connecting)
                ),
                |panel| {
                    panel.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "conversation-connecting")),
                    )
                },
            );
        // Observe caret actions before Kit handles them: a clamped movement
        // may propagate without changing the selection or emitting an event.
        macro_rules! caret_actions {
            ($($action:ty),* $(,)?) => {
                panel$(.capture_action(cx.listener(|home, _: &$action, _, cx| {
                    if let Some(view) = home.shown_key.as_ref().and_then(|key| home.views.get_mut(key)) {
                        view.shell.second_bang = false;
                    }
                    cx.propagate();
                })))*
            };
        }
        caret_actions!(
            input::MoveUp,
            input::MoveDown,
            input::MoveLeft,
            input::MoveRight,
            input::MoveHome,
            input::MoveEnd,
            input::MovePageUp,
            input::MovePageDown,
            input::MoveToStartOfLine,
            input::MoveToEndOfLine,
            input::MoveToStart,
            input::MoveToEnd,
            input::MoveToPreviousWord,
            input::MoveToNextWord,
            input::SelectAll,
            input::SelectToStartOfLine,
            input::SelectToEndOfLine,
            input::SelectToStart,
            input::SelectToEnd,
            input::SelectToPreviousWordStart,
            input::SelectToNextWordEnd,
            gpui_kit::base::actions::SelectLeft,
            gpui_kit::base::actions::SelectRight,
            gpui_kit::base::actions::SelectUp,
            gpui_kit::base::actions::SelectDown,
            gpui_kit::base::input::AddCursorAbove,
            gpui_kit::base::input::AddCursorBelow,
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests;
