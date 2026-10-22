use super::*;
use crate::home::{TestLayoutDirectory, panes};
use core::prelude::v1::test;
use gpui_kit::component::Root;
use gpui_kit::test::TestWindowExt;
use gupi_conversation::{conversation::Session, session_catalog::SessionInfo};
use serde_json::json;

fn session(key: &str) -> Session {
    let mut session = Session::new(
        SessionInfo::new(Default::default(), key.into(), "/fixture".into()),
        String::new(),
    );
    let mut entries = vec![];
    let mut parent = None;
    for index in 0..12 {
        for (role, id, text) in [
            (
                "user",
                format!("{key}-user-{index}"),
                format!("Question **{index}**"),
            ),
            (
                "assistant",
                format!("{key}-answer-{index}"),
                format!(
                    "## Answer {index}\n\n{}",
                    "Readable [label](https://example.com) and `code`.\n\n".repeat(12)
                ),
            ),
        ] {
            entries.push(serde_json::from_value(json!({"type":"message","id":id,"parentId":parent,"timestamp":"2026-10-23T00:00:00Z","message":{"role":role,"content":text,"stopReason":"stop"}})).unwrap());
            parent = Some(id);
        }
    }
    session.replace_entries_for_test(pi_rpc::protocol::Entries {
        entries,
        leaf_id: parent,
    });
    session
}
fn mount(cx: &mut TestAppContext) -> (Entity<HomeView>, &mut VisualTestContext, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::host::install_headless(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        cx.set_global(layout::LayoutState::default());
        cx.set_global(TestLayoutDirectory(directory.path().into()));
    });
    let state = cx.new(|cx| ConversationState::new("unused".into(), cx));
    state.update(cx, |state, _| {
        state.sessions_for_test().insert("a".into(), session("a"));
        state.sessions_for_test().insert("b".into(), session("b"));
        *state.selected_for_test() = Some("a".into());
    });
    let mut home = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| HomeView::with_state(state, window, cx));
        home = Some(view.clone());
        Root::new(view, window, cx)
    });
    visual.simulate_resize(size(px(1700.), px(900.)));
    visual.update(|window, _| window.activate_window());
    visual.run_until_parked();
    (home.unwrap(), visual, directory)
}

#[gpui_kit::test]
fn rail_click_drag_and_reading_shortcuts_follow_the_real_scroller(cx: &mut TestAppContext) {
    let (home, visual, _directory) = mount(cx);
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nav = home.read(cx).views["a"].navigator.read(cx);
        assert!(
            nav.shown(window),
            "enabled={} turns={} viewport={:?} gutter={:?} visible={:?}",
            nav.enabled,
            nav.turns.len(),
            nav.viewport,
            nav.gutter,
            nav.visible
        );
        window.click("conversation-turn-a-a-user-3", cx);
    });
    visual.run_until_parked();
    let nav = visual.update(|_, cx| home.read(cx).views["a"].navigator.clone());
    visual.update(|window, cx| {
        let navigator = nav.read(cx);
        assert_eq!(
            navigator.scroller.read(cx).logical_scroll_top().item_ix,
            navigator.turns[3].row
        );
        assert!(navigator.visible.contains(&3));
        home.read(cx).messages_focus.clone().focus(window, cx);
    });
    visual.simulate_keystrokes("alt-down");
    visual.run_until_parked();
    visual.update(|window, cx| {
        let navigator = nav.read(cx);
        assert_eq!(
            navigator.scroller.read(cx).logical_scroll_top().item_ix,
            navigator.turns[4].row
        );
        window.draw(cx).clear(cx);
        let from = window
            .find("conversation-turn-a-a-user-4")
            .bounds()
            .center();
        let to = window
            .find("conversation-turn-a-a-user-7")
            .bounds()
            .center()
            + point(px(500.), px(0.));
        let released = window
            .find("conversation-turn-a-a-user-8")
            .bounds()
            .center()
            + point(px(500.), px(0.));
        // Real events, with no intermediate render between down/move/up.
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position: from,
                ..Default::default()
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                pressed_button: Some(MouseButton::Left),
                position: to,
                ..Default::default()
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: released,
                ..Default::default()
            }),
            cx,
        );
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        let navigator = nav.read(cx);
        assert!(!navigator.dragging);
        assert!(
            navigator.preview.is_none(),
            "release outside the rail closes its preview"
        );
        assert_eq!(
            navigator.scroller.read(cx).logical_scroll_top().item_ix,
            navigator.turns[8].row
        );
        assert!(!navigator.turns[8].answer.contains("https://"));
        assert!(!navigator.turns[8].answer.contains("##"));
    });
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.ensure_panel(panes::Panel::History, window, cx)
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let home = home.read(cx);
        let navigator = nav.read(cx);
        assert!(home.pane_layout.history_docked);
        // The rail is a sibling: it neither widens nor recenters the transcript.
        assert!(
            (f32::from(navigator.viewport.left() - navigator.host.left())
                - home.pane_layout.body_origin)
                .abs()
                < 1.
        );
        assert!(
            (f32::from(navigator.viewport.size.width) - home.pane_layout.body_width).abs() < 1.
        );
        assert!(
            navigator.viewport.right()
                <= navigator.host.right() - px(home.pane_layout.history_width)
        );
        window.draw(cx).clear(cx);
        assert!(window.try_find("conversation-turn-a-a-user-8").is_some());
    });
}

