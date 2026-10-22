use super::*;
use gpui_kit::component::{
    Icon, IndexPath,
    list::{ListDelegate, ListItem},
};

pub(super) enum Item {
    Group(Group, usize, bool),
    File(Row, &'static str, usize),
    Directory(Directory, usize, bool, Option<(usize, usize)>),
}
pub(in crate::home::workspace) struct Delegate {
    owner: WeakEntity<HomeView>,
    pub(super) items: Vec<Item>,
    selected: Option<String>,
    pub(super) index: Option<usize>,
}
impl Delegate {
    pub fn new(owner: WeakEntity<HomeView>) -> Self {
        Self {
            owner,
            items: Vec::new(),
            selected: None,
            index: None,
        }
    }
}
impl ListDelegate for Delegate {
    type Item = ListItem;
    fn items_count(&self, _: usize, _: &App) -> usize {
        self.items.len()
    }
    fn render_empty(
        &mut self,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
    }
    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.index = ix.map(|i| i.row);
        let selected = self.index.and_then(|ix| match self.items.get(ix) {
            Some(Item::File(row, _, _)) => Some(row.path.clone()),
            _ => None,
        });
        if self.selected == selected {
            return;
        }
        self.selected = selected.clone();
        let Some(path) = selected else { return };
        let owner = self.owner.clone();
        let list = cx.entity_id();
        cx.spawn_in(window, async move |_, cx| {
            let _ = owner.update_in(cx, |home, window, cx| {
                if home.active_git().is_some_and(|tab| {
                    tab.list.entity_id() == list
                        && tab.list.read(cx).delegate().selected.as_ref() == Some(&path)
                        && tab.selected.as_ref() != Some(&path)
                }) {
                    let focus = home.active_git().unwrap().focus.clone();
                    let retained = focus.contains_focused(window, cx);
                    home.select_git_file(path, window, cx);
                    if retained {
                        focus.focus(window, cx);
                    }
                }
            });
        })
        .detach();
    }
    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<ListItem> {
        let item = self.items.get(ix.row)?;
        let mut detail = None;
        let (id, label, body) = match item {
            Item::Group(group, count, closed) => {
                let sources: &[ChangeSource] = match group {
                    Group::Staged => &[ChangeSource::Staged],
                    Group::Unstaged => &[ChangeSource::Unstaged, ChangeSource::Untracked],
                    Group::Conflicts => &[],
                };
                let (known, error) = self
                    .owner
                    .upgrade()
                    .map(|home| {
                        let tab = home.read(cx).active_git();
                        (
                            tab.is_some_and(|tab| {
                                sources.iter().all(|s| tab.known_sources.contains(s))
                            }),
                            tab.and_then(|tab| {
                                sources.iter().find_map(|s| tab.group_errors.get(s))
                            })
                            .cloned(),
                        )
                    })
                    .unwrap_or((false, None));
                detail = error.as_ref().map(|e| match e {
                    Failure::Unavailable => t(cx, "diff-git-unavailable"),
                    Failure::Query(text) => format!("{} {text}", t(cx, "diff-refresh-failed")),
                });
                let count = if known {
                    count.to_string()
                } else {
                    "—".into()
                };
                let label = format!(
                    "{}  {count}{}",
                    t(cx, group.key()),
                    if error.is_some() { " !" } else { "" }
                );
                (
                    format!("git-group-{}", group.key()),
                    label.clone(),
                    h_flex()
                        .h_7()
                        .min_w_0()
                        .gap_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            Icon::new(if *closed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            })
                            .size_3(),
                        )
                        .child(label)
                        .into_any_element(),
                )
            }
            Item::Directory(key, count, closed, stats) => {
                let label = format!("{}  {count}", key.path.display());
                detail = Some(match stats {
                    Some((added, removed)) => format!("{label}\n+{added} −{removed}"),
                    None => format!("{label}\n{}", t(cx, "diff-stats-incomplete")),
                });
                (
                    key.id(),
                    label,
                    h_flex()
                        .h_7()
                        .min_w_0()
                        .w_full()
                        .gap_1()
                        .pl(rems(key.path.components().count() as f32 * 0.75))
                        .child(
                            Icon::new(if *closed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            })
                            .size_3(),
                        )
                        .child(
                            Icon::new(IconName::Folder)
                                .size_3()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .child(
                            div().flex_1().min_w_0().text_sm().truncate().child(
                                key.path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned(),
                            ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(count.to_string()),
                        )
                        .into_any_element(),
                )
            }
            Item::File(row, code, depth) => {
                let path = std::path::Path::new(&row.label);
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let directory = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| p.display().to_string())
                    .filter(|_| *depth == 0)
                    .unwrap_or_default();
                let label = format!("{} · {}", row.label, t(cx, row.status.unwrap()));
                (
                    format!("git-file-{}", row.path),
                    label,
                    h_flex()
                        .h_7()
                        .min_w_0()
                        .w_full()
                        .gap_2()
                        .pl(rems(*depth as f32 * 0.75))
                        .child(
                            div()
                                .w_3()
                                .flex_none()
                                .text_xs()
                                .text_color(if *code == "!" {
                                    cx.theme().danger
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child(*code),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .max_w_full()
                                .text_sm()
                                .truncate()
                                .child(name),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_xs()
                                .truncate()
                                .text_color(cx.theme().muted_foreground)
                                .child(directory),
                        )
                        .when_some(row.stats, |body, (added, removed)| {
                            body.child(reader::line_stats(added, removed, cx))
                        })
                        .into_any_element(),
                )
            }
        };
        let list = cx.weak_entity();
        let tooltip = detail.unwrap_or_else(|| label.clone());
        Some(
            ListItem::new(id)
                .role(Role::Button)
                .when_some(
                    match item {
                        Item::Group(_, _, closed) | Item::Directory(_, _, closed, _) => {
                            Some(!closed)
                        }
                        Item::File(..) => None,
                    },
                    |row, expanded| row.aria_expanded(expanded),
                )
                .h_7()
                .py_0()
                .px_2()
                .accessibility_label(label.clone())
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(body)
                .on_click(move |_, window, cx| {
                    let _ = list.update(cx, |list, cx| {
                        list.set_selected_index(Some(ix), window, cx);
                        list.delegate_mut().confirm(false, window, cx);
                    });
                    cx.stop_propagation();
                }),
        )
    }
    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let owner = self.owner.clone();
        let list = cx.entity_id();
        let Some(item) = self.index.and_then(|ix| self.items.get(ix)) else {
            return;
        };
        let action = match item {
            Item::Group(group, _, _) => Navigation::Group(*group),
            Item::Directory(key, _, _, _) => Navigation::Directory(key.clone()),
            Item::File(row, _, _) => Navigation::File(row.path.clone()),
        };
        cx.spawn_in(window, async move |_, cx| {
            let _ = owner.update_in(cx, |home, window, cx| {
                if home
                    .active_git()
                    .is_some_and(|t| t.list.entity_id() == list)
                {
                    match action {
                        Navigation::Group(group) => home.toggle_git_group(group, window, cx),
                        Navigation::Directory(key) => home.toggle_git_directory(key, window, cx),
                        Navigation::File(path) => home.select_git_file(path, window, cx),
                    }
                }
            });
        })
        .detach();
    }
}

enum Navigation {
    Group(Group),
    Directory(Directory),
    File(String),
}
impl Item {
    pub(super) fn depth(&self) -> usize {
        match self {
            Self::Group(..) => 0,
            Self::Directory(key, ..) => key.path.components().count(),
            Self::File(_, _, depth) => *depth,
        }
    }

    pub(super) fn id(&self) -> String {
        match self {
            Self::Group(group, _, _) => format!("git-group-{}", group.key()),
            Self::Directory(key, _, _, _) => key.id(),
            Self::File(row, _, _) => format!("git-file-{}", row.path),
        }
    }
}
