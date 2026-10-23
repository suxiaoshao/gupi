//! Session-owned index of the displayed user-message intervals.
use super::*;
use fluent_bundle::FluentArgs;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::base::{
    Button as MarkerButton, Popup,
    motion::{Transition, transition},
};
use gpui_kit::component::{ThemeStyled, scroll::ScrollableMask};
use gupi_settings::i18n::t_with_args;
use std::{cell::RefCell, collections::BTreeSet, time::Duration};

const ROW_REM: f32 = 0.625;
const WIDTH_REM: f32 = 2.25;
const GUTTER_REM: f32 = 3.;
const OPEN_DELAY: Duration = Duration::from_millis(250);

actions!(
    conversation_navigator,
    [PreviousUser, NextUser, DismissPreview]
);

pub(in crate::home) fn init(cx: &mut App) {
    for context in ["GupiMessages", "GupiMessageNavigator"] {
        cx.bind_keys([
            KeyBinding::new("alt-up", PreviousUser, Some(context)),
            KeyBinding::new("alt-down", NextUser, Some(context)),
        ]);
    }
    cx.bind_keys([KeyBinding::new(
        "escape",
        DismissPreview,
        Some("GupiMessageNavigator"),
    )]);
}

#[derive(Clone, PartialEq)]
struct Turn {
    id: String,
    row: usize,
    end: usize,
    question: String,
    answer: String,
}

