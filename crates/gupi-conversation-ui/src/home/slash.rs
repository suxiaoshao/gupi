use super::*;
use crate::command_palette::CommandPalette;

#[derive(Default)]
pub(super) struct Completion {
    leading_slash: bool,
    dismissed: bool,
    file_dismissed: bool,
}

/// A palette owns this exact composer snapshot, never a later selection.
#[derive(Clone)]
pub(crate) struct FileInsertion {
    key: String,
    binding: u64,
    content: gpui_kit::component::input::InputContent,
    range: std::ops::Range<usize>,
}
impl Completion {
    pub(super) fn dismiss_file_query(&mut self) {
        self.file_dismissed = true;
    }
    /// Recalled text is not a newly typed command; leave the palette closed.
    pub(super) fn recalled(&mut self, text: &str) {
        self.leading_slash = text.starts_with('/');
    }
    fn changed(&mut self, text: &str) -> bool {
        let leading = text.starts_with('/');
        if !leading {
            self.dismissed = false;
        }
        let open = leading && !self.leading_slash && !self.dismissed;
        self.leading_slash = leading;
        open
    }
}
impl HomeView {
    pub fn select_resource(
        &mut self,
        command: pi_rpc::protocol::SlashCommand,
        from_composer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(token) = gupi_resources::composer_resources::token(&command, true) else {
            return;
        };
        let Some(session) = self.state.read(cx).current() else {
            return;
        };
        if !session.can_edit_draft() {
            return;
        }
        let text = self.input.read(cx).value();
        let mut range = gupi_resources::composer_resources::replacement(
            &text,
            from_composer,
            session
                .commands()
                .data()
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
        // Pi's delimiter belongs to this one insertion transaction. Reuse an
        // existing space instead of adding a separate edit/undo step.
        if text
            .get(range.end..)
            .is_some_and(|suffix| suffix.starts_with(' '))
        {
            range.end += 1;
        }
        let end = range.start + token.text().len();
        // Dismissal precedes the Change event from this user edit.
        self.slash.leading_slash = true;
        self.slash.dismissed = true;
        self.input.update(cx, |input, cx| {
            if input
                .replace_range_with_token(range, token, window, cx)
                .is_ok()
            {
                input.set_selected_range(end..end, cx);
                input.focus(window, cx);
            }
        });
    }
    pub(super) fn open_slash_if_needed(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.input.read(cx);
        let query = file_query(text, input.cursor()).filter(|range| {
            !input
                .tokens()
                .iter()
                .any(|token| token.range().start < range.end && token.range().end > range.start)
        });
        if query.is_none() {
            self.slash.file_dismissed = false;
        }
        if let Some(range) = query
            && !self.slash.file_dismissed
            && self.input.read(cx).focus_handle(cx).is_focused(window)
            && !(window.has_active_dialog(cx) || self.has_image_preview(cx))
        {
            self.open_files(Some(range), window, cx);
            return;
        }
        if self.slash.changed(text)
            && !self
                .input
                .read(cx)
                .content()
                .tokens()
                .iter()
                .any(|token| token.range().start == 0)
            && self.input.read(cx).focus_handle(cx).is_focused(window)
            && !(window.has_active_dialog(cx) || self.has_image_preview(cx))
            && self.command_input_allowed(cx)
        {
            self.open_commands(true, window, cx);
        }
    }
    pub fn command_input_allowed(&self, cx: &App) -> bool {
        let state = self.state.read(cx);
        let Some(key) = state.selected().as_ref() else {
            return false;
        };
        let Some(s) = state.current() else {
            return false;
        };
        let preview = self.views.get(key).and_then(|v| v.preview.as_deref());
        state.can_submit(key, cx) && preview.is_none_or(|id| s.history().on_current_path(id))
    }
    pub fn command_label(&self, kind: actions::Kind, cx: &App) -> String {
        if kind == actions::Kind::Stop
            && self.state.read(cx).is_temporary()
            && self.state.read(cx).current().is_none_or(|s| !s.busy())
        {
            return t(cx, "temporary-hide");
        }
        palette::label(
            kind,
            self.show_sidebar,
            self.pane_layout.visible(panes::Panel::History),
            cx,
        )
    }
    pub fn commands_closed(&mut self, from_composer: bool, cx: &mut Context<Self>) {
        self.command_panel = None;
        if from_composer {
            self.slash.leading_slash = self.input.read(cx).value().starts_with('/');
            self.slash.dismissed = self.slash.leading_slash;
        }
        self.slash.file_dismissed = true;
        cx.notify();
    }
    pub fn close_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(panel) = self.command_panel.take() {
            let from_composer = panel.read(cx).opened_from_composer();
            window.close_dialog(cx);
            self.commands_closed(from_composer, cx);
        }
    }
    pub fn open_commands(
        &mut self,
        from_composer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.command_panel.clone() {
            panel.update(cx, |panel, cx| panel.focus_input(window, cx));
            return;
        }
        self.close_session_search(window, cx);
        if window.has_active_dialog(cx) || self.has_image_preview(cx) {
            return;
        }
        let insertion = self.capture_file_insertion(None, cx);
        let quote = self.capture_quote(window, cx);
        self.command_panel = Some(CommandPalette::open_with_context(
            Some((cx.weak_entity(), self.state.clone())),
            from_composer,
            if from_composer {
                self.input.read(cx).value().to_string()
            } else {
                String::new()
            },
            insertion,
            Some(quote),
            window,
            cx,
        ));
    }

