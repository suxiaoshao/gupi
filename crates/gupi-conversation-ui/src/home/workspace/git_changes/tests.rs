use super::*;
use core::prelude::v1::test;
use gpui_kit::component::{
    Root,
    diff::{DiffLineRange, DiffMode, DiffSide},
};
use gpui_kit::test::TestWindowExt;

fn mount(cx: &mut TestAppContext) -> (Entity<HomeView>, &mut VisualTestContext, tempfile::TempDir) {
    let fixture = tempfile::tempdir().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::host::install_headless(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        cx.set_global(layout::LayoutState::default());
        cx.set_global(TestLayoutDirectory(fixture.path().to_owned()));
    });
    let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
    state.update(cx, |state, _| {
        let info = gupi_conversation::session_catalog::SessionInfo::new(
            Default::default(),
            "fixture".into(),
            fixture.path().into(),
        );
        state.sessions_for_test().insert(
            "fixture".into(),
            gupi_conversation::conversation::Session::new(info, String::new()),
        );
        *state.selected_for_test() = Some("fixture".into());
    });
    let mut home = None;
    let (_, visual) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| HomeView::with_state(state, window, cx));
        home = Some(view.clone());
        Root::new(view, window, cx)
    });
    visual.simulate_resize(size(px(1600.), px(820.)));
    visual.update(|window, _| window.activate_window());
    visual.run_until_parked();
    let home = home.unwrap();
    visual.update(|window, cx| home.update(cx, |home, cx| home.open_git_changes(window, cx)));
    visual.run_until_parked();
    (home, visual, fixture)
}
fn ready(home: &mut HomeView, window: &mut Window, cx: &mut Context<HomeView>) {
    let tab = home.git_mut().unwrap();
    tab.loading = false;
    tab.all = ["a.rs", "b.rs", "dir/c.rs"]
        .into_iter()
        .map(|p| Change::for_test(p.into(), ChangeKind::Modified))
        .collect();
    for entry in &tab.all {
        let path = entry.path().to_str().unwrap();
        let patch = format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-old\n+new\n");
        let body = ChangeContent::for_test("old\n".into(), "new\n".into(), patch.clone());
        tab.cache.insert(
            identity(tab.source, entry.path()),
            Loaded {
                entry: entry.clone(),
                content: body,
                block: Some(DiffBlock::patch(&patch, cx)),
            },
        );
    }
    tab.sync_rows(window, cx);
    home.apply_reader_options(cx);
    cx.notify();
}
#[gpui_kit::test]
fn worktree_loading_filter_navigation_and_source_switch_are_coherent(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        assert!(home.read(cx).active_git().unwrap().loading);
        assert!(home.read(cx).active_git().unwrap().list_task.is_none());
        window.draw(cx).clear(cx);
        assert_eq!(window.find("diff-next-file").disabled(), Some(true));
        home.update(cx, |home, cx| ready(home, window, cx));
    });
    // A single normal results draw is enough: model, selection and toolbar agree.
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_ne!(window.find("diff-next-file").disabled(), Some(true));
        assert_eq!(window.find("diff-previous-file").disabled(), Some(true));
        window.click("diff-next-file", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert_eq!(
            home.read(cx).active_git().unwrap().selected.as_deref(),
            Some(identity(ChangeSource::Unstaged, std::path::Path::new("b.rs")).as_str())
        );
        let filter = home.read(cx).active_git().unwrap().filter.clone();
        filter.read(cx).focus_handle(cx).focus(window, cx);
    });
    visual.simulate_input("dir/");
    visual.run_until_parked();
    visual.update(|window, cx| {
        let tab = home.read(cx).active_git().unwrap();
        assert_eq!(tab.rows.len(), 1);
        assert_eq!(
            tab.selected.as_ref(),
            Some(&identity(
                ChangeSource::Unstaged,
                std::path::Path::new("dir/c.rs")
            ))
        );
        window.draw(cx).clear(cx);
        assert_eq!(window.find("diff-next-file").disabled(), Some(true));
        window.click("diff-staged", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let tab = home.read(cx).active_git().unwrap();
        assert_eq!(tab.source, ChangeSource::Staged);
        assert!(tab.loading && tab.cache.is_empty() && tab.selected.is_none());
        assert!(tab.list_task.is_none() && tab.body_task.is_none() && tab.poll.is_none());
        home.update(cx, |home, cx| {
            home.close_workspace_tabs(GIT_TAB, 0, window, cx)
        });
        assert!(
            home.read(cx)
                .session_view()
                .unwrap()
                .workspace
                .git
                .is_none()
        );
    });
    visual.run_until_parked();
}
#[gpui_kit::test]
fn reader_preferences_keep_source_selection_and_restore_split_after_narrowing(
    cx: &mut TestAppContext,
) {
    let (home, visual, _fixture) = mount(cx);
    let (state, id) = visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            let id = tab.selected.clone().unwrap();
            let state = tab.cache[&id]
                .block
                .as_ref()
                .unwrap()
                .state()
                .unwrap()
                .clone();
            state.update(cx, |state, cx| {
                state.set_selected_lines(
                    Some(DiffLineRange::new("a.rs", DiffSide::Modified, 1, 1)),
                    cx,
                )
            });
            (state, id)
        })
    });
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            home.pane_layout.main_width = 1000.;
            home.reader_command(Command::Mode, window, cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Split);
            home.pane_layout.main_width = 300.;
            home.apply_reader_options(cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Unified);
            assert!(home.active_git().unwrap().options.split);
            home.pane_layout.main_width = 1000.;
            home.apply_reader_options(cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Split);
            let saved = state.read(cx).selected_lines();
            home.reader_command(Command::Wrap, window, cx);
            home.reader_command(Command::Word, window, cx);
            assert_eq!(state.read(cx).selected_lines(), saved);
            home.reader_command(Command::CopyContent, window, cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("new\n")
            );
            home.reader_command(Command::Collapse, window, cx);
            assert!(home.active_git().unwrap().options.collapsed);
            home.reader_command(Command::ExpandFile, window, cx);
            assert!(home.active_git().unwrap().options.collapsed);
            assert!(home.active_git().unwrap().unfolded.contains(&id));
            assert_eq!(
                home.active_git().unwrap().cache[&id]
                    .block
                    .as_ref()
                    .unwrap()
                    .state()
                    .unwrap()
                    .entity_id(),
                state.entity_id()
            );
            home.change_git_source(ChangeSource::Staged, window, cx);
            assert!(!home.active_git().unwrap().options.collapsed);
            home.change_git_source(ChangeSource::Unstaged, window, cx);
            assert!(home.active_git().unwrap().options.collapsed);
        })
    });
    visual.run_until_parked();
}

#[gpui_kit::test]
fn filter_replaces_failed_body_with_cached_selection(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            tab.body_error = Some("error for a.rs".into());
            let generation = tab.body_generation;
            tab.filter
                .update(cx, |input, cx| input.set_value("b.rs", window, cx));
            home.filter_git(window, cx);
            let tab = home.active_git().unwrap();
            assert_eq!(
                tab.selected.as_ref(),
                Some(&identity(tab.source, std::path::Path::new("b.rs")))
            );
            assert!(tab.body_error.is_none());
            assert!(tab.body_generation > generation);
            assert!(tab.cache.contains_key(tab.selected.as_ref().unwrap()));
            let tab = home.git_mut().unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("missing", window, cx));
            home.filter_git(window, cx);
            assert!(home.active_git().unwrap().selected.is_none());
            assert!(home.active_git().unwrap().body_error.is_none());
        });
    });
}
