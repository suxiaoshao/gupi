use super::*;
use crate::home::TestLayoutDirectory;
use core::prelude::v1::test;
use gpui_kit::component::{Root, diff::DiffSide};
use gpui_kit::test::TestWindowExt as _;
use gupi_conversation::conversation::{Changes, ToolActivity, ToolExecution};
use gupi_conversation::session_catalog::SessionInfo;
use serde_json::{Value, json};
use std::sync::Arc;

fn message(id: &str, value: Value) -> DisplayMessage {
    DisplayMessage::new(id.into(), value)
}
fn fixture(files: usize, operations: usize, tail: usize) -> Vec<DisplayMessage> {
    let mut rows = vec![message(
        "user",
        json!({"role":"user","timestamp":0,"content":"Edit these files"}),
    )];
    let mut timestamp = 1;
    for file in 0..files {
        for operation in 0..operations {
            let id = format!("{file}-{operation}");
            let path = format!("dir/{file}.rs");
            let name = if operation == 0 { "write" } else { "edit" };
            let args = json!({"path":path,"content":"a long original line with tabs\t and trailing spaces  \n"});
            rows.push(message(&format!("call-{id}"), json!({"role":"assistant","timestamp":timestamp,"stopReason":"toolUse","content":[{"type":"toolCall","id":id,"name":name,"arguments":args}]})));
            timestamp += 1;
            let patch = if operation == 4 {
                "unparseable recorded patch".to_owned()
            } else {
                format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-old\n+new-{id}\n")
            };
            rows.push(message(&format!("result-{id}"), json!({"role":"toolResult","timestamp":timestamp,"toolCallId":id,"toolName":name,"isError":operation==2,"details":{"patch":patch}})));
            timestamp += 1;
        }
    }
    rows.push(message("answer",json!({"role":"assistant","timestamp":timestamp,"stopReason":"stop","content":"Finished editing."})));
    for index in 0..tail {
        rows.push(message(
            &format!("later-user-{index}"),
            json!({"role":"user","timestamp":timestamp+index*2+1,"content":"Next response"}),
        ));
        rows.push(message(&format!("later-answer-{index}"),json!({"role":"assistant","timestamp":timestamp+index*2+2,"stopReason":"stop","content":"Later answer paragraph.\n\n".repeat(16)})));
    }
    rows
}
fn entries(messages: &[DisplayMessage]) -> pi_rpc::protocol::Entries {
    let mut parent = None;
    let entries = messages.iter().map(|message| {
        let id = message.id.clone();
        let entry = serde_json::from_value(json!({"id":id,"parentId":parent,"timestamp":"2026-10-15T00:00:00Z","type":"message","message":message.value})).unwrap();
        parent = Some(id); entry
    }).collect();
    pi_rpc::protocol::Entries {
        entries,
        leaf_id: parent,
    }
}
fn session(key: &str, messages: &[DisplayMessage], live: bool) -> Session {
    let mut session = Session::new(
        SessionInfo::new(Default::default(), key.into(), "/repo".into()),
        String::new(),
    );
    session.replace_entries_for_test(if live {
        entries(&[])
    } else {
        entries(messages)
    });
    if live {
        for message in messages {
            session.receive_message_for_test("message_end", &json!({"message":message.value}));
            *session.content_revision_for_test() += 1;
        }
    }
    session
}
fn mount<'a>(
    cx: &'a mut TestAppContext,
    messages: &[DisplayMessage],
    live: bool,
) -> (
    Entity<HomeView>,
    Entity<ConversationState>,
    &'a mut VisualTestContext,
) {
    mount_mode(cx, messages, live, true)
}
fn mount_mode<'a>(
    cx: &'a mut TestAppContext,
    messages: &[DisplayMessage],
    live: bool,
    temporary: bool,
) -> (
    Entity<HomeView>,
    Entity<ConversationState>,
    &'a mut VisualTestContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::host::install_headless(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gpui_tokio::init(cx);
        gupi_pi_runtime::init(cx);
        cx.set_global(layout::LayoutState::default());
        let directory = std::env::temp_dir().join(format!(
            "gupi-card-tests-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        cx.set_global(TestLayoutDirectory(directory));
    });
    let state = cx.new(|cx| {
        if temporary {
            ConversationState::temporary("unused".into(), cx)
        } else {
            ConversationState::new("unused".into(), cx)
        }
    });
    state.update(cx, |state, _| {
        state
            .sessions_for_test()
            .insert("a".into(), session("a", messages, live));
        *state.selected_for_test() = Some("a".into());
    });
    let mut home = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| HomeView::with_state(state.clone(), window, cx));
        home = Some(view.clone());
        Root::new(view, window, cx)
    });
    visual.simulate_resize(size(px(1200.), px(900.)));
    visual.run_until_parked();
    (home.unwrap(), state, visual)
}
fn run(home: &Entity<HomeView>, cx: &App) -> String {
    home.read(cx).views["a"]
        .changes
        .runs
        .keys()
        .next()
        .unwrap()
        .clone()
}
fn disclose(home: &Entity<HomeView>, visual: &mut VisualTestContext, action: Disclosure) {
    visual.update(|window, cx| {
        let run = run(home, cx);
        home.update(cx, |home, cx| {
            home.toggle_changes("a", &run, &action, window, cx)
        });
    });
    visual.run_until_parked();
}
fn emit(
    state: &Entity<ConversationState>,
    visual: &mut VisualTestContext,
    change: impl FnOnce(&mut Session),
) {
    visual.update(|_, cx| {
        state.update(cx, |state, cx| {
            change(state.sessions_for_test().get_mut("a").unwrap());
            let mut changes = Changes::default();
            changes.bodies.insert("a".into());
            cx.emit(ConversationEvent::Changed(changes));
        })
    });
    visual.run_until_parked();
}
fn receive(
    state: &Entity<ConversationState>,
    visual: &mut VisualTestContext,
    kind: &str,
    raw: Value,
) {
    emit(state, visual, |session| {
        session.receive_message_for_test(kind, &raw);
        *session.content_revision_for_test() += 1;
    });
}