    pub(super) fn open_files(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || self.has_image_preview(cx) {
            return;
        }
        let Some(session) = self.state.read(cx).current() else {
            return;
        };
        if !session.can_edit_draft() || !session.pending_ui().is_empty() {
            return;
        }
        let insertion = self.capture_file_insertion(range, cx);
        let query = insertion
            .as_ref()
            .and_then(|insertion| insertion.content.text().get(insertion.range.clone()))
            .filter(|text| text.starts_with('@'))
            .unwrap_or("@")
            .to_owned();
        self.slash.file_dismissed = true;
        let quote = self.capture_quote(window, cx);
        self.command_panel = Some(CommandPalette::open_with_context(
            Some((cx.weak_entity(), self.state.clone())),
            false,
            query,
            insertion,
            Some(quote),
            window,
            cx,
        ));
    }

    pub(crate) fn capture_file_insertion(
        &self,
        range: Option<std::ops::Range<usize>>,
        cx: &App,
    ) -> Option<FileInsertion> {
        let state = self.state.read(cx);
        let key = state.selected().as_ref()?;
        let session = state.current()?;
        let input = self.input.read(cx);
        let range = range.unwrap_or_else(|| input.selected_range());
        let content = input.content();
        content.text().get(range.clone())?;
        Some(FileInsertion {
            key: key.clone(),
            binding: session.binding(),
            content,
            range,
        })
    }

    pub(crate) fn select_file(
        &mut self,
        path: PathBuf,
        insertion: FileInsertion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.read(cx);
        if state.selected().as_ref() != Some(&insertion.key)
            || !state
                .current()
                .is_some_and(|session| session.binding() == insertion.binding)
            || self.input.read(cx).content() != insertion.content
        {
            return;
        }
        self.insert_file_reference(&path, insertion.range, window, cx);
    }

    pub(super) fn insert_file_reference(
        &mut self,
        path: &std::path::Path,
        mut range: std::ops::Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self
            .state
            .read(cx)
            .current()
            .is_some_and(|s| s.can_edit_draft())
        {
            return;
        }
        let text = self.input.read(cx).value();
        let leading_space = range.start > 0 && !text[..range.start].ends_with(char::is_whitespace);
        if text[range.end..].starts_with(' ') {
            range.end += 1;
        }
        let token = gupi_resources::composer_resources::file_token(path, leading_space);
        self.slash.file_dismissed = true;
        self.input.update(cx, |input, cx| {
            // Paths containing line breaks cannot be atomic tokens. Preserve
            // their exact reference as editable text instead of losing the file.
            match input.replace_range_with_token(range.clone(), token.clone(), window, cx) {
                Ok(()) => {}
                Err(gpui_kit::component::input::InlineTokenError::InvalidToken) => {
                    input.set_selected_range(range, cx);
                    input.replace(token.text(), window, cx);
                }
                Err(_) => return,
            }
            input.focus(window, cx);
        });
    }
}

fn file_query(text: &str, cursor: usize) -> Option<std::ops::Range<usize>> {
    let before = text.get(..cursor)?;
    let line = before.rfind('\n').map_or(0, |index| index + 1);
    for (offset, _) in before[line..].match_indices('@').rev() {
        let start = line + offset;
        let prefix = before[..start].trim_end_matches(['(', '[', '{', '<', '`']);
        if !prefix.is_empty() && !prefix.ends_with(reference_boundary) {
            continue;
        }
        let suffix = &before[start + 1..];
        if let Some(quoted) = suffix.strip_prefix('"') {
            // A closing quote may be at or just after the cursor. Consume it
            // once so token insertion preserves the remaining prose verbatim.
            if let Some(close) = quoted.find('"')
                && close + 1 != quoted.len()
            {
                continue;
            }
            let end =
                cursor + usize::from(!quoted.ends_with('"') && text[cursor..].starts_with('"'));
            return Some(start..end);
        }
        if suffix.contains(reference_boundary) {
            continue;
        }
        return Some(start..cursor);
    }
    None
}

