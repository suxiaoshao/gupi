use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use panes::Panel;
mod changes;
use changes::ChangesTab;
use gpui_kit::component::tab::{Tab, TabBar};

#[derive(Default)]
pub(super) struct Tabs {
    pub navigator_hidden: bool,
    pub changes: Vec<ChangesTab>,
    pub active: Option<String>,
    pub restore_focus: bool,
}
impl Tabs {
    pub(super) fn reconcile_records(
        &mut self,
        session: &gupi_conversation::conversation::Session,
        window: &Window,
        cx: &App,
    ) {
        if self.changes.is_empty() {
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
                    for part in content {
                        if part["type"] == "toolCall"
                            && let Some(id) = part["id"].as_str()
                        {
                            ids.insert(format!("tool-{id}"));
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
                for part in content {
                    if part["type"] == "toolCall"
                        && let Some(id) = part["id"].as_str()
                    {
                        ids.insert(format!("tool-{id}"));
                    }
                }
            }
        }
        let old_active = self.active.clone();
        let mut previous = None;
        let mut restore = false;
        self.changes.retain(|tab| {
            let keep = tab.ids.iter().any(|id| ids.contains(id));
            if !keep && Some(&tab.id) == old_active.as_ref() {
                self.active = previous.clone();
            }
            if !keep {
                restore |=
                    tab.contains_focus(window, cx) || tab.header_focus.contains_focused(window, cx);
            }
            if keep {
                previous = Some(tab.id.clone());
            }
            keep
        });
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
    fn workspace_content(&self) -> panes::geometry::Content {
        if let Some(tab) = self.active_changes() {
            return panes::geometry::Content {
                main: true,
                navigator: !tab.navigator_hidden,
            };
        }
        let main = self.source().is_some();
        panes::geometry::Content {
            main,
            navigator: !main
                || self
                    .session_view()
                    .is_none_or(|v| !v.workspace.navigator_hidden),
        }
    }
    pub(super) fn fit_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let preferences = cx.global::<layout::LayoutState>();
        let request = panes::Request {
            width: f32::from(window.viewport_size().width),
            sidebar: self.show_sidebar,
            content: self.workspace_content(),
            rem: f32::from(window.rem_size()),
            workspace_pinned: preferences.workspace_pinned,
            history_pinned: preferences.history_pinned,
        };
        if self.pane_layout.fit(request, preferences) {
            self.interrupt_pane_drag(window, cx);
        }
    }
    pub(super) fn ensure_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.fit_layout(window, cx);
        if self.pane_layout.dockable(panel) {
            match panel {
                Panel::History => cx.global_mut::<layout::LayoutState>().history_pinned = true,
                Panel::Workspace => cx.global_mut::<layout::LayoutState>().workspace_pinned = true,
            }
            self.fit_layout(window, cx);
            self.save_layout(None, None, window, cx);
        } else {
            self.pane_layout.overlay = Some(panel);
            self.pane_layout.allocate();
        }
        self.sync(false, window, cx);
        cx.notify();
    }
    pub(super) fn toggle_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.fit_layout(window, cx);
        if self.pane_layout.overlay == Some(panel) {
            self.close_navigator(window, cx);
        } else if self.pane_layout.visible(panel) {
            let region = match panel {
                Panel::History => &self.history_region,
                Panel::Workspace => &self.workspace_region,
            };
            if region.contains_focused(window, cx) {
                match panel {
                    Panel::History => &self.history_button_focus,
                    Panel::Workspace => &self.workspace_button_focus,
                }
                .focus(window, cx);
                window.focus_next(cx);
            }
            match panel {
                Panel::History => cx.global_mut::<layout::LayoutState>().history_pinned = false,
                Panel::Workspace => cx.global_mut::<layout::LayoutState>().workspace_pinned = false,
            }
            self.fit_layout(window, cx);
            self.save_layout(None, None, window, cx);
        } else {
            self.ensure_panel(panel, window, cx);
            self.focus_panel(panel, window, cx);
        }
        cx.notify();
    }
    pub(super) fn focus_panel(&self, panel: Panel, window: &mut Window, cx: &mut Context<Self>) {
        match panel {
            Panel::History => self.history_focus_handle(cx).focus(window, cx),
            Panel::Workspace => {
                if let Some(tab) = self.active_changes() {
                    if tab.navigator_hidden {
                        tab.focus_main(window, cx);
                    } else {
                        tab.focus.focus(window, cx);
                    }
                } else if self.workspace_content().navigator {
                    if let Some(files) = self.files() {
                        files.update(cx, |files, cx| files.focus(window, cx));
                    }
                } else if let Some(source) = self.source() {
                    source.update(cx, |source, cx| source.focus(window, cx));
                }
            }
        }
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
        self.views.get_mut(&key).unwrap().workspace.active = None;
        if let Some(source) = self.source().filter(|s| s.read(cx).path == path) {
            self.fit_layout(window, cx);
            if keyboard {
                source.update(cx, |s, cx| s.focus(window, cx));
            }
            return;
        }
        let root = self.state.read(cx).sessions()[&key].info().cwd.clone();
        let preview =
            cx.new(|cx| source::Preview::new(path.clone(), Some(root), keyboard, window, cx));
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
                    source::Event::Reveal => {
                        this.views
                            .get_mut(&preview_key)
                            .unwrap()
                            .workspace
                            .navigator_hidden = false;
                        this.fit_layout(window, cx);
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
        if self.pane_layout.overlay == Some(Panel::Workspace) {
            self.close_navigator(window, cx);
        } else if self.pane_layout.workspace_docked && self.workspace_content().navigator {
            if let Some(files) = self.files() {
                files.update(cx, |files, cx| files.focus_open(window, cx));
            }
        } else {
            self.focus_composer(window, cx);
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
        if let Some(panel) = self.pane_layout.overlay.take() {
            let region = match panel {
                Panel::History => &self.history_region,
                Panel::Workspace => &self.workspace_region,
            };
            if region.contains_focused(window, cx) {
                match panel {
                    Panel::History => &self.history_button_focus,
                    Panel::Workspace => &self.workspace_button_focus,
                }
                .focus(window, cx);
                window.focus_next(cx);
            }
            self.pane_layout.allocate();
        }
        cx.notify();
    }
    pub(super) fn save_source_split(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).is_temporary() {
            return;
        }
        self.source_split_touched = true;
        let ratio = self.pane_layout.ratio();
        let previous = self.source_save.take();
        let directory = Self::layout_directory(cx);
        self.source_save = Some(cx.spawn(async move |_, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let result = cx
                .background_spawn(async move {
                    directory.and_then(|dir| {
                        gupi_settings::source_split::save(&dir.join("source-split.toml"), ratio)
                    })
                })
                .await;
            if let Err(error) = result {
                tracing::warn!(%error,"source split save failed");
            }
        }));
    }

    pub(super) fn select_workspace_tab(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.workspace.active = id;
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
        let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        else {
            return;
        };
        let Some(ix) = view.workspace.changes.iter().position(|tab| tab.id == id) else {
            return;
        };
        let active_removed = view.workspace.changes.iter().enumerate().any(|(i, tab)| {
            Some(&tab.id) == view.workspace.active.as_ref()
                && match mode {
                    1 => i != ix,
                    2 => i > ix,
                    _ => i == ix,
                }
        });
        let focused = view.workspace.changes.iter().enumerate().any(|(i, tab)| {
            (match mode {
                1 => i != ix,
                2 => i > ix,
                _ => i == ix,
            }) && (tab.contains_focus(window, cx) || tab.header_focus.contains_focused(window, cx))
        });
        let next = if mode == 0 {
            ix.checked_sub(1)
                .map(|i| view.workspace.changes[i].id.clone())
        } else {
            Some(id.to_owned())
        };
        let mut position = 0;
        view.workspace.changes.retain(|_| {
            let i = position;
            position += 1;
            match mode {
                1 => i == ix,
                2 => i <= ix,
                _ => i != ix,
            }
        });
        if active_removed {
            view.workspace.active = next;
        }
        self.fit_layout(window, cx);
        if focused {
            self.focus_panel(Panel::Workspace, window, cx);
        }
        cx.notify();
    }
    pub(super) fn render_file_workspace(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = self.workspace_content();
        let active = self
            .session_view()
            .and_then(|v| v.workspace.active.as_ref());
        let index = self
            .session_view()
            .and_then(|view| {
                view.workspace
                    .changes
                    .iter()
                    .position(|tab| Some(&tab.id) == active)
            })
            .map_or(0, |i| i + 1);
        let mut all_tabs = vec![(None, t(cx, "files-title"))];
        let mut tabs = TabBar::new("workspace-tabs")
            .selected_index(index)
            .child(Tab::new().label(t(cx, "files-title")))
            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                let id = ix
                    .checked_sub(1)
                    .and_then(|i| this.session_view()?.workspace.changes.get(i))
                    .map(|tab| tab.id.clone());
                this.select_workspace_tab(id, window, cx);
            }));
        if let Some(view) = self.session_view() {
            for tab in &view.workspace.changes {
                let close = tab.id.clone();
                let menu_id = tab.id.clone();
                let owner = cx.weak_entity();
                let time = tab
                    .timestamp
                    .and_then(|millis| {
                        time::OffsetDateTime::from_unix_timestamp_nanos(
                            i128::from(millis) * 1_000_000,
                        )
                        .ok()
                    })
                    .map(|date| format!("{} {:02}:{:02}", date.date(), date.hour(), date.minute()))
                    .unwrap_or_default();
                let summary = tab
                    .title
                    .lines()
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&time);
                let title = format!("{} · {}", t(cx, "workspace-changes"), summary);
                all_tabs.push((Some(tab.id.clone()), title.clone()));
                let totals =
                    tab.files
                        .iter()
                        .filter_map(|file| file.stats())
                        .fold(None, |total, stats| {
                            let (a, b) = total.unwrap_or((0, 0));
                            Some((a + stats.added(), b + stats.removed()))
                        });
                let mut tooltip = format!("{}\n{}", tab.title, time);
                if let Some((added, removed)) = totals {
                    tooltip.push_str(&format!("\n+{added} −{removed}"));
                }
                let label = h_flex()
                    .id(format!("changes-tab-{}", tab.id))
                    .track_focus(&tab.header_focus)
                    .min_w_0()
                    .gap_2()
                    .child(div().max_w(rems(20.)).truncate().child(title.clone()))
                    .child(
                        Button::new(format!("close-changes-{}", tab.id))
                            .ghost()
                            .xsmall()
                            .icon(IconName::X)
                            .accessibility_label(t(cx, "workspace-close-tab"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.close_workspace_tabs(&close, 0, window, cx);
                            })),
                    )
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                    .context_menu(move |mut menu, _, cx| {
                        for (mode, label) in [
                            (0, "workspace-close-tab"),
                            (1, "workspace-close-others"),
                            (2, "workspace-close-right"),
                        ] {
                            let owner = owner.clone();
                            let id = menu_id.clone();
                            menu = menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                                move |_, window, cx| {
                                    let _ = owner.update(cx, |this, cx| {
                                        this.close_workspace_tabs(&id, mode, window, cx)
                                    });
                                },
                            ));
                        }
                        menu
                    });
                tabs = tabs.child(Tab::new().aria_label(title).child(label));
            }
        }
        let owner = cx.weak_entity();
        tabs = tabs.suffix(
            Button::new("workspace-all-tabs")
                .ghost()
                .small()
                .icon(IconName::ChevronDown)
                .accessibility_label(t(cx, "workspace-all-tabs"))
                .dropdown_menu(move |mut menu, _, cx| {
                    for (id, label) in &all_tabs {
                        let id = id.clone();
                        let owner = owner.clone();
                        menu = menu.item(PopupMenuItem::new(label.clone()).on_click(
                            move |_, window, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.select_workspace_tab(id.clone(), window, cx)
                                });
                            },
                        ));
                    }
                    let _ = cx;
                    menu
                }),
        );
        let mut columns = h_flex().items_stretch().flex_1().min_h_0().min_w_0();
        if let Some(tab) = self.active_changes() {
            columns = columns.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .child(self.render_changes_main(tab, cx)),
            );
        } else if let Some(source) = self.source() {
            columns = columns.child(div().flex_1().min_w_0().h_full().child(source));
        }
        if content.navigator {
            let navigator = if let Some(tab) = self.active_changes() {
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
                    .w(px(self.pane_layout.right))
                    .flex_none()
                    .h_full()
                    .when(content.main, |view| {
                        view.border_l_1().border_color(cx.theme().border)
                    })
                    .child(navigator)
                    .when(content.main, |view| {
                        view.child(self.pane_handle(panes::Side::Right, cx))
                    }),
            );
        }
        v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .track_focus(&self.workspace_region)
            .child(
                h_flex()
                    .min_w_0()
                    .child(div().flex_1().min_w_0().child(tabs))
                    .when(content.main, |row| {
                        row.child(
                            Button::new("workspace-navigator")
                                .ghost()
                                .small()
                                .icon(IconName::PanelRight)
                                .accessibility_label(t(cx, "workspace-toggle-navigator"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(view) = this
                                        .shown_key
                                        .as_ref()
                                        .and_then(|key| this.views.get_mut(key))
                                    {
                                        if let Some(tab) =
                                            view.workspace.changes.iter_mut().find(|tab| {
                                                Some(&tab.id) == view.workspace.active.as_ref()
                                            })
                                        {
                                            tab.navigator_hidden = !tab.navigator_hidden;
                                        } else {
                                            view.workspace.navigator_hidden =
                                                !view.workspace.navigator_hidden;
                                        }
                                    }
                                    this.fit_layout(window, cx);
                                    cx.notify();
                                })),
                        )
                    }),
            )
            .child(columns)
            .into_any_element()
    }
    pub(super) fn render_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
                        .w(px(self.pane_layout.workspace_width))
                        .h_full()
                        .border_l_1()
                        .border_color(cx.theme().border)
                        .child(self.render_file_workspace(cx))
                        .child(self.pane_handle(panes::Side::Source, cx)),
                )
            })
            .into_any_element()
    }
}
