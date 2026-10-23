//! Pointer-completed selections. Geometry belongs to the painted source region,
//! never to private text layout or a second selection model.
use super::*;
use gpui_kit::component::diff::DiffLineRange;
use gpui_kit::component::text::TextViewState;
use std::cell::Cell;
use std::ops::Range;

// Physical pointer slop, matching GPUI drag initiation rather than UI spacing.
const POINTER_DRAG_THRESHOLD: f64 = 2.;

#[derive(Clone)]
pub(in crate::home) enum Target {
    Text {
        state: WeakEntity<TextViewState>,
        role: String,
    },
    Source(WeakEntity<source::Preview>),
    Diff {
        state: WeakEntity<DiffState>,
        path: PathBuf,
        kind: FileKind,
    },
}
#[derive(PartialEq)]
struct Metadata {
    range: Option<Range<usize>>,
    lines: Option<DiffLineRange>,
}
impl Target {
    fn id(&self) -> EntityId {
        match self {
            Self::Text { state, .. } => state.entity_id(),
            Self::Source(state) => state.entity_id(),
            Self::Diff { state, .. } => state.entity_id(),
        }
    }
    fn selection(&self, cx: &App) -> Result<Selection, Unavailable> {
        match self {
            Self::Text { state, role } => state
                .read_with(cx, |state, _| Selection {
                    origin: Origin::Message { role: role.clone() },
                    text: state.selected_text(),
                })
                .map_err(|_| Unavailable::Selection),
            Self::Source(state) => state
                .read_with(cx, |state, cx| state.quote_selection(cx))
                .unwrap_or(Err(Unavailable::Selection)),
            Self::Diff { state, path, kind } => state
                .upgrade()
                .ok_or(Unavailable::Selection)
                .and_then(|state| diff_selection(&state, path.clone(), *kind, cx)),
        }
    }
    fn metadata(&self, cx: &App) -> Metadata {
        match self {
            Self::Text { state, .. } => Metadata {
                range: state
                    .read_with(cx, |state, _| state.selected_source_range())
                    .ok()
                    .flatten(),
                lines: None,
            },
            Self::Source(state) => Metadata {
                range: state
                    .read_with(cx, |state, cx| state.quote_range(cx))
                    .ok()
                    .flatten(),
                lines: None,
            },
            Self::Diff { state, .. } => Metadata {
                range: None,
                lines: state
                    .read_with(cx, |state, _| state.selected_lines())
                    .ok()
                    .flatten(),
            },
        }
    }
    fn text(&self, cx: &App) -> String {
        match self {
            Self::Text { state, .. } => state
                .read_with(cx, |state, _| state.selected_text())
                .unwrap_or_default(),
            Self::Source(state) => state
                .read_with(cx, |state, cx| {
                    state
                        .quote_selection(cx)
                        .map(|s| s.text)
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            Self::Diff { state, .. } => state
                .read_with(cx, |state, cx| state.selected_text(cx))
                .unwrap_or_default(),
        }
    }
    fn clear(&self, cx: &mut App) {
        match self {
            Self::Text { state, .. } => {
                let _ = state.update(cx, |state, cx| state.clear_selection(cx));
            }
            Self::Source(state) => {
                let _ = state.update(cx, |state, cx| state.clear_quote_selection(cx));
            }
            Self::Diff { state, .. } => {
                let _ = state.update(cx, |state, cx| state.set_selected_lines(None, cx));
            }
        }
    }
    fn focus(&self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Text { state, .. } => {
                let _ = state.update(cx, |state, cx| state.focus_handle().focus(window, cx));
            }
            Self::Source(state) => {
                let _ = state.update(cx, |state, cx| state.focus(window, cx));
            }
            Self::Diff { state, .. } => {
                let _ = state.update(cx, |state, cx| state.focus_handle(cx).focus(window, cx));
            }
        }
    }
}
struct Lease {
    target: Target,
    key: String,
    owner: WeakEntity<HomeView>,
    window: AnyWindowHandle,
    hit: Cell<Bounds<Pixels>>,
    area: Cell<Bounds<Pixels>>,
    _subscription: Option<Subscription>,
}
impl Lease {
    fn changed(&self, cx: &mut Context<Self>) {
        let owner = self.owner.clone();
        let handle = self.window;
        cx.defer(move |cx| {
            let _ = cx.update_window(handle, |_, window, cx| {
                let _ = owner.update(cx, |home, cx| home.check_quote_tools(window, cx));
            });
        });
    }
}
struct Press {
    region: WeakEntity<Lease>,
    point: Point<Pixels>,
    moved: bool,
    released: Option<Point<Pixels>>,
}
struct Active {
    region: WeakEntity<Lease>,
    point: Point<Pixels>,
    capture: Capture,
    text: String,
    metadata: Metadata,
}
pub(in crate::home) struct State {
    press: Option<Press>,
    active: Option<Active>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
    pointer_held: bool,
}
impl State {
    pub(in crate::home) fn new(cx: &mut App) -> Self {
        Self {
            press: None,
            active: None,
            bounds: Rc::default(),
            focus: cx.focus_handle(),
            pointer_held: false,
        }
    }
    pub(in crate::home) fn dismiss(&mut self) -> bool {
        self.press = None;
        self.pointer_held = false;
        self.bounds.set(None);
        self.active.take().is_some()
    }
}
#[derive(IntoElement)]
struct Region {
    target: Target,
    owner: WeakEntity<HomeView>,
    key: String,
}
pub(in crate::home) fn region(
    target: Target,
    owner: WeakEntity<HomeView>,
    key: String,
) -> impl IntoElement {
    Region { target, owner, key }
}
impl RenderOnce for Region {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window.window_handle();
        let lease = window.use_keyed_state(
            SharedString::from(format!(
                "quote-region-{}-{}",
                self.key,
                self.target.id().as_u64()
            )),
            cx,
            |_, cx| {
                let subscription = match &self.target {
                    Target::Text { state, .. } => state.upgrade().map(|state| {
                        cx.observe(&state, |this: &mut Lease, _, cx| this.changed(cx))
                    }),
                    Target::Source(state) => state.upgrade().map(|state| {
                        cx.observe(&state, |this: &mut Lease, _, cx| this.changed(cx))
                    }),
                    Target::Diff { state, .. } => state.upgrade().map(|state| {
                        cx.observe(&state, |this: &mut Lease, _, cx| this.changed(cx))
                    }),
                };
                cx.on_release(|lease: &mut Lease, cx| {
                    let owner = lease.owner.clone();
                    let handle = lease.window;
                    cx.defer(move |cx| {
                        let _ = cx.update_window(handle, |_, window, cx| {
                            let _ = owner.update(cx, |home, cx| home.check_quote_tools(window, cx));
                        });
                    });
                })
                .detach();
                Lease {
                    target: self.target,
                    owner: self.owner,
                    window: handle,
                    key: self.key,
                    hit: Cell::default(),
                    area: Cell::default(),
                    _subscription: subscription,
                }
            },
        );
        canvas(
            move |bounds, window, cx| {
                let mask = window.content_mask().bounds;
                let state = lease.read(cx);
                state.hit.set(bounds.intersect(&mask));
                state
                    .area
                    .set(if matches!(state.target, Target::Text { .. }) {
                        mask
                    } else {
                        bounds.intersect(&mask)
                    });
                lease.clone()
            },
            move |_, lease, window, cx| {
                let owner = lease.read(cx).owner.clone();
                let released = owner
                    .read_with(cx, |home, _| {
                        home.quote_tools
                            .press
                            .as_ref()
                            .filter(|press| press.region.entity_id() == lease.entity_id())
                            .and_then(|press| press.released)
                    })
                    .ok()
                    .flatten();
                if let Some(point) = released {
                    // TextView materializes its public selection during paint.
                    // This region is painted after its source, so sample only
                    // after this frame rather than in the MouseUp transaction.
                    let owner = owner.clone();
                    let id = lease.entity_id();
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |home, cx| {
                            if home.quote_tools.press.as_ref().is_some_and(|press| {
                                press.region.entity_id() == id && press.released == Some(point)
                            }) {
                                home.finish_quote_selection(point, window, cx);
                                home.check_quote_tools(window, cx);
                            }
                        });
                    });
                }
                window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                    if !phase.capture()
                        || event.button != MouseButton::Left
                        || !lease.read(cx).hit.get().contains(&event.position)
                    {
                        return;
                    }
                    let _ = owner.update(cx, |home, cx| {
                        if home
                            .quote_tools
                            .bounds
                            .get()
                            .is_some_and(|b| b.contains(&event.position))
                            || home.shown_key.as_deref() != Some(&lease.read(cx).key)
                        {
                            return;
                        }
                        home.quote_tools.press = Some(Press {
                            region: lease.downgrade(),
                            point: event.position,
                            moved: false,
                            released: None,
                        });
                    });
                });
            },
        )
        .absolute()
        .inset_0()
    }
}

