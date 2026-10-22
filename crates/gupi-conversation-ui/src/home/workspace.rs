use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use panes::Panel;
mod changes;
mod git_changes;
pub(super) mod layout;
mod reader;
use changes::ChangesTab;
use git_changes::{GIT_TAB, GitTab};
use layout as internal;
mod launcher;
mod strip;
pub(in crate::home) use strip::events as strip_events;
const NEW_TAB: &str = "workspace:new";
const RECENT_LIMIT: usize = 8;

pub(super) struct Tabs {
    pub order: Vec<Option<String>>,
    recent: Vec<launcher::Recent>,
    pub scroll: ScrollHandle,
    /// Whether the last layout clipped tabs, which shows the all-tabs menu.
    pub overflowing: std::rc::Rc<std::cell::Cell<bool>>,
    pub headers: HashMap<Option<String>, FocusHandle>,
    pub strip: strip::Interaction,
    temporary_lists: HashSet<Option<String>>,
    pub navigator_hidden: bool,
    next_new: u64,
    pub changes: Vec<ChangesTab>,
    git: Option<GitTab>,
    pub active: Option<String>,
    pub restore_focus: bool,
    pub(super) record_revision: Option<((u64, u64, bool), u64)>,
}
impl Default for Tabs {
    fn default() -> Self {
        Self {
            navigator_hidden: false,
            next_new: 0,
            changes: Vec::new(),
            git: None,
            active: Some(NEW_TAB.into()),
            restore_focus: false,
            record_revision: None,
            order: vec![Some(NEW_TAB.into())],
            recent: Vec::new(),
            scroll: ScrollHandle::new(),
            overflowing: Default::default(),
            headers: HashMap::new(),
            strip: Default::default(),
            temporary_lists: HashSet::new(),
        }
    }
}
impl Tabs {
    pub fn new(cx: &mut App) -> Self {
        let mut tabs = Self::default();
        tabs.activate(Some(NEW_TAB.into()), cx);
        tabs
    }
    pub(super) fn launcher(&self) -> bool {
        self.active
            .as_deref()
            .is_some_and(|id| id.starts_with(NEW_TAB))
    }
    fn activate(&mut self, id: Option<String>, cx: &mut App) {
        if self.launcher()
            && !id.as_deref().is_some_and(|id| id.starts_with(NEW_TAB))
            && self.order.contains(&id)
        {
            self.order.retain(|entry| *entry != self.active);
        }
        if !self.order.contains(&id) {
            if self.launcher() && !id.as_deref().is_some_and(|id| id.starts_with(NEW_TAB)) {
                if let Some(slot) = self.order.iter_mut().find(|slot| **slot == self.active) {
                    *slot = id.clone();
                }
            } else {
                self.order.push(id.clone());
            }
        }
        self.headers
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle());
        self.active = id;
        self.headers
            .retain(|id, _| id.as_deref() == Some(NEW_TAB) || self.order.contains(id));
        self.reveal();
    }
    fn reveal(&self) {
        if let Some(ix) = self.order.iter().position(|id| *id == self.active) {
            self.scroll.scroll_to_item(ix);
        }
    }
    fn reconcile_order(&mut self) {
        let previous = self.order.clone();
        self.order.retain(|id| {
            id.as_deref().is_none_or(|id| {
                id.starts_with(NEW_TAB)
                    || (id == GIT_TAB && self.git.is_some())
                    || self.changes.iter().any(|tab| tab.id == id)
            })
        });
        if !self.order.contains(&self.active) {
            let ix = previous
                .iter()
                .position(|id| *id == self.active)
                .unwrap_or(0);
            self.active = previous[..ix]
                .iter()
                .rev()
                .find(|id| self.order.contains(id))
                .cloned()
                .or_else(|| self.order.first().cloned())
                .unwrap_or(Some(NEW_TAB.into()));
        }
        if self.order.is_empty() {
            self.order.push(Some(NEW_TAB.into()));
            self.active = Some(NEW_TAB.into());
        }
        self.headers
            .retain(|id, _| id.as_deref() == Some(NEW_TAB) || self.order.contains(id));
        self.temporary_lists.retain(|id| self.order.contains(id));
        self.reveal();
    }
    pub(super) fn reconcile_records(
        &mut self,
        session: &gupi_conversation::conversation::Session,
        window: &Window,
        cx: &App,
    ) {
        if self.changes.is_empty() {
            return;
        }
        let revision = (
            session.tool_projection_revision(),
            session.content_revision(),
        );
        if self.record_revision == Some(revision) {
            return;
        }
        let streamed = self.record_revision.is_some_and(|(tools, content)| {
            tools == revision.0 && session.message_update_since(content).is_some()
        });
        self.record_revision = Some(revision);
        // A consecutive text/thinking update cannot remove backing records.
        if streamed {
            return;
        }
        let mut ids: HashSet<String> = session
            .tools()
            .iter()
            .map(|tool| format!("tool-{}", tool.id))
            .collect();
        for entry in &session.history().entries {
            if let Some(message) = entry.data.get("message") {
                if let Some(id) = message["toolCallId"].as_str() {
                    ids.insert(format!("tool-{id}"));
                }
                if let Some(content) = message["content"].as_array() {
                    for (part_ix, part) in content.iter().enumerate() {
                        if part["type"] == "toolCall" {
                            ids.insert(messages::tools::tool_call_id(
                                &entry.id,
                                part_ix,
                                part["id"].as_str(),
                            ));
                        }
                    }
                }
            }
        }
        for message in session.messages(None) {
            if let Some(id) = message.value["toolCallId"].as_str() {
                ids.insert(format!("tool-{id}"));
            }
            if let Some(content) = message.value["content"].as_array() {
                for (part_ix, part) in content.iter().enumerate() {
                    if part["type"] == "toolCall" {
                        ids.insert(messages::tools::tool_call_id(
                            &message.id,
                            part_ix,
                            part["id"].as_str(),
                        ));
                    }
                }
            }
        }

        let mut restore = false;
        self.changes.retain(|tab| {
            let keep = tab.ids.iter().any(|id| ids.contains(id));
            if !keep {
                restore |=
                    tab.contains_focus(window, cx) || tab.header_focus.contains_focused(window, cx);
            }
            keep
        });
        self.reconcile_order();
        self.restore_focus |= restore;
    }
}
impl HomeView {
    fn session_view(&self) -> Option<&SessionView> {
        self.shown_key.as_ref().and_then(|key| self.views.get(key))
    }
    pub(super) fn files(&self) -> Option<Entity<files::Files>> {
        self.session_view().map(|view| view.files.clone())
    }
    pub(super) fn source(&self) -> Option<Entity<source::Preview>> {
        self.session_view().and_then(|view| view.source.clone())
    }
    pub(super) fn workspace_content(&self) -> internal::Content {
        if self.session_view().is_none_or(|v| v.workspace.launcher()) {
            return internal::Content {
                main: true,
                navigator: false,
            };
        }
        if let Some(tab) = self.active_git() {
            return internal::Content {
                main: tab.selected.is_some() || tab.rows.is_empty(),
                navigator: tab.selected.is_none() || !tab.navigator_hidden,
            };
        }
        if let Some(tab) = self.active_changes() {
            return internal::Content {
                main: tab.selected.is_some() || tab.files.is_empty() || !tab.in_scope,
                navigator: tab.selected.is_none() || !tab.navigator_hidden,
            };
        }
        let main = self.source().is_some();
        internal::Content {
            main,
            navigator: !main
                || self
                    .session_view()
                    .is_none_or(|v| !v.workspace.navigator_hidden),
        }
    }
    pub(super) fn fit_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pane_layout.workspace_docked
            && (self.workspace_region.contains_focused(window, cx)
                || self.session_view().is_some_and(|view| {
                    view.workspace
                        .headers
                        .values()
                        .any(|focus| focus.contains_focused(window, cx))
                }))
        {
            self.pane_layout.foreground = panes::Foreground::Workspace;
        } else if self.pane_layout.workspace_docked
            && self.focus_handle.contains_focused(window, cx)
        {
            self.pane_layout.foreground = panes::Foreground::Conversation;
        }
        let preferences = cx.global::<gupi_settings::layout::LayoutState>();
        let request = panes::Request {
            width: f32::from(window.viewport_size().width),
            sidebar: self.show_sidebar,
            rem: f32::from(window.rem_size()),
            workspace_pinned: preferences.workspace_pinned,
            history_pinned: preferences.history_pinned,
        };
        if self.pane_layout.needs_fit(request) {
            self.interrupt_pane_drag(window, cx);
            self.pane_layout
                .fit(request, cx.global::<gupi_settings::layout::LayoutState>());
            if let Some(view) = self.session_view() {
                view.workspace.reveal();
            }
        }
        self.fit_workspace_layout(window, cx);
        let restore_focus = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
            .is_some_and(|view| {
                let mut restore = std::mem::take(&mut view.workspace.restore_focus);
                for tab in &mut view.workspace.changes {
                    let pending = std::mem::take(&mut tab.restore_focus);
                    restore |= pending && view.workspace.active.as_deref() == Some(&tab.id);
                }
                restore
            });
        if restore_focus && self.pane_layout.visible(Panel::Workspace) {
            self.focus_panel(Panel::Workspace, window, cx);
        }
        self.apply_reader_options(cx);
    }
    pub(super) fn temporary_list(&self) -> bool {
        self.session_view()
            .is_some_and(|v| v.workspace.temporary_lists.contains(&v.workspace.active))
    }
    pub(super) fn set_temporary_list(&mut self, value: bool) {
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            let active = view.workspace.active.clone();
            if value {
                view.workspace.temporary_lists.insert(active);
            } else {
                view.workspace.temporary_lists.remove(&active);
            }
        }
    }
    pub(super) fn fit_workspace_layout(&mut self, window: &mut Window, cx: &mut App) {
        let preference = if self
            .pane_drag
            .as_ref()
            .is_some_and(|drag| drag.side == panes::Side::Right)
        {
            self.workspace_layout.navigator_width
        } else {
            cx.global::<gupi_settings::layout::LayoutState>()
                .navigator_width_rem
                * f32::from(window.rem_size())
        };
        let allocation = internal::Layout::new(
            self.pane_layout.workspace_width,
            f32::from(window.rem_size()),
            self.workspace_content(),
            self.temporary_list(),
            preference,
        );
        if self
            .pane_drag
            .as_ref()
            .is_some_and(|drag| drag.side == panes::Side::Right)
            && !allocation.split
        {
            self.interrupt_pane_drag(window, cx);
            return;
        }
        let list_focused = self
            .active_git()
            .is_some_and(|tab| tab.focus.contains_focused(window, cx))
            || self
                .active_changes()
                .is_some_and(|tab| tab.focus.contains_focused(window, cx))
            || (self.active_git().is_none()
                && self.active_changes().is_none()
                && self
                    .files()
                    .is_some_and(|files| files.read(cx).is_focused(window, cx)));
        let move_focus = self.workspace_layout.navigator_visible()
            && !allocation.navigator_visible()
            && allocation.main_width > 0.
            && list_focused;
        self.workspace_layout = allocation;
        if !allocation.compact {
            self.set_temporary_list(false);
        }
        if move_focus {
            self.focus_workspace_main(window, cx);
        }
        if let Some(source) = self.source() {
            let visible = self.workspace_layout.navigator_visible();
            if source.read(cx).navigator_visible != visible {
                source.update(cx, |source, cx| {
                    source.navigator_visible = visible;
                    cx.notify();
                });
            }
        }
    }
    pub(super) fn show_conversation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.interrupt_pane_drag(window, cx);
        self.end_workspace_tab_drag(window, cx);
        self.pane_layout.maximized = false;
        self.pane_layout.foreground = panes::Foreground::Conversation;
        self.pane_layout.allocate();
        self.fit_workspace_layout(window, cx);
        self.focus_composer(window, cx);
        cx.notify();
    }
    pub(super) fn ensure_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.fit_layout(window, cx);
        match panel {
            Panel::Workspace => {
                self.pane_layout.foreground = panes::Foreground::Workspace;
                cx.global_mut::<gupi_settings::layout::LayoutState>()
                    .workspace_pinned = true;
                self.pane_layout.allocate();
                self.fit_layout(window, cx);
                self.save_layout(None, None, window, cx);
            }
            Panel::History => {
                if self.pane_layout.conversation == 0. {
                    self.show_conversation(window, cx);
                }
                if self.pane_layout.dockable(panel) {
                    cx.global_mut::<gupi_settings::layout::LayoutState>()
                        .history_pinned = true;
                    self.fit_layout(window, cx);
                    self.save_layout(None, None, window, cx);
                } else {
                    self.pane_layout.history_overlay = true;
                }
            }
        }
        self.sync(false, window, cx);
        self.sync_worktree(window, cx);
        cx.notify();
    }
    pub(super) fn toggle_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.fit_layout(window, cx);
        if panel == Panel::History && self.pane_layout.history_overlay {
            self.close_navigator(window, cx);
        } else if self.pane_layout.visible(panel) {
            let region = match panel {
                Panel::History => &self.history_region,
                Panel::Workspace => &self.workspace_region,
            };
            let header_focused = panel == Panel::Workspace
                && self.session_view().is_some_and(|view| {
                    view.workspace
                        .headers
                        .values()
                        .any(|focus| focus.contains_focused(window, cx))
                });
            if region.contains_focused(window, cx) || header_focused {
                match panel {
                    Panel::History => &self.history_button_focus,
                    Panel::Workspace => &self.workspace_button_focus,
                }
                .focus(window, cx);
                window.focus_next(cx);
            }
            match panel {
                Panel::History => {
                    cx.global_mut::<gupi_settings::layout::LayoutState>()
                        .history_pinned = false
                }
                Panel::Workspace => {
                    cx.global_mut::<gupi_settings::layout::LayoutState>()
                        .workspace_pinned = false;
                    self.pane_layout.maximized = false;
                }
            }
            self.fit_layout(window, cx);
            self.save_layout(None, None, window, cx);
        } else {
            self.ensure_panel(panel, window, cx);
            self.focus_panel(panel, window, cx);
        }
        self.sync_worktree(window, cx);
        cx.notify();
    }
    pub(super) fn focus_panel(&self, panel: Panel, window: &mut Window, cx: &mut Context<Self>) {
        match panel {
            Panel::History => self.history_focus_handle(cx).focus(window, cx),
            Panel::Workspace => {
                if self.session_view().is_none_or(|v| v.workspace.launcher()) {
                    self.workspace_region.focus(window, cx);
                } else if let Some(tab) = self.active_git() {
                    if !self.workspace_layout.navigator_visible() {
                        tab.focus_main(window, cx);
                    } else {
                        tab.focus_list(window, cx);
                    }
                } else if let Some(tab) = self.active_changes() {
                    if !self.workspace_layout.navigator_visible() {
                        tab.focus_main(window, cx);
                    } else {
                        tab.focus_list(window, cx);
                    }
                } else if self.workspace_layout.navigator_visible() {
                    if let Some(files) = self.files() {
                        files.update(cx, |files, cx| files.focus(window, cx));
                    }
                } else if let Some(source) = self.source() {
                    source.update(cx, |source, cx| source.focus(window, cx));
                }
            }
        }
    }
    pub(super) fn set_files_navigator_hidden(&mut self, hidden: bool, cx: &mut Context<Self>) {
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        view.workspace.navigator_hidden = hidden;
        if let Some(source) = &view.source {
            source.update(cx, |source, cx| {
                source.navigator_visible = !hidden;
                cx.notify();
            });
        }
    }
    /// Shows or hides the active tab's navigator: the file tree for Files, the
    /// changed-file list for a Changes tab.
    pub(super) fn toggle_workspace_navigator(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_layout.compact {
            self.set_temporary_list(!self.temporary_list());
            self.fit_workspace_layout(window, cx);
            self.focus_panel(Panel::Workspace, window, cx);
            cx.notify();
            return;
        }
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        if view.workspace.active.as_deref() == Some(GIT_TAB) {
            if let Some(tab) = &mut view.workspace.git {
                tab.navigator_hidden = !tab.navigator_hidden;
            }
        } else if let Some(tab) = view
            .workspace
            .changes
            .iter_mut()
            .find(|tab| Some(&tab.id) == view.workspace.active.as_ref())
        {
            tab.navigator_hidden = !tab.navigator_hidden;
        } else {
            let hidden = !view.workspace.navigator_hidden;
            self.set_files_navigator_hidden(hidden, cx);
        }
        self.fit_layout(window, cx);
        cx.notify();
    }
    /// The Changes header's toggle; the Files tab renders its own in the source header.
    pub(super) fn navigator_toggle(&self, cx: &mut Context<Self>) -> Button {
        let visible = self.workspace_layout.navigator_visible();
        Button::new("workspace-navigator")
            .small()
            .ghost()
            .selected(visible)
            .icon(IconName::List)
            .tooltip(t(cx, "workspace-toggle-change-list"))
            .accessibility_label(t(cx, "workspace-toggle-change-list"))
            .on_click(
                cx.listener(|this, _, window, cx| this.toggle_workspace_navigator(window, cx)),
            )
    }
    pub(super) fn open_source(
        &mut self,
        path: PathBuf,
        keyboard: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = self
            .shown_key
            .clone()
            .filter(|key| self.views.contains_key(key))
        else {
            return;
        };
        self.views
            .get_mut(&key)
            .unwrap()
            .workspace
            .activate(None, cx);
        // Opening a file targets Files; the originating tab keeps its own list state.
        let temporary_list = self.temporary_list();
        self.set_temporary_list(false);
        let keyboard = keyboard || temporary_list;
        self.sync_worktree(window, cx);
        self.remember_file(path.clone());
        if let Some(source) = self.source().filter(|s| s.read(cx).path == path) {
            self.fit_layout(window, cx);
            if keyboard {
                source.update(cx, |s, cx| s.focus(window, cx));
            }
            return;
        }
        let root = self.state.read(cx).sessions()[&key].info().cwd.clone();
        let navigator_visible = !self.views[&key].workspace.navigator_hidden;
        let preview = cx.new(|cx| {
            let mut preview = source::Preview::new(path.clone(), Some(root), keyboard, window, cx);
            preview.navigator_visible = navigator_visible;
            preview
        });
        self.files()
            .unwrap()
            .update(cx, |files, cx| files.set_open_path(Some(path), cx));
        let preview_key = key.clone();
        let subscription = cx.subscribe_in(
            &preview,
            window,
            move |this, _, event: &source::Event, window, cx| {
                if this.shown_key.as_ref() != Some(&preview_key) {
                    return;
                }
                match event {
                    source::Event::Close => this.close_source(window, cx),
                    source::Event::Return => this.return_from_source(window, cx),
                    source::Event::ToggleNavigator => this.toggle_workspace_navigator(window, cx),
                    source::Event::Reveal => {
                        this.set_files_navigator_hidden(false, cx);
                        this.fit_layout(window, cx);
                        if !this.workspace_layout.navigator_visible() {
                            this.set_temporary_list(true);
                            this.fit_workspace_layout(window, cx);
                        }
                        if let Some(path) = this.source().map(|s| s.read(cx).path.clone()) {
                            this.files()
                                .unwrap()
                                .update(cx, |files, cx| files.reveal(path, window, cx));
                        }
                    }
                }
                cx.notify();
            },
        );
        let view = self.views.get_mut(&key).unwrap();
        view.source_subscription = Some(subscription);
        view.source = Some(preview);
        self.fit_layout(window, cx);
        cx.notify();
    }
    pub(super) fn return_from_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.temporary_list() {
            self.set_temporary_list(false);
            self.fit_workspace_layout(window, cx);
            self.focus_workspace_main(window, cx);
        } else if self.pane_layout.history_overlay
            && self.history_region.contains_focused(window, cx)
        {
            self.close_navigator(window, cx);
        } else if self.pane_layout.maximized {
            self.toggle_workspace_maximized(window, cx);
        } else if !self.pane_layout.workspace_docked && self.pane_layout.visible(Panel::Workspace) {
            self.show_conversation(window, cx);
        } else if self.workspace_layout.navigator_visible() {
            self.focus_panel(Panel::Workspace, window, cx);
        } else {
            self.focus_composer(window, cx);
        }
        cx.stop_propagation();
    }
    fn focus_workspace_main(&self, window: &mut Window, cx: &mut App) {
        if let Some(tab) = self.active_git() {
            tab.focus_main(window, cx);
        } else if let Some(tab) = self.active_changes() {
            tab.focus_main(window, cx);
        } else if let Some(source) = self.source() {
            source.update(cx, |source, cx| source.focus(window, cx));
        }
    }
    pub(super) fn close_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self
            .source()
            .is_some_and(|s| s.read(cx).contains_focus(window, cx));
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.source = None;
            view.source_subscription = None;
            view.files
                .update(cx, |files, cx| files.set_open_path(None, cx));
        }
        self.fit_layout(window, cx);
        if focused {
            self.focus_panel(Panel::Workspace, window, cx);
        }
        cx.notify();
    }
    pub(super) fn close_navigator(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pane_layout.history_overlay {
            self.pane_layout.history_overlay = false;
            if self.history_region.contains_focused(window, cx) {
                self.history_button_focus.focus(window, cx);
                window.focus_next(cx);
            }
        } else {
            self.return_from_source(window, cx);
            return;
        }
        self.sync_worktree(window, cx);
        cx.notify();
    }
    pub(super) fn select_workspace_tab(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.end_workspace_tab_drag(window, cx);
        if self
            .pane_drag
            .as_ref()
            .is_some_and(|drag| drag.side == panes::Side::Right)
        {
            self.interrupt_pane_drag(window, cx);
        }
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.workspace.activate(id, cx);
        }
        self.fit_layout(window, cx);
        self.sync(false, window, cx);
        cx.notify();
    }
    pub(super) fn close_workspace_tabs(
        &mut self,
        id: &str,
        mode: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.end_workspace_tab_drag(window, cx);
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        let target = if id == "files" {
            None
        } else {
            Some(id.to_owned())
        };
        let Some(ix) = view.workspace.order.iter().position(|tab| *tab == target) else {
            return;
        };
        let removed: Vec<_> = view
            .workspace
            .order
            .iter()
            .enumerate()
            .filter(|(i, _)| match mode {
                1 => *i != ix,
                2 => *i > ix,
                _ => *i == ix,
            })
            .map(|(_, id)| id.clone())
            .collect();
        let focused = (removed.contains(&view.workspace.active)
            && self.workspace_region.contains_focused(window, cx))
            || removed
                .iter()
                .filter_map(|id| view.workspace.headers.get(id))
                .any(|focus| focus.contains_focused(window, cx));
        let old_order = view.workspace.order.clone();
        let old_active = view.workspace.active.clone();
        view.workspace.order.retain(|id| !removed.contains(id));
        view.workspace
            .changes
            .retain(|tab| !removed.contains(&Some(tab.id.clone())));
        if removed.contains(&Some(GIT_TAB.into())) {
            view.workspace.git = None;
        }
        if removed.contains(&None) {
            view.source = None;
            view.source_subscription = None;
            view.files
                .update(cx, |files, cx| files.set_open_path(None, cx));
        }
        if removed.contains(&old_active) {
            let active_ix = old_order
                .iter()
                .position(|id| *id == old_active)
                .unwrap_or(0);
            view.workspace.active = old_order[..active_ix]
                .iter()
                .rev()
                .find(|id| view.workspace.order.contains(id))
                .cloned()
                .or_else(|| view.workspace.order.first().cloned())
                .unwrap_or(Some(NEW_TAB.into()));
        }
        view.workspace.reconcile_order();
        self.fit_layout(window, cx);
        if focused {
            self.focus_panel(Panel::Workspace, window, cx);
        }
        cx.notify();
    }
    pub(super) fn render_file_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let allocation = self.workspace_layout;
        let mut columns = h_flex().items_stretch().flex_1().min_h_0().min_w_0();
        if self.session_view().is_none_or(|v| v.workspace.launcher()) {
            columns = columns.child(self.render_launcher(window, cx));
        } else if let Some(tab) = self.active_git().filter(|_| allocation.main_width > 0.) {
            columns = columns.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .child(self.render_git_main(tab, cx)),
            );
        } else if let Some(tab) = self.active_changes().filter(|_| allocation.main_width > 0.) {
            columns = columns.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .child(self.render_changes_main(tab, cx)),
            );
        } else if let Some(source) = self.source().filter(|_| allocation.main_width > 0.) {
            columns = columns.child(div().flex_1().min_w_0().h_full().child(source));
        }
        if allocation.navigator_visible() {
            let navigator = if let Some(tab) = self.active_git() {
                self.render_git_list(tab, cx)
            } else if let Some(tab) = self.active_changes() {
                self.render_changes_list(tab, cx)
            } else {
                self.files()
                    .map(|files| files.into_any_element())
                    .unwrap_or_else(|| {
                        div()
                            .p_3()
                            .child(t(cx, "files-no-session"))
                            .into_any_element()
                    })
            };
            columns = columns.child(
                div()
                    .relative()
                    .w(px(allocation.navigator_width))
                    .flex_none()
                    .h_full()
                    .when(allocation.split, |view| {
                        view.border_l_1().border_color(cx.theme().border)
                    })
                    .child(navigator)
                    .when(allocation.split, |view| {
                        view.child(self.pane_handle(panes::Side::Right, cx))
                    }),
            );
        }
        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .track_focus(&self.workspace_region)
            .key_context("GupiWorkspace")
            .on_action(
                cx.listener(|this, _: &actions::CloseNavigator, window, cx| {
                    this.return_from_source(window, cx);
                }),
            )
            .bg(cx.theme().background)
            .when(self.temporary_list(), |view| {
                view.child(
                    Button::new("workspace-list-back")
                        .small()
                        .ghost()
                        .icon(IconName::ArrowLeft)
                        .label(t(cx, "workspace-back-content"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.set_temporary_list(false);
                            this.fit_workspace_layout(window, cx);
                            this.focus_workspace_main(window, cx);
                            cx.notify();
                        })),
                )
            })
            .child(columns)
            .into_any_element()
    }
    pub(super) fn render_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.pane_layout.conversation == 0. {
            return div()
                .size_full()
                .child(self.render_file_workspace(window, cx))
                .into_any_element();
        }
        let conversation = v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .children(self.find.clone())
            .child(self.render_messages(
                px(panes::geometry::BODY_MAX_REM * f32::from(window.rem_size())),
                window,
                cx,
            ))
            .child(self.render_composer(
                px(panes::geometry::BODY_MAX_REM * f32::from(window.rem_size())),
                window,
                cx,
            ));
        let center = div()
            .relative()
            .w(px(self.pane_layout.conversation))
            .flex_none()
            .h_full()
            .min_w_0()
            .child(
                div()
                    .absolute()
                    .left(px(self.pane_layout.body_origin))
                    .top_0()
                    .bottom_0()
                    .w(px(self.pane_layout.body_width))
                    .child(conversation),
            )
            .when(self.pane_layout.history_docked, |view| {
                view.child(
                    div()
                        .absolute()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .w(px(self.pane_layout.history_width))
                        .track_focus(&self.history_region)
                        .child(self.render_history_card(false, cx))
                        .child(self.pane_handle(panes::Side::History, cx)),
                )
            });
        h_flex()
            .items_stretch()
            .size_full()
            .min_w_0()
            .child(center)
            .when(self.pane_layout.workspace_docked, |row| {
                row.child(
                    div()
                        .relative()
                        .flex_none()
                        .id("workspace-docked")
                        .debug_selector(|| "workspace-docked".into())
                        .w(px(
                            self.pane_layout.workspace_width + panes::geometry::DIVIDER
                        ))
                        .h_full()
                        .border_l_1()
                        .border_color(cx.theme().border)
                        .child(self.render_file_workspace(window, cx))
                        .child(self.pane_handle(panes::Side::Source, cx)),
                )
            })
            .into_any_element()
    }
}
