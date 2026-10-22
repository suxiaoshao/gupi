//! Per-session worktree adapter. It never impersonates recorded tool operations.
use super::*;
mod navigator;
mod projection;
use changes::navigator::Row;
use gpui_kit::component::{input::Input, menu::DropdownMenu};
use gupi_resources::git::{
    Change, ChangeContent, ChangeKind, ChangeList, ChangeSource, ContentState, Repository,
};
use gupi_settings::layout::GitView;
use messages::diff::DiffBlock;
use navigator::{Delegate, Item};
use projection::Directory;
use reader::Options;
use std::time::Duration;

pub(super) const GIT_TAB: &str = "workspace:git";
const CACHE_FILES: usize = 8;
const HEADER_REM: f32 = 2.5;
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    source: ChangeSource,
    file: Change,
}
impl Entry {
    fn id(&self) -> String {
        identity(self.source, self.file.path())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Group {
    Conflicts,
    Staged,
    Unstaged,
}
impl Group {
    fn key(self) -> &'static str {
        match self {
            Self::Conflicts => "diff-conflicts",
            Self::Staged => "diff-staged",
            Self::Unstaged => "diff-unstaged",
        }
    }
    fn of(entry: &Entry) -> Self {
        if entry.file.kind() == ChangeKind::Conflict {
            Self::Conflicts
        } else if entry.source == ChangeSource::Staged {
            Self::Staged
        } else {
            Self::Unstaged
        }
    }
}

pub(super) struct Loaded {
    entry: Change,
    pub content: ChangeContent,
    pub block: Option<DiffBlock>,
}
impl Loaded {
    fn new(entry: Change, content: ChangeContent, previous: Option<Self>, cx: &mut App) -> Self {
        let block =
            if content.state() == ContentState::Text && content.original() != content.modified() {
                if previous
                    .as_ref()
                    .is_some_and(|old| old.content.patch() == content.patch())
                {
                    previous.and_then(|old| old.block)
                } else {
                    content
                        .patch()
                        .filter(|patch| !patch.is_empty())
                        .map(|patch| DiffBlock::patch(patch, cx))
                }
            } else {
                None
            };
        Self {
            entry,
            content,
            block,
        }
    }
    pub(super) fn copy_content(&self) -> Option<&str> {
        if self.entry.kind() == ChangeKind::Deleted {
            self.content.original()
        } else {
            self.content.modified()
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
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
    repository: Option<Repository>,
    all: Vec<Entry>,
    group_errors: HashMap<ChangeSource, Failure>,
    groups_closed: HashSet<Group>,
    directories_closed: HashSet<Directory>,
    // Identifies the derived projection, never an independent preference.
    projection_view: GitView,
    order: Vec<String>,
    known_sources: HashSet<ChangeSource>,
    pub rows: Rc<Vec<Row>>,
    pub selected: Option<String>,
    pub list: Entity<ListState<Delegate>>,
    pub focus: FocusHandle,
    pub main_focus: FocusHandle,
    pub header_focus: FocusHandle,
    pub navigator_hidden: bool,
    pub options: Options,
    pub collapsed: HashSet<String>,
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
    body_target: Option<(String, Change)>,
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
        ChangeKind::Conflict => "diff-conflict",
        ChangeKind::Special => "diff-special",
    }
}
impl GitTab {
    fn new(cwd: PathBuf, window: &mut Window, cx: &mut Context<HomeView>) -> Self {
        let owner = cx.weak_entity();
        let list = cx.new(|cx| ListState::new(Delegate::new(owner), window, cx).searchable(false));
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
            repository: None,
            all: Vec::new(),
            group_errors: HashMap::new(),
            groups_closed: HashSet::new(),
            directories_closed: HashSet::new(),
            projection_view: cx.global::<gupi_settings::layout::LayoutState>().git_view,
            order: Vec::new(),
            known_sources: HashSet::new(),
            rows: Rc::new(Vec::new()),
            selected: None,
            focus: list.focus_handle(cx),
            main_focus: cx.focus_handle(),
            header_focus: cx.focus_handle(),
            list,
            navigator_hidden: false,
            options: Default::default(),
            collapsed: HashSet::new(),
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
            body_target: None,
            poll: None,
        }
    }
    fn sync_rows(&mut self, window: &mut Window, cx: &mut App) {
        let query = self.filter.read(cx).value().to_lowercase();
        let view = cx.global::<gupi_settings::layout::LayoutState>().git_view;
        self.projection_view = view;
        let (items, order) = projection::build(
            &self.all,
            view,
            &self.groups_closed,
            &self.directories_closed,
            &query,
        );
        self.order = order;
        let rows: Vec<Row> = items
            .iter()
            .filter_map(|item| match item {
                Item::File(row, _, _) => Some(row.clone()),
                _ => None,
            })
            .collect();
        // Filtering changes the candidate set; collapsing a group alone keeps
        // its selected body, while removing a record picks its nearest neighbor.
        let eligible = self.selected.as_ref().is_some_and(|id| {
            self.all.iter().any(|entry| {
                entry.id() == *id
                    && (entry
                        .file
                        .path()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query))
            })
        });
        if !eligible {
            self.selected = rows.first().map(|r| r.path.clone());
        }
        self.rows = Rc::new(rows);
        self.list.update(cx, |list, cx| {
            let selected = items.iter().position(|item| matches!(item, Item::File(row, _, _) if Some(&row.path) == self.selected.as_ref()));
            list.delegate_mut().items = items;
            list.set_selected_index(selected.map(gpui_kit::component::IndexPath::new), window, cx);
            cx.notify();
        });
    }
    fn apply_files(
        &mut self,
        results: Vec<(ChangeSource, Vec<Change>, Option<Failure>)>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let mut next = self.all.clone();
        let mut errors = self.group_errors.clone();
        let known_before = self.known_sources.clone();
        for (source, files, error) in results {
            self.known_sources.insert(source);
            next.retain(|entry| entry.source != source);
            next.extend(files.into_iter().map(|file| Entry { source, file }));
            if let Some(error) = error {
                errors.insert(source, error);
            } else {
                errors.remove(&source);
            }
        }
        next.sort_by(|a, b| {
            source_order(a.source)
                .cmp(&source_order(b.source))
                .then_with(|| a.file.path().cmp(b.file.path()))
        });
        if next == self.all && errors == self.group_errors && known_before == self.known_sources {
            return false;
        }
        let previous = self.selected.clone();
        let position = self
            .rows
            .iter()
            .position(|r| Some(&r.path) == previous.as_ref())
            .unwrap_or(0);
        self.all = next;
        let directories = projection::directories(&self.all);
        self.directories_closed
            .retain(|key| directories.contains(key));
        self.group_errors = errors;
        self.cache
            .retain(|id, _| self.all.iter().any(|e| e.id() == *id));
        self.collapsed
            .retain(|id| self.all.iter().any(|e| e.id() == *id));
        self.sync_rows(window, cx);
        if previous
            .as_ref()
            .is_some_and(|id| !self.all.iter().any(|e| e.id() == *id))
        {
            self.selected = self
                .rows
                .get(position.min(self.rows.len().saturating_sub(1)))
                .map(|r| r.path.clone());
            self.sync_rows(window, cx);
        }
        let target_valid = self.body_target.as_ref().is_none_or(|(id, file)| {
            self.selected.as_ref() == Some(id) && self.selected_entry() == Some(file)
        });
        if !target_valid || previous != self.selected {
            self.invalidate_body();
        }
        true
    }
    fn apply_refresh(
        &mut self,
        result: Result<Option<(Repository, [ChangeList; 3])>, gupi_resources::git::Error>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.list_task = None;
        self.refreshing = false;
        self.loading = false;
        match result {
            Ok(Some((repo, results))) => {
                if self.repository.as_ref().is_some_and(|old| *old != repo) {
                    self.all.clear();
                    self.directories_closed.clear();
                    self.groups_closed.clear();
                    self.cache.clear();
                    self.invalidate_body();
                }
                self.repository = Some(repo);
                self.error = None;
                self.not_repository = false;
                self.apply_files(
                    results
                        .into_iter()
                        .map(|list| {
                            let (source, files, error) = list.into_parts();
                            (source, files, error.map(Failure::from))
                        })
                        .collect(),
                    window,
                    cx,
                );
            }
            Ok(None) => {
                self.directories_closed.clear();
                self.groups_closed.clear();
                self.not_repository = true;
                self.repository = None;
                self.all.clear();
                self.cache.clear();
                self.group_errors.clear();
                self.invalidate_body();
                self.sync_rows(window, cx);
            }
            Err(error) => self.error = Some(error.into()),
        }
    }
    pub(super) fn neighbors(&self) -> (Option<String>, Option<String>) {
        let Some(ix) = self
            .order
            .iter()
            .position(|id| Some(id) == self.selected.as_ref())
        else {
            return (None, None);
        };
        let visible = |id: &&String| self.rows.iter().any(|row| &row.path == *id);
        (
            self.order[..ix].iter().rev().find(visible).cloned(),
            self.order[ix + 1..].iter().find(visible).cloned(),
        )
    }
    fn select_navigation(&self, id: &str, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            let ix = list
                .delegate()
                .items
                .iter()
                .position(|item| item.id() == id);
            list.set_selected_index(ix.map(gpui_kit::component::IndexPath::new), window, cx);
            list.scroll_to_selected_item(window, cx);
        });
    }
    fn reveal_selected(&mut self) {
        if let Some(entry) = self
            .all
            .iter()
            .find(|entry| Some(&entry.id()) == self.selected.as_ref())
        {
            let group = Group::of(entry);
            self.groups_closed.remove(&group);
            for path in entry.file.path().ancestors().skip(1) {
                self.directories_closed.remove(&Directory {
                    group,
                    path: path.into(),
                });
            }
        }
    }
    fn invalidate_body(&mut self) {
        self.body_task = None;
        self.body_target = None;
        self.body_generation += 1;
        self.body_error = None;
    }
    fn selected_source(&self) -> Option<ChangeSource> {
        self.all
            .iter()
            .find(|e| Some(&e.id()) == self.selected.as_ref())
            .map(|e| e.source)
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
    pub(super) fn can_fold(&self) -> bool {
        // The retained snapshot, not the incoming entry, owns the displayed body.
        !self.loading
            && !self.not_repository
            && self
                .selected
                .as_ref()
                .and_then(|id| self.cache.get(id))
                .is_some_and(|loaded| loaded.block.is_some())
    }
    pub(super) fn selected_entry(&self) -> Option<&Change> {
        self.all
            .iter()
            .find(|e| Some(&e.id()) == self.selected.as_ref())
            .map(|e| &e.file)
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
    pub(in crate::home) fn open_git_changes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            tab.body_target = None;
            tab.poll = None;
            tab.repository = None;
            tab.all.clear();
            tab.directories_closed.clear();
            tab.groups_closed.clear();
            tab.order.clear();
            tab.group_errors.clear();
            tab.known_sources.clear();
            tab.rows = Rc::new(Vec::new());
            tab.selected = None;
            tab.cache.clear();
            tab.loading = true;
            tab.error = None;
            tab.body_error = None;
            tab.refreshing = false;
            tab.not_repository = false;
            tab.list.update(cx, |list, cx| {
                list.delegate_mut().items.clear();
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
                tab.body_target = None;
                tab.refreshing = false;
            }
        }
        if cx.is_test() || !visible {
            return;
        }
        let Some(tab) = self.git_mut() else {
            return;
        };
        if tab.poll.is_some() {
            return;
        }
        // sync_worktree alone owns eligibility and cancels the task when it changes.
        tab.poll = Some(cx.spawn_in(window, async move |home, cx| {
            loop {
                let _ = home.update_in(cx, |home, window, cx| {
                    home.refresh_worktree(window, cx);
                });
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
        let generation = tab.generation;
        let cwd = tab.cwd.clone();
        tab.list_task = Some(cx.spawn_in(window, async move |home, cx| {
            let result = cx
                .background_spawn(async move {
                    let Some(repo) = gupi_resources::git::discover(&cwd).await? else {
                        return Ok(None);
                    };
                    let results = repo.changes().await?;
                    Ok::<_, gupi_resources::git::Error>(Some((repo, results)))
                })
                .await;
            let _ = home.update_in(cx, |home, window, cx| {
                if home.shown_key.as_ref() != Some(&key) {
                    return;
                }
                let Some(tab) = home.git_mut().filter(|t| t.generation == generation) else {
                    return;
                };
                tab.apply_refresh(result, window, cx);
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
        if tab
            .cache
            .get(&path)
            .is_some_and(|loaded| Some(&loaded.entry) == tab.selected_entry())
            || tab.body_task.is_some()
        {
            return;
        }
        let Some(repo) = tab.repository.clone() else {
            return;
        };
        let Some(entry) = tab.selected_entry().cloned() else {
            return;
        };
        let generation = tab.generation;
        let source = tab.selected_source().unwrap();
        tab.body_target = Some((path.clone(), entry.clone()));
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
                        && t.selected.as_ref() == Some(&path)
                        && t.selected_entry() == Some(&expected)
                }) else {
                    return;
                };
                tab.body_task = None;
                tab.body_target = None;
                match result {
                    Ok(content) => {
                        let old = tab.cache.remove(&path);
                        if tab.cache.len() >= CACHE_FILES
                            && let Some(old) = tab.cache.keys().find(|k| **k != path).cloned()
                        {
                            tab.cache.remove(&old);
                        }
                        tab.cache
                            .insert(path, Loaded::new(expected, content, old, cx));
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
        self.set_temporary_list(false);
        self.fit_workspace_layout(window, cx);
        if let Some(tab) = self.git_mut()
            && tab.rows.iter().any(|r| r.path == path)
        {
            if tab.selected.as_ref() != Some(&path) {
                tab.invalidate_body();
                tab.selected = Some(path);
                tab.sync_rows(window, cx);
            }
            tab.focus_main(window, cx);
        }
        self.load_git_file(window, cx);
        self.apply_reader_options(cx);
        cx.notify();
    }
    pub(super) fn filter_git(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.git_mut() {
            let previous = tab.selected.clone();
            tab.sync_rows(window, cx);
            if previous != tab.selected {
                tab.invalidate_body();
            }
        }
        self.load_git_file(window, cx);
        cx.notify();
    }
    fn toggle_git_group(&mut self, group: Group, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.git_mut() {
            if !tab.groups_closed.remove(&group) {
                tab.groups_closed.insert(group);
            }
            tab.sync_rows(window, cx);
            tab.list.update(cx, |list, cx| {
                let ix = list
                    .delegate()
                    .items
                    .iter()
                    .position(|item| matches!(item, Item::Group(value,_,_) if *value == group));
                list.set_selected_index(ix.map(gpui_kit::component::IndexPath::new), window, cx);
            });
        }
        cx.notify();
    }
    pub(in crate::home) fn sync_git_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.global::<gupi_settings::layout::LayoutState>().git_view;
        for session in self.views.values_mut() {
            if let Some(tab) = &mut session.workspace.git
                && tab.projection_view != view
            {
                if view == GitView::Tree {
                    tab.reveal_selected();
                }
                tab.sync_rows(window, cx);
                tab.list
                    .update(cx, |list, cx| list.scroll_to_selected_item(window, cx));
            }
        }
        cx.notify();
    }
    fn set_git_view(&mut self, view: GitView, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<gupi_settings::layout::LayoutState>().git_view == view {
            return;
        }
        cx.update_global::<gupi_settings::layout::LayoutState, _>(|layout, _| {
            layout.git_view = view
        });
        self.sync_git_view(window, cx);
        self.save_layout(None, None, window, cx);
    }
    fn toggle_git_directory(
        &mut self,
        key: Directory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.git_mut() {
            if !tab.filter.read(cx).value().is_empty() {
                return;
            }
            if !tab.directories_closed.remove(&key) {
                tab.directories_closed.insert(key.clone());
            }
            tab.sync_rows(window, cx);
            tab.select_navigation(&key.id(), window, cx);
        }
        cx.notify();
    }
    fn git_horizontal(&mut self, right: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_git() else {
            return;
        };
        let list = tab.list.read(cx).delegate();
        let Some(item) = list.index.and_then(|ix| list.items.get(ix)) else {
            return;
        };
        let id = item.id();
        let (closed, parent) = match item {
            Item::Group(_, _, closed) => (Some(*closed), None),
            Item::Directory(key, _, closed, _) => (Some(*closed), Some(key.parent_id())),
            Item::File(row, _, depth) => {
                let entry = tab.all.iter().find(|e| e.id() == row.path).unwrap();
                let parent = if *depth > 1 {
                    Directory {
                        group: Group::of(entry),
                        path: entry.file.path().parent().unwrap().into(),
                    }
                    .id()
                } else {
                    format!("git-group-{}", Group::of(entry).key())
                };
                (None, Some(parent))
            }
        };
        if closed == Some(right) {
            let action = match item {
                Item::Group(group, _, _) => Some((*group, None)),
                Item::Directory(key, _, _, _) => Some((key.group, Some(key.clone()))),
                _ => None,
            };
            if let Some((group, directory)) = action {
                if let Some(key) = directory {
                    self.toggle_git_directory(key, window, cx);
                } else {
                    self.toggle_git_group(group, window, cx);
                }
            }
        } else if !right {
            if let Some(parent) = parent {
                tab.select_navigation(&parent, window, cx);
            }
        } else if closed == Some(false) {
            let next = list
                .items
                .iter()
                .position(|item| item.id() == id)
                .and_then(|ix| list.items.get(ix + 1))
                .filter(|next| next.depth() > item.depth())
                .map(Item::id);
            if let Some(next) = next {
                tab.select_navigation(&next, window, cx);
            }
        }
        cx.notify();
    }
    fn git_view_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.weak_entity();
        Button::new("git-view")
            .ghost()
            .small()
            .label(t(cx, "diff-view"))
            .icon(IconName::ChevronDown)
            .tooltip(t(cx, "diff-view"))
            .accessibility_label(t(cx, "diff-view"))
            .dropdown_menu(move |menu, _, cx| {
                let selected = cx.global::<gupi_settings::layout::LayoutState>().git_view;
                [
                    (GitView::List, "diff-list-view"),
                    (GitView::Tree, "diff-tree-view"),
                ]
                .into_iter()
                .fold(menu, |menu, (view, key)| {
                    let owner = owner.clone();
                    menu.item(
                        PopupMenuItem::new(t(cx, key))
                            .checked(selected == view)
                            .on_click(move |_, window, cx| {
                                let _ = owner
                                    .update(cx, |home, cx| home.set_git_view(view, window, cx));
                            }),
                    )
                })
            })
    }
    pub(super) fn render_git_list(&self, tab: &GitTab, cx: &mut Context<Self>) -> AnyElement {
        let stats = total_stats(tab.all.iter().map(|e| e.file.stats()));
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(rems(HEADER_REM))
                    .flex_none()
                    .px_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().text_sm().child(t(cx, "diff-files")))
                    .when_some(stats, |row, (added, removed)| {
                        row.child(reader::line_stats(added, removed, cx))
                    })
                    .child(div().flex_1())
                    .child(self.git_view_button(cx))
                    .child(self.git_refresh_button(tab, cx)),
            )
            .child(div().p_2().child(Input::new(&tab.filter).small()))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .on_key_down(cx.listener(|home, event: &KeyDownEvent, window, cx| {
                        if home
                            .active_git()
                            .is_some_and(|t| t.focus.contains_focused(window, cx))
                        {
                            match event.keystroke.key.as_str() {
                                "left" | "right" => {
                                    home.git_horizontal(event.keystroke.key == "right", window, cx);
                                    cx.stop_propagation();
                                }
                                _ => {}
                            }
                        }
                    }))
                    .child(gpui_kit::component::list::List::new(&tab.list)),
            )
            .into_any_element()
    }
    fn git_refresh_button(&self, tab: &GitTab, cx: &mut Context<Self>) -> Button {
        let detail = tab
            .error
            .as_ref()
            .or_else(|| tab.group_errors.values().next())
            .map(|error| match error {
                Failure::Unavailable => t(cx, "diff-git-unavailable"),
                Failure::Query(message) => format!("{} {message}", t(cx, "diff-refresh-failed")),
            });
        Button::new("git-refresh")
            .ghost()
            .small()
            .icon(if detail.is_some() {
                IconName::CircleAlert
            } else {
                IconName::RefreshCw
            })
            .loading(tab.refreshing)
            .tooltip(detail.unwrap_or_else(|| t(cx, "diff-refresh")))
            .accessibility_label(t(cx, "diff-refresh"))
            .on_click(cx.listener(|home, _, window, cx| home.refresh_worktree(window, cx)))
    }
    pub(super) fn render_git_main(&self, tab: &GitTab, cx: &mut Context<Self>) -> AnyElement {
        let entry = tab.selected_entry();
        let path = entry
            .map(|e| e.path().display().to_string())
            .unwrap_or_else(|| t(cx, "diff-worktree"));
        let source = tab
            .selected_source()
            .map(|s| t(cx, source_key(s)))
            .unwrap_or_default();
        let direction = tab
            .selected_source()
            .map(|s| {
                t(
                    cx,
                    match s {
                        ChangeSource::Staged => "diff-direction-staged",
                        ChangeSource::Unstaged => "diff-direction-unstaged",
                        ChangeSource::Untracked => "diff-direction-untracked",
                    },
                )
            })
            .unwrap_or_default();
        let updating = tab
            .selected
            .as_ref()
            .and_then(|p| tab.cache.get(p))
            .is_some_and(|loaded| Some(&loaded.entry) != entry);
        let path_tooltip = format!("{path} · {source} · {direction}");
        let header = h_flex()
            .id("git-file-header")
            .test_support()
            .debug_selector(|| "git-file-header".into())
            .h(rems(HEADER_REM))
            .flex_none()
            .min_w_0()
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.reader_fold_button(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .truncate()
                    .id("git-path")
                    .tooltip(move |window, cx| Tooltip::new(path_tooltip.clone()).build(window, cx))
                    .child(path),
            )
            .child(
                div()
                    .max_w_40()
                    .min_w_0()
                    .text_xs()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(if tab.body_error.is_some() {
                        t(cx, "diff-update-failed")
                    } else if updating {
                        t(cx, "diff-updating")
                    } else {
                        direction
                    }),
            )
            .when(!self.workspace_layout.navigator_visible(), |row| {
                row.child(self.git_refresh_button(tab, cx))
            })
            .child(self.reader_controls(cx))
            .child(self.navigator_toggle(cx));
        let mut content = v_flex()
            .id("git-body")
            .test_support()
            .debug_selector(|| "git-body".into())
            .flex_1()
            .min_h_0()
            .min_w_0();
        let message = if tab.loading {
            Some("diff-loading")
        } else if tab.not_repository {
            Some("diff-not-repository")
        } else if tab.selected.is_none() {
            Some(if tab.all.is_empty() {
                "diff-empty"
            } else {
                "diff-no-results"
            })
        } else {
            None
        };
        if let Some(error) = &tab.body_error
            && !tab
                .selected
                .as_ref()
                .is_some_and(|id| tab.cache.contains_key(id))
        {
            content = content.child(
                div().p_3().text_sm().child(error.clone()).child(
                    Button::new("git-retry")
                        .small()
                        .label(t(cx, "diff-retry"))
                        .on_click(cx.listener(|home, _, window, cx| {
                            if let Some(tab) = home.git_mut() {
                                tab.invalidate_body();
                            }
                            home.refresh_worktree(window, cx);
                            home.load_git_file(window, cx);
                        })),
                ),
            );
        }
        if tab.body_error.is_some()
            && !tab
                .selected
                .as_ref()
                .is_some_and(|id| tab.cache.contains_key(id))
        {
            // The error body above owns this slot until retry; no loading row.
        } else if let Some(message) = message {
            if !tab.loading
                && tab.all.is_empty()
                && (tab.error.is_some() || !tab.group_errors.is_empty())
            {
                content = content.child(div().p_3().text_sm().child(t(cx, "diff-refresh-failed")));
            } else {
                content = content.child(div().p_3().text_sm().child(t(cx, message)));
            }
        } else if let Some(loaded) = tab.selected.as_ref().and_then(|p| tab.cache.get(p)) {
            if let Some(block) = &loaded.block {
                content = content.child(block.render_reader(tab.options.wrap));
            } else {
                let key = match loaded.content.state() {
                    ContentState::Binary => "diff-binary",
                    ContentState::TooLarge => "diff-too-large",
                    ContentState::Conflict => "diff-conflict",
                    ContentState::Special => "diff-special",
                    ContentState::InvalidPath => "diff-invalid-path",
                    ContentState::NoTextChanges => "diff-no-text-changes",
                    ContentState::Text => "diff-empty-file",
                };
                content = content.child(
                    div()
                        .id("git-content-state")
                        .test_support()
                        .role(Role::Status)
                        .aria_label(t(cx, key))
                        .p_3()
                        .text_sm()
                        .child(t(cx, key)),
                );
            }
        } else {
            content = content.child(div().p_3().text_sm().child(t(cx, "diff-loading")));
        }
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .track_focus(&tab.main_focus)
            .key_context("GupiWorkspaceDiff")
            .on_action(
                cx.listener(|home, _: &actions::ReturnFromSource, window, cx| {
                    home.return_from_source(window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(header)
            .child(content)
            .into_any_element()
    }
}
// Like Zed, show only nonzero known-text totals; unknown entries contribute nothing.
fn total_stats(stats: impl IntoIterator<Item = Option<(usize, usize)>>) -> Option<(usize, usize)> {
    let total = stats
        .into_iter()
        .flatten()
        .fold((0, 0), |(a, r), (x, y)| (a + x, r + y));
    (total != (0, 0)).then_some(total)
}
fn source_order(source: ChangeSource) -> usize {
    match source {
        ChangeSource::Staged => 0,
        ChangeSource::Unstaged => 1,
        ChangeSource::Untracked => 2,
    }
}
fn status_code(entry: &Entry) -> &'static str {
    if entry.source == ChangeSource::Untracked {
        "U"
    } else {
        match entry.file.kind() {
            ChangeKind::Added => "A",
            ChangeKind::Deleted => "D",
            ChangeKind::Modified => "M",
            ChangeKind::Conflict => "!",
            ChangeKind::Special => "?",
        }
    }
}
#[cfg(test)]
mod tests;
