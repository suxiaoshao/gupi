use super::*;
use core::prelude::v1::test;
use gpui_kit::component::{
    Root,
    diff::{DiffLineRange, DiffMode, DiffSide},
};
use gpui_kit::test::{TestQueryExt, TestWindowExt};

fn mount(cx: &mut TestAppContext) -> (Entity<HomeView>, &mut VisualTestContext, tempfile::TempDir) {
    mount_view(cx, GitView::List)
}
fn mount_view(
    cx: &mut TestAppContext,
    view: GitView,
) -> (Entity<HomeView>, &mut VisualTestContext, tempfile::TempDir) {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("state.toml");
    let mut saved = gupi_settings::layout::LayoutState::default();
    saved.git_view = view;
    gupi_settings::layout::save(&path, &saved).unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::host::install_headless(cx);
        app_theme::init(cx);
        gupi_settings::theme::init(cx);
        gupi_settings::i18n::apply(Default::default(), cx);
        gupi_pi_runtime::init(cx);
        cx.set_global(gupi_settings::layout::read(&path).unwrap());
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
    tab.apply_files(
        vec![
            (
                ChangeSource::Staged,
                Ok(vec![Change::for_test("a.rs".into(), ChangeKind::Modified)]),
            ),
            (
                ChangeSource::Unstaged,
                Ok(["a.rs", "b.rs", "dir/c.rs"]
                    .into_iter()
                    .map(|p| Change::for_test(p.into(), ChangeKind::Modified))
                    .collect()),
            ),
            (
                ChangeSource::Untracked,
                Ok(vec![Change::for_test("new.txt".into(), ChangeKind::Added)]),
            ),
        ],
        window,
        cx,
    );
    for entry in &tab.all {
        let path = entry.file.path().to_str().unwrap();
        let patch = format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-old\n+new\n");
        tab.cache.insert(
            entry.id(),
            Loaded {
                entry: entry.file.clone(),
                content: ChangeContent::for_test("old\n".into(), "new\n".into(), patch.clone()),
                block: Some(DiffBlock::patch(&patch, cx)),
            },
        );
    }
    home.apply_reader_options(cx);
    cx.notify();
}
#[gpui_kit::test]
fn grouped_git_search_navigation_and_empty_sections(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            assert!(home.active_git().unwrap().loading);
            ready(home, window, cx);
            let tab = home.active_git().unwrap();
            assert_eq!(tab.rows.len(), 5);
            assert_ne!(tab.rows[0].path, tab.rows[1].path);
            assert!(tab.list_task.is_none() && tab.body_task.is_none() && tab.poll.is_none());
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_ne!(window.find("diff-next-file").disabled(), Some(true));
        window.click("diff-next-file", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            assert_eq!(
                home.active_git().unwrap().selected_source(),
                Some(ChangeSource::Unstaged)
            );
            let tab = home.git_mut().unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("new", window, cx));
            home.filter_git(window, cx);
            assert_eq!(home.active_git().unwrap().rows.len(), 1);
            assert_eq!(
                home.active_git().unwrap().selected_source(),
                Some(ChangeSource::Untracked)
            );
            home.toggle_git_group(Group::Staged, window, cx);
            let tab = home.active_git().unwrap();
            assert!(
                tab.list
                    .read(cx)
                    .delegate()
                    .items
                    .iter()
                    .any(|item| matches!(item, Item::Group(Group::Staged, 0, _)))
            );
        })
    });
}
#[gpui_kit::test]
fn reader_preferences_preserve_selection_and_collapse_only_current_file(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.active_git().unwrap();
            let id = tab.selected.clone().unwrap();
            let state = tab.cache[&id]
                .block
                .as_ref()
                .unwrap()
                .state()
                .unwrap()
                .clone();
            state.update(cx, |s, cx| {
                s.set_selected_lines(
                    Some(DiffLineRange::new("a.rs", DiffSide::Modified, 1, 1)),
                    cx,
                )
            });
            home.workspace_layout.main_width = 1000.;
            home.reader_command(reader::Command::Split, window, cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Split);
            home.workspace_layout.main_width = 300.;
            home.apply_reader_options(cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Unified);
            assert!(home.active_git().unwrap().options.split);
            home.workspace_layout.main_width = 1000.;
            home.apply_reader_options(cx);
            assert_eq!(state.read(cx).mode(), DiffMode::Split);
            let selected = state.read(cx).selected_lines();
            home.reader_command(reader::Command::Wrap, window, cx);
            home.reader_command(reader::Command::Word, window, cx);
            assert_eq!(state.read(cx).selected_lines(), selected);
            home.reader_command(reader::Command::ToggleFile, window, cx);
            assert!(state.read(cx).is_file_collapsed("a.rs"));
            home.select_git_file(
                identity(ChangeSource::Unstaged, std::path::Path::new("b.rs")),
                window,
                cx,
            );
            let tab = home.active_git().unwrap();
            let block = tab.cache[tab.selected.as_ref().unwrap()]
                .block
                .as_ref()
                .unwrap();
            assert!(!block.state().unwrap().read(cx).is_file_collapsed("b.rs"));
            home.select_git_file(id.clone(), window, cx);
            assert!(home.active_git().unwrap().collapsed.contains(&id));
        })
    });
}
#[gpui_kit::test]
fn filter_replaces_failed_body_with_cached_selection(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            tab.body_error = Some("error a".into());
            let generation = tab.body_generation;
            tab.filter
                .update(cx, |input, cx| input.set_value("b.rs", window, cx));
            home.filter_git(window, cx);
            let tab = home.active_git().unwrap();
            assert_eq!(
                tab.selected_entry().unwrap().path(),
                std::path::Path::new("b.rs")
            );
            assert!(tab.body_error.is_none());
            assert!(tab.body_generation > generation);
            let tab = home.git_mut().unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("missing", window, cx));
            home.filter_git(window, cx);
            assert!(home.active_git().unwrap().selected.is_none());
        })
    });
}
#[gpui_kit::test]
fn static_refresh_keeps_geometry_scroll_selection_and_pending_body(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    let state = visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            let extra = (0..100)
                .map(|i| Change::for_test(format!("file-{i:03}.rs").into(), ChangeKind::Modified))
                .collect();
            tab.apply_files(vec![(ChangeSource::Untracked, Ok(extra))], window, cx);
            let id = tab.selected.clone().unwrap();
            let mut patch = String::from("--- a/a.rs\n+++ b/a.rs\n@@ -1,300 +1,300 @@\n");
            for i in 0..300 {
                patch.push_str(&format!("-old row {i}\n+new row {i}\n"));
            }
            let loaded = tab.cache.get_mut(&id).unwrap();
            loaded.content = ChangeContent::for_test("old".into(), "new".into(), patch.clone());
            loaded.block = Some(DiffBlock::patch(&patch, cx));
            let state = loaded.block.as_ref().unwrap().state().unwrap().clone();
            state.update(cx, |s, cx| {
                s.set_selected_lines(
                    Some(DiffLineRange::new("a.rs", DiffSide::Modified, 1, 1)),
                    cx,
                )
            });
            tab.list
                .read(cx)
                .scroll_handle()
                .base_handle()
                .set_offset(point(px(0.), px(-120.)));
            state
        })
    });
    visual.run_until_parked();
    let (header, body) = visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        (
            window.find("git-file-header").bounds(),
            window.find("git-body").bounds(),
        )
    });
    let before_scroll = visual.update(|window, _| {
        window
            .within("git-body")
            .elements()
            .into_iter()
            .filter_map(|e| e.label().map(|label| (label.to_owned(), e.bounds())))
            .collect::<Vec<_>>()
    });
    visual.simulate_event(ScrollWheelEvent {
        position: body.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-180.))),
        ..Default::default()
    });
    visual.run_until_parked();
    let painted = visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        window
            .within("git-body")
            .elements()
            .into_iter()
            .filter_map(|e| e.label().map(|label| (label.to_owned(), e.bounds())))
            .collect::<Vec<_>>()
    });
    assert!(!painted.is_empty());
    assert_ne!(
        painted, before_scroll,
        "the fixture must actually scroll before asserting retention"
    );
    let (rows, list_offset, selected, generation) = visual.update(|_, cx| {
        let tab = home.read(cx).active_git().unwrap();
        (
            tab.rows.clone(),
            tab.list.read(cx).scroll_handle().base_handle().offset(),
            state.read(cx).selected_lines(),
            tab.body_generation,
        )
    });
    for _ in 0..2 {
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.git_mut().unwrap().refreshing = true;
                cx.notify();
            });
            window.draw(cx).clear(cx);
            assert_eq!(window.find("git-file-header").bounds(), header);
            assert_eq!(window.find("git-body").bounds(), body);
            home.update(cx, |home, cx| {
                let tab = home.git_mut().unwrap();
                let results = SOURCES
                    .into_iter()
                    .map(|s| {
                        (
                            s,
                            Ok(tab
                                .all
                                .iter()
                                .filter(|e| e.source == s)
                                .map(|e| e.file.clone())
                                .collect()),
                        )
                    })
                    .collect();
                tab.body_task = Some(Task::ready(()));
                tab.body_target = Some((
                    tab.selected.clone().unwrap(),
                    tab.selected_entry().unwrap().clone(),
                ));
                assert!(!tab.apply_files(results, window, cx));
                assert!(tab.body_task.is_some());
                tab.refreshing = false;
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
            let tab = home.read(cx).active_git().unwrap();
            assert!(Rc::ptr_eq(&rows, &tab.rows));
            assert_eq!(tab.body_generation, generation);
            assert_eq!(
                tab.cache[tab.selected.as_ref().unwrap()]
                    .block
                    .as_ref()
                    .unwrap()
                    .state()
                    .unwrap()
                    .entity_id(),
                state.entity_id()
            );
            assert_eq!(state.read(cx).selected_lines(), selected);
            assert_eq!(
                tab.list.read(cx).scroll_handle().base_handle().offset(),
                list_offset
            );
            assert_eq!(window.find("git-file-header").bounds(), header);
            assert_eq!(window.find("git-body").bounds(), body);
            let after = window
                .within("git-body")
                .elements()
                .into_iter()
                .filter_map(|e| e.label().map(|label| (label.to_owned(), e.bounds())))
                .collect::<Vec<_>>();
            assert_eq!(after, painted);
        });
    }
}
#[gpui_kit::test]
fn changed_and_failed_snapshots_retain_current_body_until_ready(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
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
                .entity_id();
            tab.apply_files(
                vec![
                    (
                        ChangeSource::Staged,
                        Ok(vec![Change::for_test("a.rs".into(), ChangeKind::Added)]),
                    ),
                    (
                        ChangeSource::Untracked,
                        Err(Failure::Query("offline".into())),
                    ),
                ],
                window,
                cx,
            );
            assert_eq!(
                tab.cache[&id]
                    .block
                    .as_ref()
                    .unwrap()
                    .state()
                    .unwrap()
                    .entity_id(),
                state
            );
            assert_ne!(Some(&tab.cache[&id].entry), tab.selected_entry());
            assert!(tab.all.iter().any(|e| e.source == ChangeSource::Untracked));
            assert!(tab.group_errors.contains_key(&ChangeSource::Untracked));
        })
    });
}

