use super::*;
use gpui_kit::base::input::InlineToken;
use gpui_kit::test::TestWindowExt as _;
use std::prelude::v1::test;

fn setup(cx: &mut TestAppContext) -> (Entity<HomeView>, &mut VisualTestContext) {
    super::super::panes::tests::setup_initialized(
        cx,
        layout::LayoutState::default(),
        false,
        |home, window, cx| {
            home.update(cx, |home, cx| {
                home.state.update(cx, |state, cx| {
                    *state
                        .sessions_for_test()
                        .get_mut("empty")
                        .unwrap()
                        .state_for_test() = Some(
                        serde_json::from_value(serde_json::json!({
                            "sessionId":"shell-fixture", "isStreaming":false, "isCompacting":false
                        }))
                        .unwrap(),
                    );
                    state
                        .sessions_for_test()
                        .get_mut("empty")
                        .unwrap()
                        .attachments_for_test()
                        .push(gupi_conversation::attachments::Attachment::file(
                            "keep.txt".into(),
                            0,
                        ));
                    gupi_conversation::conversation::test_support::notify_session("empty", cx);
                });
                home.sync(false, window, cx);
                home.input.update(cx, |input, cx| {
                    input.set_value("keep ", window, cx);
                    input
                        .replace_range_with_token(
                            5..5,
                            InlineToken::new("keep-token", "@keep.txt"),
                            window,
                            cx,
                        )
                        .unwrap();
                    input.set_selected_range(0..0, cx);
                    input.focus(window, cx);
                });
            });
        },
    )
}

#[gpui_kit::test]
fn shell_key_entry_preserves_rich_draft_and_normal_character_routes(cx: &mut TestAppContext) {
    let (home, visual) = setup(cx);
    visual.run_until_parked();
    let original = visual.update(|_, cx| home.read(cx).input.read(cx).content());
    visual.simulate_keystrokes("!");
    visual.run_until_parked();
    visual.update(|window, cx| {
        let home = home.read(cx);
        assert!(home.shell_open());
        assert_eq!(home.input.read(cx).content(), original);
        assert_eq!(home.input.read(cx).selected_range(), 0..0);
        assert!(
            home.views["empty"]
                .shell
                .editor
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(
            home.state.read(cx).current().unwrap().attachments().len(),
            1
        );
    });
    visual.simulate_keystrokes("!");
    visual.run_until_parked();
    visual.update(|_, cx| {
        let shell = &home.read(cx).views["empty"].shell;
        assert!(shell.exclude);
        assert!(shell.editor.read(cx).value().is_empty());
    });
    // The third bang is command text, not another hidden prefix.
    visual.simulate_keystrokes("!");
    visual.simulate_input(" echo");
    visual.simulate_keystrokes("shift-enter");
    visual.simulate_input("pwd");
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert_eq!(
            home.read(cx).views["empty"].shell.editor.read(cx).value(),
            "! echo\npwd"
        );
        assert_eq!(home.read(cx).input.read(cx).content(), original);
        window.click("shell-return", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let home = home.read(cx);
        assert!(!home.shell_open());
        assert_eq!(home.input.read(cx).content(), original);
        assert_eq!(home.input.read(cx).selected_range(), 0..0);
        assert!(home.input.focus_handle(cx).is_focused(window));
    });
    // The original token insertion is still undoable after visiting Shell.
    visual.dispatch_action(input::Undo);
    visual.update(|_, cx| {
        assert!(home.read(cx).input.read(cx).content().tokens().is_empty());
    });
    visual.dispatch_action(input::Redo);
    visual.update(|_, cx| {
        assert_eq!(home.read(cx).input.read(cx).content(), original);
        let home = home.read(cx);
        let input = home.input.clone();
        input.update(cx, |input, cx| input.set_selected_range(1..1, cx));
    });
    visual.simulate_keystrokes("!");
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert!(!home.read(cx).shell_open());
        assert!(home.read(cx).input.read(cx).value().starts_with("k!eep"));
    });
    visual.dispatch_action(input::Undo);
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert_eq!(home.read(cx).input.read(cx).content(), original);
        let input = home.read(cx).input.clone();
        input.update(cx, |input, cx| input.set_selected_range(0..4, cx));
    });
    visual.simulate_keystrokes("!");
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert!(!home.read(cx).shell_open());
        assert!(home.read(cx).input.read(cx).value().starts_with("! "));
        assert_eq!(
            home.read(cx).input.read(cx).content().tokens().len(),
            original.tokens().len()
        );
    });
}