#[gpui_kit::test]
fn bounded_pages_copy_focus_narrow_layout_and_virtual_unmount(cx: &mut TestAppContext) {
    let (home, _, visual) = mount(cx, &fixture(5, 6, 12), false);
    visual.update(|_, cx| {
        let scroller = home.read(cx).views["a"].scroller.clone();
        scroller.update(cx, |state, cx| {
            state.scroll_to_item(1, cx);
        });
    });
    visual.run_until_parked();
    // Keyboard enters the first file through the retained row focus.
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let focus = home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())]
            .focus
            .clone();
        focus.focus(window, cx);
    });
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    disclose(&home, visual, Disclosure::File("dir/1.rs".into()));
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        let run = run(&home, cx);
        assert!(view.changes.files[&(run.clone(), "dir/0.rs".into())].open);
        assert!(view.changes.files[&(run.clone(), "dir/1.rs".into())].open);
        assert_eq!(view.changes.blocks.len(), 6);
        assert!(!view.changes.blocks.contains_key("tool-0-3"));
        assert!(view.edited_files.stats(&run).is_none());
        assert!(!view.scroller.read(cx).is_following_tail());
        let file = &view.edited_files.files(&run)[0];
        assert!(file.written() && file.failed());
        assert!(file.operations()[4].stats().is_none());
    });
    disclose(&home, visual, Disclosure::Next("dir/0.rs".into()));
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        let run = run(&home, cx);
        assert_eq!(view.changes.files[&(run, "dir/0.rs".into())].page, 3);
        assert_eq!(view.changes.blocks.len(), 9);
    });
    disclose(&home, visual, Disclosure::Previous("dir/0.rs".into()));
    // Unified Diff: real gutter selection and the window-scoped clipboard action.
    let (entity, selection) = visual.update(|window, cx| {
        let state = home.read(cx).views["a"].changes.blocks["tool-0-1"]
            .state()
            .unwrap()
            .clone();
        window.render_frame(cx);
        window
            .within("tool-0-1")
            .within(("new", 0usize))
            .click(("line", 0usize), cx);
        window.within("tool-0-1").press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("new-0-1\n")
        );
        assert_eq!(
            state.read(cx).mode(),
            gpui_kit::component::diff::DiffMode::Unified
        );
        assert_eq!(
            state.read(cx).selected_lines().unwrap().side(),
            DiffSide::Modified
        );
        (state.entity_id(), state.read(cx).selected_lines())
    });
    // Collapse by the production action while focus is inside the code body.
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    visual.update(|window, cx| {
        let run = run(&home, cx);
        assert!(
            home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())]
                .focus
                .is_focused(window)
        );
    });
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    disclose(&home, visual, Disclosure::Files);
    disclose(&home, visual, Disclosure::File("dir/4.rs".into()));
    visual.update(|window, cx| {
        let view = &home.read(cx).views["a"];
        view.changes.blocks["tool-4-1"]
            .focus_handle(cx)
            .unwrap()
            .focus(window, cx);
    });
    disclose(&home, visual, Disclosure::Files);
    visual.update(|window, cx| {
        let run = run(&home, cx);
        assert!(
            home.read(cx).views["a"].changes.runs[&run]
                .focus
                .is_focused(window)
        );
    });
    // Switching to another viewer transfers the window's one active selection.
    // Re-select here before checking resize and virtual unmount (no user selection change).
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    visual.update(|window, cx| {
        window.render_frame(cx);
        window
            .within("tool-0-1")
            .within(("new", 0usize))
            .click(("line", 0usize), cx);
        let state = home.read(cx).views["a"].changes.blocks["tool-0-1"]
            .state()
            .unwrap();
        assert_eq!(state.read(cx).selected_lines(), selection);
    });
    visual.simulate_resize(size(px(600.), px(900.)));
    visual.run_until_parked();
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        let state = view.changes.blocks["tool-0-1"].state().unwrap();
        assert_eq!(state.entity_id(), entity);
        assert_eq!(state.read(cx).selected_lines(), selection);
        let scroller = view.scroller.clone();
        scroller.update(cx, |state, cx| {
            state.scroll_to_item(state.item_count() - 1, cx);
        });
    });
    visual.run_until_parked();
    assert!(visual.debug_bounds("change-file-row-dir/0.rs").is_none());
    visual.update(|_, cx| {
        let scroller = home.read(cx).views["a"].scroller.clone();
        scroller.update(cx, |state, cx| {
            state.scroll_to_item(1, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let view = &home.read(cx).views["a"];
        let block = &view.changes.blocks["tool-0-1"];
        assert_eq!(block.state().unwrap().entity_id(), entity);
        assert_eq!(block.saved_selection(), selection);
        let focus = block.focus_handle(cx).unwrap();
        focus.focus(window, cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let state = home.read(cx).views["a"].changes.blocks["tool-0-1"]
            .state()
            .unwrap();
        assert_eq!(state.read(cx).selected_lines(), selection);
        window.within("tool-0-1").press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("new-0-1\n")
        );
    });
}

#[gpui_kit::test]
fn streaming_cache_persistence_session_switch_and_branch_reset(cx: &mut TestAppContext) {
    let source = fixture(4, 4, 0);
    let (home, state, visual) = mount(cx, &source, true);
    disclose(&home, visual, Disclosure::Files);
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    disclose(&home, visual, Disclosure::Next("dir/0.rs".into()));
    receive(
        &state,
        visual,
        "message_start",
        json!({"message":{"role":"assistant","timestamp":1000,"content":[{"type":"text","text":""}]}}),
    );
    let (tool, entity, focus) = visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        let run = run(&home, cx);
        (
            view.tools.shared_tool("tool-0-1").unwrap().clone(),
            view.changes.blocks["tool-0-3"].state_id(),
            view.changes.files[&(run, "dir/0.rs".into())].focus.clone(),
        )
    });
    for _ in 0..20 {
        receive(
            &state,
            visual,
            "message_update",
            json!({"assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"stream"}}),
        );
        visual.update(|_, cx| {
            assert!(Rc::ptr_eq(
                &tool,
                home.read(cx).views["a"]
                    .tools
                    .shared_tool("tool-0-1")
                    .unwrap()
            ))
        });
    }
    receive(
        &state,
        visual,
        "message_update",
        json!({"assistantMessageEvent":{"type":"toolcall_start","contentIndex":1,"id":"stream-write","toolName":"write"}}),
    );
    let args = json!({"path":"new.txt","content":"line\n".repeat(300)}).to_string();
    for chunk in args.as_bytes().chunks(80) {
        receive(
            &state,
            visual,
            "message_update",
            json!({"assistantMessageEvent":{"type":"toolcall_delta","contentIndex":1,"delta":std::str::from_utf8(chunk).unwrap()}}),
        );
        visual.update(|_, cx| {
            let view = &home.read(cx).views["a"];
            assert!(Rc::ptr_eq(
                &tool,
                view.tools.shared_tool("tool-0-1").unwrap()
            ));
            assert_eq!(view.changes.blocks["tool-0-3"].state_id(), entity);
        });
    }
    emit(&state, visual, |session| {
        session.replace_entries_for_test(entries(&source));
        session.live_for_test().clear();
        *session.content_revision_for_test() += 1;
    });
    visual.update(|window, cx| {
        let view = &home.read(cx).views["a"];
        let run = run(&home, cx);
        let file = &view.changes.files[&(run.clone(), "dir/0.rs".into())];
        assert!(file.open);
        assert_eq!(file.page, 3);
        assert_eq!(file.focus, focus);
        assert!(view.changes.runs[&run].all);
        assert_eq!(view.changes.blocks["tool-0-3"].state_id(), entity);
        let focus = file.focus.clone();
        focus.focus(window, cx);
    });
    visual.simulate_keystrokes("space");
    visual.run_until_parked();
    visual.update(|_, cx| {
        let run = run(&home, cx);
        assert!(!home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())].open);
    });
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    visual.update(|window, cx| {
        state.update(cx, |state, _| {
            state
                .sessions_for_test()
                .insert("b".into(), session("b", &fixture(1, 1, 0), false));
            *state.selected_for_test() = Some("b".into());
        });
        home.update(cx, |home, cx| home.sync(true, window, cx));
        state.update(cx, |state, _| *state.selected_for_test() = Some("a".into()));
        home.update(cx, |home, cx| home.sync(true, window, cx));
        let run = run(&home, cx);
        assert!(home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())].open);
    });
    visual.run_until_parked();
    // Descendant growth retains state; replacing it by a sibling does not.
    let opening = visual.update(|_, cx| {
        state.read(cx).sessions()["a"]
            .history()
            .leaf
            .clone()
            .unwrap()
    });
    emit(&state, visual, |session| {
        let mut entries = session.history().entries.clone();
        entries.push(serde_json::from_value(json!({"id":"growth","parentId":opening,"type":"label","timestamp":"2026-10-15T00:00:00Z","label":"grow"})).unwrap());
        session.replace_entries_for_test(pi_rpc::protocol::Entries {
            entries,
            leaf_id: Some("growth".into()),
        });
        *session.content_revision_for_test() += 1;
    });
    visual.update(|_, cx| {
        let run = run(&home, cx);
        assert!(home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())].open);
    });
    emit(&state, visual, |session| {
        let mut entries = session.history().entries.clone();
        entries.push(serde_json::from_value(json!({"id":"sibling","parentId":opening,"type":"label","timestamp":"2026-10-15T00:00:00Z","label":"other"})).unwrap());
        session.replace_entries_for_test(pi_rpc::protocol::Entries {
            entries,
            leaf_id: Some("sibling".into()),
        });
        *session.content_revision_for_test() += 1;
    });
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        assert!(view.changes.blocks.is_empty());
        assert!(view.changes.files.values().all(|state| !state.open));
    });
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.views.get_mut("a").unwrap().preview = Some(opening.clone());
            home.sync_messages("a", true, window, cx);
        })
    });
    visual.run_until_parked();
    visual.update(|_, cx| assert!(home.read(cx).views["a"].changes.blocks.is_empty()));
}

