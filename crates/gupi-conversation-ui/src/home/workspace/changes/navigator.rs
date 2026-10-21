use super::*;

#[derive(Clone)]
pub(in crate::home::workspace) struct Row {
    pub path: String,
    pub label: String,
    pub status: Option<&'static str>,
    pub stats: Option<(usize, usize)>,
    pub failed: bool,
}
impl From<&EditedFile> for Row {
    fn from(file: &EditedFile) -> Self {
        Self {
            path: file.path().into(),
            label: file.path().into(),
            status: file.written().then_some("conversation-edit-write"),
            stats: file.stats().map(|s| (s.added(), s.removed())),
            failed: file.failed(),
        }
    }
}
use gpui_kit::component::IndexPath;
use gpui_kit::component::list::ListDelegate;

pub(in crate::home::workspace) struct Delegate {
    owner: WeakEntity<HomeView>,
    tab: String,
    pub files: Rc<Vec<Row>>,
    selected: Option<String>,
}

impl Delegate {
    pub fn new(owner: WeakEntity<HomeView>, tab: String, files: Rc<Vec<Row>>) -> Self {
        Self {
            owner,
            tab,
            files,
            selected: None,
        }
    }
}

impl ListDelegate for Delegate {
    type Item = Button;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.files.len()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        let selected = ix
            .and_then(|ix| self.files.get(ix.row))
            .map(|f| f.path.clone());
        if selected == self.selected {
            return;
        }
        self.selected = selected.clone();
        let Some(path) = selected else {
            return;
        };
        let owner = self.owner.clone();
        let list = cx.entity_id();
        cx.spawn_in(window, async move |_, cx| {
            let _ = owner.update_in(cx, |home, window, cx| {
                let current_selection = home
                    .active_git()
                    .filter(|t| t.list.entity_id() == list)
                    .map(|t| t.list.read(cx).delegate().selected.clone())
                    .or_else(|| {
                        home.active_changes()
                            .filter(|t| t.list.entity_id() == list)
                            .map(|t| t.list.read(cx).delegate().selected.clone())
                    })
                    .flatten();
                if current_selection.as_ref() != Some(&path) {
                    return;
                }
                let changed = home.active_git().is_some_and(|t| {
                    t.list.entity_id() == list && t.selected.as_ref() != Some(&path)
                }) || home.active_changes().is_some_and(|t| {
                    t.list.entity_id() == list && t.in_scope && t.selected.as_ref() != Some(&path)
                });
                if changed {
                    let focus = home
                        .active_git()
                        .map(|t| t.focus.clone())
                        .or_else(|| home.active_changes().map(|t| t.focus.clone()))
                        .unwrap();
                    let keep_focus = focus.contains_focused(window, cx);
                    home.select_reader_file(path, window, cx);
                    if keep_focus {
                        focus.focus(window, cx);
                    }
                }
            });
        })
        .detach();
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Button> {
        let file = self.files.get(ix.row)?;
        let path = file.path.clone();
        let name = std::path::Path::new(&file.label)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let directory = file.label.strip_suffix(&name).unwrap_or("").to_owned();
        let selector = format!("changes-file-{}-{path}", self.tab);
        let row = h_flex()
            .debug_selector(move || selector.clone())
            .min_w_0()
            .w_full()
            .gap_2()
            .child(
                Icon::new(IconName::File)
                    .size_4()
                    .flex_none()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div().flex_1().min_w_0().truncate().child(
                    h_flex()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_color(cx.theme().muted_foreground)
                                .child(directory),
                        )
                        .child(div().flex_none().child(name)),
                ),
            )
            .when_some(file.status, |row, key| {
                row.child(div().text_xs().child(t(cx, key)))
            })
            .when(file.failed, |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "conversation-edit-error")),
                )
            })
            .child(match file.stats {
                Some((added, removed)) => h_flex()
                    .gap_1()
                    .text_xs()
                    .child(
                        div()
                            .text_color(cx.theme().success)
                            .child(format!("+{added}")),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().danger)
                            .child(format!("−{removed}")),
                    )
                    .into_any_element(),
                None => div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("—")
                    .into_any_element(),
            });
        let list = cx.weak_entity();
        Some(
            Button::new(format!("changes-file-{}-{path}", self.tab))
                .ghost()
                .small()
                .w_full()
                .mb_1()
                .tab_stop(false)
                .accessibility_label(file.label.clone())
                .tooltip(file.label.clone())
                .child(row)
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
        let Some(path) = self.selected.clone() else {
            return;
        };
        let owner = self.owner.clone();
        let list = cx.entity_id();
        // Selecting a file synchronizes this list; release its delegate lock first.
        cx.spawn_in(window, async move |_, cx| {
            let _ = owner.update_in(cx, |home, window, cx| {
                let current = home
                    .active_git()
                    .is_some_and(|t| t.list.entity_id() == list)
                    || home
                        .active_changes()
                        .is_some_and(|t| t.list.entity_id() == list && t.in_scope);
                if current {
                    home.select_reader_file(path, window, cx);
                }
            });
        })
        .detach();
    }
}
