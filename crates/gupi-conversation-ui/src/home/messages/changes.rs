//! Inline recorded-edit cards. The session owns disclosure, focus and Diff lifetimes.
use super::diff::DiffBlock;
use super::edited_files::{ChangeBody, EditedFiles, LineStats};
use super::tools::ToolIndex;
use super::*;
use fluent_bundle::FluentArgs;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, StyledExt as _};
use gupi_settings::i18n::t_with_args;
use std::cell::{Cell, RefCell};

const PAGE: usize = 3;

#[derive(Clone)]
pub(super) enum Disclosure {
    File(String),
    Files,
    Next(String),
    Previous(String),
}
struct Trigger {
    action: Disclosure,
    focus: FocusHandle,
    label: String,
    expanded: bool,
}
impl Trigger {
    fn new(action: Disclosure, focus: &FocusHandle, label: String, expanded: bool) -> Self {
        Self {
            action,
            focus: focus.clone(),
            label,
            expanded,
        }
    }
}

struct FileState {
    open: bool,
    page: usize,
    position: usize,
    combined: Option<DiffBlock>,
    combined_mode: bool,
    ids: Vec<String>,
    focus: FocusHandle,
    body_focus: FocusHandle,
    next_focus: FocusHandle,
    previous_focus: FocusHandle,
}
impl FileState {
    fn contains_focus(
        &self,
        blocks: &HashMap<String, DiffBlock>,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.focus.is_focused(window)
            || self.body_focus.contains_focused(window, cx)
            || self.next_focus.is_focused(window)
            || self.previous_focus.is_focused(window)
            || self
                .combined
                .as_ref()
                .and_then(|block| block.focus_handle(cx))
                .is_some_and(|focus| focus.is_focused(window))
            || self
                .ids
                .iter()
                .filter_map(|id| blocks.get(id))
                .filter_map(|block| block.focus_handle(cx))
                .any(|focus| focus.is_focused(window))
    }
}
struct RunState {
    all: bool,
    ids: Vec<String>,
    focus: FocusHandle,
    geometry: Rc<Geometry>,
}
#[derive(Default)]
struct Geometry {
    current: RefCell<HashMap<String, Bounds<Pixels>>>,
    previous: RefCell<HashMap<String, Bounds<Pixels>>>,
    reveal: RefCell<Option<String>>,
    anchor: RefCell<Option<(String, Pixels)>>,
    focused: RefCell<Option<String>>,
}
#[derive(Default)]
pub(in crate::home) struct ChangesState {
    files: HashMap<(String, String), FileState>,
    runs: HashMap<String, RunState>,
    blocks: HashMap<String, DiffBlock>,
    scope: Option<(Option<String>, Option<String>)>,
    pub(super) viewport: Rc<Cell<Option<Bounds<Pixels>>>>,
}
impl ChangesState {
    pub fn sync_scope(&mut self, session: &Session, preview: Option<&str>) {
        let preview = preview.map(str::to_owned);
        if self.scope.as_ref().is_some_and(|(old_preview, leaf)| {
            old_preview != &preview
                || (preview.is_none()
                    && leaf != &session.history().leaf
                    && leaf
                        .as_deref()
                        .is_some_and(|leaf| !session.history().on_current_path(leaf)))
        }) {
            self.files.clear();
            self.runs.clear();
            self.blocks.clear();
        }
        self.scope = Some((preview, session.history().leaf.clone()));
    }
    pub fn capture_anchors(&self) {
        let Some(viewport) = self.viewport.get() else {
            return;
        };
        for run in self.runs.values() {
            if run.geometry.anchor.borrow().is_some() {
                continue;
            }
            let nodes = run.geometry.previous.borrow();
            let focused = run.geometry.focused.borrow();
            let node = focused
                .as_ref()
                .and_then(|id| nodes.get_key_value(id))
                .filter(|(_, bounds)| {
                    bounds.bottom() > viewport.top() && bounds.top() < viewport.bottom()
                })
                .or_else(|| {
                    nodes
                        .iter()
                        .filter(|(_, bounds)| {
                            bounds.bottom() > viewport.top() && bounds.top() < viewport.bottom()
                        })
                        .min_by_key(|(_, bounds)| bounds.top())
                });
            if let Some((id, bounds)) = node {
                *run.geometry.anchor.borrow_mut() = Some((id.clone(), bounds.top()));
            }
        }
    }
    /// Returns true only when a removed, focused card needs the composer.
    pub fn sync(
        &mut self,
        data: &EditedFiles,
        tools: &ToolIndex,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let mut lost_focus = None;
        let mut files = HashMap::new();
        for ((old_run, path), state) in self.files.drain() {
            let run = state
                .ids
                .iter()
                .find_map(|id| tools.owner(id))
                .unwrap_or(&old_run)
                .to_owned();
            if data.files(&run).iter().any(|file| file.path() == path) {
                files.entry((run, path)).or_insert(state);
            } else if state.contains_focus(&self.blocks, window, cx) {
                lost_focus = Some((run, state.position));
            }
        }
        self.files = files;
        let mut runs = HashMap::new();
        for (old_run, state) in self.runs.drain() {
            let run = state
                .ids
                .iter()
                .find_map(|id| tools.owner(id))
                .unwrap_or(&old_run)
                .to_owned();
            if !data.files(&run).is_empty() {
                runs.entry(run).or_insert(state);
            } else if state.focus.is_focused(window) {
                lost_focus = Some((run, 0));
            }
        }
        self.runs = runs;
        let mut keep = HashSet::new();
        for (run, files) in data.runs() {
            let run_state = self.runs.entry(run.to_owned()).or_insert_with(|| RunState {
                all: false,
                ids: vec![],
                focus: cx.focus_handle(),
                geometry: Rc::default(),
            });
            run_state.ids = files
                .iter()
                .flat_map(|file| {
                    file.operations()
                        .iter()
                        .map(|operation| operation.id().to_owned())
                })
                .collect();
            for (position, file) in files.iter().enumerate() {
                let state = self
                    .files
                    .entry((run.to_owned(), file.path().to_owned()))
                    .or_insert_with(|| FileState {
                        open: false,
                        page: 0,
                        position,
                        combined: None,
                        combined_mode: file.combined().is_some(),
                        ids: vec![],
                        focus: cx.focus_handle(),
                        body_focus: cx.focus_handle(),
                        next_focus: cx.focus_handle(),
                        previous_focus: cx.focus_handle(),
                    });
                state.position = position;
                state.ids = file
                    .operations()
                    .iter()
                    .map(|operation| operation.id().to_owned())
                    .collect();
                let combined_mode = file.combined().is_some();
                if state.combined_mode != combined_mode
                    && state.contains_focus(&self.blocks, window, cx)
                {
                    state.focus.focus(window, cx);
                    *run_state.geometry.reveal.borrow_mut() = Some(format!("file-{}", file.path()));
                }
                state.combined_mode = combined_mode;
                let visible = run_state.all || position < PAGE;
                if let Some(body) = file.combined() {
                    if let Some(block) = state.combined.as_mut() {
                        block.update(body, file.path(), cx);
                    } else if state.open && visible {
                        state.combined = Some(DiffBlock::new(body, file.path(), cx));
                    }
                }
                state.page = state
                    .page
                    .min(file.operations().len().saturating_sub(1) / PAGE * PAGE);
                for (ix, operation) in file.operations().iter().enumerate() {
                    keep.insert(operation.id().to_owned());
                    if let Some(block) = self.blocks.get_mut(operation.id()) {
                        block.update(operation.shared_body(), file.path(), cx);
                    } else if !combined_mode
                        && state.open
                        && visible
                        && (state.page..state.page + PAGE).contains(&ix)
                    {
                        self.blocks.insert(
                            operation.id().to_owned(),
                            DiffBlock::new(operation.shared_body(), file.path(), cx),
                        );
                    }
                }
            }
        }
        self.blocks.retain(|id, _| keep.contains(id));
        if let Some((run, position)) = lost_focus {
            let files = data.files(&run);
            if let Some(file) = files.get(position).or_else(|| files.last()) {
                let state = &self.files[&(run.clone(), file.path().to_owned())];
                state.focus.focus(window, cx);
                let run_state = self.runs.get_mut(&run).unwrap();
                if state.position >= PAGE {
                    run_state.all = true;
                }
                *run_state.geometry.reveal.borrow_mut() = Some(format!("file-{}", file.path()));
            } else {
                return true;
            }
        }
        false
    }
    fn toggle(
        &mut self,
        run: &str,
        action: &Disclosure,
        data: &EditedFiles,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(run_state) = self.runs.get_mut(run) else {
            return;
        };
        let target = match action {
            Disclosure::Files => {
                if run_state.all
                    && data.files(run).iter().skip(PAGE).any(|file| {
                        self.files
                            .get(&(run.to_owned(), file.path().to_owned()))
                            .is_some_and(|state| {
                                state.focus.is_focused(window)
                                    || state.body_focus.contains_focused(window, cx)
                            })
                    })
                {
                    run_state.focus.focus(window, cx);
                }
                run_state.all = !run_state.all;
                "files-more".to_owned()
            }
            Disclosure::File(path) | Disclosure::Next(path) | Disclosure::Previous(path) => {
                let Some(state) = self.files.get_mut(&(run.to_owned(), path.clone())) else {
                    return;
                };
                match action {
                    Disclosure::File(_) => {
                        if state.open && state.body_focus.contains_focused(window, cx) {
                            state.focus.focus(window, cx);
                        }
                        state.open = !state.open;
                    }
                    Disclosure::Next(_) => {
                        let count = data
                            .files(run)
                            .iter()
                            .find(|file| file.path() == path)
                            .map(|file| file.operations().len())
                            .unwrap_or(0);
                        state.page = (state.page + PAGE).min(count.saturating_sub(1) / PAGE * PAGE);
                        if state.page + PAGE >= count {
                            state.previous_focus.focus(window, cx);
                        }
                    }
                    Disclosure::Previous(_) => {
                        state.page = state.page.saturating_sub(PAGE);
                        if state.page == 0 {
                            state.next_focus.focus(window, cx);
                        }
                    }
                    _ => unreachable!(),
                }
                format!("file-{path}")
            }
        };
        run_state.geometry.anchor.borrow_mut().take();
        *run_state.geometry.reveal.borrow_mut() = Some(target);
    }
}
fn count(cx: &App, key: &str, count: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("count", count as i64);
    t_with_args(cx, key, &args)
}
fn stats(stats: Option<LineStats>, cx: &App) -> AnyElement {
    statistics(stats, "conversation-edit-stats-tooltip", cx)
}
fn statistics(stats: Option<LineStats>, tooltip_key: &str, cx: &App) -> AnyElement {
    let Some(stats) = stats else {
        return div().into_any_element();
    };
    let tooltip = t(cx, tooltip_key);
    h_flex()
        .id("line-statistics")
        .gap_1()
        .flex_none()
        .text_xs()
        .font_family("monospace")
        .child(
            div()
                .text_color(cx.theme().success)
                .child(format!("+{}", stats.added())),
        )
        .child(
            div()
                .text_color(cx.theme().danger)
                .child(format!("−{}", stats.removed())),
        )
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .into_any_element()
}
fn record(id: String, child: AnyElement, geometry: &Rc<Geometry>) -> AnyElement {
    let geometry = geometry.clone();
    let selector = format!("change-node-{id}");
    div()
        .min_w_0()
        .child(
            div()
                .id(id.clone())
                .min_w_0()
                .debug_selector(move || selector.clone())
                .child(child),
        )
        .on_children_prepainted(move |bounds, _, _| {
            if let Some(bounds) = bounds.first() {
                geometry.current.borrow_mut().insert(id.clone(), *bounds);
            }
        })
        .into_any_element()
}
impl HomeView {
    fn toggle_changes(
        &mut self,
        key: &str,
        run: &str,
        action: &Disclosure,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.views.get_mut(key) else {
            return;
        };
        view.changes
            .toggle(run, action, &view.edited_files, window, cx);
        view.changes
            .sync(&view.edited_files, &view.tools, window, cx);
        if let Some(index) = view.row_positions.borrow().get(run).copied() {
            view.scroller.update(cx, |state, cx| {
                if state.is_following_tail() {
                    // A disclosure starts reading this response. Stop the next frame
                    // snapping back to its tail before revealing the clicked row.
                    state.scroll_to_item(index, cx);
                }
                state.remeasure_items(index..index + 1, cx);
            });
        }
        cx.notify();
    }
    fn change_trigger(
        &self,
        location: (&str, &str),
        trigger: Trigger,
        button: Button,
        owner: WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let (key, run) = location;
        let Trigger {
            action,
            focus,
            label,
            expanded,
        } = trigger;
        let focus_color = cx.theme().primary;
        let selector = match &action {
            Disclosure::File(path) => format!("change-file-row-{path}"),
            Disclosure::Files => "change-more-files".into(),
            Disclosure::Next(path) => format!("change-next-{path}"),
            Disclosure::Previous(path) => format!("change-previous-{path}"),
        };
        let id = format!("{key}-{run}-{selector}");
        let retained_focus = focus.clone();
        let focus = focus.clone();
        let key = key.to_owned();
        let run = run.to_owned();
        let activate = Rc::new(move |window: &mut Window, cx: &mut App| {
            focus.focus(window, cx);
            let _ = owner.update(cx, |this, cx| {
                this.toggle_changes(&key, &run, &action, window, cx)
            });
        });
        let pointer = activate.clone();
        let keyboard = activate.clone();
        let keyboard_focus = retained_focus.clone();
        let accessible = activate.clone();
        div()
            .id(id)
            .min_w_0()
            .track_focus(&retained_focus)
            .tab_stop(true)
            .role(Role::Button)
            .debug_selector(move || selector.clone())
            .aria_label(label.clone())
            .aria_expanded(expanded)
            .border_1()
            .border_color(focus_color.opacity(0.))
            .focus_visible(|style| style.rounded_md().border_color(focus_color))
            .on_key_down(move |event, window, cx| {
                if keyboard_focus.is_focused(window)
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    cx.stop_propagation();
                    let keyboard = keyboard.clone();
                    window.defer(cx, move |window, cx| keyboard(window, cx));
                }
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                accessible(window, cx);
            })
            .child(button.tab_stop(false).accessibility_label(label).on_click(
                move |_, window, cx| {
                    cx.stop_propagation();
                    pointer(window, cx);
                },
            ))
    }

    pub(super) fn edited_files_card(
        &self,
        key: &str,
        run: &str,
        owner: WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let view = &self.views[key];
        let files = view.edited_files.files(run);
        let Some(run_state) = view.changes.runs.get(run) else {
            return div().into_any_element();
        };
        let mut focused_nodes = Vec::new();
        let mut card = v_flex()
            .id(format!("changes-{key}-{run}"))
            .min_w_0()
            .w_full()
            .gap_1()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .min_w_0()
                    .child(Icon::new(IconName::FileDiff).size_4())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .font_semibold()
                            .child(count(cx, "conversation-edited-files", files.len())),
                    )
                    .child(stats(view.edited_files.stats(run), cx)),
            );
        for file in files
            .iter()
            .take(if run_state.all { files.len() } else { PAGE })
        {
            let path = file.path();
            let state = &view.changes.files[&(run.to_owned(), path.to_owned())];
            focused_nodes.push((format!("file-{path}"), state.focus.clone()));
            let (directory, basename) = path
                .rsplit_once('/')
                .map(|(directory, name)| (format!("{directory}/"), name))
                .unwrap_or((String::new(), path));
            let mut args = FluentArgs::new();
            args.set("path", path);
            args.set(
                "state",
                t(
                    cx,
                    if state.open {
                        "files-expanded"
                    } else {
                        "files-collapsed"
                    },
                ),
            );
            let row = self
                .change_trigger(
                    (key, run),
                    Trigger::new(
                        Disclosure::File(path.to_owned()),
                        &state.focus,
                        t_with_args(cx, "conversation-edit-file-label", &args),
                        state.open,
                    ),
                    Button::new(format!("change-file-{key}-{run}-{path}"))
                        .ghost()
                        .small()
                        .w_full()
                        .min_w_0()
                        .tooltip(path.to_owned())
                        .child(
                            h_flex()
                                .w_full()
                                .min_w_0()
                                .gap_2()
                                .child(
                                    Icon::new(if state.open {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .size_3()
                                    .flex_none(),
                                )
                                .child(
                                    h_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(directory),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .font_semibold()
                                                .child(basename.to_owned()),
                                        ),
                                )
                                .when(file.written(), |row| {
                                    row.child(
                                        div()
                                            .flex_none()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(t(cx, "conversation-edit-write")),
                                    )
                                })
                                .when(file.failed(), |row| {
                                    row.child(
                                        div()
                                            .flex_none()
                                            .text_xs()
                                            .text_color(cx.theme().danger)
                                            .child(t(cx, "conversation-edit-error")),
                                    )
                                })
                                .child(stats(file.stats(), cx)),
                        ),
                    owner.clone(),
                    cx,
                )
                .into_any_element();
            card = card.child(record(format!("file-{path}"), row, &run_state.geometry));
            if state.open {
                let mut body = v_flex()
                    .min_w_0()
                    .gap_2()
                    .px_3()
                    .pb_2()
                    .track_focus(&state.body_focus);
                if file.combined().is_some() {
                    if let Some(block) = &state.combined {
                        if let Some(focus) = block.focus_handle(cx) {
                            focused_nodes.push((format!("combined-{path}"), focus));
                        }
                        if file.written() {
                            body = body.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t(cx, "conversation-written-content")),
                            );
                        }
                        body = body.child(record(
                            format!("combined-{path}"),
                            block.render_retained(
                                self.message_frame.clone(),
                                view.changes.viewport.clone(),
                            ),
                            &run_state.geometry,
                        ));
                    }
                } else {
                    body = body.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "conversation-edit-uncombined")),
                    );
                    for (ix, operation) in file
                        .operations()
                        .iter()
                        .enumerate()
                        .skip(state.page)
                        .take(PAGE)
                    {
                        let Some(block) = view.changes.blocks.get(operation.id()) else {
                            continue;
                        };
                        if let Some(focus) = block.focus_handle(cx) {
                            focused_nodes.push((operation.id().to_owned(), focus));
                        }
                        let title = if matches!(operation.body(), ChangeBody::Written { .. }) {
                            t(cx, "conversation-written-content")
                        } else {
                            let mut args = FluentArgs::new();
                            args.set("index", (ix + 1) as i64);
                            t_with_args(cx, "conversation-edit-operation", &args)
                        };
                        let operation_body = v_flex()
                            .min_w_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .min_w_0()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(div().flex_1().min_w_0().child(title))
                                    .when(operation.status() == ToolStatus::Failed, |row| {
                                        row.child(t(cx, "conversation-edit-error"))
                                    })
                                    .child(statistics(
                                        operation.stats(),
                                        "conversation-edit-operation-stats-tooltip",
                                        cx,
                                    )),
                            )
                            .child(block.render_retained(
                                self.message_frame.clone(),
                                view.changes.viewport.clone(),
                            ))
                            .into_any_element();
                        body = body.child(record(
                            operation.id().to_owned(),
                            operation_body,
                            &run_state.geometry,
                        ));
                    }
                    let mut pages = h_flex().gap_2().min_w_0();
                    if state.page > 0 {
                        pages = pages.child(
                            self.change_trigger(
                                (key, run),
                                Trigger::new(
                                    Disclosure::Previous(path.to_owned()),
                                    &state.previous_focus,
                                    t(cx, "conversation-edit-previous-operations"),
                                    false,
                                ),
                                Button::new(format!("change-prev-{key}-{run}-{path}"))
                                    .ghost()
                                    .small()
                                    .label(t(cx, "conversation-edit-previous-operations")),
                                owner.clone(),
                                cx,
                            ),
                        );
                    }
                    let remaining = file.operations().len().saturating_sub(state.page + PAGE);
                    if remaining > 0 {
                        let label =
                            count(cx, "conversation-edit-more-operations", remaining.min(PAGE));
                        pages = pages.child(
                            self.change_trigger(
                                (key, run),
                                Trigger::new(
                                    Disclosure::Next(path.to_owned()),
                                    &state.next_focus,
                                    label.clone(),
                                    false,
                                ),
                                Button::new(format!("change-next-{key}-{run}-{path}"))
                                    .ghost()
                                    .small()
                                    .label(label),
                                owner.clone(),
                                cx,
                            ),
                        );
                    }
                    body = body.child(pages);
                }
                card = card.child(body);
            }
        }
        if files.len() > PAGE {
            let label = if run_state.all {
                t(cx, "conversation-edit-less-files")
            } else {
                count(cx, "conversation-edit-more-files", files.len() - PAGE)
            };
            let more = self
                .change_trigger(
                    (key, run),
                    Trigger::new(
                        Disclosure::Files,
                        &run_state.focus,
                        label.clone(),
                        run_state.all,
                    ),
                    Button::new(format!("change-more-{key}-{run}"))
                        .ghost()
                        .small()
                        .w_full()
                        .label(label),
                    owner,
                    cx,
                )
                .into_any_element();
            card = card.child(record("files-more".into(), more, &run_state.geometry));
        }
        let geometry = run_state.geometry.clone();
        let viewport = view.changes.viewport.clone();
        div()
            .min_w_0()
            .child(card)
            .on_children_prepainted(move |_, window, cx| {
                *geometry.focused.borrow_mut() = focused_nodes
                    .iter()
                    .find(|(_, focus)| focus.contains_focused(window, cx))
                    .map(|(id, _)| id.clone());
                let current = geometry.current.borrow();
                if let Some(target) = geometry.reveal.borrow_mut().take() {
                    if let Some(bounds) = current.get(&target) {
                        window.request_autoscroll(*bounds);
                    }
                } else if let Some((id, previous_y)) = geometry.anchor.borrow_mut().take()
                    && let Some(bounds) = current.get(&id)
                    && let Some(viewport) = viewport.get()
                {
                    let delta = bounds.top() - previous_y;
                    if delta.abs() > px(0.5) {
                        // The list's public autoscroll protocol moves by this delta.
                        // A point past the appropriate edge preserves the exact reading anchor.
                        let y = if delta > px(0.) {
                            viewport.bottom() + delta
                        } else {
                            viewport.top() + delta
                        };
                        window.request_autoscroll(Bounds::new(
                            point(viewport.left(), y),
                            size(px(0.), px(0.)),
                        ));
                    }
                }
                *geometry.previous.borrow_mut() = current.clone();
                drop(current);
                geometry.current.borrow_mut().clear();
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