#[gpui_kit::test]
fn live_growth_preserves_reading_anchor_and_tail_following(cx: &mut TestAppContext) {
    let mut source = fixture(2, 1, 12);
    source.insert(5,message("pending",json!({"role":"assistant","timestamp":900,"content":[{"type":"toolCall","id":"later-a","name":"edit","arguments":{"path":"dir/0.rs"}}]})));
    let (home, state, visual) = mount(cx, &source, false);
    visual.update(|_, cx| {
        let scroller = home.read(cx).views["a"].scroller.clone();
        scroller.update(cx, |state, cx| {
            state.scroll_to_item(1, cx);
        });
    });
    visual.run_until_parked();
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    disclose(&home, visual, Disclosure::File("dir/1.rs".into()));
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let focus = home.read(cx).views["a"].changes.files[&(run, "dir/1.rs".into())]
            .focus
            .clone();
        focus.focus(window, cx);
        window.render_frame(cx);
    });
    let before = visual
        .debug_bounds("change-file-row-dir/1.rs")
        .unwrap()
        .top();
    emit(&state, visual, |session| {
        session.tools_for_test().push(Arc::new(ToolActivity::new("later-a".into(),"edit".into(),Value::Null,ToolExecution::Complete(json!({"details":{"patch":"--- a/dir/0.rs\n+++ b/dir/0.rs\n@@ -1 +1 @@\n-old\n+new\n"}})))))
    });
    let after = visual
        .debug_bounds("change-file-row-dir/1.rs")
        .unwrap()
        .top();
    assert!(
        (after - before).abs() < px(1.),
        "anchor moved: {before:?} -> {after:?}"
    );
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        assert!(!view.scroller.read(cx).is_following_tail());
        assert_eq!(view.changes.blocks.len(), 2);
        let scroller = view.scroller.clone();
        scroller.update(cx, |state, cx| state.scroll_to_end(cx));
    });
    visual.run_until_parked();
    receive(
        &state,
        visual,
        "message_start",
        json!({"message":{"role":"assistant","timestamp":2000,"content":[{"type":"text","text":""}]}}),
    );
    receive(
        &state,
        visual,
        "message_update",
        json!({"assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"new tail content"}}),
    );
    visual.update(|_, cx| {
        assert!(
            home.read(cx).views["a"]
                .scroller
                .read(cx)
                .is_following_tail()
        )
    });
}

#[gpui_kit::test]
fn disclosure_from_tail_keeps_the_clicked_row_visible(cx: &mut TestAppContext) {
    let (home, _, visual) = mount(cx, &fixture(1, 6, 0), false);
    visual.update(|_, cx| {
        assert!(
            home.read(cx).views["a"]
                .scroller
                .read(cx)
                .is_following_tail()
        );
    });
    let row = visual.debug_bounds("change-file-row-dir/0.rs").unwrap();
    visual.simulate_click(row.center(), Modifiers::default());
    visual.run_until_parked();
    // A later frame must not undo the disclosure's reveal by following the tail.
    visual.update(|window, cx| window.render_frame(cx));
    visual.run_until_parked();
    let row = visual.debug_bounds("change-file-row-dir/0.rs").unwrap();
    visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        let viewport = view.changes.viewport.get().unwrap();
        assert!(!view.scroller.read(cx).is_following_tail());
        assert!(row.top() >= viewport.top(), "{row:?} outside {viewport:?}");
        assert!(
            row.bottom() <= viewport.bottom(),
            "{row:?} outside {viewport:?}"
        );
    });
}

