pub(super) mod directory;

use super::*;
use gpui_kit::component::list::{List, ListDelegate, ListItem};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, IndexPath};

pub(super) struct OpenFile(pub PathBuf, pub bool);
impl EventEmitter<OpenFile> for Files {}
use fluent_bundle::FluentArgs;
use gupi_settings::i18n::t_with_args;
use std::path::Path;

actions!(project_files, [Expand, Collapse, First, Last]);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("right", Expand, Some("ProjectFiles")),
        KeyBinding::new("left", Collapse, Some("ProjectFiles")),
        KeyBinding::new("home", First, Some("ProjectFiles")),
        KeyBinding::new("end", Last, Some("ProjectFiles")),
    ]);
}

#[derive(Clone)]
struct Row {
    path: PathBuf,
    kind: directory::Kind,
    link: bool,
    target: Option<PathBuf>,
    depth: usize,
    expanded: bool,
    status: Option<&'static str>,
    error: Option<String>,
}

struct Delegate {
    owner: WeakEntity<Files>,
    rows: Rc<Vec<Row>>,
    selected: Option<(PathBuf, bool)>,
    open_path: Option<PathBuf>,
}

impl ListDelegate for Delegate {
    type Item = ListItem;
    fn items_count(&self, _: usize, _: &App) -> usize {
        self.rows.len()
    }
    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix
            .and_then(|ix| self.rows.get(ix.row))
            .map(|r| (r.path.clone(), r.status.is_some()));
    }
    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<ListItem> {
        let row = self.rows.get(ix.row)?.clone();
        let title = row.status.map(|key| t(cx, key)).unwrap_or_else(|| {
            row.path
                .file_name()
                .unwrap_or(row.path.as_os_str())
                .to_string_lossy()
                .into_owned()
        });
        let owner = self.owner.clone();
        let mut tooltip = row.path.to_string_lossy().into_owned();
        if let Some(target) = &row.target {
            tooltip.push_str(&format!("\n→ {}", target.display()));
        }
        let unavailable_link = row.link && row.error.is_some();
        if unavailable_link {
            tooltip.push_str(&format!("\n{}", t(cx, "files-link-unavailable")));
        }
        if let Some(error) = &row.error {
            tooltip.push('\n');
            tooltip.push_str(error);
        }
        let list = cx.weak_entity();
        let directory = row.kind == directory::Kind::Directory;
        let label = if directory {
            format!(
                "{} — {}",
                title,
                t(
                    cx,
                    if row.expanded {
                        "files-expanded"
                    } else {
                        "files-collapsed"
                    }
                )
            )
        } else {
            title.clone()
        };
        let label = if row.link {
            let mut args = FluentArgs::new();
            args.set("name", title.clone());
            args.set(
                "state",
                t(
                    cx,
                    if row.expanded {
                        "files-expanded"
                    } else {
                        "files-collapsed"
                    },
                ),
            );
            t_with_args(
                cx,
                if unavailable_link {
                    "files-link-unavailable-label"
                } else if directory {
                    "files-link-directory-label"
                } else {
                    "files-link-label"
                },
                &args,
            )
        } else {
            label
        };
        Some(
            ListItem::new(ElementId::Path(row.path.clone().into()))
                .accessibility_label(label)
                .child(
                    h_flex()
                        .h_5()
                        .gap_1()
                        .min_w_0()
                        .w_full()
                        .pl(rems(row.depth as f32))
                        .child(div().w_4().flex_none().when(directory, |view| {
                            view.child(
                                Icon::new(if row.expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size_3(),
                            )
                        }))
                        .child(
                            Icon::new(if row.link {
                                IconName::Link
                            } else if directory {
                                IconName::Folder
                            } else {
                                IconName::FileText
                            })
                            .size_4(),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .when(self.open_path.as_ref() == Some(&row.path), |v| {
                                    v.font_weight(FontWeight::MEDIUM)
                                })
                                .when(unavailable_link, |view| {
                                    view.text_color(cx.theme().muted_foreground)
                                })
                                .child(title),
                        )
                        .child(
                            div()
                                .w_4()
                                .flex_none()
                                .when(self.open_path.as_ref() == Some(&row.path), |v| {
                                    v.child(Icon::new(IconName::FileCode).size_3())
                                }),
                        ),
                )
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .on_click(move |_, window, cx| {
                    let _ = list.update(cx, |list, cx| {
                        list.set_selected_index(Some(ix), window, cx);
                        list.focus_handle(cx).focus(window, cx);
                        cx.notify();
                    });
                    let _ = owner.update(cx, |this, cx| {
                        if directory || row.status.is_some() {
                            this.activate(row.clone(), window, cx);
                        } else if row.kind == directory::Kind::File {
                            cx.emit(OpenFile(row.path.clone(), false));
                        }
                        cx.notify();
                    });
                    cx.stop_propagation();
                }),
        )
    }
    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let row = self
            .selected
            .as_ref()
            .and_then(|(path, status)| {
                self.rows
                    .iter()
                    .find(|r| &r.path == path && r.status.is_some() == *status)
            })
            .cloned();
        let owner = self.owner.clone();
        // Defer past the delegate lock before changing the list projection.
        if let Some(row) = row {
            cx.spawn_in(window, async move |_, cx| {
                let _ = owner.update_in(cx, |this, window, cx| this.activate(row, window, cx));
            })
            .detach();
        }
    }
}

