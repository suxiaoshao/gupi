use super::files::directory;
use super::*;
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Selectable};

pub(super) struct Preview {
    focus: FocusHandle,
    keyboard_open: bool,
    pub(super) path: PathBuf,
    root: Option<PathBuf>,
    editor: Entity<EditorState>,
    error: Option<&'static str>,
    loading: bool,
    loaded: bool,
    long_line: bool,
    wrap: bool,
    task: Option<Task<()>>,
}

impl Preview {
    pub(super) fn new(
        path: PathBuf,
        root: Option<PathBuf>,
        keyboard_open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language("text")
                .line_number(true)
                .folding(false)
                .soft_wrap(false);
            state.set_readonly(true, cx);
            state
        });
        let mut this = Self {
            focus: cx.focus_handle(),
            keyboard_open,
            path,
            root,
            editor,
            error: None,
            loading: false,
            loaded: false,
            long_line: false,
            wrap: false,
            task: None,
        };
        if keyboard_open {
            this.focus.focus(window, cx);
        }
        this.read(window, cx);
        this
    }

    fn read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        let path = self.path.clone();
        let task = cx.background_spawn(async move { directory::source(&path) });
        self.task = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = task.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.loading = false;
                match result {
                    Ok(source) => {
                        this.long_line = source.long_line;
                        this.editor.update(cx, |editor, cx| {
                            editor.set_highlighter(
                                if source.long_line {
                                    "text"
                                } else {
                                    directory::language(&this.path)
                                },
                                cx,
                            );
                            editor.set_value(source.text, window, cx);
                            if this.keyboard_open && this.focus.is_focused(window) {
                                editor.focus(window, cx);
                            }
                        });
                        this.loaded = true;
                    }
                    Err(error) => this.error = Some(error),
                }
                this.keyboard_open = false;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

