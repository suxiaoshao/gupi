use super::*;
use gupi_resources::git::{Head, Repository};
use std::time::Duration;

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
type Projection = Result<Option<(Repository, Head)>, String>;

#[derive(Default)]
pub(super) struct CurrentRepository {
    cwd: Option<PathBuf>,
    generation: u64,
    // None = not yet queried. Ok(None) = not a worktree. Err = retryable failure.
    result: Option<Projection>,
    refresh: Option<Task<()>>,
}

impl CurrentRepository {
    pub(super) fn head(&self) -> Option<&Head> {
        self.result
            .as_ref()?
            .as_ref()
            .ok()?
            .as_ref()
            .map(|(_, head)| head)
    }

    fn retarget(&mut self, cwd: Option<PathBuf>) {
        if self.cwd != cwd {
            self.refresh = None;
            self.generation += 1;
            self.cwd = cwd;
            self.result = None;
        }
    }

    fn apply(&mut self, generation: u64, result: Projection) -> bool {
        if self.generation != generation {
            return false;
        }
        let changed = self.result.as_ref() != Some(&result);
        self.result = Some(result);
        changed
    }
}

impl HomeView {
    pub(super) fn sync_git(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.shown_key != *self.state.read(cx).selected() {
            self.git.refresh = None;
            self.git.result = None;
            self.git.generation += 1;
        }
        let cwd = self
            .state
            .read(cx)
            .current()
            .map(|session| session.info().cwd.clone());
        self.git.retarget(cwd);
        if !self.notification_visible || !window.is_window_active() || !window.is_visible() {
            self.git.refresh = None;
            return;
        }
        if self.git.refresh.is_some() {
            return;
        }
        let Some(cwd) = self.git.cwd.clone() else {
            return;
        };
        let generation = self.git.generation;
        self.git.refresh = Some(cx.spawn(async move |owner, cx| {
            loop {
                let cwd = cwd.clone();
                let result = cx
                    .background_spawn(async move {
                        let Some(repository) = gupi_resources::git::discover(&cwd).await? else {
                            return Ok(None);
                        };
                        let head = repository.head().await?;
                        Ok::<_, gupi_resources::git::Error>(Some((repository, head)))
                    })
                    .await
                    .map_err(|error| error.to_string());
                let keep = owner
                    .update(cx, |this, cx| {
                        if this.git.generation != generation {
                            return false;
                        }
                        if this.git.apply(generation, result) {
                            cx.notify();
                        }
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
                cx.background_executor().timer(REFRESH_INTERVAL).await;
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[gpui_kit::test]
    fn branch_uses_available_titlebar_space_and_shrinks_before_actions(cx: &mut TestAppContext) {
        use gpui_kit::component::Root;
        let fixture = tempfile::tempdir().unwrap();
        let branch = "feature/titlebar-branch-check";
        let status = std::process::Command::new("git")
            .current_dir(fixture.path())
            .args(["init", "--quiet", &format!("--initial-branch={branch}")])
            .status()
            .unwrap();
        assert!(status.success());
        let repository = smol::block_on(gupi_resources::git::discover(fixture.path()))
            .unwrap()
            .unwrap();
        let head = smol::block_on(repository.head()).unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(layout::LayoutState::default());
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            let mut info = gupi_conversation::session_catalog::SessionInfo::new(
                Default::default(),
                "fixture".into(),
                fixture.path().to_owned(),
            );
            info.name = Some("Short".into());
            state.sessions_for_test().insert(
                "fixture".into(),
                gupi_conversation::conversation::Session::new(info, String::new()),
            );
            *state.selected_for_test() = Some("fixture".into());
        });
        let mut home = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state, window, cx));
            view.update(cx, |home, _| {
                home.show_sidebar = false;
                home.git.result = Some(Ok(Some((repository, head))));
            });
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        visual.simulate_resize(size(px(760.), px(600.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let wide = visual.debug_bounds("session-git-head").unwrap();
        assert!(
            wide.size.width > px(160.),
            "branch unnecessarily capped: {wide:?}"
        );
        visual.simulate_resize(size(px(360.), px(600.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let narrow = visual.debug_bounds("session-git-head").unwrap();
        assert!(narrow.size.width < wide.size.width);
        visual.update(|window, cx| {
            let history = home.read(cx).panel_button_bounds[0].get().unwrap();
            assert!(narrow.right() <= history.left());
            assert!(history.right() <= window.viewport_size().width);
        });
    }

    #[test]
    fn cwd_switch_clears_projection_and_rejects_late_results() {
        let mut current = CurrentRepository::default();
        current.retarget(Some("old".into()));
        let old = current.generation;
        current.result = Some(Err("old error".into()));
        current.retarget(Some("new".into()));
        assert!(current.result.is_none());
        assert!(!current.apply(old, Ok(None)));
        assert!(current.result.is_none());
        assert!(current.apply(current.generation, Ok(None)));
        assert!(current.head().is_none());
    }
}