pub(super) struct Files {
    root: PathBuf,
    generation: u64,
    directories: HashMap<PathBuf, Result<directory::Directory, String>>,
    expanded: HashSet<PathBuf>,
    tasks: HashMap<PathBuf, Task<()>>,
    list: Entity<ListState<Delegate>>,
    open_path: Option<PathBuf>,
    reveal_path: Option<PathBuf>,
    _subscriptions: Vec<Subscription>,
}

impl Files {
    pub(super) fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let owner = cx.weak_entity();
        let list = cx.new(|inner| {
            ListState::new(
                Delegate {
                    owner,
                    rows: Rc::new(vec![]),
                    selected: None,
                    open_path: None,
                },
                window,
                inner,
            )
            .searchable(false)
        });
        let subscriptions = vec![cx.observe(&list, |_, _, cx| cx.notify())];
        Self {
            root,
            generation: 0,
            directories: HashMap::new(),
            expanded: HashSet::new(),
            tasks: HashMap::new(),
            list,
            open_path: None,
            reveal_path: None,
            _subscriptions: subscriptions,
        }
    }
    pub(super) fn ensure_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.directories.contains_key(&self.root) && !self.is_loading(&self.root) {
            self.read(self.root.clone(), window, cx);
            self.rebuild(window, cx);
        }
    }
    #[cfg(test)]
    pub(super) fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.list.focus_handle(cx).is_focused(window)
    }
    pub(super) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.list.focus_handle(cx).focus(window, cx);
    }
    fn is_loading(&self, path: &Path) -> bool {
        self.tasks.contains_key(path)
    }
    fn read(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_loading(&path) {
            return;
        }
        let generation = self.generation;
        let target = path.clone();
        let result_path = path.clone();
        let read = cx.background_spawn(async move { directory::read(&target) });
        let task = cx.spawn_in(window, async move |owner, cx| {
            let result = read.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.install(generation, result_path, result, window, cx);
            });
        });
        self.tasks.insert(path, task);
    }
    fn install(
        &mut self,
        generation: u64,
        path: PathBuf,
        result: Result<directory::Directory, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if generation != self.generation {
            return;
        }
        self.tasks.remove(&path);
        if let Ok(directory) = &result {
            let children: HashSet<_> = directory
                .entries
                .iter()
                .filter(|entry| entry.kind == directory::Kind::Directory)
                .map(|entry| entry.path.as_path())
                .collect();
            // Remove former directory subtrees, including reads still in flight.
            let keep = |candidate: &PathBuf| {
                candidate
                    .ancestors()
                    .find(|ancestor| ancestor.parent() == Some(path.as_path()))
                    .is_none_or(|child| children.contains(child))
            };
            self.expanded.retain(keep);
            self.directories.retain(|candidate, _| keep(candidate));
            self.tasks.retain(|candidate, _| keep(candidate));
        }
        self.directories.insert(path, result);
        if let Some(path) = self.reveal_path.clone() {
            self.reveal(path, window, cx);
        }
        self.rebuild(window, cx);
    }
    fn rows(&self, path: &Path, depth: usize, rows: &mut Vec<Row>) {
        match self.directories.get(path) {
            Some(Ok(dir)) => {
                for entry in &dir.entries {
                    let expanded = entry.kind == directory::Kind::Directory
                        && self.expanded.contains(&entry.path);
                    rows.push(Row {
                        path: entry.path.clone(),
                        kind: entry.kind,
                        link: entry.link,
                        target: entry.target.clone(),
                        depth,
                        expanded,
                        status: None,
                        error: entry.error.clone(),
                    });
                    if expanded {
                        self.rows(&entry.path, depth + 1, rows);
                    }
                }
                if dir.incomplete {
                    rows.push(Row {
                        path: path.to_owned(),
                        kind: directory::Kind::Other,
                        link: false,
                        target: None,
                        depth,
                        expanded: false,
                        status: Some("files-incomplete"),
                        error: None,
                    });
                }
            }
            result => {
                let loading = self.is_loading(path);
                if result.is_none() && !loading {
                    return;
                }
                rows.push(Row {
                    path: path.to_owned(),
                    kind: directory::Kind::Other,
                    link: false,
                    target: None,
                    depth,
                    expanded: false,
                    status: Some(if loading {
                        "files-loading"
                    } else {
                        "files-read-failed"
                    }),
                    error: (!loading)
                        .then(|| result.and_then(|result| result.as_ref().err()).cloned())
                        .flatten(),
                });
            }
        }
    }
    fn rebuild(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rows = vec![];
        self.rows(&self.root, 0, &mut rows);
        let unread: Vec<_> = rows
            .iter()
            .filter(|row| {
                row.kind == directory::Kind::Directory
                    && row.expanded
                    && !self.directories.contains_key(&row.path)
                    && !self.is_loading(&row.path)
            })
            .map(|row| row.path.clone())
            .collect();
        if !unread.is_empty() {
            for path in unread {
                self.read(path, window, cx);
            }
            rows.clear();
            self.rows(&self.root, 0, &mut rows);
        }
        self.list.update(cx, |list, cx| {
            let mut selected = list
                .delegate()
                .selected
                .as_ref()
                .map(|(path, _)| path.clone());
            while selected
                .as_ref()
                .is_some_and(|p| !rows.iter().any(|r| &r.path == p))
            {
                selected = selected.and_then(|p| p.parent().map(Path::to_owned));
            }
            let ix = selected
                .and_then(|p| rows.iter().position(|r| r.path == p))
                .map(IndexPath::new);
            list.delegate_mut().rows = Rc::new(rows);
            list.set_selected_index(ix, window, cx);
            cx.notify();
        });
        cx.notify();
    }
    fn selected(&self, cx: &App) -> Option<Row> {
        let list = self.list.read(cx);
        list.selected_index()
            .and_then(|ix| list.delegate().rows.get(ix.row))
            .cloned()
    }
    fn activate(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        if !row.path.starts_with(&self.root) {
            return;
        }
        if row.status.is_some() {
            self.read(row.path, window, cx);
            self.rebuild(window, cx);
        } else if row.kind == directory::Kind::Directory {
            if !self.expanded.remove(&row.path) {
                self.expanded.insert(row.path.clone());
                if !self.directories.contains_key(&row.path) {
                    self.read(row.path, window, cx);
                }
            }
            self.rebuild(window, cx);
        } else if row.kind == directory::Kind::File {
            cx.emit(OpenFile(row.path, true));
        }
    }
    pub(super) fn set_open_path(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.open_path = path.clone();
        self.list.update(cx, |list, cx| {
            list.delegate_mut().open_path = path;
            cx.notify();
        });
        cx.notify();
    }
    pub(super) fn reveal(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_path = Some(path.clone());
        if let Ok(relative) = path.strip_prefix(&self.root) {
            let mut parent = self.root.clone();
            for part in relative.components() {
                match self.directories.get(&parent) {
                    Some(Ok(_)) => {}
                    Some(Err(_)) => {
                        self.reveal_path = None;
                        break;
                    }
                    None => {
                        self.read(parent, window, cx);
                        self.rebuild(window, cx);
                        return;
                    }
                }
                parent.push(part);
                if parent != path {
                    self.expanded.insert(parent.clone());
                }
            }
        }
        self.reveal_path = None;
        self.rebuild(window, cx);
        self.focus_open(window, cx);
    }
    pub(super) fn focus_open(&self, window: &mut Window, cx: &mut Context<Self>) {
        let mut path = self.open_path.clone();
        while let Some(p) = path {
            if let Some(ix) = self
                .list
                .read(cx)
                .delegate()
                .rows
                .iter()
                .position(|r| r.path == p && r.status.is_none())
            {
                self.select(ix, window, cx);
                break;
            }
            path = p.parent().map(Path::to_owned);
        }
        self.focus(window, cx);
    }
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let visible: Vec<_> = self
            .list
            .read(cx)
            .delegate()
            .rows
            .iter()
            .filter(|r| r.kind == directory::Kind::Directory && r.expanded)
            .map(|r| r.path.clone())
            .collect();
        self.generation += 1;
        self.tasks.clear();
        self.directories
            .retain(|p, _| p == &root || visible.contains(p));
        self.read(root, window, cx);
        for path in visible {
            self.read(path, window, cx);
        }
        self.rebuild(window, cx);
    }
    fn navigate(&mut self, expand: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.selected(cx) else {
            return;
        };
        if row.kind == directory::Kind::Directory && row.expanded != expand {
            self.activate(row, window, cx);
            return;
        }
        let rows = &self.list.read(cx).delegate().rows;
        let target = if expand && row.expanded {
            rows.iter()
                .position(|r| r.path == row.path)
                .and_then(|ix| rows.get(ix + 1))
                .filter(|r| r.depth > row.depth)
                .map(|r| r.path.clone())
        } else if !expand {
            row.path.parent().map(Path::to_owned)
        } else {
            None
        };
        let ix = target.and_then(|p| rows.iter().position(|r| r.path == p));
        if let Some(ix) = ix {
            self.select(ix, window, cx);
        }
    }
    fn select(&self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| {
            list.set_selected_index(Some(IndexPath::new(ix)), window, cx);
            list.scroll_to_selected_item(window, cx);
            cx.notify();
        });
    }
}

