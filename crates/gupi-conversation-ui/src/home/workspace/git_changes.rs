//! Per-session worktree adapter. It never impersonates recorded tool operations.
use super::*;
use changes::navigator::{Delegate, Row};
use gpui_kit::component::input::Input;
use gupi_resources::git::{
    Change, ChangeContent, ChangeKind, ChangeSource, ContentState, Repository,
};
use messages::diff::DiffBlock;
use reader::{Command, Options};
use std::time::Duration;

pub(super) const GIT_TAB: &str = "workspace:git";
const CACHE_FILES: usize = 8;

pub(super) struct Loaded {
    entry: Change,
    pub content: ChangeContent,
    pub block: Option<DiffBlock>,
}
enum Failure {
    Unavailable,
    Query(String),
}
impl From<gupi_resources::git::Error> for Failure {
    fn from(error: gupi_resources::git::Error) -> Self {
        match error {
            gupi_resources::git::Error::Unavailable => Self::Unavailable,
            error => Self::Query(error.to_string()),
        }
    }
}
pub(super) struct GitTab {
    cwd: PathBuf,
    pub source: ChangeSource,
    repository: Option<Repository>,
    all: Vec<Change>,
    pub rows: Rc<Vec<Row>>,
    pub selected: Option<String>,
    pub list: Entity<ListState<Delegate>>,
    pub focus: FocusHandle,
    pub main_focus: FocusHandle,
    pub header_focus: FocusHandle,
    pub navigator_hidden: bool,
    pub options: Options,
    pub unfolded: HashSet<String>,
    preferences: HashMap<ChangeSource, Options>,
    filter: Entity<InputState>,
    _filter_subscription: Subscription,
    generation: u64,
    pub cache: HashMap<String, Loaded>,
    loading: bool,
    refreshing: bool,
    error: Option<Failure>,
    body_error: Option<String>,
    not_repository: bool,
    list_task: Option<Task<()>>,
    body_task: Option<Task<()>>,
    body_generation: u64,
    poll: Option<Task<()>>,
}
fn identity(source: ChangeSource, path: &std::path::Path) -> String {
    let path: String = path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{source:?}:{path}")
}
fn source_key(source: ChangeSource) -> &'static str {
    match source {
        ChangeSource::Unstaged => "diff-unstaged",
        ChangeSource::Staged => "diff-staged",
        ChangeSource::Untracked => "diff-untracked",
    }
}
fn kind_key(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "diff-added",
        ChangeKind::Deleted => "diff-deleted",
        ChangeKind::Modified => "diff-modified",
        ChangeKind::Renamed => "diff-renamed",
        ChangeKind::Conflict => "diff-conflict",
        ChangeKind::Special => "diff-special",
    }
}
impl GitTab {
    fn new(cwd: PathBuf, window: &mut Window, cx: &mut Context<HomeView>) -> Self {
        let owner = cx.weak_entity();
        let list = cx.new(|cx| {
            ListState::new(
                Delegate::new(owner, GIT_TAB.into(), Rc::new(Vec::new())),
                window,
                cx,
            )
            .searchable(false)
        });
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "diff-filter")));
        let subscription = cx.subscribe_in(
            &filter,
            window,
            |home, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) && home.active_git().is_some() {
                    home.filter_git(window, cx);
                }
            },
        );
        Self {
            cwd,
            source: Default::default(),
            repository: None,
            all: Vec::new(),
            rows: Rc::new(Vec::new()),
            selected: None,
            focus: list.focus_handle(cx),
            main_focus: cx.focus_handle(),
            header_focus: cx.focus_handle(),
            list,
            navigator_hidden: false,
            options: Default::default(),
            unfolded: HashSet::new(),
            preferences: HashMap::new(),
            filter,
            _filter_subscription: subscription,
            generation: 0,
            cache: HashMap::new(),
            loading: true,
            refreshing: false,
            error: None,
            body_error: None,
            not_repository: false,
            list_task: None,
            body_task: None,
            body_generation: 0,
            poll: None,
        }
    }
    fn sync_rows(&mut self, window: &mut Window, cx: &mut App) {
        let query = self.filter.read(cx).value().to_lowercase();
        self.rows = Rc::new(
            self.all
                .iter()
                .filter(|f| {
                    f.path().to_string_lossy().to_lowercase().contains(&query)
                        || f.previous_path()
                            .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(&query))
                })
                .map(|f| Row {
                    path: identity(self.source, f.path()),
                    label: f
                        .previous_path()
                        .map(|p| format!("{} → {}", p.display(), f.path().display()))
                        .unwrap_or_else(|| f.path().display().to_string()),
                    status: Some(kind_key(f.kind())),
                    stats: f.stats(),
                    failed: false,
                })
                .collect(),
        );
        if !self
            .rows
            .iter()
            .any(|r| Some(&r.path) == self.selected.as_ref())
        {
            self.selected = self.rows.first().map(|r| r.path.clone());
        }
        self.list.update(cx, |list, cx| {
            list.delegate_mut().files = self.rows.clone();
            let selected = self
                .rows
                .iter()
                .position(|r| Some(&r.path) == self.selected.as_ref());
            list.set_selected_index(
                selected.map(gpui_kit::component::IndexPath::new),
                window,
                cx,
            );
            cx.notify();
        });
    }
    pub fn focus_main(&self, window: &mut Window, cx: &mut App) {
        self.selected
            .as_ref()
            .and_then(|p| self.cache.get(p))
            .and_then(|l| l.block.as_ref())
            .and_then(|b| b.focus_handle(cx))
            .unwrap_or_else(|| self.main_focus.clone())
            .focus(window, cx);
    }
    pub fn focus_list(&self, window: &mut Window, cx: &mut App) {
        self.list
            .update(cx, |list, cx| list.scroll_to_selected_item(window, cx));
        self.focus.focus(window, cx);
    }
    pub(super) fn external_path(&self) -> Option<PathBuf> {
        let entry = self.selected_entry()?;
        entry
            .can_open_worktree()
            .then(|| {
                self.repository
                    .as_ref()
                    .map(|r| r.root().join(entry.path()))
            })
            .flatten()
    }
    pub(super) fn selected_entry(&self) -> Option<&Change> {
        self.all
            .iter()
            .find(|f| Some(identity(self.source, f.path())) == self.selected)
    }
}
impl HomeView {
    pub(super) fn active_git(&self) -> Option<&GitTab> {
        let view = self.session_view()?;
        (view.workspace.active.as_deref() == Some(GIT_TAB))
            .then_some(view.workspace.git.as_ref())
            .flatten()
    }
    pub(super) fn git_mut(&mut self) -> Option<&mut GitTab> {
        let view = self
            .shown_key
            .as_ref()
            .and_then(|k| self.views.get_mut(k))?;
        if view.workspace.active.as_deref() != Some(GIT_TAB) {
            return None;
        }
        view.workspace.git.as_mut()
    }
    pub(super) fn open_git_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.shown_key.clone() else {
            return;
        };
        let Some(cwd) = self.state.read(cx).current().map(|s| s.info().cwd.clone()) else {
            return;
        };
        let Some(view) = self.views.get_mut(&key) else {
            return;
        };
        if view.workspace.git.as_ref().is_none_or(|g| g.cwd != cwd) {
            view.workspace.git = Some(GitTab::new(cwd, window, cx));
        }
        view.workspace.activate(Some(GIT_TAB.into()), cx);
        view.workspace.headers.insert(
            Some(GIT_TAB.into()),
            view.workspace.git.as_ref().unwrap().header_focus.clone(),
        );
        self.ensure_panel(Panel::Workspace, window, cx);
        self.refresh_worktree(window, cx);
        self.focus_panel(Panel::Workspace, window, cx);
        cx.notify();
    }
    /// Called from lifecycle/sync hooks, never render. TestScheduler owns no Git I/O.
    pub(in crate::home) fn sync_worktree(&mut self, window: &Window, cx: &mut Context<Self>) {
        let cwd = self.state.read(cx).current().map(|s| s.info().cwd.clone());
        if let Some(cwd) = cwd
            && let Some(tab) = self.git_mut()
            && tab.cwd != cwd
        {
            tab.cwd = cwd;
            tab.generation += 1;
            tab.body_generation += 1;
            tab.list_task = None;
            tab.body_task = None;
            tab.poll = None;
            tab.repository = None;
            tab.all.clear();
            tab.rows = Rc::new(Vec::new());
            tab.selected = None;
            tab.cache.clear();
            tab.loading = true;
            tab.error = None;
            tab.body_error = None;
            tab.refreshing = false;
            tab.not_repository = false;
            tab.list.update(cx, |list, cx| {
                list.delegate_mut().files = Rc::new(Vec::new());
                cx.notify();
            });
        }
        let visible = self.shown_key == *self.state.read(cx).selected()
            && self.notification_visible
            && window.is_window_active()
            && window.is_visible()
            && self.pane_layout.visible(Panel::Workspace);
        for (key, view) in &mut self.views {
            if let Some(tab) = &mut view.workspace.git
                && (!visible
                    || Some(key) != self.shown_key.as_ref()
                    || view.workspace.active.as_deref() != Some(GIT_TAB))
            {
                tab.poll = None;
                tab.list_task = None;
                tab.body_task = None;
                tab.refreshing = false;
            }
        }
        if cx.is_test() || !visible {
            return;
        }
        let Some(key) = self.shown_key.clone() else {
            return;
        };
        let Some(tab) = self.git_mut() else {
            return;
        };
        if tab.poll.is_some() {
            return;
        }
        tab.poll = Some(cx.spawn_in(window, async move |home, cx| {
            loop {
                if !home
                    .update_in(cx, |home, window, cx| {
                        if home.shown_key.as_ref() != Some(&key)
                            || home.active_git().is_none()
                            || !home.notification_visible
                            || !window.is_visible()
                            || !window.is_window_active()
                            || !home.pane_layout.visible(Panel::Workspace)
                        {
                            return false;
                        }
                        home.refresh_worktree(window, cx);
                        true
                    })
                    .unwrap_or(false)
                {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(5)).await;
            }
        }));
    }
    fn refresh_worktree(&mut self, window: &Window, cx: &mut Context<Self>) {
        if cx.is_test() {
            return;
        }
        let Some(key) = self.shown_key.clone() else {
            return;
        };
        let Some(tab) = self.git_mut() else {
            return;
        };
        if tab.list_task.is_some() {
            return;
        }
        tab.refreshing = true;
        tab.error = None;
        let generation = tab.generation;
        let source = tab.source;
        let cwd = tab.cwd.clone();
        tab.list_task = Some(cx.spawn_in(window, async move |home, cx| {
            let result = cx
                .background_spawn(async move {
                    let Some(repo) = gupi_resources::git::discover(&cwd).await? else {
                        return Ok(None);
                    };
                    let list = repo.changes(source).await?;
                    Ok::<_, gupi_resources::git::Error>(Some((repo, list)))
                })
                .await;
            let _ = home.update_in(cx, |home, window, cx| {
                if home.shown_key.as_ref() != Some(&key) {
                    return;
                }
                let Some(tab) = home
                    .git_mut()
                    .filter(|t| t.generation == generation && t.source == source)
                else {
                    return;
                };
                tab.list_task = None;
                tab.refreshing = false;
                tab.loading = false;
                match result {
                    Ok(Some((repo, list))) => {
                        tab.body_task = None;
                        tab.body_error = None;
                        tab.body_generation += 1;
                        tab.repository = Some(repo);
                        tab.not_repository = false;
                        tab.all = list.files().to_vec();
                        tab.cache.retain(|path, body| {
                            tab.all.iter().any(|f| {
                                identity(tab.source, f.path()) == *path && *f == body.entry
                            })
                        });
                        tab.sync_rows(window, cx);
                    }
                    Ok(None) => {
                        tab.not_repository = true;
                        tab.repository = None;
                        tab.all.clear();
                        tab.cache.clear();
                        tab.sync_rows(window, cx);
                    }
                    Err(error) => tab.error = Some(error.into()),
                }
                home.load_git_file(window, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn load_git_file(&mut self, window: &Window, cx: &mut Context<Self>) {
        if cx.is_test() {
            return;
        }
        let Some(key) = self.shown_key.clone() else {
            return;
        };
        let Some(tab) = self.git_mut() else {
            return;
        };
        let Some(path) = tab.selected.clone() else {
            return;
        };
        if tab.cache.contains_key(&path) || tab.body_task.is_some() {
            return;
        }
        let Some(repo) = tab.repository.clone() else {
            return;
        };
        let Some(entry) = tab.selected_entry().cloned() else {
            return;
        };
        let generation = tab.generation;
        let source = tab.source;
        tab.body_error = None;
        tab.body_generation += 1;
        let body_generation = tab.body_generation;
        tab.body_task = Some(cx.spawn_in(window, async move |home, cx| {
            let expected = entry.clone();
            let result = cx
                .background_spawn(async move { repo.change_content(source, &entry).await })
                .await;
            let _ = home.update_in(cx, |home, window, cx| {
                if home.shown_key.as_ref() != Some(&key) {
                    return;
                }
                let Some(tab) = home.git_mut().filter(|t| {
                    t.generation == generation
                        && t.body_generation == body_generation
                        && t.source == source
                        && t.selected.as_ref() == Some(&path)
                        && t.selected_entry() == Some(&expected)
                }) else {
                    return;
                };
                tab.body_task = None;
                match result {
                    Ok(content) => {
                        let block = content
                            .patch()
                            .filter(|p| !p.is_empty())
                            .map(|p| DiffBlock::patch(p, cx));
                        if tab.cache.len() >= CACHE_FILES
                            && let Some(old) = tab.cache.keys().find(|k| **k != path).cloned()
                        {
                            tab.cache.remove(&old);
                        }
                        tab.cache.insert(
                            path,
                            Loaded {
                                entry: expected,
                                content,
                                block,
                            },
                        );
                    }
                    Err(error) => tab.body_error = Some(error.to_string()),
                }
                if tab.main_focus.is_focused(window) {
                    tab.focus_main(window, cx);
                }
                home.apply_reader_options(cx);
                cx.notify();
            });
        }));
    }
    pub(super) fn select_git_file(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.git_mut()
            && tab.rows.iter().any(|r| r.path == path)
        {
            tab.body_task = None;
            tab.body_generation += 1;
            tab.body_error = None;
            tab.selected = Some(path);
            tab.sync_rows(window, cx);
            tab.focus_main(window, cx);
        }
        self.load_git_file(window, cx);
        self.apply_reader_options(cx);
        cx.notify();
    }
    pub(super) fn filter_git(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.git_mut() {
            tab.body_task = None;
            tab.body_generation += 1;
            tab.body_error = None;
            tab.sync_rows(window, cx);
        }
        self.load_git_file(window, cx);
        cx.notify();
    }
    fn change_git_source(
        &mut self,
        source: ChangeSource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.git_mut() {
            if tab.source == source {
                return;
            }
            tab.preferences.insert(tab.source, tab.options);
            tab.options = tab.preferences.get(&source).copied().unwrap_or_default();
            tab.source = source;
            tab.generation += 1;
            tab.list_task = None;
            tab.body_task = None;
            tab.cache.clear();
            tab.unfolded.clear();
            tab.all.clear();
            tab.selected = None;
            tab.loading = true;
            tab.error = None;
            tab.body_error = None;
            tab.not_repository = false;
            tab.sync_rows(window, cx);
        }
        self.refresh_worktree(window, cx);
        cx.notify();
    }
    pub(super) fn render_git_list(&self, tab: &GitTab) -> AnyElement {
        v_flex()
            .size_full()
            .min_h_0()
            .p_2()
            .gap_2()
            .child(Input::new(&tab.filter).small())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(gpui_kit::component::list::List::new(&tab.list)),
            )
            .into_any_element()
    }
    pub(super) fn render_git_main(&self, tab: &GitTab, cx: &mut Context<Self>) -> AnyElement {
        let mut source = h_flex().flex_wrap().gap_1().px_2().py_1();
        for value in [
            ChangeSource::Unstaged,
            ChangeSource::Staged,
            ChangeSource::Untracked,
        ] {
            source = source.child(
                Button::new(source_key(value))
                    .ghost()
                    .small()
                    .selected(tab.source == value)
                    .label(t(cx, source_key(value)))
                    .on_click(cx.listener(move |home, _, window, cx| {
                        home.change_git_source(value, window, cx)
                    })),
            );
        }
        source = source
            .child(
                Button::new("git-refresh")
                    .ghost()
                    .small()
                    .icon(IconName::RefreshCw)
                    .disabled(tab.refreshing)
                    .tooltip(t(cx, "diff-refresh"))
                    .accessibility_label(t(cx, "diff-refresh"))
                    .on_click(cx.listener(|home, _, window, cx| home.refresh_worktree(window, cx))),
            )
            .child(self.navigator_toggle(cx));
        let message = if tab.loading {
            Some("diff-loading")
        } else if tab.not_repository {
            Some("diff-not-repository")
        } else if tab.error.is_some() && tab.repository.is_none() {
            None
        } else if tab.selected.is_none() {
            Some(if tab.all.is_empty() {
                "diff-empty"
            } else {
                "diff-no-results"
            })
        } else {
            None
        };
        let mut body = v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .track_focus(&tab.main_focus)
            .key_context("GupiWorkspaceDiff")
            .on_action(
                cx.listener(|home, _: &actions::ReturnFromSource, window, cx| {
                    if home.pane_layout.maximized {
                        home.toggle_workspace_maximized(window, cx);
                    } else if home.pane_layout.overlay == Some(Panel::Workspace) {
                        home.close_navigator(window, cx);
                    } else if let Some(tab) = home.active_git().filter(|t| !t.navigator_hidden) {
                        tab.focus_list(window, cx);
                    } else {
                        home.focus_composer(window, cx);
                    }
                    cx.stop_propagation();
                }),
            )
            .child(div().px_3().pt_2().text_sm().child(t(cx, "diff-worktree")))
            .child(source)
            .child(self.reader_toolbar(cx));
        if tab.refreshing && !tab.loading {
            body = body.child(div().px_3().text_xs().child(t(cx, "diff-refreshing")));
        }
        if let Some(error) = &tab.error {
            let text = match error {
                Failure::Unavailable => t(cx, "diff-git-unavailable"),
                Failure::Query(message) => format!("{} {message}", t(cx, "diff-refresh-failed")),
            };
            body = body.child(
                div()
                    .px_3()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(text),
            );
        }
        if let Some(message) = message {
            body = body.child(
                div()
                    .p_4()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, message)),
            );
        } else if let Some(entry) = tab.selected_entry() {
            body = body.child(
                div().px_3().py_2().text_sm().child(
                    entry
                        .previous_path()
                        .map(|p| format!("{} → {}", p.display(), entry.path().display()))
                        .unwrap_or_else(|| entry.path().display().to_string()),
                ),
            );
            if let Some(error) = &tab.body_error {
                body = body.child(
                    div().p_3().text_sm().child(error.clone()).child(
                        Button::new("git-retry")
                            .small()
                            .label(t(cx, "diff-retry"))
                            .on_click(cx.listener(|home, _, window, cx| {
                                if let Some(tab) = home.git_mut() {
                                    tab.body_task = None;
                                }
                                home.refresh_worktree(window, cx);
                                home.load_git_file(window, cx);
                            })),
                    ),
                );
            } else if let Some(loaded) = tab.selected.as_ref().and_then(|p| tab.cache.get(p)) {
                body = body.child(
                    div()
                        .px_3()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "diff-current-file-note"))
                        .when(loaded.content.disk_path().is_none(), |view| {
                            view.child(t(cx, "diff-unavailable-file"))
                        }),
                );
                if tab.options.collapsed
                    && !tab
                        .selected
                        .as_ref()
                        .is_some_and(|p| tab.unfolded.contains(p))
                {
                    body = body.child(
                        Button::new("diff-expand-file")
                            .small()
                            .label(t(cx, "diff-expand-file"))
                            .on_click(cx.listener(|home, _, window, cx| {
                                home.reader_command(Command::ExpandFile, window, cx)
                            })),
                    );
                }
                if let Some(block) = &loaded.block {
                    body = body.child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .child(block.render_reader(tab.options.wrap)),
                    );
                } else {
                    let key = match loaded.content.state() {
                        ContentState::Binary => "diff-binary",
                        ContentState::TooLarge => "diff-too-large",
                        ContentState::Conflict => "diff-conflict",
                        ContentState::Special => "diff-special",
                        ContentState::Renamed => "diff-renamed",
                        ContentState::Text => "diff-empty-file",
                    };
                    body = body.child(div().p_3().text_sm().child(t(cx, key)));
                }
            } else {
                body = body.child(div().p_3().text_sm().child(t(cx, "diff-loading")));
            }
        }
        body.into_any_element()
    }
}

#[cfg(test)]
mod tests;