#[gpui_kit::test]
fn redesign_collapsing_filtered_group_preserves_body_request(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("a.rs", window, cx));
            home.filter_git(window, cx);
            let tab = home.git_mut().unwrap();
            let id = tab.selected.clone().unwrap();
            let entry = tab.selected_entry().unwrap().clone();
            let other = identity(ChangeSource::Unstaged, std::path::Path::new("a.rs"));
            tab.cache.remove(&other);
            tab.body_task = Some(Task::ready(()));
            tab.body_target = Some((id.clone(), entry));
            let generation = tab.body_generation;
            home.toggle_git_group(Group::Staged, window, cx);
            let tab = home.active_git().unwrap();
            assert_eq!(tab.selected.as_ref(), Some(&id));
            assert!(tab.body_task.is_some());
            assert_eq!(tab.body_target.as_ref().map(|(id, _)| id), Some(&id));
            assert_eq!(tab.body_generation, generation);
            assert!(!tab.cache.contains_key(&other));
        })
    });
}
#[gpui_kit::test]
fn redesign_copy_uses_displayed_snapshot_while_deletion_updates(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let tab = home.git_mut().unwrap();
            let id = tab.selected.clone().unwrap();
            tab.cache.get_mut(&id).unwrap().content = ChangeContent::for_test(
                "old original\n".into(),
                "displayed modified\n".into(),
                "fixture patch".into(),
            );
            tab.apply_files(
                vec![(
                    ChangeSource::Staged,
                    Ok(vec![Change::for_test("a.rs".into(), ChangeKind::Deleted)]),
                )],
                window,
                cx,
            );
            tab.body_error = Some("replacement failed".into());
            home.reader_command(reader::Command::CopyContent, window, cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("displayed modified\n")
            );
        })
    });
}