#[gpui_kit::test]
fn pointer_disclosure_keeps_keyboard_activation(cx: &mut TestAppContext) {
    let (home, _, visual) = mount(cx, &fixture(1, 1, 0), false);
    let row = visual.debug_bounds("change-file-row-dir/0.rs").unwrap();
    visual.simulate_click(row.center(), Modifiers::default());
    visual.run_until_parked();
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let file = &home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())];
        assert!(file.open);
        assert!(file.focus.is_focused(window));
    });
    visual.simulate_keystrokes("space");
    visual.run_until_parked();
    visual.update(|_, cx| {
        let run = run(&home, cx);
        assert!(!home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())].open);
    });
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    visual.update(|_, cx| {
        let run = run(&home, cx);
        assert!(home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())].open);
    });
}

fn net_fixture(files: usize) -> Vec<DisplayMessage> {
    let mut source = fixture(files, 1, 0);
    for message in &mut source {
        if let Some(parts) = message.value["content"].as_array_mut() {
            for part in parts {
                if part["type"] == "toolCall" {
                    part["name"] = json!("edit");
                }
            }
        }
        if message.value["role"] == "toolResult" {
            message.value["toolName"] = json!("edit");
        }
    }
    source
}
fn append_edit(session: &mut Session, id: &str, path: &str, old: &str, new: &str, timestamp: u64) {
    let patch = format!("--- {path}\n+++ {path}\n@@ -1 +1 @@\n-{old}\n+{new}\n");
    for message in [
        json!({"role":"assistant","timestamp":timestamp,"stopReason":"toolUse","content":[{"type":"toolCall","id":id,"name":"edit","arguments":{"path":path}}]}),
        json!({"role":"toolResult","timestamp":timestamp+1,"toolCallId":id,"toolName":"edit","details":{"patch":patch}}),
    ] {
        session.receive_message_for_test("message_end", &json!({"message":message}));
        *session.content_revision_for_test() += 1;
    }
}