impl Render for Files {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.clone();
        v_flex()
            .size_full()
            .min_h_0()
            .key_context("ProjectFiles")
            .on_action(cx.listener(|this, _: &Expand, window, cx| this.navigate(true, window, cx)))
            .on_action(
                cx.listener(|this, _: &Collapse, window, cx| this.navigate(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &First, window, cx| this.select(0, window, cx)))
            .on_action(cx.listener(|this, _: &Last, window, cx| {
                if let Some(ix) = this.list.read(cx).delegate().rows.len().checked_sub(1) {
                    this.select(ix, window, cx);
                }
            }))
            .map(|view| {
                let full = root.to_string_lossy().into_owned();
                let name = root
                    .file_name()
                    .unwrap_or(root.as_os_str())
                    .to_string_lossy()
                    .into_owned();
                view.child(
                    h_flex()
                        .h_9()
                        .px_3()
                        .gap_1()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .id("files-root")
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(name)
                                .tooltip({
                                    let full = full.clone();
                                    move |window, cx| Tooltip::new(full.clone()).build(window, cx)
                                }),
                        )
                        .child(
                            Button::new("files-refresh")
                                .ghost()
                                .small()
                                .icon(IconName::RefreshCw)
                                .accessibility_label(t(cx, "files-refresh"))
                                .tooltip(t(cx, "files-refresh"))
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                                ),
                        )
                        .child(
                            Button::new("files-root-actions")
                                .ghost()
                                .small()
                                .icon(IconName::Ellipsis)
                                .accessibility_label(t(cx, "files-actions"))
                                .dropdown_menu(move |menu, _, cx| {
                                    menu.item(
                                        gpui_kit::component::menu::PopupMenuItem::new(t(
                                            cx,
                                            "files-copy-path",
                                        ))
                                        .on_click({
                                            let full = full.clone();
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    full.clone(),
                                                ))
                                            }
                                        }),
                                    )
                                }),
                        ),
                )
                .child(div().flex_1().min_h_0().child(List::new(&self.list)))
            })
    }
}

