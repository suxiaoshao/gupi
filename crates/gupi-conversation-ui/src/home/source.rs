use super::files::directory;
use super::*;
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Selectable};

pub(super) struct Preview {
    quote_owner: Option<(WeakEntity<HomeView>, String)>,
    focus: FocusHandle,
    keyboard_open: bool,
    pub(super) path: PathBuf,
    root: Option<PathBuf>,
    editor: Entity<EditorState>,
    revision: u64,
    pub(super) quote_decoration: Option<gpui_kit::component::input::RangeDecorationCollection>,
    error: Option<&'static str>,
    loading: bool,
    loaded: bool,
    long_line: bool,
    wrap: bool,
    /// Mirrors the owning Files tab's tree visibility for the header toggle.
    pub(super) navigator_visible: bool,
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
            quote_owner: None,
            focus: cx.focus_handle(),
            keyboard_open,
            path,
            root,
            editor,
            revision: 0,
            quote_decoration: None,
            error: None,
            loading: false,
            loaded: false,
            long_line: false,
            wrap: false,
            navigator_visible: true,
            task: None,
        };
        if keyboard_open {
            this.focus.focus(window, cx);
        }
        this.read(window, cx);
        this
    }

    pub(super) fn with_quote_owner(mut self, owner: WeakEntity<HomeView>, key: String) -> Self {
        self.quote_owner = Some((owner, key));
        self
    }
    pub(super) fn quote_selection(&self, cx: &App) -> Result<quote::Selection, quote::Unavailable> {
        if !self.loaded {
            return Err(quote::Unavailable::Selection);
        }
        let editor = self.editor.read(cx);
        let range = editor.selected_range();
        if range.is_empty() {
            return Err(quote::Unavailable::Selection);
        }
        let text = editor.text().to_string();
        let selected = text
            .get(range.clone())
            .ok_or(quote::Unavailable::Selection)?
            .to_owned();
        let start = text[..range.start].bytes().filter(|b| *b == b'\n').count() + 1;
        let end = text[..range.end]
            .strip_suffix('\n')
            .unwrap_or(&text[..range.end])
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        Ok(quote::Selection::new(
            quote::Origin::File {
                path: self.path.clone(),
                start,
                end,
                language: editor.language_name().to_string(),
                kind: quote::FileKind::Source,
                original: false,
            },
            selected,
        ))
    }
    pub(super) fn quote_source(
        &self,
        state: WeakEntity<Self>,
        cx: &App,
    ) -> Option<quote::highlight::Source> {
        if !self.loaded || self.loading || self.error.is_some() {
            return None;
        }
        self.quote_range(cx)
            .filter(|range| !range.is_empty())
            .map(|range| quote::highlight::Source::File {
                state,
                revision: self.revision,
                range,
            })
    }
    pub(super) fn quote_editor(&self, revision: u64, _: &App) -> Option<Entity<EditorState>> {
        (self.loaded && !self.loading && self.error.is_none() && self.revision == revision)
            .then(|| self.editor.clone())
    }
    fn read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        if let Some(collection) = self.quote_decoration.take() {
            collection.clear(cx);
            collection.dispose(cx);
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
    ToggleNavigator,
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
    pub(super) fn quote_range(&self, cx: &App) -> Option<std::ops::Range<usize>> {
        self.loaded.then(|| self.editor.read(cx).selected_range())
    }
    pub(super) fn clear_quote_selection(&self, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            let end = editor.selected_range().end;
            editor.set_selected_range(end..end, cx);
        });
    }
    pub(super) fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }
}
impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let path = self.path.to_string_lossy().into_owned();
        let preview = cx.weak_entity();
        let quote_owner = self.quote_owner.clone();
        let quote_events = self.quote_owner.clone().map(|(owner, key)| {
            quote::toolbar::region(quote::toolbar::Target::Source(preview.clone()), owner, key)
        });
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
                        Button::new("source-navigator")
                            .small()
                            .ghost()
                            .selected(self.navigator_visible)
                            .icon(IconName::FolderTree)
                            .tooltip(t(cx, "workspace-toggle-file-tree"))
                            .accessibility_label(t(cx, "workspace-toggle-file-tree"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::ToggleNavigator))),
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
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .children(quote_events)
                        .child(
                            Editor::new(&self.editor)
                                .context_menu(move |menu, _, cx| {
                                    let selected = preview
                                        .read_with(cx, |source, cx| source.quote_selection(cx))
                                        .unwrap_or(Err(quote::Unavailable::Selection));
                                    let empty =
                                        selected.as_ref().map_or(true, |s| s.text.is_empty());
                                    let capture = quote_owner
                                        .as_ref()
                                        .map(|(owner, key)| {
                                            quote::capture(
                                                owner,
                                                key,
                                                selected,
                                                preview
                                                    .read_with(cx, |source, cx| {
                                                        source.quote_source(preview.clone(), cx)
                                                    })
                                                    .ok()
                                                    .flatten(),
                                                cx,
                                            )
                                        })
                                        .unwrap_or(Err(quote::Unavailable::Composer));
                                    let reason = capture.as_ref().err().copied();
                                    menu.menu_with_disabled(
                                        t(cx, "menu-copy"),
                                        empty,
                                        Box::new(gpui_kit::component::input::Copy),
                                    )
                                    .menu_with_disabled(
                                        reason
                                            .map(|reason| reason.menu_label(cx))
                                            .unwrap_or_else(|| t(cx, "quote-in-composer")),
                                        reason.is_some(),
                                        Box::new(quote::Quote(capture.ok())),
                                    )
                                    .separator()
                                    .menu(
                                        t(cx, "menu-select-all"),
                                        Box::new(gpui_kit::component::input::SelectAll),
                                    )
                                })
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
            window.press("secondary-f", cx);
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
            preview.update(cx, |this, cx| {
                this.editor
                    .update(cx, |editor, cx| editor.set_selected_range(3..7, cx))
            });
            let source = preview
                .read(cx)
                .quote_source(preview.downgrade(), cx)
                .unwrap();
            let mut active = crate::home::quote::highlight::Active::new(source.clone());
            active.paint(true, cx);
            let collection = preview.read(cx).quote_decoration.clone().unwrap();
            assert_eq!(
                collection.get_ranges(cx),
                vec![3..7],
                "partial-line source range is retained"
            );
            active.clear(cx);
            assert!(
                collection.get_ranges(cx).is_empty(),
                "ending edit disposes its fill"
            );
            active.paint(true, cx);
            let reloaded = preview.read(cx).quote_decoration.clone().unwrap();
            preview.update(cx, |this, cx| this.read(window, cx));
            active.paint(true, cx);
            assert!(!source.valid(cx));
            assert!(
                reloaded.get_ranges(cx).is_empty(),
                "reload cannot reuse the old source version"
            );
        })
        .unwrap();
    }
}