#[gpui_kit::test]
fn composed_files_retain_entities_selection_and_persisted_ownership(cx: &mut TestAppContext) {
    let source = net_fixture(2);
    let (home, state, visual) = mount(cx, &source, true);
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    disclose(&home, visual, Disclosure::File("dir/1.rs".into()));
    let (entity, body, focus, selected) = visual.update(|window, cx| {
        let run = run(&home, cx);
        window.render_frame(cx);
        window
            .within("combined-dir/0.rs")
            .within(("new", 0usize))
            .click(("line", 0usize), cx);
        window.within("combined-dir/0.rs").press("secondary-c", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("new-0-0\n")
        );
        let view = &home.read(cx).views["a"];
        assert!(view.changes.blocks.is_empty());
        let file = &view.changes.files[&(run.clone(), "dir/0.rs".into())];
        let block = file.combined.as_ref().unwrap();
        assert_ne!(
            block.state_id(),
            view.changes.files[&(run.clone(), "dir/1.rs".into())]
                .combined
                .as_ref()
                .unwrap()
                .state_id()
        );
        (
            block.state_id(),
            view.edited_files.files(&run)[0].combined().unwrap().clone(),
            file.focus.clone(),
            block.state().unwrap().read(cx).selected_lines(),
        )
    });
    // Both operations arrive in one data update. Their final net result is unchanged.
    emit(&state, visual, |session| {
        append_edit(
            session,
            "temporary",
            "dir/0.rs",
            "new-0-0",
            "temporary",
            2000,
        );
        append_edit(session, "restore", "dir/0.rs", "temporary", "new-0-0", 2002);
    });
    visual.update(|_, cx| {
        let run = run(&home, cx);
        let view = &home.read(cx).views["a"];
        let block = view.changes.files[&(run.clone(), "dir/0.rs".into())]
            .combined
            .as_ref()
            .unwrap();
        assert_eq!(block.state_id(), entity);
        assert_eq!(block.state().unwrap().read(cx).selected_lines(), selected);
        assert!(Rc::ptr_eq(
            &body,
            view.edited_files.files(&run)[0].combined().unwrap()
        ));
        let stats = view.edited_files.stats(&run).unwrap();
        assert_eq!((stats.added(), stats.removed()), (2, 2));
    });
    emit(&state, visual, |session| {
        append_edit(session, "final", "dir/0.rs", "new-0-0", "final", 2010);
    });
    visual.update(|_, cx| {
        let run = run(&home, cx);
        let block = home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())]
            .combined
            .as_ref()
            .unwrap();
        assert_eq!(block.state_id(), entity); // content updates set_files, not the Entity
    });
    emit(&state, visual, |session| {
        let messages = session.messages(None);
        session.replace_entries_for_test(entries(&messages));
        session.live_for_test().clear();
        *session.content_revision_for_test() += 1;
    });
    visual.update(|_, cx| {
        let run = run(&home, cx);
        let file = &home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())];
        assert!(file.open);
        assert_eq!(file.focus, focus);
        assert_eq!(file.combined.as_ref().unwrap().state_id(), entity);
    });
}