fn reference_boundary(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '，' | '。'
                | '、'
                | '．'
                | '：'
                | '；'
                | '！'
                | '？'
                | '（'
                | '）'
                | '［'
                | '］'
                | '｛'
                | '｝'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '…'
                | '—'
                | '"'
                | '\''
                | '='
        )
}
#[cfg(test)]
mod tests {
    use super::Completion;
    #[test]
    fn file_reference_boundaries_quotes_and_cursor_suffix() {
        for text in [
            "@src",
            "请看：@src",
            "见，@文档",
            "(@src",
            "[(@src",
            "text `@src",
            "@app/[slug]/page",
            "@src/(group)/file",
            "(@app/[slug]/page",
            "`@src/(group)/file",
            "@\"path with spaces",
            "@\"path with spaces\"",
        ] {
            let range = super::file_query(text, text.len()).unwrap();
            assert_eq!(&text[range], &text[text.find('@').unwrap()..]);
        }
        for text in [
            "mail@example.com",
            "正文@src",
            "@src text",
            "@\"closed\" prose",
            "@old\nnew",
        ] {
            assert!(super::file_query(text, text.len()).is_none(), "{text}");
        }
        let text = "prefix @\"path with spaces\" suffix";
        let cursor = text.find("\" suffix").unwrap();
        let range = super::file_query(text, cursor).unwrap();
        assert_eq!(&text[range.clone()], "@\"path with spaces\"");
        assert_eq!(&text[range.end..], " suffix");
    }

