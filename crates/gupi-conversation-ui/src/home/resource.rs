//! Session-bound desktop resource navigation; Pi never executes these opens.
use super::*;
use gpui_kit::component::notification::Notification;

#[derive(Clone)]
pub(super) struct Context {
    owner: WeakEntity<HomeView>,
    key: String,
    binding: u64,
    pub(super) cwd: PathBuf,
}

impl Context {
    pub(super) fn open(&self, path: PathBuf, keyboard: bool, window: &mut Window, cx: &mut App) {
        let request = self.clone();
        let _ = self.owner.update(cx, |home, cx| {
            home.open_resource(request, path, keyboard, window, cx);
        });
    }

    pub(super) fn open_link(
        &self,
        value: &str,
        base: &std::path::Path,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if value.starts_with('#') {
            return;
        }
        if let Some(path) = local_path(value, base) {
            self.open(path, matches!(event, ClickEvent::Keyboard(_)), window, cx);
        } else {
            cx.open_url(value);
        }
    }

    pub(super) fn is_current(&self, home: &HomeView, cx: &App) -> bool {
        home.notification_visible
            && home.shown_key.as_ref() == Some(&self.key)
            && home.state.read(cx).selected().as_ref() == Some(&self.key)
            && home
                .state
                .read(cx)
                .sessions()
                .get(&self.key)
                .is_some_and(|session| {
                    session.binding() == self.binding && session.info().cwd == self.cwd
                })
    }
}

/// URL resolution strips fragments and decodes escaped paths without shell expansion.
pub(super) fn local_path(value: &str, base: &std::path::Path) -> Option<PathBuf> {
    // Drive/UNC paths are native paths, not URI schemes such as `c:`.
    #[cfg(windows)]
    if std::path::Path::new(value).is_absolute() {
        return Some(PathBuf::from(value));
    }
    if value.starts_with('#') {
        return None;
    }
    let url = url::Url::parse(value)
        .or_else(|_| {
            url::Url::from_directory_path(base)
                .map_err(|_| url::ParseError::RelativeUrlWithoutBase)?
                .join(value)
        })
        .ok()?;
    url.to_file_path().ok()
}

enum Target {
    File(PathBuf),
    Directory(PathBuf),
    External,
}

fn resolve(cwd: &std::path::Path, path: &std::path::Path) -> Target {
    let Ok(root) = cwd.canonicalize() else {
        return Target::External;
    };
    let Ok(real) = path.canonicalize() else {
        return Target::External;
    };
    let Ok(relative) = real.strip_prefix(&root) else {
        return Target::External;
    };
    // Map canonical identity back to the existing tree's spelling (including symlink roots).
    let tree_path = cwd.join(relative);
    if real.is_dir() {
        Target::Directory(tree_path)
    } else {
        Target::File(tree_path)
    }
}

pub(super) fn open_external(path: PathBuf, window: &mut Window, cx: &mut App) {
    let task = crate::host::open_file(path, cx);
    window
        .spawn(cx, async move |cx| {
            if let Err(error) = task.await {
                tracing::warn!(%error, "resource open failed");
                let _ = cx.update(|window, cx| {
                    window
                        .push_notification(Notification::error(t(cx, "resource-open-failed")), cx);
                });
            }
        })
        .detach();
}

impl HomeView {
    pub(super) fn resource_context(&self, key: &str, cx: &App) -> Option<resource::Context> {
        let state = self.state.read(cx);
        let session = state.sessions().get(key)?;
        Some(resource::Context {
            owner: self.resource_owner.clone(),
            key: key.to_owned(),
            binding: session.binding(),
            cwd: session.info().cwd.clone(),
        })
    }

