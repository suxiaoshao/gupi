use super::*;

const LEFT_MIN: f32 = 160.;
const LEFT_MAX: f32 = 480.;
const RIGHT_MAX: f32 = 520.;

#[derive(Clone, Copy)]
pub(super) enum Side {
    Left,
    Right,
    Source,
}

/// Displayed widths are independent of persisted preferences. Re-fit them only
/// when the available space or visible panels change, never after a sibling drag.
#[derive(Default)]
pub(super) struct PaneLayout {
    container: Option<(f32, bool, bool, bool, f32)>,
    pub conversation: f32,
    pub work: f32,
    pub single: bool,
    ratio: Option<f32>,
    pub left: f32,
    pub right: f32,
    pub overlay: bool,
}

impl PaneLayout {
    pub fn fit_workspace(
        &mut self,
        width: f32,
        left: bool,
        right: bool,
        source: bool,
        rem: f32,
        preferences: &layout::LayoutState,
    ) -> bool {
        let container = (width, left, right, source, rem);
        if self.container == Some(container) {
            return false;
        }
        self.container = Some(container);
        self.left = preferences.sidebar_width.min((width - 24. * rem).max(0.));
        let main = width - if left { self.left } else { 0. };
        let required = if source { 54. * rem } else { 24. * rem };
        self.overlay = right && main < required + 15. * rem;
        self.right = if right {
            if self.overlay {
                preferences
                    .history_width
                    .min((main - 6.25 * rem).max(15. * rem))
            } else {
                preferences
                    .history_width
                    .clamp(15. * rem, (main - required).max(15. * rem))
            }
        } else {
            0.
        };
        self.work = main
            - if right && !self.overlay {
                self.right
            } else {
                0.
            };
        self.single = source && self.work < 54. * rem;
        self.fit_split(rem);
        true
    }
    fn fit_split(&mut self, rem: f32) {
        self.conversation = if self.single {
            self.work
        } else {
            (self.ratio.unwrap_or(0.4) * self.work).clamp(
                (24. * rem).min(self.work),
                (self.work - 30. * rem).max((24. * rem).min(self.work)),
            )
        };
    }
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = Some(ratio);
        self.container = None;
    }
    pub fn ratio(&self) -> f32 {
        self.ratio.unwrap_or(0.4)
    }
    pub fn width(&self, side: Side) -> f32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
            Side::Source => self.conversation,
        }
    }

    fn drag_to(&mut self, side: Side, x: f32) {
        let Some((width, left, _, _, _)) = self.container else {
            return;
        };
        self.resize(
            side,
            match side {
                Side::Left => x,
                Side::Right => width - x,
                Side::Source => x - if left { self.left } else { 0. },
            },
        );
    }

    pub fn resize(&mut self, side: Side, requested: f32) {
        let Some((width, left_visible, _, source, rem)) = self.container else {
            return;
        };
        if matches!(side, Side::Source) {
            if !self.single && source && self.work > 0. {
                self.conversation = requested.clamp(24. * rem, self.work - 30. * rem);
                self.ratio = Some(self.conversation / self.work);
            }
            return;
        }
        let content_min = if source && !self.single {
            54. * rem
        } else {
            24. * rem
        };
        let (minimum, maximum) = match side {
            Side::Left => (
                LEFT_MIN,
                (width - content_min - if self.overlay { 0. } else { self.right }).min(LEFT_MAX),
            ),
            Side::Source => unreachable!(),
            Side::Right => (
                15. * rem,
                if self.overlay {
                    width - 6.25 * rem
                } else {
                    width - content_min - if left_visible { self.left } else { 0. }
                }
                .min(RIGHT_MAX),
            ),
        };
        let value = requested.clamp(minimum.min(maximum.max(0.)), maximum.max(0.));
        match side {
            Side::Left => self.left = value,
            Side::Right => self.right = value,
            Side::Source => unreachable!(),
        }
        let main = width - if left_visible { self.left } else { 0. };
        self.work = main - if self.overlay { 0. } else { self.right };
        self.fit_split(rem);
    }
}

pub(super) struct Drag {
    side: Side,
    start_width: f32,
}