#[gpui_kit::test]
fn cancelled_files_restore_only_the_removed_focus_and_hide_the_empty_card(cx: &mut TestAppContext) {
    let (home, state, visual) = mount(cx, &net_fixture(3), true);
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    visual.update(|window, cx| {
        let run = run(&home, cx);
        home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())]
            .combined
            .as_ref()
            .unwrap()
            .focus_handle(cx)
            .unwrap()
            .focus(window, cx);
    });
    emit(&state, visual, |session| {
        append_edit(session, "cancel-0", "dir/0.rs", "new-0-0", "old", 3000)
    });
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let view = &home.read(cx).views["a"];
        assert_eq!(view.edited_files.files(&run).len(), 2);
        assert!(
            view.changes.files[&(run, "dir/1.rs".into())]
                .focus
                .is_focused(window)
        );
    });
    visual.update(|window, cx| home.update(cx, |home, cx| home.focus_composer(window, cx)));
    emit(&state, visual, |session| {
        append_edit(session, "cancel-1", "dir/1.rs", "new-1-0", "old", 3002)
    });
    visual.update(|window, cx| {
        assert!(
            home.read(cx)
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        )
    });
    disclose(&home, visual, Disclosure::File("dir/2.rs".into()));
    visual.update(|window, cx| {
        let run = run(&home, cx);
        home.read(cx).views["a"].changes.files[&(run, "dir/2.rs".into())]
            .combined
            .as_ref()
            .unwrap()
            .focus_handle(cx)
            .unwrap()
            .focus(window, cx);
    });
    emit(&state, visual, |session| {
        append_edit(session, "cancel-2", "dir/2.rs", "new-2-0", "old", 3004)
    });
    visual.update(|window, cx| {
        assert!(home.read(cx).views["a"].changes.runs.is_empty());
        assert!(
            home.read(cx)
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    assert!(visual.debug_bounds("change-file-row-dir/2.rs").is_none());
}

#[gpui_kit::test]
fn composed_fallback_write_recovery_preserves_file_entity_and_restores_focus(
    cx: &mut TestAppContext,
) {
    let (home, state, visual) = mount(cx, &net_fixture(1), true);
    disclose(&home, visual, Disclosure::File("dir/0.rs".into()));
    let entity = visual.update(|window, cx| {
        let run = run(&home, cx);
        let block = home.read(cx).views["a"].changes.files[&(run, "dir/0.rs".into())]
            .combined
            .as_ref()
            .unwrap();
        let focus = block.focus_handle(cx).unwrap();
        let entity = block.state_id();
        focus.focus(window, cx);
        entity
    });
    emit(&state, visual, |session| {
        append_edit(
            session,
            "inconsistent",
            "dir/0.rs",
            "external",
            "after",
            4000,
        )
    });
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let view = &home.read(cx).views["a"];
        assert!(view.edited_files.files(&run)[0].combined().is_none());
        assert!(
            view.changes.files[&(run.clone(), "dir/0.rs".into())]
                .focus
                .is_focused(window)
        );
        assert!(view.edited_files.stats(&run).is_none());
        view.changes.blocks["tool-0-0"]
            .focus_handle(cx)
            .unwrap()
            .focus(window, cx);
    });
    emit(&state, visual, |session| {
        for message in [
            json!({"role":"assistant","timestamp":4010,"stopReason":"toolUse","content":[{"type":"toolCall","id":"recovery","name":"write","arguments":{"path":"dir/0.rs","content":"recovered\n"}}]}),
            json!({"role":"toolResult","timestamp":4011,"toolCallId":"recovery","toolName":"write","content":[]}),
        ] {
            session.receive_message_for_test("message_end", &json!({"message":message}));
            *session.content_revision_for_test() += 1;
        }
    });
    visual.update(|window, cx| {
        let run = run(&home, cx);
        let view = &home.read(cx).views["a"];
        let file = &view.changes.files[&(run.clone(), "dir/0.rs".into())];
        assert!(file.focus.is_focused(window));
        assert_eq!(file.combined.as_ref().unwrap().state_id(), entity);
        assert!(matches!(view.edited_files.files(&run)[0].combined().unwrap().as_ref(), ChangeBody::Written { content } if content == "recovered\n"));
        assert!(view.edited_files.stats(&run).is_none());
    });
}