fn project(rows: &[Rc<ChatRow>]) -> Vec<Turn> {
    let mut turns: Vec<Turn> = vec![];
    for (index, row) in rows.iter().enumerate() {
        match &row.kind {
            RowKind::User(message) => {
                if let Some(previous) = turns.last_mut() {
                    previous.end = index;
                }
                let text = message.text();
                let question = if let Some(skill) = resources::skill(&text) {
                    if skill.arguments.trim().is_empty() {
                        skill.name.to_owned()
                    } else {
                        skill.arguments.to_owned()
                    }
                } else {
                    text
                };
                let question = if question.trim().is_empty() {
                    message.value["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|part| part["mimeType"].as_str())
                        .collect::<Vec<_>>()
                        .join(" · ")
                } else {
                    gupi_conversation::history::plain_summary(&question)
                };
                turns.push(Turn {
                    id: row.id.clone(),
                    row: index,
                    end: rows.len(),
                    question,
                    answer: String::new(),
                });
            }
            RowKind::Run {
                messages, active, ..
            } => {
                if let Some(turn) = turns.last_mut() {
                    let content = RunContent::project(messages, &[], *active);
                    if !content.answer_text.trim().is_empty() {
                        turn.answer =
                            gupi_conversation::history::plain_summary(&content.answer_text);
                    }
                }
            }
            _ => {}
        }
    }
    turns
}

/// This buffer lives for one layout only; it never stores whole-transcript heights.
#[derive(Default)]
pub(in crate::home) struct Frame {
    pub rows: Vec<(usize, Bounds<Pixels>)>,
    pub serial: u64,
    pub host: Option<Bounds<Pixels>>,
}
pub(in crate::home) struct ViewportFrame {
    pub revision: u64,
    pub viewport: Bounds<Pixels>,
    pub host: Bounds<Pixels>,
    pub painted: Vec<(usize, Bounds<Pixels>)>,
    pub column: Pixels,
    pub top: ListOffset,
}
struct MarkerFocus {
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

pub(in crate::home) struct Navigator {
    owner: WeakEntity<HomeView>,
    key: String,
    scroller: Entity<MessageScrollerState>,
    rows: Rc<Vec<Rc<ChatRow>>>,
    path: Option<String>,
    revision: u64,
    pub frame: Rc<RefCell<Frame>>,
    turns: Vec<Turn>,
    visible: BTreeSet<usize>,
    enabled: bool,
    viewport: Bounds<Pixels>,
    host: Bounds<Pixels>,
    gutter: Pixels,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    markers: HashMap<String, MarkerFocus>,
    painted_markers: Rc<RefCell<HashSet<String>>>,
    target: Option<String>,
    preview: Option<String>,
    pending: Option<Task<()>>,
    dragging: bool,
    pointer_inside: bool,
    focus_preview: bool,
    pointer_focus: Option<String>,
    last_jump: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl Navigator {
    pub fn new(
        owner: WeakEntity<HomeView>,
        key: String,
        scroller: Entity<MessageScrollerState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.on_focus_out(&focus, window, |this, _, _, cx| this.dismiss(cx)),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.dismiss(cx);
                }
            }),
        ];
        Self {
            owner,
            key,
            scroller,
            rows: Rc::new(vec![]),
            path: None,
            revision: 0,
            frame: Rc::new(RefCell::new(Frame::default())),
            turns: vec![],
            visible: BTreeSet::new(),
            enabled: false,
            viewport: Bounds::default(),
            host: Bounds::default(),
            gutter: px(0.),
            scroll: UniformListScrollHandle::new(),
            focus,
            markers: HashMap::new(),
            painted_markers: Rc::default(),
            target: None,
            preview: None,
            pending: None,
            dragging: false,
            pointer_inside: false,
            focus_preview: false,
            pointer_focus: None,
            last_jump: None,
            _subscriptions: subscriptions,
        }
    }
    pub fn sync(
        &mut self,
        rows: Rc<Vec<Rc<ChatRow>>>,
        path: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if Rc::ptr_eq(&self.rows, &rows) && self.path == path {
            return;
        }
        if self.path != path {
            self.pointer_focus = None;
            self.dismiss(cx);
        }
        self.rows = rows;
        self.path = path;
        self.revision += 1;
        self.turns = project(&self.rows);
        if self
            .pointer_focus
            .as_ref()
            .is_some_and(|id| !self.turns.iter().any(|turn| &turn.id == id))
        {
            self.pointer_focus = None;
        }
        self.visible.clear();
        if self
            .target
            .as_ref()
            .is_some_and(|id| !self.turns.iter().any(|turn| &turn.id == id))
        {
            self.dismiss(cx);
        }
        cx.notify();
    }
    pub fn update_row(&mut self, rows: Rc<Vec<Rc<ChatRow>>>, index: usize, cx: &mut Context<Self>) {
        self.rows = rows;
        self.revision += 1;
        let Some(turn) = self
            .turns
            .partition_point(|turn| turn.row <= index)
            .checked_sub(1)
        else {
            return;
        };
        let range = self.turns[turn].row..self.turns[turn].end;
        if let Some(updated) = project(&self.rows[range]).into_iter().next() {
            let target = &mut self.turns[turn];
            if target.question != updated.question || target.answer != updated.answer {
                target.question = updated.question;
                target.answer = updated.answer;
                cx.notify();
            }
        }
    }
    pub fn set_enabled(&mut self, enabled: bool, _window: &mut Window, cx: &mut Context<Self>) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if !enabled {
            self.pointer_inside = false;
            self.pointer_focus = None;
            self.markers.clear();
            self.dismiss(cx);
        }
        cx.notify();
    }
    fn allowed(&self, window: &mut Window, cx: &mut App) -> bool {
        self.enabled
            && window.is_window_active()
            && !window.has_active_dialog(cx)
            && self
                .owner
                .read_with(cx, |home, cx| {
                    home.shown_key.as_deref() == Some(&self.key)
                        && home.notification_visible
                        && !home.has_image_preview(cx)
                        && !home.pane_layout.history_overlay
                })
                .unwrap_or(false)
    }
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        let changed = self.target.is_some() || self.preview.is_some() || self.dragging;
        self.pending = None;
        self.focus_preview = false;
        self.target = None;
        self.preview = None;
        self.dragging = false;
        self.last_jump = None;
        if changed {
            cx.notify();
        }
    }
    fn point_to(
        &mut self,
        id: String,
        immediate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.allowed(window, cx) {
            self.dismiss(cx);
            return;
        }
        if self.target.as_ref() == Some(&id)
            && (self.preview.is_some() || (!immediate && self.pending.is_some()))
        {
            return;
        }
        self.pending = None;
        self.target = Some(id.clone());
        if immediate || self.preview.is_some() {
            self.preview = Some(id);
        } else {
            self.pending = Some(cx.spawn_in(window, async move |view, cx| {
                cx.background_executor().timer(OPEN_DELAY).await;
                let _ = view.update_in(cx, |view, window, cx| {
                    view.pending = None;
                    if view.allowed(window, cx) && view.target.as_ref() == Some(&id) {
                        view.preview = Some(id);
                        cx.notify();
                    } else {
                        view.dismiss(cx);
                    }
                });
            }));
        }
        cx.notify();
    }
    fn jump(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.allowed(window, cx) {
            self.dismiss(cx);
            return;
        }
        let Some(turn) = self.turns.iter().find(|turn| turn.id == id) else {
            return;
        };
        if self.dragging && self.last_jump.as_deref() == Some(id) {
            return;
        }
        let row = turn.row;
        self.scroller.update(cx, |state, cx| {
            state.scroll_to_item(row, cx);
        });
        self.last_jump = Some(id.into());
    }
    fn sweep(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.focus_preview = false;
        let state = self.scroll.0.borrow();
        let bounds = state.base_handle.bounds();
        let offset = state.base_handle.offset().y;
        drop(state);
        if bounds.size.height <= px(0.) {
            return;
        }
        let y = position.y.clamp(bounds.top(), bounds.bottom() - px(0.001));
        let index = ((y - bounds.top() - offset) / (window.rem_size() * ROW_REM)).floor() as usize;
        if let Some(id) = self.turns.get(index).map(|turn| turn.id.clone()) {
            self.point_to(id.clone(), true, window, cx);
            self.jump(&id, window, cx);
        }
    }
    pub fn step(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.allowed(window, cx) {
            return;
        }
        let top = self.scroller.read(cx).logical_scroll_top();
        let Some(current) = self.turns.iter().rposition(|turn| turn.row <= top.item_ix) else {
            if let Some(first) = self.turns.first().map(|turn| turn.id.clone()) {
                self.jump(&first, window, cx);
            }
            return;
        };
        if next && current + 1 == self.turns.len() {
            return;
        }
        let index = if next {
            current + 1
        } else if top.item_ix > self.turns[current].row || top.offset_in_item > px(0.) {
            current
        } else {
            current.saturating_sub(1)
        };
        let id = self.turns[index].id.clone();
        self.jump(&id, window, cx);
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn measure(&mut self, frame: ViewportFrame, window: &mut Window, cx: &mut Context<Self>) {
        let ViewportFrame {
            revision,
            viewport,
            host,
            painted,
            column,
            top,
        } = frame;
        if revision != self.revision {
            return;
        }
        if !self.allowed(window, cx) {
            self.dismiss(cx);
            return;
        }
        let gutter = (column - host.left()).max(px(0.));
        let geometry_changed =
            self.viewport != viewport || self.host != host || self.gutter != gutter;
        self.viewport = viewport;
        self.host = host;
        self.gutter = gutter;
        let mut visible = BTreeSet::new();
        for (row, bounds) in painted {
            if bounds.bottom() <= viewport.top() || bounds.top() >= viewport.bottom() {
                continue;
            }
            if let Some(turn) = self
                .turns
                .iter()
                .rposition(|turn| turn.row <= row && row < turn.end)
            {
                visible.insert(turn);
            }
        }
        if top.item_ix < self.rows.len()
            && let Some(turn) = self.turns.iter().rposition(|turn| turn.row <= top.item_ix)
        {
            visible.insert(turn);
        }
        if self.visible != visible {
            self.visible = visible;
            if self.target.is_none()
                && !self.dragging
                && let Some(&first) = self.visible.first()
            {
                self.scroll.scroll_to_item(first, ScrollStrategy::Nearest);
            }
            cx.notify();
        }
        if geometry_changed {
            cx.notify();
        }
        let painted = self.painted_markers.borrow();
        self.markers.retain(|id, _| painted.contains(id));
        let target_gone = self.target.as_ref().is_some_and(|id| !painted.contains(id));
        drop(painted);
        if target_gone {
            self.dismiss(cx);
        }
        if !self.shown(window) {
            if self.focus.contains_focused(window, cx) {
                let owner = self.owner.clone();
                let key = self.key.clone();
                let old_focus = window.focused(cx);
                window.defer(cx, move |window, cx| {
                    if window.has_active_dialog(cx)
                        || window
                            .focused(cx)
                            .is_some_and(|focus| Some(focus) != old_focus)
                    {
                        return;
                    }
                    let target = owner
                        .read_with(cx, |home, cx| {
                            (home.shown_key.as_deref() == Some(&key)
                                && home.notification_visible
                                && (home.state.read(cx).is_temporary()
                                    || home.pane_layout.conversation > 0.)
                                && !home.pane_layout.history_overlay
                                && !home.has_image_preview(cx))
                            .then(|| home.messages_focus.clone())
                        })
                        .ok()
                        .flatten();
                    if let Some(target) = target {
                        target.focus(window, cx);
                    }
                });
            }
            self.markers.clear();
            self.dismiss(cx);
        }
    }
    fn shown(&self, window: &Window) -> bool {
        self.enabled && self.turns.len() >= 4 && self.gutter >= window.rem_size() * GUTTER_REM
    }

    fn marker(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let turn = self.turns[index].clone();
        let id = turn.id.clone();
        let focus = if let Some(marker) = self.markers.get(&id) {
            marker.focus.clone()
        } else {
            let focus = cx.focus_handle().tab_stop(true);
            let target = id.clone();
            let subscriptions = vec![cx.on_focus(&focus, window, move |this, window, cx| {
                if !this
                    .markers
                    .get(&target)
                    .is_some_and(|marker| marker.focus.is_focused(window))
                {
                    if this.pointer_focus.as_ref() == Some(&target) {
                        this.pointer_focus = None;
                    }
                    return;
                }
                // Pointer-down already opened the preview; its delayed focus event
                // must not reopen it after an outside release or Escape.
                if this.pointer_focus.as_ref() == Some(&target) {
                    this.pointer_focus = None;
                    return;
                }
                this.focus_preview = true;
                this.point_to(target.clone(), true, window, cx)
            })];
            self.markers.insert(
                id.clone(),
                MarkerFocus {
                    focus: focus.clone(),
                    _subscriptions: subscriptions,
                },
            );
            focus
        };
        let target_index = self
            .target
            .as_ref()
            .and_then(|id| self.turns.iter().position(|turn| &turn.id == id));
        let distance = target_index.map(|target| target.abs_diff(index));
        let width = match distance {
            Some(0) => 1.625,
            Some(1) => 1.25,
            Some(2) => 0.875,
            Some(3) => 0.625,
            _ => 0.375,
        };
        let width = transition(
            (
                ("conversation-marker", cx.entity_id()),
                SharedString::from(id.clone()),
            ),
            width,
            Transition::new(if self.dragging {
                Duration::ZERO
            } else {
                cx.theme().motion.duration_fast
            })
            .easing(cx.theme().motion.easing_move.clone()),
            window,
            cx,
        );
        let current = self.visible.contains(&index);
        let emphasized = distance == Some(0);
        let color = if emphasized {
            cx.theme().foreground
        } else if current && target_index.is_none() {
            cx.theme().foreground.opacity(0.6)
        } else {
            cx.theme().muted_foreground.opacity(0.4)
        };
        let question = if turn.question.is_empty() {
            t(cx, "conversation-navigation-no-text")
        } else {
            turn.question.clone()
        };
        let mut args = FluentArgs::new();
        args.set("number", (index + 1) as i64);
        args.set("question", question.clone());
        let label = t_with_args(
            cx,
            if current {
                "conversation-navigation-current"
            } else {
                "conversation-navigation-jump"
            },
            &args,
        );
        let button_id = format!("conversation-turn-{}-{}", self.key, turn.id);
        let click_id = id.clone();
        let hover_id = id.clone();
        let down_id = id.clone();
        let down_focus = focus.clone();
        let ring = cx.theme().ring;
        let button = MarkerButton::new(button_id)
            .track_focus(&focus)
            .accessibility_label(label)
            .aria_selected(current)
            .w(rems(WIDTH_REM))
            .h(rems(ROW_REM))
            .px_0()
            .justify_start()
            .rounded_sm()
            .focus_visible(move |style| style.border_1().border_color(ring))
            .child(div().h(rems(0.125)).w(rems(width)).bg(color).rounded_full())
            .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, window, cx| {
                if !this.dragging {
                    this.focus_preview = false;
                    this.point_to(hover_id.clone(), false, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    if this.allowed(window, cx) {
                        if !down_focus.is_focused(window) {
                            this.pointer_focus = Some(down_id.clone());
                        }
                        down_focus.focus(window, cx);
                        this.focus_preview = false;
                        this.dragging = true;
                        this.point_to(down_id.clone(), true, window, cx);
                        this.jump(&down_id, window, cx);
                    }
                }),
            )
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if !matches!(event, ClickEvent::Mouse(_)) {
                    this.focus_preview = matches!(event, ClickEvent::Keyboard(_));
                    this.point_to(click_id.clone(), true, window, cx);
                    this.jump(&click_id, window, cx);
                }
            }));
        let mut popup = Popup::new(format!("turn-preview-{}-{}", self.key, id), button)
            .anchor(Anchor::LeftCenter);
        if self.preview.as_ref() == Some(&id) {
            popup = popup.content(
                div()
                    .id("conversation-navigation-preview")
                    .test_support()
                    .popover_style(cx)
                    .p_2()
                    .w(rems(20.))
                    .max_w((window.viewport_size().width - window.rem_size()).max(px(0.)))
                    .max_h(self.viewport.size.height)
                    .overflow_hidden()
                    .text_sm()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(question),
                    )
                    .when(!turn.answer.is_empty(), |card| {
                        card.child(
                            div()
                                .mt_1()
                                .text_color(cx.theme().muted_foreground)
                                .line_clamp(3)
                                .child(turn.answer),
                        )
                    }),
            );
        }
        let painted = self.painted_markers.clone();
        div()
            .h(rems(ROW_REM))
            .w(rems(WIDTH_REM))
            .child(popup)
            .on_children_prepainted(move |_, _, _| {
                painted.borrow_mut().insert(id.clone());
            })
            .into_any_element()
    }
}
impl Render for Navigator {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.allowed(window, cx) || !self.shown(window) {
            return div().into_any_element();
        }
        let owner = cx.weak_entity();
        let events = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moves = owner.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if !phase.capture() {
                        return;
                    }
                    let _ = moves.update(cx, |this, cx| {
                        if !this.dragging {
                            return;
                        }
                        if event.pressed_button != Some(MouseButton::Left) {
                            this.dismiss(cx);
                            return;
                        }
                        this.sweep(event.position, window, cx);
                    });
                });
                let releases = owner.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase.capture() && event.button == MouseButton::Left {
                        let _ = releases.update(cx, |this, cx| {
                            if this.dragging {
                                this.sweep(event.position, window, cx);
                                this.dragging = false;
                                this.last_jump = None;
                                let inside = this
                                    .scroll
                                    .0
                                    .borrow()
                                    .base_handle
                                    .bounds()
                                    .contains(&event.position);
                                this.pointer_inside = inside;
                                if !inside {
                                    this.dismiss(cx);
                                }
                                cx.notify();
                            }
                        });
                    }
                });
            },
        )
        .absolute()
        .size_0();
        let height = (window.rem_size() * (self.turns.len() as f32 * ROW_REM))
            .min(self.viewport.size.height * 0.7)
            .min(window.rem_size() * 40.);
        div()
            .id("conversation-navigator")
            .test_support()
            .key_context("GupiMessageNavigator")
            .track_focus(&self.focus)
            .absolute()
            .left(rems(0.75))
            .top(self.viewport.top() - self.host.top() + (self.viewport.size.height - height) / 2.)
            .w(rems(WIDTH_REM))
            .h(height)
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                let leaving = this.pointer_inside && !*hovered;
                this.pointer_inside = *hovered;
                let focused_target = this.focus_preview
                    && this
                        .target
                        .as_ref()
                        .and_then(|id| this.markers.get(id))
                        .is_some_and(|marker| marker.focus.is_focused(_window));
                if leaving && !this.dragging && !focused_target {
                    this.dismiss(cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &PreviousUser, window, cx| this.step(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &NextUser, window, cx| this.step(true, window, cx)))
            .on_action(cx.listener(|this, _: &DismissPreview, window, cx| {
                if window.has_active_dialog(cx) {
                    cx.propagate();
                } else {
                    this.dismiss(cx);
                }
            }))
            .child(
                canvas(
                    {
                        let painted = self.painted_markers.clone();
                        move |_, _, _| painted.borrow_mut().clear()
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_0(),
            )
            .child(
                uniform_list(
                    "conversation-turns",
                    self.turns.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, window, cx| {
                        range
                            .map(|index| this.marker(index, window, cx))
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll)
                .size_full(),
            )
            .child(super::viewport::edge_fades(
                self.scroll.0.borrow().base_handle.clone(),
                cx.theme().background,
            ))
            .child(ScrollableMask::new(
                Axis::Vertical,
                &self.scroll.0.borrow().base_handle,
            ))
            .child(events)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