struct DragPreview;
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl HomeView {
    pub(super) fn resizing_sidebar(&self) -> bool {
        matches!(
            self.pane_drag,
            Some(Drag {
                side: Side::Left,
                ..
            })
        )
    }

    pub(super) fn pane_handle(&self, side: Side, cx: &Context<Self>) -> impl IntoElement + use<> {
        let owner = cx.weak_entity();
        gpui_kit::base::resize_handle(
            match side {
                Side::Left => "sidebar-resize",
                Side::Right => "history-resize",
                Side::Source => "source-resize",
            },
            Axis::Horizontal,
        )
        .on_drag(side, move |_, _, window, cx| {
            let _ = owner.update(cx, |this, cx| {
                this.pane_drag = Some(Drag {
                    side,
                    start_width: this.pane_layout.width(side),
                });
                this.pane_layout
                    .drag_to(side, f32::from(window.mouse_position().x));
                cx.notify();
            });
            cx.new(|_| DragPreview)
        })
    }

    fn finish_pane_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.pane_drag.take() else {
            return;
        };
        let width = self.pane_layout.width(drag.side);
        if (width - drag.start_width).abs() > 0.01 {
            match drag.side {
                Side::Left => self.save_layout(Some(width), None, window, cx),
                Side::Right => self.save_layout(None, Some(width), window, cx),
                Side::Source => self.save_source_split(cx),
            }
        }
        cx.notify();
    }
}

