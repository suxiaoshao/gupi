mod navigator;

use super::*;
use gpui_kit::component::Icon;
use messages::diff::DiffBlock;
use messages::edited_files::{ChangeBody, EditedFile, EditedFiles};
use messages::tools::ToolIndex;

pub(in crate::home) struct ChangesTab {
    pub id: String,
    pub run: String,
    pub title: String,
    pub timestamp: Option<i64>,
    pub ids: HashSet<String>,
    pub navigator_hidden: bool,
    pub in_scope: bool,
    pub files: Rc<Vec<EditedFile>>,
    pub selected: Option<String>,
    pub focus: FocusHandle,
    pub main_focus: FocusHandle,
    pub header_focus: FocusHandle,
    list: Entity<ListState<navigator::Delegate>>,
    states: HashMap<String, FileDiff>,
}
struct FileDiff {
    combined: Option<DiffBlock>,
    operations: HashMap<String, DiffBlock>,
    page: usize,
}
impl ChangesTab {
    pub fn new(
        run: &str,
        title: String,
        timestamp: Option<i64>,
        files: &[EditedFile],
        path: &str,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let ids: HashSet<_> = files
            .iter()
            .flat_map(|f| f.operations().iter().map(|op| op.id().to_owned()))
            .collect();
        let id = files
            .iter()
            .flat_map(|f| f.operations())
            .next()
            .map(|op| op.id())
            .unwrap_or(run)
            .to_owned();
        let files = Rc::new(files.to_vec());
        let owner = cx.weak_entity();
        let list = cx.new(|inner| {
            ListState::new(
                navigator::Delegate::new(owner, id.clone(), files.clone()),
                window,
                inner,
            )
            .searchable(false)
        });
        let mut tab = Self {
            id,
            run: run.to_owned(),
            title,
            timestamp,
            ids,
            navigator_hidden: false,
            in_scope: true,
            files,
            selected: Some(path.to_owned()),
            focus: list.focus_handle(cx),
            main_focus: cx.focus_handle(),
            header_focus: cx.focus_handle(),
            list,
            states: HashMap::new(),
        };
        tab.prepare(cx);
        tab.sync_list(window, cx);
        tab
    }
    pub fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx) || self.main_focus.contains_focused(window, cx)
    }
    pub fn focus_list(&self, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            let ix = self
                .files
                .iter()
                .position(|file| Some(file.path()) == self.selected.as_deref());
            list.set_selected_index(ix.map(gpui_kit::component::IndexPath::new), window, cx);
            list.scroll_to_selected_item(window, cx);
        });
        self.focus.focus(window, cx);
    }
    fn sync_list(&self, window: &mut Window, cx: &mut App) {
        self.list.update(cx, |list, cx| {
            let delegate = list.delegate_mut();
            delegate.files = if self.in_scope {
                self.files.clone()
            } else {
                Rc::new(Vec::new())
            };
            let ix = delegate
                .files
                .iter()
                .position(|file| Some(file.path()) == self.selected.as_deref());
            list.set_selected_index(ix.map(gpui_kit::component::IndexPath::new), window, cx);
            cx.notify();
        });
    }
    pub fn focus_main(&self, window: &mut Window, cx: &mut App) {
        let state = self
            .selected
            .as_ref()
            .and_then(|path| self.states.get(path));
        let block = state.and_then(|state| {
            state.combined.as_ref().or_else(|| {
                self.files
                    .iter()
                    .find(|f| Some(f.path()) == self.selected.as_deref())
                    .and_then(|f| f.operations().get(state.page))
                    .and_then(|op| state.operations.get(op.id()))
            })
        });
        block
            .and_then(|block| block.focus_handle(cx))
            .unwrap_or_else(|| self.main_focus.clone())
            .focus(window, cx);
    }
    pub fn sync(
        &mut self,
        data: &EditedFiles,
        tools: &ToolIndex,
        window: &mut Window,
        cx: &mut App,
    ) {
        let owner = self
            .ids
            .iter()
            .find_map(|id| tools.owner(id))
            .map(str::to_owned);
        let was_in_scope = self.in_scope;
        self.in_scope = owner.is_some();
        let Some(run) = owner else {
            self.sync_list(window, cx);
            if was_in_scope && self.contains_focus(window, cx) {
                self.main_focus.focus(window, cx);
            }
            return;
        };
        self.run = run;
        let files = data.files(&self.run);
        let position = self
            .files
            .iter()
            .position(|f| Some(f.path()) == self.selected.as_deref())
            .unwrap_or(0);
        let selection_gone = !files
            .iter()
            .any(|f| Some(f.path()) == self.selected.as_deref());
        let focused = self.contains_focus(window, cx);
        let mode_changed = files
            .iter()
            .find(|file| Some(file.path()) == self.selected.as_deref())
            .is_some_and(|file| {
                self.states
                    .get(file.path())
                    .is_some_and(|state| state.combined.is_some() != file.combined().is_some())
            });
        self.ids.extend(
            files
                .iter()
                .flat_map(|f| f.operations().iter().map(|op| op.id().to_owned())),
        );
        self.files = Rc::new(files.to_vec());
        if selection_gone {
            self.selected = files
                .get(position.min(files.len().saturating_sub(1)))
                .map(|f| f.path().to_owned());
        }
        self.states
            .retain(|path, _| files.iter().any(|f| f.path() == path));
        self.prepare(cx);
        self.sync_list(window, cx);
        if (selection_gone || mode_changed) && focused {
            if self.navigator_hidden {
                self.main_focus.focus(window, cx);
            } else {
                self.focus.focus(window, cx);
            }
        }
    }
    fn prepare(&mut self, cx: &mut App) {
        for file in self.files.iter() {
            let state = self
                .states
                .entry(file.path().to_owned())
                .or_insert_with(|| FileDiff {
                    combined: None,
                    operations: HashMap::new(),
                    page: 0,
                });
            if let Some(body) = file.combined() {
                if let Some(block) = state.combined.as_mut() {
                    block.update(body, file.path(), cx);
                } else {
                    state.combined = Some(DiffBlock::new(body, file.path(), cx));
                }
                state.operations.clear();
            } else {
                state.combined = None;
                state
                    .operations
                    .retain(|id, _| file.operations().iter().any(|op| op.id() == id));
                for operation in file.operations() {
                    if let Some(block) = state.operations.get_mut(operation.id()) {
                        block.update(operation.shared_body(), file.path(), cx);
                    } else {
                        state.operations.insert(
                            operation.id().to_owned(),
                            DiffBlock::new(operation.shared_body(), file.path(), cx),
                        );
                    }
                }
            }
            state.page = state
                .page
                .min(file.operations().len().saturating_sub(1) / 3 * 3);
        }
    }
}
impl HomeView {
    pub(in crate::home) fn open_changes(
        &mut self,
        key: &str,
        run: &str,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_changes(key, run, path, true, window, cx);
    }
    pub(super) fn navigate_changes(
        &mut self,
        key: &str,
        run: &str,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_changes(key, run, path, false, window, cx);
    }
    fn activate_changes(
        &mut self,
        key: &str,
        run: &str,
        path: &str,
        ensure: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.views.get_mut(key) else {
            return;
        };
        let index =
            if let Some(index) = view.workspace.changes.iter().position(|tab| tab.run == run) {
                index
            } else {
                let row_ix = view.rows.iter().position(|row| row.id == run).unwrap_or(0);
                let title = view.rows[..row_ix]
                    .iter()
                    .rev()
                    .find_map(|row| row.user_text())
                    .unwrap_or_default();
                let timestamp = view.rows.get(row_ix).and_then(|row| row.started_at());
                view.workspace.changes.push(ChangesTab::new(
                    run,
                    title,
                    timestamp,
                    view.edited_files.files(run),
                    path,
                    window,
                    cx,
                ));
                view.workspace.changes.len() - 1
            };
        let tab = &mut view.workspace.changes[index];
        tab.selected = Some(path.to_owned());
        tab.sync_list(window, cx);
        let id = tab.id.clone();
        let title = tab.title.clone();
        let header_focus = tab.header_focus.clone();
        view.workspace.activate(Some(id.clone()), cx);
        view.workspace
            .headers
            .insert(Some(id.clone()), header_focus);
        self.remember_change(id, title);
        if ensure {
            self.ensure_panel(Panel::Workspace, window, cx);
        } else {
            self.fit_layout(window, cx);
            self.sync(false, window, cx);
        }
        if let Some(tab) = self.active_changes() {
            tab.focus_main(window, cx);
        }
        cx.notify();
    }
    pub(super) fn active_changes(&self) -> Option<&ChangesTab> {
        let view = self.session_view()?;
        view.workspace
            .changes
            .iter()
            .find(|tab| Some(&tab.id) == view.workspace.active.as_ref())
    }
    pub(super) fn select_change_file(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
            && let Some(tab) = view
                .workspace
                .changes
                .iter_mut()
                .find(|tab| Some(&tab.id) == view.workspace.active.as_ref())
        {
            tab.selected = Some(path);
            tab.sync_list(window, cx);
            tab.focus_main(window, cx);
        }
        cx.notify();
    }
    fn render_change_header(&self, file: &EditedFile, cx: &mut Context<Self>) -> AnyElement {
        let path = file.path().to_owned();
        let name = std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let directory = path.strip_suffix(&name).unwrap_or("").to_owned();
        h_flex()
            .h_10()
            .flex_none()
            .pl_3()
            .pr_2()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .child(
                        div()
                            .truncate()
                            .text_color(cx.theme().muted_foreground)
                            .child(directory),
                    )
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .child(name),
                    ),
            )
            .when(file.written(), |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "conversation-edit-write")),
                )
            })
            .when(file.failed(), |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "conversation-edit-error")),
                )
            })
            .child(messages::changes::stats(file.stats(), cx))
            .child(
                Button::new("change-open-file")
                    .small()
                    .ghost()
                    .label(t(cx, "workspace-open-file"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let root = this
                            .state
                            .read(cx)
                            .current()
                            .map(|s| PathBuf::from(&s.info().cwd));
                        if let Some(root) = root {
                            this.select_workspace_tab(None, window, cx);
                            this.open_source(root.join(&path), true, window, cx);
                        }
                    })),
            )
            .child(self.navigator_toggle(cx))
            .into_any_element()
    }
    pub(super) fn render_changes_main(
        &self,
        tab: &ChangesTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let body = if !tab.in_scope {
            v_flex()
                .size_full()
                .justify_center()
                .items_center()
                .p_6()
                .gap_3()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::FileDiff).size_6())
                .child(t(cx, "workspace-out-of-scope"))
                .into_any_element()
        } else if let Some(file) = tab
            .files
            .iter()
            .find(|f| Some(f.path()) == tab.selected.as_deref())
        {
            let state = &tab.states[file.path()];
            let mut body = v_flex()
                .size_full()
                .min_h_0()
                .min_w_0()
                .child(self.render_change_header(file, cx));
            if let Some(block) = &state.combined {
                body = body.child(div().flex_1().min_h_0().child(block.render_workspace()));
            } else {
                let mut operations = v_flex()
                    .id(format!("workspace-operations-{}", tab.id))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap_2()
                    .child(
                        div()
                            .px_3()
                            .text_sm()
                            .child(t(cx, "conversation-edit-uncombined")),
                    );
                for (ix, operation) in file
                    .operations()
                    .iter()
                    .enumerate()
                    .skip(state.page)
                    .take(3)
                {
                    if let Some(block) = state.operations.get(operation.id()) {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("index", (ix + 1) as i64);
                        let title = if matches!(operation.body(), ChangeBody::Written { .. }) {
                            t(cx, "conversation-written-content")
                        } else {
                            gupi_settings::i18n::t_with_args(
                                cx,
                                "conversation-edit-operation",
                                &args,
                            )
                        };
                        let header = h_flex()
                            .px_3()
                            .gap_2()
                            .text_xs()
                            .child(div().flex_1().child(title))
                            .when(operation.failed(), |row| {
                                row.child(t(cx, "conversation-edit-error"))
                            })
                            .when_some(operation.stats(), |row, stats| {
                                row.child(format!("+{} −{}", stats.added(), stats.removed()))
                            });
                        operations = operations
                            .child(v_flex().min_w_0().child(header).child(block.render()));
                    }
                }
                let mut controls = h_flex().gap_2().px_3();
                for (forward, enabled, label) in [
                    (
                        false,
                        state.page > 0,
                        "conversation-edit-previous-operations",
                    ),
                    (
                        true,
                        state.page + 3 < file.operations().len(),
                        "workspace-next-operations",
                    ),
                ] {
                    controls = controls.child(
                        Button::new(if forward {
                            "changes-next"
                        } else {
                            "changes-previous"
                        })
                        .small()
                        .ghost()
                        .label(t(cx, label))
                        .disabled(!enabled)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(view) = this
                                .shown_key
                                .as_ref()
                                .and_then(|key| this.views.get_mut(key))
                                && let Some(tab) = view
                                    .workspace
                                    .changes
                                    .iter_mut()
                                    .find(|tab| Some(&tab.id) == view.workspace.active.as_ref())
                                && let Some(state) = tab
                                    .selected
                                    .as_ref()
                                    .and_then(|path| tab.states.get_mut(path))
                            {
                                state.page = if forward {
                                    state.page + 3
                                } else {
                                    state.page.saturating_sub(3)
                                };
                            }
                            cx.notify();
                        })),
                    );
                }
                body = body.child(operations).child(controls);
            }
            body.into_any_element()
        } else {
            v_flex()
                .size_full()
                .justify_center()
                .items_center()
                .p_6()
                .gap_3()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::FileDiff).size_6())
                .child(t(cx, "workspace-net-zero"))
                .into_any_element()
        };
        div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .track_focus(&tab.main_focus)
            .key_context("GupiWorkspaceDiff")
            .on_action(
                cx.listener(|this, _: &actions::ReturnFromSource, window, cx| {
                    if this.pane_layout.maximized {
                        this.toggle_workspace_maximized(window, cx);
                    } else if this.pane_layout.overlay == Some(Panel::Workspace) {
                        this.close_navigator(window, cx);
                    } else if let Some(tab) =
                        this.active_changes().filter(|tab| !tab.navigator_hidden)
                    {
                        tab.focus_list(window, cx);
                    } else {
                        this.focus_composer(window, cx);
                    }
                    cx.stop_propagation();
                }),
            )
            .child(body)
            .into_any_element()
    }
    pub(super) fn render_changes_list(
        &self,
        tab: &ChangesTab,
        _: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(format!("changes-list-{}", tab.id))
            .size_full()
            .min_h_0()
            .p_2()
            .child(gpui_kit::component::list::List::new(&tab.list))
            .into_any_element()
    }
}