#[gpui_kit::test]
fn shell_paste_escape_and_explicit_recall_preserve_each_input(cx: &mut TestAppContext) {
    let (home, visual) = setup(cx);
    visual.run_until_parked();
    let original = visual.update(|_, cx| home.read(cx).input.read(cx).content());
    visual.update(|_, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string("!!! echo one\npwd".into()))
    });
    visual.dispatch_action(input::Paste);
    visual.run_until_parked();
    visual.update(|_, cx| {
        let shell = &home.read(cx).views["empty"].shell;
        assert!(shell.open && shell.exclude);
        assert_eq!(shell.editor.read(cx).value(), "! echo one\npwd");
    });
    // No connected Pi: rejection must retain input and cannot send the message draft.
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            let shell = &mut home.views.get_mut("empty").unwrap().shell;
            assert_eq!(shell.editor.read(cx).value(), "! echo one\npwd");
            shell.history = vec![
                Command {
                    text: "pwd".into(),
                    exclude: false,
                },
                Command {
                    text: "echo last".into(),
                    exclude: true,
                },
            ];
            shell.editor.update(cx, |input, cx| {
                input.set_value("next command", window, cx);
                input.set_selected_range(2..4, cx);
            });
            assert!(home.action_enabled(actions::Kind::PreviousShellCommand, cx));
            home.run_action(
                &actions::Run::new(actions::Kind::PreviousShellCommand),
                window,
                cx,
            );
            assert_eq!(
                home.views["empty"].shell.editor.read(cx).value(),
                "echo last"
            );
            home.run_action(
                &actions::Run::new(actions::Kind::PreviousShellCommand),
                window,
                cx,
            );
            assert_eq!(home.views["empty"].shell.editor.read(cx).value(), "pwd");
            assert!(!home.views["empty"].shell.exclude);
            home.run_action(
                &actions::Run::new(actions::Kind::NextShellCommand),
                window,
                cx,
            );
            home.run_action(
                &actions::Run::new(actions::Kind::NextShellCommand),
                window,
                cx,
            );
            assert_eq!(
                home.views["empty"].shell.editor.read(cx).value(),
                "next command"
            );
            assert_eq!(
                home.views["empty"].shell.editor.read(cx).selected_range(),
                2..4
            );
            assert!(home.views["empty"].shell.exclude);
            home.views["empty"]
                .shell
                .editor
                .update(cx, |input, cx| input.open_search(false, cx));
        });
    });
    visual.run_until_parked();
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            assert!(home.shell_open(), "first Escape closes only editor search");
            let input = home.views["empty"].shell.editor.clone();
            assert!(!input.read(cx).search_session().is_active());
            input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.replace_and_mark_text_in_range(None, "拼", None, window, cx);
            });
        });
    });
    visual.run_until_parked();
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert!(
            home.read(cx).shell_open(),
            "canceling IME must not exit Shell"
        )
    });
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert!(!home.read(cx).shell_open());
        assert_eq!(home.read(cx).input.read(cx).content(), original);
        let input = home.read(cx).input.clone();
        input.update(cx, |input, cx| {
            input.set_selected_range(0..0, cx);
            input.focus(window, cx);
        });
    });
    visual.simulate_keystrokes("!");
    visual.run_until_parked();
    visual.simulate_keystrokes("left !");
    visual.run_until_parked();
    visual.update(|window, cx| {
        let shell = &home.read(cx).views["empty"].shell;
        assert_eq!(
            shell.editor.read(cx).value(),
            "!",
            "even a clamped caret operation ends the second-prefix opportunity"
        );
        let input = shell.editor.clone();
        input.update(cx, |input, cx| input.set_value("", window, cx));
    });
    visual.run_until_parked();
    visual.simulate_keystrokes("backspace");
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert!(!home.read(cx).shell_open());
        assert_eq!(home.read(cx).input.read(cx).content(), original);
    });
}
