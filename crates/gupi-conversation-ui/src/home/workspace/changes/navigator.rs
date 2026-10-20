use super::*;
use gpui_kit::component::IndexPath;
use gpui_kit::component::list::ListDelegate;

pub(super) struct Delegate {
    owner: WeakEntity<HomeView>,
    tab: String,
    pub files: Rc<Vec<EditedFile>>,
    selected: Option<String>,
}

impl Delegate {
    pub fn new(owner: WeakEntity<HomeView>, tab: String, files: Rc<Vec<EditedFile>>) -> Self {
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
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix
            .and_then(|ix| self.files.get(ix.row))
            .map(|file| file.path().to_owned());
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
        let path = file.path().to_owned();
        let name = std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let directory = path.strip_suffix(&name).unwrap_or("").to_owned();
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
            .when(file.written(), |row| {
                row.child(div().text_xs().child(t(cx, "conversation-edit-write")))
            })
            .when(file.failed(), |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "conversation-edit-error")),
                )
            })
            .child(messages::changes::stats(file.stats(), cx));
        let list = cx.weak_entity();
        Some(
            Button::new(format!("changes-file-{}-{path}", self.tab))
                .ghost()
                .small()
                .w_full()
                .mb_1()
                .tab_stop(false)
                .accessibility_label(path.clone())
                .tooltip(path)
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
                if home
                    .active_changes()
                    .is_some_and(|active| active.list.entity_id() == list && active.in_scope)
                {
                    home.select_change_file(path, window, cx);
                }
            });
        })
        .detach();
    }
}