pub(super) enum Event {
    Close,
    Return,
    Reveal,
}
impl EventEmitter<Event> for Preview {}
impl Preview {
    fn return_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            cx.propagate();
            return;
        }
        if self.editor.read(cx).search_session().open {
            self.editor.update(cx, |editor, cx| {
                editor.close_search(cx);
                editor.focus(window, cx);
            });
        } else {
            cx.emit(Event::Return);
        }
        cx.stop_propagation();
    }
    pub(super) fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loaded {
            self.editor.update(cx, |e, cx| e.focus(window, cx));
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(super) fn find(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.open_search(false, cx);
        });
    }
    pub(super) fn focus_container(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }
    pub(super) fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }
}
impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let path = self.path.to_string_lossy().into_owned();
        v_flex()
            .id("source-pane")
            .relative()
            .debug_selector(|| "source-pane".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .track_focus(&self.focus)
            .key_context("SourcePreview")
            .on_action(
                cx.listener(|this, _: &actions::ReturnFromSource, window, cx| {
                    this.return_focus(window, cx)
                }),
            )
            .capture_action(cx.listener(
                |this, _: &gpui_kit::component::input::Escape, window, cx| {
                    this.return_focus(window, cx);
                },
            ))
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.focus.is_focused(window) && event.keystroke.key == "enter" {
                    this.focus(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                h_flex()
                    .h_10()
                    .flex_none()
                    .px_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .id("preview-path")
                            .flex_none()
                            .max_w(rems(14.))
                            .min_w_0()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(
                                self.path
                                    .file_name()
                                    .unwrap_or(self.path.as_os_str())
                                    .to_string_lossy()
                                    .into_owned(),
                            )
                            .tooltip(move |window, cx| {
                                Tooltip::new(path.clone()).build(window, cx)
                            }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                self.path
                                    .parent()
                                    .map(|p| {
                                        self.root
                                            .as_ref()
                                            .and_then(|r| p.strip_prefix(r).ok())
                                            .unwrap_or(p)
                                            .to_string_lossy()
                                            .into_owned()
                                    })
                                    .unwrap_or_default(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_none()
                            .text_color(cx.theme().muted_foreground)
                            .child(Icon::new(IconName::Lock).size_3())
                            .child(div().text_xs().child(t(cx, "files-readonly"))),
                    )
                    .child(
                        Button::new("preview-wrap")
                            .small()
                            .ghost()
                            .selected(self.wrap)
                            .icon(IconName::TextWrap)
                            .tooltip(t(cx, "files-wrap"))
                            .accessibility_label(t(cx, "files-wrap"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.wrap = !this.wrap;
                                this.editor.update(cx, |editor, cx| {
                                    editor.set_soft_wrap(this.wrap, window, cx)
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("preview-reload")
                            .small()
                            .ghost()
                            .disabled(self.loading)
                            .icon(IconName::RefreshCw)
                            .tooltip(t(cx, "files-reload"))
                            .accessibility_label(t(cx, "files-reload"))
                            .on_click(cx.listener(|this, _, window, cx| this.read(window, cx))),
                    )
                    .child(
                        Button::new("source-actions")
                            .small()
                            .ghost()
                            .icon(IconName::Ellipsis)
                            .accessibility_label(t(cx, "files-actions"))
                            .dropdown_menu({
                                let owner = cx.weak_entity();
                                let path = self.path.to_string_lossy().into_owned();
                                move |menu, _, cx| {
                                    menu.item(
                                        PopupMenuItem::new(t(cx, "files-copy-path")).on_click({
                                            let path = path.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    path.clone(),
                                                ))
                                            }
                                        }),
                                    )
                                    .item(
                                        PopupMenuItem::new(t(cx, "files-reveal")).on_click({
                                            let owner = owner.clone();
                                            move |_, _, cx| {
                                                let _ = owner
                                                    .update(cx, |_, cx| cx.emit(Event::Reveal));
                                            }
                                        }),
                                    )
                                }
                            }),
                    )
                    .child(
                        Button::new("source-close")
                            .small()
                            .ghost()
                            .icon(IconName::X)
                            .accessibility_label(t(cx, "files-close-source"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::Close))),
                    ),
            )
            .when(self.loading, |view| {
                view.child(div().p_3().text_sm().child(t(cx, "files-loading")))
            })
            .when_some(self.error, |view, error| {
                view.child(
                    div()
                        .p_3()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(t(cx, error)),
                )
            })
            .when(self.long_line, |view| {
                view.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "files-long-line")),
                )
            })
            .when(self.loaded, |view| {
                view.child(
                    div().flex_1().min_h_0().child(
                        Editor::new(&self.editor)
                            .readonly(true)
                            .bordered(false)
                            .aria_label(t(cx, "files-preview"))
                            .h_full(),
                    ),
                )
            })
            .when(self.focus.is_focused(window), |view| {
                view.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .border_2()
                        .border_color(cx.theme().ring),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::Preview;
    use gpui_kit::component::Root;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, ClipboardItem, TestAppContext, px, size};

    #[gpui_kit::test]
    fn source_preview_rejects_edits_and_keeps_copy_available(cx: &mut TestAppContext) {
        const SOURCE: &str = "fn main() { println!(\"hello\"); }";
        cx.update(|cx| {
            gpui_kit::init(cx);
            app_theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
        });
        let mut preview = None;
        let window = cx.open_window(size(px(900.), px(600.)), |window, cx| {
            let view = cx.new(|cx| Preview::new("main.rs".into(), None, false, window, cx));
            view.update(cx, |this, cx| {
                // Install a deterministic source snapshot in the production view.
                this.task = None;
                this.loading = false;
                this.loaded = true;
                this.editor.update(cx, |editor, cx| {
                    editor.set_highlighter("rust", cx);
                    editor.set_value(SOURCE, window, cx);
                });
            });
            preview = Some(view.clone());
            Root::new(view, window, cx)
        });
        let preview = preview.unwrap();
        cx.update_window(window.into(), |_, window, cx| {
            window.render_frame(cx);
            preview.update(cx, |this, cx| {
                this.editor
                    .update(cx, |editor, cx| editor.focus(window, cx))
            });
            window.press("secondary-a", cx);
            window.press("secondary-c", cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(SOURCE)
            );
            cx.write_to_clipboard(ClipboardItem::new_string("overwrite".into()));
            window.press("secondary-v", cx);
            window.press("secondary-x", cx);
            window.input("changed", cx);
            window.press("backspace", cx);
            window.press("enter", cx);
            window.press("secondary-z", cx);
            preview.update(cx, |this, cx| {
                assert_eq!(this.editor.read(cx).value().as_str(), SOURCE);
            });
            preview.update(cx, |this, cx| this.find(window, cx));
            window.render_frame(cx);
            preview.update(cx, |this, cx| {
                assert!(this.editor.read(cx).search_session().open)
            });
            window.press("escape", cx);
            preview.update(cx, |this, cx| {
                assert!(!this.editor.read(cx).search_session().open)
            });
            window.click("preview-wrap", cx);
            preview.update(cx, |this, _| assert!(this.wrap));
        })
        .unwrap();
    }
}