impl HomeView {
    fn check_quote_tools(&mut self, window: &Window, cx: &mut Context<Self>) {
        let using_tools =
            self.quote_tools.pointer_held || self.quote_tools.focus.contains_focused(window, cx);
        let invalid = self.quote_tools.active.as_ref().is_some_and(|active| {
            active
                .region
                .read_with(cx, |lease, cx| {
                    let text = lease.target.text(cx);
                    // A selection control may clear its visual selection as the
                    // toolbar takes focus. Keep that already-captured payload,
                    // but still reject a different nonempty selection.
                    let cleared_for_tools = using_tools && text.is_empty();
                    self.shown_key.as_deref() != Some(&lease.key)
                        || self.state.read(cx).selected().as_deref() != Some(lease.key.as_str())
                        || (!cleared_for_tools
                            && (lease.target.metadata(cx) != active.metadata
                                || text != active.text))
                        || active.capture.as_ref().is_ok_and(|snapshot| {
                            self.state
                                .read(cx)
                                .sessions()
                                .get(&snapshot.key)
                                .is_none_or(|s| s.binding() != snapshot.binding)
                        })
                })
                .unwrap_or(true)
        });
        if invalid && self.quote_tools.dismiss() {
            cx.notify();
        }
    }
    fn finish_quote_selection(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(press) = self.quote_tools.press.take().filter(|press| {
            press.moved || (point - press.point).magnitude() > POINTER_DRAG_THRESHOLD
        }) else {
            return;
        };
        let Some(region) = press.region.upgrade() else {
            return;
        };
        let lease = region.read(cx);
        if self.shown_key.as_deref() != Some(&lease.key) || !window.is_window_active() {
            return;
        }
        let text = lease.target.text(cx);
        if text.is_empty() {
            return;
        }
        let capture = self.bind_quote(lease.target.selection(cx), cx);
        self.quote_tools.active = Some(Active {
            region: region.downgrade(),
            point,
            capture,
            text,
            metadata: lease.target.metadata(cx),
        });
        cx.notify();
    }
    fn use_quote_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = self.quote_tools.active.take() else {
            return;
        };
        let anchor = self.quote_tools.bounds.replace(None);
        let Ok(snapshot) = active.capture else {
            return;
        };
        if !self.quote_snapshot_current(&snapshot, cx) {
            return;
        }
        if let Some(region) = active.region.upgrade() {
            region.read(cx).target.clone().clear(cx);
        }
        if let Some(id) = self.add_quote_snapshot(&snapshot, window, cx)
            && let Some(anchor) = anchor
            && let Some(view) = self.views.get(&snapshot.key)
        {
            view.quote_cards
                .update(cx, |cards, cx| cards.open_floating(&id, anchor, window, cx));
        }
        cx.notify();
    }
    pub(in crate::home) fn render_quote_tools(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(cards) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get(key))
            .map(|view| view.quote_cards.clone())
            && cards.read(cx).floating()
        {
            return cards.update(cx, |cards, cx| cards.render_floating(cx));
        }
        self.check_quote_tools(window, cx);
        if window.has_active_dialog(cx) {
            self.quote_tools.dismiss();
        }
        let Some(active) = &self.quote_tools.active else {
            return div().into_any_element();
        };
        let region = active.region.clone();
        let anchor = active.point;
        let reason = active
            .capture
            .as_ref()
            .err()
            .copied()
            .or_else(|| (!self.quote_allowed(cx)).then_some(Unavailable::Composer));
        let child = h_flex()
            .id("quote-selection-toolbar")
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .min_w_0()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .shadow_lg()
            .track_focus(&self.quote_tools.focus)
            .child(
                Button::new("quote-selection-add")
                    .ghost()
                    .xsmall()
                    .text_sm()
                    .icon(IconName::Quote)
                    .label(t(cx, "quote-action"))
                    .disabled(reason.is_some())
                    .when_some(reason, |button, reason| button.tooltip(t(cx, reason.key())))
                    .on_click(cx.listener(|home, _, window, cx| home.use_quote_tools(window, cx))),
            );
        let bounds = self.quote_tools.bounds.clone();
        let owner = cx.weak_entity();
        canvas(
            move |_, window, cx| {
                bounds.set(None);
                // The source is prepainted earlier in this same Home frame.
                // Resolve its latest clip before measuring; no second render is
                // needed to repair a resize or other layout change.
                let region = region.upgrade()?;
                let source = region.read(cx);
                let area = source.area.get().intersect(&window.content_mask().bounds);
                let hit = source.hit.get();
                let gap = window.rem_size() * 0.5;
                if area.size.width <= gap * 2.
                    || area.size.height <= gap * 2.
                    || hit.size.width <= px(0.)
                    || hit.size.height <= px(0.)
                {
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |home, cx| {
                            if home.quote_tools.dismiss() {
                                cx.notify();
                            }
                        });
                    });
                    return None;
                }
                let width = area.size.width - gap * 2.;
                let mut child = child.max_w(width).into_any_element();
                let measured = child.layout_as_root(
                    size(AvailableSpace::MaxContent, AvailableSpace::MinContent),
                    window,
                    cx,
                );
                let x = (anchor.x - measured.width / 2.).clamp(
                    area.left() + gap,
                    (area.right() - gap - measured.width).max(area.left() + gap),
                );
                let above = anchor.y - gap - measured.height;
                let y = if above >= area.top() + gap {
                    above
                } else {
                    (anchor.y + gap)
                        .min(area.bottom() - gap - measured.height)
                        .max(area.top() + gap)
                };
                let origin = point(x, y);
                let visible = Bounds::new(origin, measured).intersect(&area);
                if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
                    return None;
                }
                bounds.set(Some(visible));
                window.with_content_mask(Some(ContentMask { bounds: area }), |window| {
                    child.prepaint_at(origin, window, cx);
                });
                Some((child, area))
            },
            move |_, prepared, window, cx| {
                if let Some((mut child, area)) = prepared {
                    window.with_content_mask(Some(ContentMask { bounds: area }), |window| {
                        child.paint(window, cx)
                    });
                }
            },
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }
}