#[gpui_kit::test]
fn git_rows_expose_button_names_and_only_groups_disclose(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| ready(home, window, cx));
        window.draw(cx).clear(cx);
        let group = window.find("git-group-diff-staged");
        assert_eq!(group.role(), Some(Role::Button));
        assert_eq!(group.expanded(), Some(true));
        assert!(group.label().unwrap().contains('1'));
        let file = window.find(format!(
            "git-file-{}",
            identity(ChangeSource::Staged, std::path::Path::new("a.rs"))
        ));
        assert_eq!(file.role(), Some(Role::Button));
        assert!(file.label().unwrap().contains("a.rs"));
        assert!(file.label().unwrap().contains(&t(cx, "diff-modified")));
        assert_eq!(file.expanded(), None);
        window.click("git-group-diff-staged", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find("git-group-diff-staged").expanded(), Some(false));
    });
}

#[gpui_kit::test]
fn file_header_separates_reading_options_and_actions(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| ready(home, window, cx));
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(
            window.find("diff-toggle-file").bounds().right()
                <= window.find("diff-previous-file").bounds().left()
        );
        let reason = t(cx, "diff-reading-options-narrow");
        assert_eq!(
            window.find("diff-reading-options").label(),
            Some(reason.as_str())
        );
        window.click("diff-reading-options", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find_by_label(&t(cx, "diff-split")).is_some());
        assert!(window.try_find_by_label(&t(cx, "diff-unified")).is_some());
        home.update(cx, |view, cx| {
            let owner = cx.entity();
            assert!(matches!(
                view.reader_menu_item(reader::Command::Split, "diff-split", false, &owner, cx),
                PopupMenuItem::Item {
                    disabled: true,
                    checked: false,
                    ..
                }
            ));
            assert!(matches!(
                view.reader_menu_item(reader::Command::Unified, "diff-unified", true, &owner, cx),
                PopupMenuItem::Item {
                    disabled: false,
                    checked: true,
                    ..
                }
            ));
            view.git_mut().unwrap().options.split = true;
            view.reader_command(reader::Command::Split, window, cx);
            assert!(view.active_git().unwrap().options.split);
        });
        for key in ["diff-word", "diff-wrap", "diff-context"] {
            assert!(window.try_find_by_label(&t(cx, key)).is_some());
        }
        assert!(window.try_find_by_label(&t(cx, "diff-copy-path")).is_none());
    });
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.click("diff-more", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        for key in ["diff-copy-path", "diff-copy-patch", "diff-copy-content"] {
            assert!(window.try_find_by_label(&t(cx, key)).is_some());
        }
        assert_eq!(window.find_all_by_label(&t(cx, "diff-external")).len(), 2);
        home.update(cx, |view, cx| {
            assert!(matches!(
                view.reader_menu_item(
                    reader::Command::External,
                    "diff-external",
                    false,
                    &cx.entity(),
                    cx
                ),
                PopupMenuItem::Item { disabled: true, .. }
            ));
        });
        assert!(window.try_find_by_label(&t(cx, "diff-wrap")).is_none());
    });
}