#[gpui_kit::test]
fn home_preview_switches_keep_current_path_live_tools(cx: &mut TestAppContext) {
    let messages = vec![
        message(
            "call",
            json!({"role":"assistant","content":[{"type":"toolCall","id":"one","name":"write","arguments":{"path":"a.rs","content":"new"}}]}),
        ),
        message("current", json!({"role":"user","content":"steer"})),
    ];
    let (home, state, visual) = mount(cx, &messages, false);
    state.update(visual, |state, _| {
        let session = state.sessions_for_test().get_mut("a").unwrap();
        let mut history = entries(&messages);
        history.entries.push(serde_json::from_value(json!({"id":"sibling","parentId":"call","timestamp":"2026-10-15T00:00:00Z","type":"message","message":{"role":"user","content":"other branch"}})).unwrap());
        session.replace_entries_for_test(history);
        session.tools_for_test().push(Arc::new(ToolActivity::new(
            "one".into(), "write".into(), json!({}),
            ToolExecution::Complete(json!({"content":"live output"})),
        )));
    });
    for preview in [
        None,
        Some("call"),
        Some("current"),
        Some("sibling"),
        Some("call"),
    ] {
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.views.get_mut("a").unwrap().preview = preview.map(str::to_owned);
                home.sync_messages("a", true, window, cx);
                let view = &home.views["a"];
                let tool = view.tools.tool("tool-one").unwrap();
                assert_eq!(
                    tool.result["content"].as_str(),
                    (preview != Some("sibling")).then_some("live output")
                );
                assert_eq!(view.tool_projection.as_ref().unwrap().1.as_deref(), preview);
            })
        });
    }
}

#[gpui_kit::test]
fn main_card_view_changes_reuses_run_and_selects_first_file(cx: &mut TestAppContext) {
    let (home, _, visual) = mount_mode(cx, &fixture(2, 1, 0), false, false);
    let row = visual.debug_bounds("change-file-row-dir/1.rs").unwrap();
    visual.simulate_click(row.center(), Modifiers::default());
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| home.close_navigator(window, cx));
    });
    visual.run_until_parked();
    let button = visual.debug_bounds("view-run-changes").unwrap();
    visual.simulate_click(button.center(), Modifiers::default());
    visual.run_until_parked();
    visual.update(|_, cx| {
        let home = home.read(cx);
        let view = &home.views["a"];
        assert_eq!(view.workspace.changes.len(), 1);
        assert_eq!(
            view.workspace.changes[0].selected.as_deref(),
            Some("dir/0.rs")
        );
        assert!(
            home.pane_layout
                .visible(crate::home::panes::Panel::Workspace)
        );
        assert!(
            !view.changes.files[&(view.workspace.changes[0].run.clone(), "dir/0.rs".into())].open
        );
    });
}