impl HomeView {
    pub(super) fn render_right_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        use gpui_kit::component::tab::{Tab, TabBar};
        v_flex()
            .size_full()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .child(
                h_flex()
                    .h_10()
                    .px_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        TabBar::new("navigator-tabs")
                            .segmented()
                            .small()
                            .selected_index(usize::from(self.files_tab))
                            .child(Tab::new().label(t(cx, "conversation-history")))
                            .child(Tab::new().label(t(cx, "files-title")))
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.files_tab = *ix == 1;
                                this.sync(!this.files_tab, window, cx);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("right-close")
                            .small()
                            .ghost()
                            .icon(IconName::X)
                            .accessibility_label(t(cx, "files-close"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_navigator(window, cx);
                            })),
                    ),
            )
            .child(div().flex_1().min_h_0().child(if self.files_tab {
                self.files()
                    .map(|files| files.into_any_element())
                    .unwrap_or_else(|| {
                        div()
                            .p_3()
                            .child(t(cx, "files-no-session"))
                            .into_any_element()
                    })
            } else {
                self.render_history(cx)
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::Files;
    use super::directory::{Directory, Entry, Kind};
    use gpui_kit::component::Root;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, TestAppContext, px, size};
    use std::path::PathBuf;

    #[gpui_kit::test]
    fn conversations_own_independent_file_trees_and_source_previews(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gpui_tokio::init(cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(gupi_settings::layout::LayoutState::default());
        });
        let state = cx.new(|cx| {
            gupi_conversation::conversation::ConversationState::new("unused-pi".into(), cx)
        });
        state.update(cx, |state, _| {
            for (key, cwd) in [
                ("a", "/project-a"),
                ("b", "/project-b"),
                ("c", "/project-a"),
            ] {
                let info = gupi_conversation::session_catalog::SessionInfo::new(
                    Default::default(),
                    key.into(),
                    cwd.into(),
                );
                state.sessions_for_test().insert(
                    key.into(),
                    gupi_conversation::conversation::Session::new(info, String::new()),
                );
            }
            *state.selected_for_test() = Some("a".into());
        });
        let mut home = None;
        let window = cx.open_window(size(px(2000.), px(740.)), |window, cx| {
            let view = cx.new(|cx| super::HomeView::with_state(state.clone(), window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        let mut original_tree = None;
        let mut original_source = None;
        cx.update_window(window.into(), |_, window, cx| {
            home.update(cx, |home, cx| {
                let files = home.files().unwrap();
                files.update(cx, |files, cx| {
                    let root = files.root.clone();
                    files.install(
                        0,
                        root.clone(),
                        Ok(Directory {
                            entries: (0..100)
                                .map(|index| Entry {
                                    path: root.join(format!("{index:03}")),
                                    kind: Kind::Directory,
                                    link: false,
                                    target: None,
                                    error: None,
                                })
                                .collect(),
                            incomplete: false,
                        }),
                        window,
                        cx,
                    );
                    files.select(80, window, cx);
                    files.expanded.insert(root.join("080"));
                    files.directories.insert(
                        root.join("080"),
                        Ok(Directory {
                            entries: vec![],
                            incomplete: false,
                        }),
                    );
                    files.rebuild(window, cx);
                });
                home.show_history = true;
                home.files_tab = true;
                home.open_source("/project-a/preview.rs".into(), false, window, cx);
                original_tree = Some(files);
                original_source = home.source();
            });
            window.render_frame(cx);
        })
        .unwrap();
        let files_a = original_tree.unwrap();
        let source_a = original_source.unwrap();
        let offset_a = cx.update(|cx| {
            files_a
                .read(cx)
                .list
                .read(cx)
                .scroll_handle()
                .base_handle()
                .offset()
        });
        assert!(offset_a.y < px(0.));
        cx.update_window(window.into(), |_, window, cx| {
            for key in ["b", "c"] {
                state.update(cx, |state, _| *state.selected_for_test() = Some(key.into()));
                home.update(cx, |home, cx| {
                    home.sync(false, window, cx);
                    let files = home.files().unwrap();
                    assert_ne!(files.entity_id(), files_a.entity_id());
                    assert!(files.read(cx).expanded.is_empty());
                    assert!(files.read(cx).list.read(cx).selected_index().is_none());
                    assert_eq!(
                        files
                            .read(cx)
                            .list
                            .read(cx)
                            .scroll_handle()
                            .base_handle()
                            .offset()
                            .y,
                        px(0.)
                    );
                    assert!(home.source().is_none());
                    home.open_source(format!("/project-{key}/other.rs").into(), false, window, cx);
                    assert_ne!(home.source().unwrap().entity_id(), source_a.entity_id());
                });
                window.render_frame(cx);
            }
            state.update(cx, |state, _| *state.selected_for_test() = Some("a".into()));
            home.update(cx, |home, cx| {
                home.sync(false, window, cx);
                assert_eq!(home.files().unwrap().entity_id(), files_a.entity_id());
                assert_eq!(home.source().unwrap().entity_id(), source_a.entity_id());
                let files = files_a.read(cx);
                assert!(files.expanded.contains(&files.root.join("080")));
                assert_eq!(
                    files.list.read(cx).selected_index(),
                    Some(gpui_kit::component::IndexPath::new(80))
                );
                assert_eq!(
                    files.list.read(cx).scroll_handle().base_handle().offset(),
                    offset_a
                );
                assert!(home.source_active());
            });
            window.render_frame(cx);
            assert_eq!(
                files_a
                    .read(cx)
                    .list
                    .read(cx)
                    .scroll_handle()
                    .base_handle()
                    .offset(),
                offset_a
            );
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn refresh_reloads_expanded_descendants_when_they_become_visible(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            app_theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
        });
        let root = std::env::temp_dir().join(format!(
            "gupi-tree-refresh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let a = root.join("A");
        let b = a.join("B");
        let file = b.join("file.rs");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(&file, "fn main() {}").unwrap();
        let mut files = None;
        let window = cx.open_window(size(px(500.), px(600.)), |window, cx| {
            let view = cx.new(|cx| Files::new(root.clone(), window, cx));
            view.update(cx, |this, cx| {
                for path in [&root, &a, &b] {
                    this.install(0, path.clone(), super::directory::read(path), window, cx);
                }
                this.expanded.extend([a.clone(), b.clone()]);
                this.rebuild(window, cx);
                let row = this.list.read(cx).delegate().rows[0].clone();
                this.activate(row, window, cx); // Collapse A, retaining B's expansion.
                this.refresh(window, cx);
                assert!(this.expanded.contains(&b));
                assert!(!this.directories.contains_key(&b));
                assert!(!this.is_loading(&b));
                let mut rows = vec![];
                this.rows(&b, 0, &mut rows);
                assert!(rows.is_empty(), "no task means no loading placeholder");
                this.directories
                    .insert(b.clone(), Err("old read error".into()));
                this.tasks.insert(
                    b.clone(),
                    cx.spawn(async |_, _| std::future::pending::<()>().await),
                );
                this.rows(&b, 0, &mut rows);
                assert_eq!(rows[0].status, Some("files-loading"));
                assert!(rows[0].error.is_none());
                this.tasks.remove(&b);
                rows.clear();
                this.rows(&b, 0, &mut rows);
                assert_eq!(rows[0].status, Some("files-read-failed"));
                this.directories.remove(&b);
            });
            files = Some(view.clone());
            Root::new(view, window, cx)
        });
        let files = files.unwrap();
        cx.run_until_parked();
        cx.update_window(window.into(), |_, window, cx| {
            files.update(cx, |this, cx| {
                let row = this.list.read(cx).delegate().rows[0].clone();
                this.activate(row, window, cx); // Reopen A and lazily reread A/B.
                assert!(this.is_loading(&a));
                assert!(
                    this.list
                        .read(cx)
                        .delegate()
                        .rows
                        .iter()
                        .any(|row| { row.path == a && row.status == Some("files-loading") })
                );
            });
        })
        .unwrap();
        cx.run_until_parked();
        files.update(cx, |this, cx| {
            assert!(!this.is_loading(&a));
            assert!(!this.is_loading(&b));
            assert!(this.directories.get(&b).is_some_and(Result::is_ok));
            let rows = &this.list.read(cx).delegate().rows;
            assert!(
                rows.iter()
                    .any(|row| row.path == file && row.status.is_none())
            );
            assert!(rows.iter().all(|row| row.status != Some("files-loading")));
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[gpui_kit::test]
    fn refreshed_parent_removes_former_directory_subtrees(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            app_theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
        });
        let mut files = None;
        let window = cx.open_window(size(px(500.), px(600.)), |window, cx| {
            let view = cx.new(|cx| Files::new("/project".into(), window, cx));
            files = Some(view.clone());
            Root::new(view, window, cx)
        });
        let files = files.unwrap();
        cx.update_window(window.into(), |_, window, cx| {
            files.update(cx, |this, cx| {
                let changed = PathBuf::from("/project/changed");
                let nested = changed.join("nested");
                let kept = PathBuf::from("/project/kept");
                for replacement in [Some(Kind::File), Some(Kind::Other), None] {
                    for path in [&changed, &nested, &kept] {
                        this.expanded.insert(path.clone());
                        this.directories.insert(
                            path.clone(),
                            Ok(Directory {
                                entries: vec![],
                                incomplete: false,
                            }),
                        );
                        this.tasks.insert(
                            path.clone(),
                            cx.spawn(async |_, _| std::future::pending::<()>().await),
                        );
                    }
                    let entry = |path: PathBuf, kind| Entry {
                        path,
                        kind,
                        link: false,
                        target: None,
                        error: None,
                    };
                    let mut entries = vec![entry(kept.clone(), Kind::Directory)];
                    if let Some(kind) = replacement {
                        entries.insert(0, entry(changed.clone(), kind));
                    }
                    this.install(
                        0,
                        "/project".into(),
                        Ok(Directory {
                            entries,
                            incomplete: false,
                        }),
                        window,
                        cx,
                    );
                    for path in [&changed, &nested] {
                        assert!(!this.expanded.contains(path));
                        assert!(!this.directories.contains_key(path));
                        assert!(!this.tasks.contains_key(path));
                    }
                    assert!(this.expanded.contains(&kept));
                    assert!(this.directories.contains_key(&kept));
                    assert!(this.tasks.contains_key(&kept));
                    let rows = &this.list.read(cx).delegate().rows;
                    let kept_rows: Vec<_> = rows.iter().filter(|row| row.path == kept).collect();
                    assert_eq!(kept_rows.len(), 1);
                    assert!(kept_rows[0].expanded);
                    assert!(kept_rows[0].status.is_none());
                    // Even stale expansion state cannot add children to a file.
                    this.expanded.insert(changed.clone());
                    this.directories
                        .insert(changed.clone(), Err("not a directory".into()));
                    this.rebuild(window, cx);
                    assert!(this.list.read(cx).delegate().rows.iter().all(|row| {
                        !row.path.starts_with(&changed)
                            || (row.path == changed && row.status.is_none() && !row.expanded)
                    }));
                }
            });
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn tree_navigation_and_old_results_preserve_the_current_project(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            super::init(cx);
            app_theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
        });
        let mut files = None;
        let window = cx.open_window(size(px(500.), px(600.)), |window, cx| {
            let view = cx.new(|cx| Files::new("/project".into(), window, cx));
            view.update(cx, |this, cx| {
                this.install(
                    0,
                    "/project".into(),
                    Ok(Directory {
                        entries: vec![
                            Entry {
                                path: "/project/src".into(),
                                kind: Kind::Directory,
                                link: true,
                                target: Some("/external-target".into()),
                                error: None,
                            },
                            Entry {
                                path: "/project/README.md".into(),
                                kind: Kind::File,
                                link: true,
                                target: Some("/external-target".into()),
                                error: None,
                            },
                            Entry {
                                path: "/project/broken".into(),
                                kind: Kind::Other,
                                link: true,
                                target: Some("/missing-target".into()),
                                error: Some("target unavailable".into()),
                            },
                        ],
                        incomplete: false,
                    }),
                    window,
                    cx,
                );
                this.directories.insert(
                    "/project/src".into(),
                    Ok(Directory {
                        entries: vec![Entry {
                            path: "/project/src/main.rs".into(),
                            kind: Kind::File,
                            link: false,
                            target: None,
                            error: None,
                        }],
                        incomplete: false,
                    }),
                );
            });
            files = Some(view.clone());
            Root::new(view, window, cx)
        });
        let files = files.unwrap();
        let opened = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let received = opened.clone();
        let _subscription = files.update(cx, |_, cx| {
            cx.subscribe(&files, move |_, _, event: &super::OpenFile, _| {
                received.borrow_mut().push(event.0.clone());
            })
        });
        cx.update_window(window.into(), |_, window, cx| {
            window.render_frame(cx);
            files.update(cx, |this, cx| this.focus(window, cx));
            window.press("down", cx);
            window.press("right", cx);
            files.update(cx, |this, _| {
                assert!(this.expanded.contains(&PathBuf::from("/project/src")));
            });
            window.press("right", cx);
            files.update(cx, |this, cx| {
                assert_eq!(
                    this.selected(cx).unwrap().path,
                    PathBuf::from("/project/src/main.rs")
                )
            });
            window.press("left", cx);
            window.press("left", cx);
            window.press("end", cx);
            window.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(opened.borrow().is_empty());
        cx.update_window(window.into(), |_, window, cx| {
            window.press("up", cx);
            window.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(*opened.borrow(), vec![PathBuf::from("/project/README.md")]);
        cx.update_window(window.into(), |_, window, cx| {
            files.update(cx, |this, cx| {
                assert!(!this.expanded.contains(&PathBuf::from("/project/src")));
                // A directory read finishing after collapse must not reopen it.
                this.install(
                    0,
                    "/project/src".into(),
                    Ok(Directory {
                        entries: vec![],
                        incomplete: false,
                    }),
                    window,
                    cx,
                );
                assert!(!this.expanded.contains(&PathBuf::from("/project/src")));
                this.generation += 1;
                this.install(
                    0,
                    "/project".into(),
                    Ok(Directory {
                        entries: vec![],
                        incomplete: false,
                    }),
                    window,
                    cx,
                );
                assert_eq!(this.generation, 1);
                assert_eq!(this.root, PathBuf::from("/project"));
                assert_eq!(
                    this.directories[&PathBuf::from("/project")]
                        .as_ref()
                        .unwrap()
                        .entries
                        .len(),
                    3
                );
            });
        })
        .unwrap();
    }
}