#[gpui_kit::test]
fn tree_first_projection_keyboard_search_and_hidden_file_neighbors(cx: &mut TestAppContext) {
    let (home, visual, fixture) = mount_view(cx, GitView::Tree);
    let directory = Directory {
        group: Group::Unstaged,
        path: "dir".into(),
    };
    let file = identity(ChangeSource::Unstaged, std::path::Path::new("dir/c.rs"));
    let body = visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            assert_eq!(home.active_git().unwrap().projection_view, GitView::Tree);
            assert!(
                home.active_git()
                    .unwrap()
                    .list
                    .read(cx)
                    .delegate()
                    .items
                    .iter()
                    .any(|i| matches!(i, Item::Directory(..)))
            );
            home.select_git_file(file.clone(), window, cx);
            home.active_git().unwrap().focus_list(window, cx);
            home.active_git().unwrap().cache[&file]
                .block
                .as_ref()
                .unwrap()
                .state()
                .unwrap()
                .clone()
        })
    });
    visual.run_until_parked();
    visual.simulate_keystrokes("left");
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find(directory.id()).expanded(), Some(true));
        assert_eq!(
            home.read(cx)
                .active_git()
                .unwrap()
                .list
                .read(cx)
                .delegate()
                .items[home
                .read(cx)
                .active_git()
                .unwrap()
                .list
                .read(cx)
                .delegate()
                .index
                .unwrap()]
            .id(),
            directory.id()
        );
    });
    visual.simulate_keystrokes("left");
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(window.find(directory.id()).expanded(), Some(false));
        home.update(cx, |home, cx| {
            let tab = home.active_git().unwrap();
            assert_eq!(tab.selected.as_ref(), Some(&file));
            assert_eq!(
                tab.cache[&file].block.as_ref().unwrap().state().unwrap(),
                &body
            );
            assert!(!tab.rows.iter().any(|row| row.path == file));
            let (previous, next) = tab.neighbors();
            assert_eq!(
                previous,
                Some(identity(ChangeSource::Staged, std::path::Path::new("a.rs")))
            );
            assert_eq!(
                next,
                Some(identity(
                    ChangeSource::Unstaged,
                    std::path::Path::new("a.rs")
                ))
            );
            let tab = home.git_mut().unwrap();
            tab.filter
                .update(cx, |input, cx| input.set_value("c.rs", window, cx));
            home.filter_git(window, cx);
            assert!(
                home.active_git()
                    .unwrap()
                    .rows
                    .iter()
                    .any(|row| row.path == file)
            );
            home.toggle_git_directory(directory.clone(), window, cx);
            assert!(
                home.active_git()
                    .unwrap()
                    .directories_closed
                    .contains(&directory)
            );
            home.git_mut()
                .unwrap()
                .filter
                .update(cx, |input, cx| input.set_value("", window, cx));
            home.filter_git(window, cx);
            assert!(
                !home
                    .active_git()
                    .unwrap()
                    .rows
                    .iter()
                    .any(|row| row.path == file)
            );
            home.set_git_view(GitView::List, window, cx);
            assert_eq!(
                home.active_git().unwrap().cache[&file]
                    .block
                    .as_ref()
                    .unwrap()
                    .state()
                    .unwrap(),
                &body
            );
            home.set_git_view(GitView::Tree, window, cx);
            assert!(
                !home
                    .active_git()
                    .unwrap()
                    .directories_closed
                    .contains(&directory)
            );
            assert!(
                home.active_git()
                    .unwrap()
                    .rows
                    .iter()
                    .any(|row| row.path == file)
            );
            assert!(home.active_git().unwrap().list_task.is_none());
        });
    });
    visual.run_until_parked();
    assert_eq!(
        gupi_settings::layout::read(&fixture.path().join("state.toml"))
            .unwrap()
            .git_view,
        GitView::Tree
    );
}