#[gpui_kit::test]
fn main_card_opens_retained_tab_and_persistence_scope_and_net_zero_preserve_identity(
    cx: &mut TestAppContext,
) {
    let source = net_fixture(1);
    let (home, state, visual) = mount_mode(cx, &source, true, false);
    let row = visual.debug_bounds("change-file-row-dir/0.rs").unwrap();
    visual.simulate_click(row.center(), Modifiers::default());
    visual.run_until_parked();
    let (identity, original_run) = visual.update(|_, cx| {
        let view = &home.read(cx).views["a"];
        assert_eq!(view.workspace.changes.len(), 1);
        let tab = &view.workspace.changes[0];
        assert_eq!(view.workspace.active.as_ref(), Some(&tab.id));
        assert_eq!(tab.selected.as_deref(), Some("dir/0.rs"));
        assert!(!view.changes.files[&(tab.run.clone(), "dir/0.rs".into())].open);
        (tab.id.clone(), tab.run.clone())
    });
    emit(&state, visual, |session| {
        session.replace_entries_for_test(entries(&source));
        session.live_for_test().clear();
        *session.content_revision_for_test() += 1;
    });
    visual.update(|window, cx| {
        let view = &home.read(cx).views["a"];
        assert_eq!(view.workspace.changes.len(), 1);
        assert_eq!(view.workspace.changes[0].id, identity);
        assert_ne!(view.workspace.changes[0].run, original_run);
        let run = view.workspace.changes[0].run.clone();
        home.update(cx, |home, cx| {
            home.open_changes("a", &run, "dir/0.rs", window, cx)
        });
        assert_eq!(home.read(cx).views["a"].workspace.changes.len(), 1);
    });
    // A sibling branch excludes this response but must retain its tab.
    emit(&state, visual, |session| {
        let mut records = entries(&source);
        records.entries.push(serde_json::from_value(json!({"id":"sibling", "parentId":"user", "timestamp":"2026-10-15T00:00:00Z", "type":"message", "message":{"role":"user", "content":"Other branch"}})).unwrap());
        session.replace_entries_for_test(records);
    });
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.views.get_mut("a").unwrap().preview = Some("sibling".into());
            home.sync_messages("a", true, window, cx);
            assert!(!home.views["a"].workspace.changes[0].in_scope);
            home.views.get_mut("a").unwrap().preview = None;
            home.sync_messages("a", true, window, cx);
            assert!(home.views["a"].workspace.changes[0].in_scope);
        })
    });
    emit(&state, visual, |session| {
        append_edit(session, "cancel", "dir/0.rs", "new-0-0", "old", 2000)
    });
    visual.update(|_, cx| {
        let tab = &home.read(cx).views["a"].workspace.changes[0];
        assert_eq!(tab.id, identity);
        assert!(tab.in_scope && tab.files.is_empty());
        assert!(tab.selected.is_none());
    });
    emit(&state, visual, |session| {
        session.replace_entries_for_test(entries(&[]));
        session.live_for_test().clear();
        *session.content_revision_for_test() += 1;
    });
    visual.update(|_, cx| assert!(home.read(cx).views["a"].workspace.changes.is_empty()));
}

#[gpui_kit::test]
fn multiple_runs_keep_tabs_and_closing_main_only_tab_promotes_files_to_overlay(
    cx: &mut TestAppContext,
) {
    let mut source = fixture(1, 1, 0);
    let second = fixture(1, 1, 0).into_iter().map(|mut message| {
        message.id = format!("second-{}", message.id);
        message.value =
            serde_json::from_str(&message.value.to_string().replace("0-0", "second-operation"))
                .unwrap();
        message
    });
    source.extend(second);
    let (home, _, visual) = mount_mode(cx, &source, false, false);
    visual.simulate_resize(size(px(1600.), px(800.)));
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            let runs: Vec<_> = home.views["a"]
                .edited_files
                .runs()
                .map(|(run, _)| run.to_owned())
                .collect();
            assert_eq!(runs.len(), 2);
            for run in runs {
                home.open_changes("a", &run, "dir/0.rs", window, cx);
            }
            assert_eq!(home.views["a"].workspace.changes.len(), 2);
            home.open_source("/repo/example.rs".into(), false, window, cx);
            let id = home.views["a"].workspace.changes[0].id.clone();
            home.views.get_mut("a").unwrap().workspace.changes[0].navigator_hidden = true;
            home.select_workspace_tab(Some(id), window, cx);
        })
    });
    visual.simulate_resize(size(px(1150.), px(800.)));
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.fit_layout(window, cx);
            assert!(home.pane_layout.workspace_docked);
            let id = home.views["a"].workspace.changes[0].id.clone();
            home.views["a"].workspace.changes[0]
                .header_focus
                .focus(window, cx);
            home.close_workspace_tabs(&id, 0, window, cx);
            assert!(home.views["a"].files.read(cx).is_focused(window, cx));
            assert!(home.views["a"].workspace.active.is_none());
            assert_eq!(
                home.pane_layout.overlay,
                Some(crate::home::panes::Panel::Workspace)
            );
            assert!(home.pane_layout.right > 0.);
            assert_eq!(home.views["a"].workspace.changes.len(), 1);
        })
    });
}