pub(in crate::home) fn events(owner: WeakEntity<HomeView>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let down = owner.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                if phase.capture() {
                    let _ = down.update(cx, |home, cx| {
                        home.quote_tools.pointer_held = event.button == MouseButton::Left
                            && home
                                .quote_tools
                                .bounds
                                .get()
                                .is_some_and(|b| b.contains(&event.position));
                        if !home
                            .quote_tools
                            .bounds
                            .get()
                            .is_some_and(|b| b.contains(&event.position))
                        {
                            if home.quote_tools.active.take().is_some() {
                                home.quote_tools.bounds.set(None);
                                cx.notify();
                            }
                            let keep_press = home.quote_tools.press.as_ref().is_some_and(|press| {
                                press
                                    .region
                                    .read_with(cx, |lease, _| {
                                        lease.hit.get().contains(&event.position)
                                    })
                                    .unwrap_or(false)
                            });
                            if !keep_press {
                                home.quote_tools.press = None;
                            }
                        }
                    });
                }
            });
            let moves = owner.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase.capture() {
                    let _ = moves.update(cx, |home, _| {
                        if let Some(press) = &mut home.quote_tools.press
                            && (event.position - press.point).magnitude() > POINTER_DRAG_THRESHOLD
                        {
                            press.moved = true;
                        }
                    });
                }
            });
            let up = owner.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.capture() && event.button == MouseButton::Left {
                    let _ = up.update(cx, |home, cx| {
                        if let Some(press) = &mut home.quote_tools.press {
                            press.released = Some(event.position);
                            cx.notify();
                        }
                    });
                    let owner = up.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |home, cx| {
                            home.quote_tools.pointer_held = false;
                            home.check_quote_tools(window, cx);
                        });
                    });
                }
            });
            let scroll = owner.clone();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _, cx| {
                if phase.capture() {
                    let _ = scroll.update(cx, |home, cx| {
                        let inside = home.quote_tools.active.as_ref().is_some_and(|active| {
                            active
                                .region
                                .read_with(cx, |lease, _| {
                                    lease.area.get().contains(&event.position)
                                })
                                .unwrap_or(true)
                        });
                        if inside && home.quote_tools.dismiss() {
                            cx.notify();
                        }
                    });
                }
            });
            let keys = owner.clone();
            window.on_key_event(move |event: &KeyDownEvent, phase, window, cx| {
                if phase.capture() && event.keystroke.key == "escape" {
                    let _ = keys.update(cx, |home, cx| {
                        if let Some(active) = home.quote_tools.active.take() {
                            if home.quote_tools.focus.contains_focused(window, cx)
                                && let Some(region) = active.region.upgrade()
                            {
                                region.read(cx).target.clone().focus(window, cx);
                            }
                            home.quote_tools.dismiss();
                            cx.stop_propagation();
                            cx.notify();
                        }
                    });
                }
            });
            window.on_key_event(move |_: &KeyUpEvent, phase, window, cx| {
                if phase.capture() {
                    let owner = owner.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |home, cx| home.check_quote_tools(window, cx));
                    });
                }
            });
        },
    )
    .absolute()
    .size_0()
}
