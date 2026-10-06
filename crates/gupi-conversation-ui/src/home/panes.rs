use super::*;

const LEFT_MIN_REM: f32 = 10.;
const LEFT_MAX_REM: f32 = 30.;
const RIGHT_MIN_REM: f32 = 15.;
const RIGHT_MAX_REM: f32 = 32.5;

fn sidebar_bounds(width: f32, reserved: f32, content: f32, rem: f32) -> (f32, f32) {
    let maximum = (width - reserved - content).clamp(0., LEFT_MAX_REM * rem);
    ((LEFT_MIN_REM * rem).min(maximum), maximum)
}

fn navigator_bounds(main: f32, content: f32, overlay: bool, rem: f32) -> (f32, f32) {
    let reserved = if overlay { 6.25 * rem } else { content };
    let maximum = (main - reserved).clamp(0., RIGHT_MAX_REM * rem);
    ((RIGHT_MIN_REM * rem).min(maximum), maximum)
}

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
        let (minimum, maximum) = sidebar_bounds(width, 0., 24. * rem, rem);
        self.left = (preferences.sidebar_width_rem * rem).clamp(minimum, maximum);
        let main = width - if left { self.left } else { 0. };
        let required = if source { 54. * rem } else { 24. * rem };
        self.overlay = right && main < required + RIGHT_MIN_REM * rem;
        self.right = if right {
            let (minimum, maximum) = navigator_bounds(main, required, self.overlay, rem);
            (preferences.navigator_width_rem * rem).clamp(minimum, maximum)
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
            Side::Left => sidebar_bounds(
                width,
                if self.overlay { 0. } else { self.right },
                content_min,
                rem,
            ),
            Side::Right => navigator_bounds(
                width - if left_visible { self.left } else { 0. },
                content_min,
                self.overlay,
                rem,
            ),
            Side::Source => unreachable!(),
        };
        let value = requested.clamp(minimum, maximum);
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
    start_ratio: Option<f32>,
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
                    start_ratio: this.pane_layout.ratio,
                });
                this.pane_layout
                    .drag_to(side, f32::from(window.mouse_position().x));
                cx.notify();
            });
            cx.new(|_| DragPreview)
        })
    }

    pub(super) fn interrupt_pane_drag(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(drag) = self.pane_drag.take() {
            self.pane_layout.ratio = drag.start_ratio;
            self.pane_layout.fit_split(f32::from(window.rem_size()));
            cx.stop_active_drag(window);
        }
    }

    pub(super) fn finish_pane_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::PaneLayout;
    use super::Side;
    use gupi_settings::layout;

    fn setup(
        cx: &mut gpui_kit::TestAppContext,
    ) -> (
        gpui_kit::Entity<super::HomeView>,
        &mut gpui_kit::VisualTestContext,
    ) {
        use crate::home::HomeView;
        use gpui_kit::AppContext;
        use gpui_kit::component::Root;
        use gpui_kit::px;
        use gpui_kit::size;
        use gupi_conversation::conversation::ConversationState;
        cx.update(|cx| {
            gpui_kit::init(cx);
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            cx.set_global(super::super::TestLayoutDirectory(
                std::env::temp_dir().join(format!("gupi-resize-{}-{unique}", std::process::id())),
            ));
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_settings::keybindings::apply(&Default::default(), cx);
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
        visual.simulate_resize(size(px(1728.), px(800.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        (home, visual)
    }

    #[gpui_kit::test]
    fn sidebar_drag_tracks_pointer_without_width_transitions(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::Modifiers;
        use gpui_kit::MouseButton;
        use gpui_kit::point;
        use gpui_kit::px;
        use gpui_kit::size;
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(1200.), px(800.)));
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
            record.sidebar_width_rem = 30.;
            record.navigator_width_rem = 32.5;
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
            record.sidebar_width_rem = 18.75;
            record.navigator_width_rem = 18.75;
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
            record.sidebar_width_rem = 30.;
            record.navigator_width_rem = 32.5;
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
            (
                preferences.sidebar_width_rem,
                preferences.navigator_width_rem
            ),
            (30., 32.5)
        );
    }
    #[test]
    fn overlay_and_visibility_changes_keep_the_original_pixel_preferences() {
        let preferences = {
            let mut record = layout::LayoutState::default();
            record.sidebar_width_rem = 30.;
            record.navigator_width_rem = 32.5;
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
        assert_eq!(preferences.navigator_width_rem, 20.);
    }
    #[test]
    fn navigator_fit_drag_and_reopen_share_feasible_rem_bounds() {
        for rem in [12., 13., 14., 16., 20., 40.] {
            for width in [800., 1200., 4000.] {
                let mut preferences = layout::LayoutState::default();
                preferences.sidebar_width_rem = 30.;
                let mut panes = PaneLayout::default();
                panes.fit_workspace(width, true, true, false, rem, &preferences);
                for requested in [0., 10000.] {
                    panes.resize(Side::Right, requested);
                    let dragged = panes.right;
                    assert!(dragged > 0.);
                    preferences.navigator_width_rem = dragged / rem;
                    panes.fit_workspace(width, true, false, false, rem, &preferences);
                    panes.fit_workspace(width, true, true, false, rem, &preferences);
                    assert!((panes.right - dragged).abs() < 0.001);
                }
            }
        }
        let mut panes = PaneLayout::default();
        let preferences = layout::LayoutState::default();
        panes.fit_workspace(4000., false, true, false, 14., &preferences);
        assert_eq!(panes.right, 20. * 14.);
        panes.fit_workspace(4000., false, true, false, 20., &preferences);
        assert_eq!(panes.right, 20. * 20.);
    }

    fn start_drag(
        home: &gpui_kit::Entity<super::HomeView>,
        visual: &mut gpui_kit::VisualTestContext,
        side: Side,
        delta: f32,
    ) -> gpui_kit::Point<gpui_kit::Pixels> {
        use gpui_kit::{Modifiers, MouseButton, point, px};
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let x = visual.update(|window, cx| {
            let panes = &home.read(cx).pane_layout;
            match side {
                Side::Left => panes.left,
                Side::Right => f32::from(window.viewport_size().width) - panes.right,
                Side::Source => panes.left + panes.conversation,
            }
        });
        let start = point(px(x), px(300.));
        let end = point(px(x + delta), px(300.));
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        visual.update(|window, cx| window.draw(cx).clear(cx));
        visual.update(|_, cx| assert!(home.read(cx).pane_drag.is_some()));
        end
    }

    #[gpui_kit::test]
    async fn escape_and_release_commit_dragged_sizes(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Modifiers, MouseButton};
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.show_history = true;
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.pane_layout.set_ratio(0.4);
            })
        });
        visual.run_until_parked();
        start_drag(&home, visual, Side::Right, 40.);
        let width = visual.update(|_, cx| home.read(cx).pane_layout.right);
        visual.simulate_keystrokes("escape");
        visual.update(|window, cx| {
            assert!(home.read(cx).pane_drag.is_none());
            assert_eq!(
                cx.global::<layout::LayoutState>().navigator_width_rem
                    * f32::from(window.rem_size()),
                width
            );
            assert!(home.read(cx).layout_save.is_some());
        });
        for escape in [true, false] {
            let end = start_drag(&home, visual, Side::Source, 50.);
            let ratio = visual.update(|_, cx| home.read(cx).pane_layout.ratio());
            if escape {
                visual.simulate_keystrokes("escape");
            } else {
                visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            }
            visual.update(|_, cx| {
                assert!(home.read(cx).pane_drag.is_none());
                assert!(home.read(cx).source_split_touched);
                assert!(home.read(cx).source_save.is_some());
                assert_eq!(home.read(cx).pane_layout.ratio(), ratio);
            });
            let (layout, source) = visual.update(|_, cx| {
                home.update(cx, |home, _| {
                    (home.layout_save.take(), home.source_save.take())
                })
            });
            if let Some(save) = layout {
                save.await;
            }
            if let Some(save) = source {
                save.await;
            }
            let path = visual
                .update(|_, cx| super::HomeView::layout_directory(cx).unwrap())
                .join("source-split.toml");
            assert!((gupi_settings::source_split::load(&path) - ratio).abs() < 0.0001);
            assert_eq!(
                layout::load(&path.with_file_name("state.toml")).navigator_width_rem,
                visual.update(|window, _| width / f32::from(window.rem_size()))
            );
        }
        let directory = visual.update(|_, cx| super::HomeView::layout_directory(cx).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[gpui_kit::test]
    fn layout_change_interrupts_source_drag_without_saving(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{px, size};
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
            })
        });
        visual.run_until_parked();
        for initial in [None, Some(0.45)] {
            visual.update(|_, cx| {
                home.update(cx, |home, cx| {
                    home.pane_layout.ratio = initial;
                    home.pane_layout.container = None;
                    cx.notify();
                })
            });
            start_drag(&home, visual, Side::Source, 80.);
            visual.simulate_resize(size(px(1800.), px(800.)));
            visual.update(|window, cx| {
                window.draw(cx).clear(cx);
                let home = home.read(cx);
                assert!(home.pane_drag.is_none());
                assert_eq!(home.pane_layout.ratio, initial);
                assert!(home.source_save.is_none());
                let panes = &home.pane_layout;
                let expected = (initial.unwrap_or(0.4) * panes.work).clamp(
                    24. * f32::from(window.rem_size()),
                    panes.work - 30. * f32::from(window.rem_size()),
                );
                assert!((panes.conversation - expected).abs() < 0.01);
            });
            visual.simulate_resize(size(px(1728.), px(800.)));
        }
    }
}
