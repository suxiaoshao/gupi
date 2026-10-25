//! Pi's Up/Down history recall for the conversation composer.
use super::*;
use gpui_kit::component::input::{BufferPoint, InputContent, RopeExt as _};
use std::ops::Range;

actions!(gupi, [PreviousInput, NextInput]);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(
            "up",
            PreviousInput,
            Some("GupiComposer > Input && !GupiQuoteComment"),
        ),
        KeyBinding::new(
            "down",
            NextInput,
            Some("GupiComposer > Input && !GupiQuoteComment"),
        ),
    ]);
}

pub(super) struct Browse {
    /// Distance from the newest entry.
    back: usize,
    shown: String,
    draft: InputContent,
    selection: Range<usize>,
}

impl HomeView {
    /// The caret's visual row and the row count, from Kit's soft-wrap map.
    fn composer_rows(&self, cx: &App) -> (usize, usize) {
        let input = self.input.read(cx);
        let point = input.text().offset_to_point(input.cursor());
        let map = input.display_map();
        let row = map
            .buffer_pos_to_display_pos(BufferPoint::new(point.row, point.column))
            .row;
        (row, map.display_row_count().max(1))
    }
    fn recall_target(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        let key = self.shown_key.clone()?;
        let session = self.state.read(cx).sessions().get(&key)?;
        let input = self.input.read(cx);
        if !session.can_edit_draft()
            || !input.presentation().is_editable()
            || !input.selected_range().is_empty()
        {
            return None;
        }
        self.input
            .update(cx, |input, cx| {
                input.marked_text_range(window, cx).is_none()
            })
            .then_some(key)
    }
    pub(super) fn previous_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.recall_target(window, cx) else {
            return cx.propagate();
        };
        if self.composer_rows(cx).0 != 0 {
            return cx.propagate();
        }
        let history_len = self.state.read(cx).sessions()[&key].input_history().len();
        let browse = self.views[&key].input_browse.as_ref();
        let input = self.input.read(cx);
        let recall = input.text().len() == 0 || browse.is_some() || input.cursor() == 0;
        if !recall {
            self.input
                .update(cx, |input, cx| input.set_selected_range(0..0, cx));
            return;
        }
        let back = browse.map_or(0, |browse| browse.back + 1);
        if back >= history_len {
            return;
        }
        if browse.is_none() {
            let draft = input.content();
            let selection = input.selected_range();
            self.views.get_mut(&key).unwrap().input_browse = Some(Browse {
                back,
                shown: String::new(),
                draft,
                selection,
            });
        }
        self.show_input(&key, back, true, window, cx);
    }
    pub(super) fn next_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.recall_target(window, cx) else {
            return cx.propagate();
        };
        let (row, rows) = self.composer_rows(cx);
        if row + 1 != rows {
            return cx.propagate();
        }
        let Some(back) = self.views[&key].input_browse.as_ref().map(|b| b.back) else {
            let end = self.input.read(cx).text().len();
            self.input
                .update(cx, |input, cx| input.set_selected_range(end..end, cx));
            return;
        };
        if back == 0 {
            let browse = self
                .views
                .get_mut(&key)
                .unwrap()
                .input_browse
                .take()
                .unwrap();
            // Restoring keeps the draft's inline tokens, which set_value does not report.
            self.input.update(cx, |input, cx| {
                input.set_value(browse.draft.clone(), window, cx);
                input.set_selected_range(browse.selection, cx);
            });
            self.state.update(cx, |state, cx| {
                state.set_draft_content(&key, browse.draft, cx)
            });
            self.views.get_mut(&key).unwrap().input_draft =
                self.state.read(cx).sessions()[&key].draft().clone();
            return;
        }
        self.show_input(&key, back - 1, false, window, cx);
    }
    fn show_input(
        &mut self,
        key: &str,
        back: usize,
        older: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let history = self.state.read(cx).sessions()[key].input_history();
        let text = history[history.len() - 1 - back].clone();
        let browse = self
            .views
            .get_mut(key)
            .unwrap()
            .input_browse
            .as_mut()
            .unwrap();
        browse.back = back;
        browse.shown = text.clone();
        let caret = if older { 0 } else { text.len() };
        // An undoable edit, so the change reports the recalled draft like typing does.
        self.slash.recalled(&text);
        self.input.update(cx, |input, cx| {
            input.replace_all(text, window, cx);
            input.set_selected_range(caret..caret, cx);
        });
    }
    /// Editing recalled text leaves history browsing, as in Pi.
    pub(super) fn input_edited(&mut self, key: &str, text: &str) {
        if let Some(view) = self.views.get_mut(key)
            && view
                .input_browse
                .as_ref()
                .is_some_and(|browse| browse.shown != text)
        {
            view.input_browse = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::base::input::InlineToken;
    use gpui_kit::test::TestWindowExt as _;
    use gupi_conversation::conversation::Session;
    use gupi_conversation::session_catalog::SessionInfo;
    use serde_json::json;
    use std::prelude::v1::test;

    fn session(key: &str, texts: &[&str]) -> Session {
        let mut session = Session::new(
            SessionInfo::new(Default::default(), key.into(), Default::default()),
            String::new(),
        );
        *session.state_for_test() = Some(
            serde_json::from_value(
                json!({"sessionId":key,"isStreaming":false,"isCompacting":false}),
            )
            .unwrap(),
        );
        session.replace_entries_for_test(pi_rpc::protocol::Entries {
            entries: texts.iter().enumerate().map(|(i, text)| serde_json::from_value(json!({
                "type":"message", "id":i.to_string(), "parentId":i.checked_sub(1).map(|i|i.to_string()),
                "timestamp":"2026-10-25T00:00:00Z", "message":{"role":"user", "content":text}
            })).unwrap()).collect(),
            leaf_id: texts.len().checked_sub(1).map(|i|i.to_string()),
        });
        session
    }
    fn mount(cx: &mut TestAppContext) -> (Entity<HomeView>, &mut VisualTestContext) {
        super::super::panes::tests::setup_initialized(
            cx,
            layout::LayoutState::default(),
            false,
            |home, window, cx| {
                home.update(cx, |home, cx| {
                    home.state.update(cx, |state, _| {
                        state
                            .sessions_for_test()
                            .insert("empty".into(), session("empty", &["older", "/latest"]));
                        state
                            .sessions_for_test()
                            .insert("other".into(), session("other", &["other session"]));
                    });
                    home.sync(false, window, cx);
                    home.input.focus_handle(cx).focus(window, cx);
                });
            },
        )
    }
    #[gpui_kit::test]
    fn recall_preserves_rich_draft_undo_and_session_ownership(cx: &mut TestAppContext) {
        let (home, visual) = mount(cx);
        let original = visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.input.update(cx, |input, cx| {
                    input.set_value("draft ", window, cx);
                    input
                        .replace_range_with_token(
                            6..6,
                            InlineToken::new("file", "@keep.rs"),
                            window,
                            cx,
                        )
                        .unwrap();
                    input.set_selected_range(0..0, cx);
                });
                home.input.read(cx).content()
            })
        });
        visual.run_until_parked();
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert_eq!(home.read(cx).input.read(cx).value(), "/latest");
            assert!(home.read(cx).command_panel.is_none());
        });
        visual.simulate_keystrokes("secondary-z");
        visual.run_until_parked();
        visual.update(|_, cx| assert_eq!(home.read(cx).input.read(cx).content(), original));
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
        // The other session must neither inherit the browse index nor overwrite it.
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.state
                    .update(cx, |state, cx| state.select_existing("other", cx));
                home.sync(false, window, cx);
                home.input.focus_handle(cx).focus(window, cx);
            })
        });
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
        visual.update(|_, cx| assert_eq!(home.read(cx).input.read(cx).value(), "other session"));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.state
                    .update(cx, |state, cx| state.select_existing("empty", cx));
                home.sync(false, window, cx);
                home.input.focus_handle(cx).focus(window, cx);
                let end = home.input.read(cx).text().len();
                home.input
                    .update(cx, |input, cx| input.set_selected_range(end..end, cx));
            })
        });
        visual.simulate_keystrokes("down");
        visual.run_until_parked();
        visual.update(|_, cx| {
            let home = home.read(cx);
            assert_eq!(home.input.read(cx).content(), original);
            assert_eq!(home.input.read(cx).selected_range(), 0..0);
            assert_eq!(home.state.read(cx).sessions()["empty"].draft(), &original);
        });
    }
    #[gpui_kit::test]
    fn arrows_respect_visual_rows_and_editing_leaves_history(cx: &mut TestAppContext) {
        let (home, visual) = mount(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.input.update(cx, |input, cx| {
                    input.set_value("wrapped words ".repeat(100), window, cx);
                    let end = input.text().len();
                    input.set_selected_range(end..end, cx);
                });
            })
        });
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(
                home.read(cx)
                    .input
                    .read(cx)
                    .display_map()
                    .display_row_count()
                    > 1
            );
        });
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                assert!(home.views["empty"].input_browse.is_none());
                assert!(home.input.read(cx).value().starts_with("wrapped words"));
                home.input.update(cx, |input, cx| {
                    input.set_value("  ", window, cx);
                    input.set_selected_range(2..2, cx);
                });
            })
        });
        // Spaces are a nonempty draft: first Up goes to its beginning.
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert_eq!(home.read(cx).input.read(cx).value(), "  ");
            assert_eq!(home.read(cx).input.read(cx).selected_range(), 0..0);
        });
        visual.simulate_keystrokes("up up up");
        visual.run_until_parked();
        visual.update(|_, cx| assert_eq!(home.read(cx).input.read(cx).value(), "older"));
        visual.simulate_input("edit ");
        visual.run_until_parked();
        visual.update(|_, cx| assert!(home.read(cx).views["empty"].input_browse.is_none()));
        visual.simulate_keystrokes("down");
        visual.run_until_parked();
        visual.update(|_, cx| {
            let input = home.read(cx).input.read(cx);
            assert_eq!(input.value(), "edit older");
            assert_eq!(
                input.selected_range(),
                input.text().len()..input.text().len()
            );
        });
    }
}