    pub(super) fn open_resource(
        &mut self,
        request: resource::Context,
        path: PathBuf,
        keyboard: bool,
        window: &mut Window,
        cx: &mut gpui_kit::Context<Self>,
    ) {
        if !request.is_current(self, cx) {
            return;
        }
        self.resource_open = None;
        let path = if path.is_absolute() {
            path
        } else {
            request.cwd.join(path)
        };
        if self.state.read(cx).is_temporary() {
            open_external(path, window, cx);
            return;
        }
        let cwd = request.cwd.clone();
        let original = path.clone();
        let worker = cx.background_spawn(async move { resolve(&cwd, &path) });
        self.resource_open = Some(cx.spawn_in(window, async move |home, cx| {
            let target = worker.await;
            let _ = home.update_in(cx, |home, window, cx| {
                if !request.is_current(home, cx) {
                    return;
                }
                match target {
                    Target::External => open_external(original, window, cx),
                    Target::File(path) => {
                        home.select_workspace_tab(None, window, cx);
                        home.ensure_panel(panes::Panel::Workspace, window, cx);
                        home.open_source(path, keyboard, window, cx);
                        if let Some(source) = home.source() {
                            source.update(cx, |source, cx| {
                                source.set_display_path(original.clone(), cx)
                            });
                        }
                    }
                    Target::Directory(path) => {
                        home.select_workspace_tab(None, window, cx);
                        home.set_files_navigator_hidden(false, cx);
                        home.ensure_panel(panes::Panel::Workspace, window, cx);
                        if !home.workspace_layout.navigator_visible() {
                            home.set_temporary_list(true);
                            home.fit_workspace_layout(window, cx);
                        }
                        if let Some(files) = home.files() {
                            files.update(cx, |files, cx| files.reveal(path, window, cx));
                        }
                    }
                }
            });
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::InputEvent as _;

    // Prevent project-prefix and symlink aliases from routing to the wrong owner.
    #[test]
    fn canonical_targets_and_file_urls_keep_project_boundaries() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("project");
        let other = directory.path().join("project-other");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let file = root.join("sub/a b.md");
        std::fs::write(&file, "# local").unwrap();
        let outside = other.join("file.md");
        std::fs::write(&outside, "outside").unwrap();
        assert!(matches!(resolve(&root, &file), Target::File(path) if path == file));
        assert!(
            matches!(resolve(&root, &root.join("sub")), Target::Directory(path) if path == root.join("sub"))
        );
        assert!(matches!(resolve(&root, &root), Target::Directory(path) if path == root));
        assert!(matches!(resolve(&root, &outside), Target::External));
        assert_eq!(
            local_path("sub/a%20b.md#heading", &root),
            Some(file.clone())
        );
        assert_eq!(local_path("https://example.com/file.md", &root), None);
        assert_eq!(local_path("#heading", &root), None);
        assert_eq!(
            local_path(
                &format!("{}#heading", url::Url::from_file_path(&file).unwrap()),
                &root
            ),
            Some(file.clone())
        );
        #[cfg(windows)]
        for value in [
            r"C:\project\file.md",
            "C:/project/file.md",
            r"\\server\share\image.png",
        ] {
            assert_eq!(local_path(value, &root), Some(PathBuf::from(value)));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&outside, root.join("outside")).unwrap();
            symlink(&root, directory.path().join("alias")).unwrap();
            symlink(&file, other.join("inside")).unwrap();
            assert!(matches!(
                resolve(&root, &root.join("outside")),
                Target::External
            ));
            assert!(
                matches!(resolve(&root, &other.join("inside")), Target::File(path) if path == file)
            );
            let alias = directory.path().join("alias");
            assert!(
                matches!(resolve(&alias, &file), Target::File(path) if path == alias.join("sub/a b.md"))
            );
        }
    }

    // Exercise the rendered resource button and the semantic command's async
    // ownership: an old session or an older open cannot change the visible file.
    #[gpui_kit::test]
    async fn resource_buttons_route_to_files_and_drop_obsolete_opens(cx: &mut TestAppContext) {
        use gpui_kit::component::Root;
        use gpui_kit::test::{TestAppContextExt as _, TestWindowExt as _};
        use gupi_conversation::{
            conversation::{ConversationState, Session},
            history::DisplayMessage,
            session_catalog::SessionInfo,
        };
        struct Opened(Vec<PathBuf>);
        impl Global for Opened {}

        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("project");
        std::fs::create_dir_all(root.join("folder")).unwrap();
        let file = root.join("file.rs");
        let latest = root.join("latest.rs");
        let outside = fixture.path().join("outside.rs");
        for path in [&file, &latest, &outside] {
            std::fs::write(path, "fn main() {}\n").unwrap();
        }
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_settings::keybindings::apply(&Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(gupi_settings::layout::LayoutState::default());
            cx.set_global(TestLayoutDirectory(fixture.path().join("settings")));
            cx.set_global(Opened(vec![]));
            cx.set_global(crate::host::Host::new(
                |path, cx| {
                    cx.global_mut::<Opened>().0.push(path);
                    Task::ready(Ok(()))
                },
                |_, _, _| {},
                |_, _| {},
                |_, _, _, _| {},
            ));
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            for key in ["a", "b"] {
                let mut session = Session::new(SessionInfo::new(Default::default(), key.into(), root.clone()), String::new());
                *session.binding_for_test() = 1;
                *session.state_for_test() = Some(serde_json::from_value(serde_json::json!({"sessionId":key,"isStreaming":false,"isCompacting":false})).unwrap());
                session.mark_transcript_message_for_test();
                session.live_for_test().push(DisplayMessage::new("resource".into(), serde_json::json!({"role":"assistant","stopReason":"stop","content":[{"type":"text","text":format!("[Open file]({})", url::Url::from_file_path(&file).unwrap())}]})));
                *session.content_revision_for_test() += 1;
                state.sessions_for_test().insert(key.into(), session);
            }
            *state.selected_for_test() = Some("a".into());
        });
        let mut home = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state.clone(), window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        let handle = cx.update(|window, _| window.window_handle());
        cx.simulate_resize(size(px(1728.), px(900.)));
        cx.update(|window, cx| window.click("file-link-0", cx));
        cx.condition(&home, |home, _| home.source().is_some()).await;
        cx.update(|window, cx| {
            let view = home.read(cx);
            assert_eq!(view.source().unwrap().read(cx).path, file);
            assert!(view.pane_layout.visible(panes::Panel::Workspace));
            assert!(cx.global::<Opened>().0.is_empty());
            let request = view.resource_context("a", cx).unwrap();
            request.open(root.join("folder"), true, window, cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let view = home.read(cx);
            assert_eq!(
                view.source().unwrap().read(cx).path,
                file,
                "directory open retains the preview"
            );
            assert!(view.files().unwrap().read(cx).is_focused(window, cx));
            let request = view.resource_context("a", cx).unwrap();
            // The later intent wins even if both background resolutions are queued.
            request.open(file.clone(), false, window, cx);
            request.open(latest.clone(), false, window, cx);
        });
        cx.condition(&home, |home, cx| {
            home.source()
                .is_some_and(|source| source.read(cx).path == latest)
        })
        .await;
        cx.update(|window, cx| {
            home.read(cx)
                .resource_context("a", cx)
                .unwrap()
                .open(file.clone(), false, window, cx);
            home.update(cx, |home, cx| {
                home.open_source(latest.clone(), false, window, cx)
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| assert_eq!(home.read(cx).source().unwrap().read(cx).path, latest));
        cx.update(|window, cx| {
            let request = home.read(cx).resource_context("a", cx).unwrap();
            request.open(outside.clone(), false, window, cx);
            state.update(cx, |state, _| *state.selected_for_test() = Some("b".into()));
            home.update(cx, |home, cx| home.sync(false, window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                cx.global::<Opened>().0.is_empty(),
                "stale external open has no side effect"
            );
            assert!(home.read(cx).source().is_none());
            state.update(cx, |state, _| *state.selected_for_test() = Some("a".into()));
            home.update(cx, |home, cx| home.sync(false, window, cx));
            assert_eq!(home.read(cx).source().unwrap().read(cx).path, latest);
            let request = home.read(cx).resource_context("a", cx).unwrap();
            request.open(outside.clone(), false, window, cx);
            home.update(cx, |home, cx| {
                home.set_notification_visible(false, window, cx)
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(cx.global::<Opened>().0.is_empty());
            home.update(cx, |home, cx| {
                home.set_notification_visible(true, window, cx)
            });
            let request = home.read(cx).resource_context("a", cx).unwrap();
            request.open(outside.clone(), false, window, cx);
        });
        cx.run_until_parked();
        cx.update(|_, cx| assert_eq!(cx.global::<Opened>().0, vec![outside.clone()]));
        cx.update(|window, cx| {
            let input = home.read(cx).input.clone();
            let token = gupi_resources::composer_resources::file_token(&file, false);
            let range = 0..token.text().len();
            input.update(cx, |input, cx| {
                input
                    .replace_range_with_token(0..0, token, window, cx)
                    .unwrap();
            });
            window.render_frame(cx);
            let bounds = input.read(cx).range_to_bounds(&range).unwrap();
            let composer = window.find("conversation-composer").bounds();
            window.click_at(
                "conversation-composer",
                bounds.center() - composer.origin,
                cx,
            );
        });
        cx.condition(&home, |home, cx| {
            home.source()
                .is_some_and(|source| source.read(cx).path == file)
        })
        .await;
        for (name, width, height) in [("picture.png", 1600, 400), ("small.png", 100, 100)] {
            let picture = root.join(name);
            image::RgbaImage::from_pixel(width, height, image::Rgba([0, 128, 255, 255]))
                .save(&picture)
                .unwrap();
            cx.update(|window, cx| {
                home.read(cx).resource_context("a", cx).unwrap().open(
                    picture.clone(),
                    false,
                    window,
                    cx,
                );
            });
            cx.wait_for(handle, std::time::Duration::from_secs(2), |window, cx| {
                home.read(cx)
                    .source()
                    .is_some_and(|source| source.read(cx).path == picture)
                    && window
                        .try_find("file-image-zoom-in")
                        .is_some_and(|button| button.disabled() != Some(true))
            })
            .await;
            cx.update(|window, cx| {
                if width == 100 {
                    assert_eq!(
                        window.find("image-preview-bitmap").bounds().size,
                        size(px(100.), px(100.)),
                        "initial Fit uses the small image's natural size before any controls"
                    );
                }
                for id in [
                    "file-image-original",
                    "file-image-zoom-out",
                    "file-image-zoom-in",
                    "file-image-fit",
                ] {
                    window.click(id, cx);
                }
                let before = window.find("image-preview-bitmap").bounds().size.width;
                assert!(
                    before <= px(width as f32),
                    "Fit never enlarges the natural image"
                );
                if width == 100 {
                    assert_eq!(before, px(100.), "small images fit at natural size");
                }
                window.click("file-image-zoom-in", cx);
                window.render_frame(cx);
                assert!(
                    window.find("image-preview-bitmap").bounds().size.width > before,
                    "zooming in from Fit must enlarge the displayed image"
                );
                window.click("file-image-fit", cx);
                window.render_frame(cx);
                let surface = window.find("file-image-canvas").bounds();
                let position =
                    surface.origin + point(surface.size.width * 0.35, surface.size.height * 0.6);
                let prior = window.find("image-preview-bitmap").bounds();
                let unit = point(
                    (position.x - prior.left()) / prior.size.width,
                    (position.y - prior.top()) / prior.size.height,
                );
                let modifiers = Modifiers {
                    control: true,
                    ..Default::default()
                };
                window.dispatch_event(
                    MouseMoveEvent {
                        position,
                        pressed_button: None,
                        modifiers,
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
                window.dispatch_event(
                    ScrollWheelEvent {
                        position,
                        delta: ScrollDelta::Pixels(point(px(0.), px(20.))),
                        modifiers,
                        touch_phase: TouchPhase::Moved,
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
                let after = window.find("image-preview-bitmap").bounds().size.width;
                assert!(after > before, "modified wheel zooms the loaded image");
                let zoomed = window.find("image-preview-bitmap").bounds();
                let anchored =
                    zoomed.origin + point(zoomed.size.width * unit.x, zoomed.size.height * unit.y);
                assert!(
                    (anchored.x - position.x).abs() <= px(1. / window.scale_factor())
                        && (anchored.y - position.y).abs() <= px(1. / window.scale_factor()),
                    "wheel zoom keeps the image point under the pointer: {prior:?} -> {zoomed:?}, pointer={position:?}, actual={anchored:?}"
                );
                window.dispatch_event(
                    PinchEvent {
                        position,
                        delta: 0.1,
                        modifiers: Modifiers::default(),
                        phase: TouchPhase::Moved,
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
                assert!(window.find("image-preview-bitmap").bounds().size.width > after);
                for _ in 0..10 {
                    if window.find("image-preview-bitmap").bounds().size.width > surface.size.width { break; }
                    window.click("file-image-zoom-in", cx);
                }
                assert!(window.find("image-preview-bitmap").bounds().size.width > surface.size.width);
                for direction in [1., -1.] {
                    for _ in 0..20 {
                        let image = window.find("image-preview-bitmap").bounds();
                        if (direction > 0. && image.left() >= surface.left())
                            || (direction < 0. && image.right() <= surface.right())
                        {
                            break;
                        }
                        window.drag(
                            surface.center(),
                            surface.center() + point(surface.size.width * 0.4 * direction, px(0.)),
                            cx,
                        );
                    }
                    let image = window.find("image-preview-bitmap").bounds();
                    let edge = if direction > 0. {
                        image.left()
                    } else {
                        image.right()
                    };
                    assert!(
                        edge >= surface.left() && edge <= surface.right(),
                        "both edges of an enlarged image can be reached by panning"
                    );
                }
            });
        }
        let broken = root.join("broken.png");
        std::fs::write(&broken, "not a bitmap").unwrap();
        cx.update(|window, cx| {
            home.read(cx).resource_context("a", cx).unwrap().open(
                broken.clone(),
                false,
                window,
                cx,
            );
        });
        cx.wait_for(handle, std::time::Duration::from_secs(2), |window, _| {
            window.try_find("preview-open-external").is_some()
        })
        .await;
        cx.update(|window, cx| window.click("preview-open-external", cx));
        cx.run_until_parked();
        cx.update(|_, cx| assert_eq!(cx.global::<Opened>().0, vec![outside, broken]));
        // A temporary Home must use the external opener even for its own cwd.
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                let session = state.sessions_for_test().remove("a").unwrap();
                let mut temporary = ConversationState::temporary("unused-pi".into(), cx);
                temporary.sessions_for_test().insert("a".into(), session);
                *temporary.selected_for_test() = Some("a".into());
                *state = temporary;
            });
            home.update(cx, |home, cx| home.sync(false, window, cx));
            let request = home.read(cx).resource_context("a", cx).unwrap();
            request.open(file.clone(), false, window, cx);
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert_eq!(cx.global::<Opened>().0.last(), Some(&file));
            assert_ne!(home.read(cx).source().unwrap().read(cx).path, file);
        });
    }
}