#[gpui_kit::test]
fn tree_unchanged_refresh_preserves_projection_scroll_and_prunes_removed_directories(
    cx: &mut TestAppContext,
) {
    let (home, visual, _fixture) = mount_view(cx, GitView::Tree);
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            ready(home, window, cx);
            let directory = Directory {
                group: Group::Unstaged,
                path: "dir".into(),
            };
            home.toggle_git_directory(directory.clone(), window, cx);
            let tab = home.git_mut().unwrap();
            let rows = tab.rows.clone();
            let generation = tab.body_generation;
            let selected = tab.selected.clone();
            let offset = point(px(0.), px(-20.));
            tab.list
                .read(cx)
                .scroll_handle()
                .base_handle()
                .set_offset(offset);
            let results = SOURCES
                .into_iter()
                .map(|source| {
                    (
                        source,
                        Ok(tab
                            .all
                            .iter()
                            .filter(|entry| entry.source == source)
                            .map(|entry| entry.file.clone())
                            .collect()),
                    )
                })
                .collect();
            assert!(!tab.apply_files(results, window, cx));
            assert!(Rc::ptr_eq(&rows, &tab.rows));
            assert_eq!(tab.selected, selected);
            assert_eq!(tab.body_generation, generation);
            assert!(tab.directories_closed.contains(&directory));
            assert_eq!(
                tab.list.read(cx).scroll_handle().base_handle().offset(),
                offset
            );
            tab.apply_files(vec![(ChangeSource::Unstaged, Ok(vec![]))], window, cx);
            assert!(!tab.directories_closed.contains(&directory));
        })
    });
}