    #[gpui_kit::test]
    fn file_palette_directory_tab_accept_escape_and_stale_range(cx: &mut gpui_kit::TestAppContext) {
        use super::*;
        use gpui_kit::component::Root;
        use gpui_kit::test::TestWindowExt as _;
        let fixture = tempfile::tempdir().unwrap();
        std::fs::create_dir(fixture.path().join("src")).unwrap();
        std::fs::create_dir(fixture.path().join("src/components")).unwrap();
        std::fs::write(
            fixture.path().join("src/components/Button.rs"),
            "struct Button;\n",
        )
        .unwrap();
        let file = fixture.path().join("src/main.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_settings::keybindings::apply(&Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(layout::LayoutState::default());
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            let info = gupi_conversation::session_catalog::SessionInfo::new(Default::default(), "files".into(), fixture.path().to_owned());
            let mut session = gupi_conversation::conversation::Session::new(info, String::new());
            *session.state_for_test() = Some(serde_json::from_value(serde_json::json!({"sessionId":"fixture","isStreaming":false,"isCompacting":false})).unwrap());
            state.sessions_for_test().insert("files".into(), session);
            *state.selected_for_test() = Some("files".into());
        });
        let mut home = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state, window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        visual.run_until_parked();
        let original = "see @\"src\" suffix";
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.slash.file_dismissed = true;
                home.input.update(cx, |input, cx| {
                    input.set_value(original, window, cx);
                    input.set_selected_range(9..9, cx);
                    input.focus(window, cx);
                });
                home.open_files(Some(4..10), window, cx);
            })
        });
        fn settle(home: &Entity<HomeView>, visual: &mut VisualTestContext) {
            visual.run_until_parked();
            // The test platform has no native frame loop. Draw the loading
            // state, then exactly one results frame, without forcing refresh
            // or a second render_frame that repairs stale footer state.
            visual.update(|window, cx| window.draw(cx).clear(cx));
            visual
                .executor()
                .advance_clock(std::time::Duration::from_millis(50));
            for _ in 0..200 {
                visual.run_until_parked();
                if visual.update(|_, cx| {
                    home.read(cx)
                        .command_panel
                        .as_ref()
                        .is_some_and(|panel| panel.read(cx).file_search_ready())
                }) {
                    visual.update(|window, cx| window.draw(cx).clear(cx));
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            panic!("file query did not settle");
        }
        settle(&home, visual);
        visual.simulate_keystrokes("down");
        visual.simulate_keystrokes("tab");
        visual.update(|_, cx| {
            assert!(home.read(cx).command_panel.is_some());
            assert_eq!(home.read(cx).input.read(cx).value().as_ref(), original);
        });
        settle(&home, visual);
        visual.update(|window, cx| {
            let action = window.find("command-send");
            assert_eq!(
                action.label(),
                Some(t(cx, "command-select-resource").as_str())
            );
            assert_ne!(action.disabled(), Some(true));
        });
        visual.simulate_keystrokes("enter");
        visual.run_until_parked();
        visual.update(|window, cx| {
            let home = home.read(cx);
            assert!(home.command_panel.is_none());
            let input = home.input.read(cx);
            assert_eq!(input.content().tokens().len(), 1);
            assert!(input.value().starts_with("see "));
            assert!(input.value().ends_with(" suffix"));
            assert!(input.focus_handle(cx).is_focused(window));
        });
        visual.dispatch_action(gpui_kit::component::input::Undo);
        visual.update(|_, cx| assert_eq!(home.read(cx).input.read(cx).value().as_ref(), original));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| home.open_files(Some(4..10), window, cx))
        });
        settle(&home, visual);
        visual.simulate_keystrokes("escape");
        visual.update(|window, cx| {
            let home = home.read(cx);
            assert!(home.command_panel.is_none());
            assert_eq!(home.input.read(cx).value().as_ref(), original);
            assert!(home.input.read(cx).focus_handle(cx).is_focused(window));
        });
        // The previous cancelled picker cannot lend its range to either
        // ordinary command-palette entry route.
        for from_composer in [false, true] {
            visual.update(|window, cx| {
                home.update(cx, |home, cx| {
                    home.input
                        .update(cx, |input, cx| input.set_selected_range(0..0, cx));
                    home.open_commands(from_composer, window, cx);
                })
            });
            visual.dispatch_action(gpui_kit::component::input::SelectAll);
            visual.simulate_input(if from_composer {
                "@src/main.rs"
            } else {
                "@src/"
            });
            settle(&home, visual);
            visual.simulate_keystrokes("down");
            visual.simulate_keystrokes("enter");
            if !from_composer {
                visual.update(|_, cx| {
                    assert!(home.read(cx).command_panel.is_some());
                    assert_eq!(home.read(cx).input.read(cx).value().as_ref(), original);
                });
                settle(&home, visual);
                visual.update(|window, _| {
                    let action = window.find("command-send");
                    assert_ne!(action.disabled(), Some(true));
                });
                // Enter on a directory must leave its first file selected.
                visual.simulate_keystrokes("enter");
            }
            visual.update(|_, cx| {
                let home = home.read(cx);
                let content = home.input.read(cx).content();
                assert!(home.command_panel.is_none());
                assert_eq!(content.tokens().len(), 1);
                assert_eq!(content.tokens()[0].range().start, 0);
                assert!(content.text().ends_with(original));
            });
            visual.dispatch_action(gpui_kit::component::input::Undo);
            visual.update(|_, cx| {
                assert_eq!(home.read(cx).input.read(cx).value().as_ref(), original)
            });
        }
        // A changed source range must not fall back to replacing the new selection.
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                let insertion = home.capture_file_insertion(Some(4..10), cx).unwrap();
                home.open_files(Some(4..10), window, cx);
                home.input.update(cx, |input, cx| {
                    input.set_value("new @\"src\" suffix", window, cx)
                });
                home.select_file(file.clone(), insertion, window, cx);
                assert_eq!(home.input.read(cx).value().as_ref(), "new @\"src\" suffix");
                home.close_commands(window, cx);
                home.input
                    .update(cx, |input, cx| input.set_selected_range(0..0, cx));
                let empty = home.capture_file_insertion(None, cx).unwrap();
                home.open_files(None, window, cx);
                home.input.update(cx, |input, cx| {
                    input.set_value("replacement draft", window, cx)
                });
                home.select_file(file, empty, window, cx);
                assert_eq!(home.input.read(cx).value().as_ref(), "replacement draft");
                home.close_commands(window, cx);
            })
        });
    }

    #[test]
    fn escape_dismissal_survives_edits_until_leading_slash_is_removed() {
        let mut c = Completion::default();
        assert!(c.changed("/"));
        c.dismissed = true;
        assert!(!c.changed("/review"));
        assert!(!c.changed("/rev"));
        assert!(!c.changed(""));
        assert!(c.changed("/"));
        assert!(!c.changed("/ 参数"));
    }
}