/// Listen across the window so releasing outside the narrow handle still ends
/// the gesture. The kit owns handle appearance, hit testing and drag initiation.
pub(super) struct ResizeEvents(pub WeakEntity<HomeView>);
impl IntoElement for ResizeEvents {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for ResizeEvents {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (window.request_layout(Style::default(), None, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let owner = self.0.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if !phase.bubble() || event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            let _ = owner.update(cx, |this, cx| {
                if let Some(drag) = &this.pane_drag {
                    this.pane_layout
                        .drag_to(drag.side, f32::from(event.position.x));
                    cx.notify();
                    cx.stop_propagation();
                }
            });
        });
        let owner = self.0.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if !phase.bubble() || event.button != MouseButton::Left {
                return;
            }
            let _ = owner.update(cx, |this, cx| this.finish_pane_drag(window, cx));
        });
        let owner = self.0.clone();
        window.on_key_event(move |event: &KeyDownEvent, phase, window, cx| {
            if !phase.capture() || event.keystroke.key != "escape" {
                return;
            }
            let _ = owner.update(cx, |this, cx| {
                if this.pane_drag.take().is_some() {
                    this.pane_layout.container = None;
                    cx.stop_active_drag(window);
                    cx.notify();
                    cx.stop_propagation();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::PaneLayout;
    use super::Side;
    use gupi_settings::layout;

    #[gpui_kit::test]
    fn sidebar_drag_tracks_pointer_without_width_transitions(cx: &mut gpui_kit::TestAppContext) {
        use crate::home::HomeView;
        use gpui_kit::AppContext;
        use gpui_kit::Modifiers;
        use gpui_kit::MouseButton;
        use gpui_kit::component::Root;
        use gpui_kit::point;
        use gpui_kit::px;
        use gpui_kit::size;
        use gupi_conversation::conversation::ConversationState;
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(layout::LayoutState::default());
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            let info = gupi_conversation::session_catalog::SessionInfo::new(
                Default::default(),
                "empty".into(),
                Default::default(),
            );
            state.sessions_for_test().insert(
                "empty".into(),
                gupi_conversation::conversation::Session::new(info, String::new()),
            );
            *state.selected_for_test() = Some("empty".into());
        });
        let mut home = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state, window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        visual.simulate_resize(size(px(1200.), px(800.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let initial = visual.debug_bounds("conversation-center").unwrap().origin.x;
        let start = point(initial, px(300.));
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        for x in [
            initial + px(50.),
            initial + px(120.),
            initial + px(40.),
            initial,
        ] {
            visual.simulate_mouse_move(point(x, start.y), MouseButton::Left, Modifiers::default());
            visual.update(|window, cx| window.draw(cx).clear(cx));
            visual.update(|_, cx| assert!(home.read(cx).resizing_sidebar()));
            // Inspect the first frame after each motion, without advancing the
            // animation clock: both content and chrome must already be at x.
            assert_eq!(
                visual.debug_bounds("conversation-center").unwrap().origin.x,
                x
            );
            assert_eq!(
                visual.debug_bounds("titlebar-sidebar").unwrap().size.width,
                x
            );
        }
        // Return to the starting width so this UI test does not persist layout.
        visual.simulate_mouse_up(start, MouseButton::Left, Modifiers::default());
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert!(!home.read(cx).resizing_sidebar());
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            visual.debug_bounds("conversation-center").unwrap().origin.x,
            initial
        );
        assert_eq!(
            visual.debug_bounds("titlebar-sidebar").unwrap().size.width,
            initial
        );
    }

    #[test]
    fn hidden_sidebar_keeps_animation_width_without_reserving_layout_space() {
        let preferences = {
            let mut record = layout::LayoutState::default();
            record.sidebar_width = 480.;
            record.history_width = 520.;
            record
        };
        let mut panes = PaneLayout::default();
        panes.fit_workspace(1200., true, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 336.));
        panes.fit_workspace(1200., false, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 520.));
        panes.resize(Side::Right, 520.);
        assert_eq!(panes.right, 520.);
        panes.fit_workspace(1200., true, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 336.));
    }
    #[test]
    fn resizing_one_sidebar_never_borrows_from_the_other() {
        let preferences = {
            let mut record = layout::LayoutState::default();
            record.sidebar_width = 300.;
            record.history_width = 300.;
            record
        };
        let mut panes = PaneLayout::default();
        panes.fit_workspace(1200., true, true, false, 16., &preferences);
        panes.resize(Side::Right, 600.);
        assert_eq!((panes.left, panes.right), (300., 516.));
        panes.resize(Side::Left, 480.);
        assert_eq!((panes.left, panes.right), (300., 516.));
        panes.resize(Side::Right, 100.);
        assert_eq!((panes.left, panes.right), (300., 240.));
    }
    #[test]
    fn temporary_constraints_and_sibling_drag_do_not_restore_or_overwrite_preferences() {
        let preferences = {
            let mut record = layout::LayoutState::default();
            record.sidebar_width = 480.;
            record.history_width = 520.;
            record
        };
        let mut panes = PaneLayout::default();
        panes.fit_workspace(1100., true, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 520.));
        panes.resize(Side::Left, 400.);
        // A redraw after dragging the left edge must not expand the right edge.
        assert!(!panes.fit_workspace(1100., true, true, false, 16., &preferences));
        assert_eq!((panes.left, panes.right), (400., 520.));
        panes.fit_workspace(1500., true, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 520.));
        assert_eq!(
            (preferences.sidebar_width, preferences.history_width),
            (480., 520.)
        );
    }
    #[test]
    fn overlay_and_visibility_changes_keep_the_original_pixel_preferences() {
        let preferences = {
            let mut record = layout::LayoutState::default();
            record.sidebar_width = 480.;
            record.history_width = 520.;
            record
        };
        let mut panes = PaneLayout::default();
        panes.fit_workspace(800., true, true, false, 16., &preferences);
        assert!(panes.overlay);
        assert_eq!((panes.left, panes.right), (416., 284.));
        panes.resize(Side::Right, 300.);
        assert_eq!(panes.left, 416.);
        panes.fit_workspace(1500., true, false, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 0.));
        panes.fit_workspace(1500., true, true, false, 16., &preferences);
        assert_eq!((panes.left, panes.right), (480., 520.));
    }
    #[test]
    fn source_layout_fits_minima_and_keeps_the_saved_split() {
        let preferences = layout::LayoutState::default();
        let mut panes = PaneLayout::default();
        panes.set_ratio(0.45);
        panes.fit_workspace(1728., true, true, true, 16., &preferences);
        assert!(!panes.overlay && !panes.single);
        assert!(panes.conversation >= 384. && panes.work - panes.conversation >= 480.);
        panes.resize(Side::Source, 600.);
        let ratio = panes.ratio();
        panes.fit_workspace(1200., true, true, true, 16., &preferences);
        assert!(panes.overlay && !panes.single);
        panes.fit_workspace(960., true, false, true, 16., &preferences);
        assert!(panes.single);
        assert_eq!(panes.ratio(), ratio);
        panes.fit_workspace(1440., true, true, true, 20., &preferences);
        assert!(panes.overlay && !panes.single);
        assert_eq!(preferences.history_width, 300.);
    }
}