#[gpui_kit::test]
fn mode_only_body_has_localized_state_and_copies_original_metadata_patch(cx: &mut TestAppContext) {
    let (home, visual, _fixture) = mount(cx);
    let patch = "diff --git a/mode.sh b/mode.sh\nold mode 100644\nnew mode 100755\n";
    for source in [ChangeSource::Unstaged, ChangeSource::Staged] {
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                let file = Change::for_test("mode.sh".into(), ChangeKind::Modified);
                let tab = home.git_mut().unwrap();
                tab.loading = false;
                tab.all.clear();
                tab.apply_files(vec![(source, Ok(vec![file.clone()]))], window, cx);
                let id = identity(source, file.path());
                tab.selected = Some(id.clone());
                let body = ChangeContent::for_test("same\n".into(), "same\n".into(), patch.into());
                let loaded = Loaded::new(file, body, None, cx);
                assert!(loaded.block.is_none());
                tab.cache.insert(id, loaded);
                home.reader_command(reader::Command::CopyPatch, window, cx);
                assert_eq!(
                    cx.read_from_clipboard().unwrap().text().as_deref(),
                    Some(patch)
                );
                assert!(home.reader_command_available(reader::Command::CopyContent, cx));
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                window.find("git-content-state").label(),
                Some(t(cx, "diff-no-text-changes").as_str())
            );
        });
    }
}

#[cfg(unix)]
#[gpui_kit::test]
fn non_unicode_rows_keep_byte_identity_show_localized_state_and_disable_copy(
    cx: &mut TestAppContext,
) {
    use std::os::unix::ffi::OsStringExt;
    let (home, visual, _fixture) = mount(cx);
    let raw = PathBuf::from(std::ffi::OsString::from_vec(b"bad-\xff.txt".to_vec()));
    let displayed = PathBuf::from("bad-\u{fffd}.txt");
    assert_ne!(
        identity(ChangeSource::Untracked, &raw),
        identity(ChangeSource::Untracked, &displayed)
    );
    visual.update(|window, cx| {
        home.update(cx, |home, cx| {
            let file = Change::for_test(raw.clone(), ChangeKind::Added);
            let tab = home.git_mut().unwrap();
            tab.loading = false;
            tab.apply_files(
                vec![(
                    ChangeSource::Untracked,
                    Ok(vec![
                        file.clone(),
                        Change::for_test(displayed, ChangeKind::Added),
                    ]),
                )],
                window,
                cx,
            );
            assert_eq!(tab.rows.len(), 2);
            let id = identity(ChangeSource::Untracked, &raw);
            tab.selected = Some(id.clone());
            tab.sync_rows(window, cx);
            let body = ChangeContent::unavailable_for_test(ContentState::InvalidPath);
            let loaded = Loaded::new(file, body, None, cx);
            assert!(loaded.block.is_none());
            tab.cache.insert(id, loaded);
            for command in [
                reader::Command::CopyPath,
                reader::Command::CopyPatch,
                reader::Command::CopyContent,
                reader::Command::External,
            ] {
                assert!(!home.reader_command_available(command, cx));
            }
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert_eq!(
            window.find("git-content-state").label(),
            Some(t(cx, "diff-invalid-path").as_str())
        );
    });
}
