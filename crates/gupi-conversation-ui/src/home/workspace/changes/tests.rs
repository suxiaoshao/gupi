use super::*;
use core::prelude::v1::test;
use gpui_kit::component::{Root, diff::DiffMode};
use gpui_kit::test::TestWindowExt as _;
use serde_json::json;

fn records(paths: &[&str], operations: usize) -> pi_rpc::protocol::Entries {
    let mut entries = Vec::new();
    let mut parent = None;
    for path in paths {
        for ix in 0..operations {
            let call = format!("{path}-{ix}");
            let patch = if ix == 4 {
                "unparseable recorded patch".repeat(30)
            } else {
                format!(
                    "--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-old\n+{}\n",
                    "long recorded line ".repeat(30)
                )
            };
            for value in [
                json!({"role":"assistant","timestamp":1,"stopReason":"toolUse","content":[{"type":"toolCall","id":call,"name":"edit","arguments":{"path":path}}]}),
                json!({"role":"toolResult","timestamp":2,"toolCallId":call,"toolName":"edit","details":{"patch":patch}}),
            ] {
                let id = format!("record-{}", entries.len());
                entries.push(serde_json::from_value(json!({"id":id,"parentId":parent,"timestamp":"2026-10-15T00:00:00Z","type":"message","message":value})).unwrap());
                parent = Some(id);
            }
        }
    }
    pi_rpc::protocol::Entries {
        entries,
        leaf_id: parent,
    }
}

fn mount(
    cx: &mut TestAppContext,
    operations: usize,
) -> (Entity<HomeView>, &mut VisualTestContext, tempfile::TempDir) {
    let fixture = tempfile::tempdir().unwrap();
    for path in ["a.rs", "b.rs"] {
        std::fs::write(fixture.path().join(path), "current disk content").unwrap();
    }
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::host::install_headless(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        cx.set_global(layout::LayoutState::default());
        cx.set_global(TestLayoutDirectory(fixture.path().into()));
    });
    let state = cx.new(|cx| ConversationState::new("unused".into(), cx));
    state.update(cx, |state, _| {
        let info = gupi_conversation::session_catalog::SessionInfo::new(
            Default::default(),
            "fixture".into(),
            fixture.path().into(),
        );
        let mut session = gupi_conversation::conversation::Session::new(info, String::new());
        session.replace_entries_for_test(records(&["a.rs", "b.rs"], operations));
        state.sessions_for_test().insert("fixture".into(), session);
        *state.selected_for_test() = Some("fixture".into());
    });
    let mut home = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| HomeView::with_state(state, window, cx));
        home = Some(view.clone());
        Root::new(view, window, cx)
    });
    visual.simulate_resize(size(px(1800.), px(900.)));
    visual.run_until_parked();
    let home = home.unwrap();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            let run = home.views["fixture"]
                .tools
                .owner("tool-a.rs-0")
                .unwrap()
                .to_owned();
            home.open_changes("fixture", &run, "a.rs", window, cx);
        })
    });
    visual.run_until_parked();
    (home, visual, fixture)
}

#[gpui_kit::test]
fn implicit_run_selection_never_keeps_another_files_external_path(cx: &mut TestAppContext) {
    let (home, visual, fixture) = mount(cx, 1);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            let tab = home
                .views
                .get_mut("fixture")
                .unwrap()
                .workspace
                .changes
                .first_mut()
                .unwrap();
            tab.external = Some(fixture.path().join("a.rs"));
            tab.filter
                .update(cx, |input, cx| input.set_value("b.rs", window, cx));
            home.filter_reader(window, cx);
            assert_eq!(
                home.active_changes().unwrap().selected.as_deref(),
                Some("b.rs")
            );
            assert!(home.active_changes().unwrap().external.is_none());
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            assert_eq!(
                home.active_changes().unwrap().external.as_deref(),
                Some(fixture.path().join("b.rs").as_path())
            );
            let tab = home
                .views
                .get_mut("fixture")
                .unwrap()
                .workspace
                .changes
                .first_mut()
                .unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("missing", window, cx));
            home.filter_reader(window, cx);
            assert!(home.active_changes().unwrap().selected.is_none());
            assert!(home.active_changes().unwrap().external.is_none());
            assert!(!home.reader_command_available(super::super::reader::Command::External, cx));
            let tab = home
                .views
                .get_mut("fixture")
                .unwrap()
                .workspace
                .changes
                .first_mut()
                .unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("", window, cx));
            home.filter_reader(window, cx);
            home.select_change_file("a.rs".into(), window, cx);
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            assert_eq!(
                home.active_changes().unwrap().external.as_deref(),
                Some(fixture.path().join("a.rs").as_path())
            );
            home.state.update(cx, |state, _| {
                state
                    .sessions_for_test()
                    .get_mut("fixture")
                    .unwrap()
                    .replace_entries_for_test(records(&["b.rs"], 1))
            });
            home.sync(true, window, cx);
            assert_eq!(
                home.active_changes().unwrap().selected.as_deref(),
                Some("b.rs")
            );
            assert!(home.active_changes().unwrap().external.is_none());
        })
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert_eq!(
            home.read(cx).active_changes().unwrap().external.as_deref(),
            Some(fixture.path().join("b.rs").as_path())
        )
    });
}

#[gpui_kit::test]
fn operation_paging_applies_retained_reader_options_immediately(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx, 5);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.pane_layout.main_width = 1200.;
            use super::super::reader::Command;
            for command in [
                Command::Split,
                Command::Wrap,
                Command::Word,
                Command::ToggleFile,
            ] {
                home.reader_command(command, window, cx);
            }
            // Context isn't available for noncomposable operations in the menu;
            // retain the preference as when switching from a composable file.
            home.views.get_mut("fixture").unwrap().workspace.changes[0]
                .options
                .expanded = true;
        });
        window.draw(cx).clear(cx);
        window.click("changes-next", cx);
        let tab = home.read(cx).active_changes().unwrap();
        let file = &tab.states["a.rs"];
        assert_eq!(file.page, 3);
        assert!(tab.options.wrap && tab.collapsed.contains("a.rs"));
        assert_eq!(file.operations.len(), 2);
        let state = file.operations["tool-a.rs-3"].state().unwrap().read(cx);
        assert_eq!(state.mode(), DiffMode::Split);
        assert_eq!(state.context_lines(), None);
        assert_eq!(state.inline_unit(), None);
        assert!(state.is_file_collapsed("a.rs"));
        assert!(file.operations["tool-a.rs-4"].state().is_none());
    });
    visual.run_until_parked();
}