#[gpui_kit::test]
fn focus_preview_closes_on_narrowing_and_session_switch_without_crossing_owners(
    cx: &mut TestAppContext,
) {
    let (home, visual, _directory) = mount(cx);
    let nav = visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nav = home.read(cx).views["a"].navigator.clone();
        window.click("conversation-turn-a-a-user-2", cx);
        nav
    });
    visual.run_until_parked();
    visual.simulate_keystrokes("tab");
    visual.run_until_parked();
    visual.update(|window, cx| {
        let navigator = nav.read(cx);
        assert_eq!(
            navigator.preview.as_deref(),
            Some("a-user-3"),
            "target={:?} focus={} enabled={} root_focus={} pending={} gutter={:?} actual={:?} focused_markers={:?}",
            navigator.target,
            navigator.markers["a-user-3"].focus.is_focused(window),
            navigator.enabled,
            navigator.focus.contains_focused(window, cx),
            navigator.pending.is_some(),
            navigator.gutter, window.focused(cx), navigator.markers.iter().filter(|(_, m)| m.focus.is_focused(window)).map(|(id,_)|id).collect::<Vec<_>>()
        );
        window.draw(cx).clear(cx);
        assert!(window.try_find("conversation-navigation-preview").is_some());
    });
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    visual.update(|_, cx| assert!(nav.read(cx).preview.is_none()));
    visual.update(|_, cx| {
        let scroller = nav.read(cx).scroller.clone();
        scroller.update(cx, |state, cx| {
            state.scroll_to_item(0, cx);
        });
    });
    visual.simulate_resize(size(px(650.), px(900.)));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("conversation-navigator").is_none());
        assert!(nav.read(cx).markers.is_empty());
        home.read(cx).messages_focus.clone().focus(window, cx);
    });
    visual.simulate_keystrokes("alt-down");
    visual.run_until_parked();
    visual.simulate_resize(size(px(1700.), px(900.)));
    visual.run_until_parked();
    let row = visual.update(|window, cx| {
        let row = nav.read(cx).scroller.read(cx).logical_scroll_top().item_ix;
        assert_eq!(row, nav.read(cx).turns[1].row);
        nav.read(cx).markers["a-user-2"]
            .focus
            .clone()
            .focus(window, cx);
        let state = home.read(cx).state.clone();
        state.update(cx, |state, _| *state.selected_for_test() = Some("b".into()));
        home.update(cx, |home, cx| home.sync(false, window, cx));
        let input = home.read(cx).views["b"].input.clone();
        input.update(cx, |input, cx| input.focus(window, cx));
        row
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert!(!nav.read(cx).enabled);
        assert!(nav.read(cx).preview.is_none());
        assert!(
            home.read(cx).views["b"]
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        let state = home.read(cx).state.clone();
        state.update(cx, |state, _| *state.selected_for_test() = Some("a".into()));
        home.update(cx, |home, cx| home.sync(false, window, cx));
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert_eq!(
            home.read(cx).views["a"].navigator.entity_id(),
            nav.entity_id()
        );
        assert_eq!(
            nav.read(cx).scroller.read(cx).logical_scroll_top().item_ix,
            row
        );
        assert!(
            home.read(cx).views["b"]
                .navigator
                .read(cx)
                .turns
                .iter()
                .all(|turn| turn.id.starts_with("b-"))
        );
    });
}
