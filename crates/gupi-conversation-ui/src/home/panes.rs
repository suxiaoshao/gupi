use super::*;
pub(super) mod geometry;
use geometry::{Content, History, Workspace, clamp};

const LEFT_MIN_REM: f32 = 10.;
const LEFT_MAX_REM: f32 = 30.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Panel {
    History,
    Workspace,
}
#[derive(Clone, Copy)]
pub(super) enum Side {
    Left,
    Right,
    Source,
    History,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Request {
    pub width: f32,
    pub sidebar: bool,
    pub content: Content,
    pub rem: f32,
    pub workspace_pinned: bool,
    pub history_pinned: bool,
}

#[derive(Default)]
pub(super) struct PaneLayout {
    container: Option<Request>,
    ratio: Option<f32>,
    pub left: f32,
    pub right: f32,
    pub conversation: f32,
    pub work: f32,
    pub workspace_width: f32,
    pub main_width: f32,
    pub history_width: f32,
    pub body_width: f32,
    pub body_origin: f32,
    pub workspace_docked: bool,
    pub history_docked: bool,
    pub overlay: Option<Panel>,
    pub maximized: bool,
    navigator_preference: f32,
    history_preference: f32,
}
impl PaneLayout {
    pub fn available(&self) -> f32 {
        self.container
            .map_or(0., |r| r.width - if r.sidebar { self.left } else { 0. })
    }
    pub fn fit(&mut self, request: Request, preferences: &layout::LayoutState) -> bool {
        if self.container == Some(request) {
            return false;
        }
        let content_changed = self
            .container
            .is_some_and(|old| old.content != request.content);
        let was_docked = self.workspace_docked;
        self.container = Some(request);
        self.left = clamp(
            preferences.sidebar_width_rem * request.rem,
            LEFT_MIN_REM * request.rem,
            (request.width - geometry::CONVERSATION_MIN_REM * request.rem)
                .min(LEFT_MAX_REM * request.rem),
        );
        self.navigator_preference = preferences.navigator_width_rem * request.rem;
        self.history_preference = preferences.history_width_rem * request.rem;
        if content_changed && was_docked && !request.content.dockable(self.available(), request.rem)
        {
            self.overlay = Some(Panel::Workspace);
        }
        self.allocate();
        true
    }
    pub fn invalidate(&mut self) {
        self.container = None;
    }
    pub fn allocate(&mut self) {
        let Some(request) = self.container else {
            return;
        };
        let available = self.available();
        self.workspace_docked =
            request.workspace_pinned && request.content.dockable(available, request.rem);
        if self.workspace_docked && self.overlay == Some(Panel::Workspace) {
            self.overlay = None;
        }
        let workspace = if self.workspace_docked {
            Workspace::docked(
                available,
                request.content,
                self.navigator_preference,
                self.ratio(),
                request.rem,
            )
        } else if self.overlay == Some(Panel::Workspace) {
            Workspace::overlay(
                available,
                request.content,
                self.navigator_preference,
                request.rem,
            )
        } else {
            Workspace {
                conversation: available,
                ..Default::default()
            }
        };
        self.conversation = workspace.conversation;
        self.right = workspace.navigator;
        self.main_width = workspace.main;
        self.workspace_width = workspace.width;
        self.work = workspace.ratio_width;
        let history = History::new(
            self.conversation,
            self.history_preference,
            request.history_pinned,
            request.rem,
        );
        self.history_docked = request.history_pinned && history.dockable;
        if self.history_docked && self.overlay == Some(Panel::History) {
            self.overlay = None;
        }
        self.history_width = history.width;
        self.body_width = history.body;
        self.body_origin = history.origin;
        if !self.workspace_docked {
            self.maximized = false;
        }
        if self.maximized {
            self.conversation = 0.;
            self.history_docked = false;
            self.workspace_width = available;
            self.right = if request.content.navigator {
                clamp(
                    self.navigator_preference,
                    geometry::NAVIGATOR_MIN_REM * request.rem,
                    available
                        - if request.content.main {
                            geometry::CONTENT_MIN_REM * request.rem
                        } else {
                            0.
                        },
                )
            } else {
                0.
            };
            self.main_width = (available
                - self.right
                - if request.content.navigator {
                    geometry::DIVIDER
                } else {
                    0.
                })
            .max(0.);
        }
    }
    pub fn dockable(&self, panel: Panel) -> bool {
        self.container.is_some_and(|r| match panel {
            Panel::Workspace => r.content.dockable(self.available(), r.rem),
            Panel::History => {
                History::new(self.conversation, self.history_preference, true, r.rem).dockable
            }
        })
    }
    pub fn visible(&self, panel: Panel) -> bool {
        self.overlay == Some(panel)
            || match panel {
                Panel::History => self.history_docked,
                Panel::Workspace => self.workspace_docked,
            }
    }
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = Some(ratio);
        self.invalidate();
    }
    pub fn ratio(&self) -> f32 {
        self.ratio.unwrap_or(0.4)
    }
    pub fn width(&self, side: Side) -> f32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
            Side::Source => self.conversation,
            Side::History => self.history_width,
        }
    }
    fn drag_to(&mut self, side: Side, x: f32) {
        let Some(r) = self.container else {
            return;
        };
        self.resize(
            side,
            match side {
                Side::Left => x,
                Side::Right => r.width - x,
                Side::Source => x - if r.sidebar { self.left } else { 0. },
                Side::History => {
                    r.width
                        - self.workspace_width
                        - if self.workspace_docked {
                            geometry::DIVIDER
                        } else {
                            0.
                        }
                        - x
                }
            },
        );
    }
    pub fn resize(&mut self, side: Side, requested: f32) {
        let Some(r) = self.container else {
            return;
        };
        match side {
            Side::Source => {
                if self.workspace_docked && r.content.main && self.work > 0. {
                    self.ratio = Some(
                        clamp(
                            requested,
                            geometry::CONVERSATION_MIN_REM * r.rem,
                            self.work - geometry::CONTENT_MIN_REM * r.rem,
                        ) / self.work,
                    );
                }
            }
            Side::Left => {
                let reserve = if self.workspace_docked {
                    r.content.minimum(r.rem)
                } else {
                    geometry::CONVERSATION_MIN_REM * r.rem
                };
                self.left = clamp(
                    requested,
                    LEFT_MIN_REM * r.rem,
                    (r.width - reserve).min(LEFT_MAX_REM * r.rem),
                );
            }
            Side::Right => self.navigator_preference = requested,
            Side::History => {
                self.history_preference = clamp(
                    requested,
                    geometry::HISTORY_MIN_REM * r.rem,
                    self.conversation
                        - (geometry::BODY_MIN_REM + geometry::HISTORY_GAP_REM) * r.rem,
                )
            }
        }
        self.allocate();
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
                Side::History => "history-panel-resize",
            },
            Axis::Horizontal,
        )
        .when(
            matches!(side, Side::History | Side::Right | Side::Source),
            |handle| {
                let appearance = gpui_kit::component::resizable::resize_handle_appearance();
                handle.with_appearance(Rc::new(move |handle, window, cx| {
                    // The card or column already paints its boundary. Preserve the kit's
                    // hit band and engaged feedback without a second resting line.
                    if handle.state() == gpui_kit::base::ResizeHandleState::Idle {
                        Some(div().into_any_element())
                    } else {
                        appearance(handle, window, cx)
                    }
                }))
            },
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
            self.pane_layout.allocate();
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
                Side::History => {
                    cx.global_mut::<layout::LayoutState>().history_width_rem =
                        width / f32::from(window.rem_size());
                    self.save_layout(None, None, window, cx);
                }
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
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::test::TestWindowExt as _;
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
    fn first_file_promotes_tree_to_overlay_and_escape_returns_to_trigger(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(950.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.select_workspace_tab(None, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                assert!(home.pane_layout.workspace_docked);
                home.open_source("/tmp/workspace-fixture.rs".into(), true, window, cx);
                assert_eq!(home.pane_layout.overlay, Some(Panel::Workspace));
                assert!(home.pane_layout.right > 0.);
                assert!(!home.views["empty"].workspace.navigator_hidden);
                home.source()
                    .unwrap()
                    .update(cx, |source, cx| source.focus(window, cx));
            })
        });
        visual.run_until_parked();
        visual.simulate_keystrokes("escape");
        visual.update(|window, cx| {
            assert!(home.read(cx).pane_layout.overlay.is_none());
            assert!(
                home.read(cx)
                    .workspace_button_focus
                    .contains_focused(window, cx)
            );
            assert!(cx.global::<layout::LayoutState>().workspace_pinned);
        });
    }
    #[gpui_kit::test]
    fn opening_docked_history_replaces_workspace_overlay(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(950.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.select_workspace_tab(None, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                home.open_source("/tmp/workspace-fixture.rs".into(), true, window, cx);
                assert_eq!(home.pane_layout.overlay, Some(Panel::Workspace));
                assert!(home.pane_layout.dockable(Panel::History));
                home.focus_panel(Panel::Workspace, window, cx);
                home.toggle_panel(Panel::History, window, cx);
            });
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            let home = home.read(cx);
            assert!(home.pane_layout.history_docked);
            assert!(home.pane_layout.visible(Panel::History));
            assert!(home.pane_layout.overlay.is_none());
            assert_eq!(home.pane_layout.workspace_width, 0.);
            assert!(home.history_focus_handle(cx).contains_focused(window, cx));
            assert!(cx.global::<layout::LayoutState>().workspace_pinned);
            assert!(cx.global::<layout::LayoutState>().history_pinned);
        });
    }

    #[gpui_kit::test]
    fn overlay_titlebar_toggle_closes_without_reopening(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(800.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx)
            });
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("toggle-workspace", cx);
        });
        visual.run_until_parked();
        assert!(home.read_with(visual, |home, _| home.pane_layout.overlay.is_none()));
    }

    #[gpui_kit::test]
    fn explicit_open_pins_resized_overlay_but_internal_navigation_does_not(
        cx: &mut TestAppContext,
    ) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(800.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                assert_eq!(home.pane_layout.overlay, Some(Panel::Workspace));
                assert!(!cx.global::<layout::LayoutState>().workspace_pinned);
            })
        });
        visual.simulate_resize(size(px(2000.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.fit_layout(window, cx);
                assert_eq!(home.pane_layout.overlay, Some(Panel::Workspace));
                home.select_workspace_tab(None, window, cx);
                assert_eq!(home.pane_layout.overlay, Some(Panel::Workspace));
                home.ensure_panel(Panel::Workspace, window, cx);
                assert!(home.pane_layout.workspace_docked);
                assert!(home.pane_layout.overlay.is_none());
                assert!(cx.global::<layout::LayoutState>().workspace_pinned);
            })
        });
    }
    #[test]
    fn resize_does_not_write_preferences_and_history_can_redock() {
        let mut preferences = layout::LayoutState::default();
        preferences.workspace_pinned = true;
        preferences.history_pinned = true;
        let mut panes = PaneLayout::default();
        let mut request = Request {
            width: 1400.,
            sidebar: false,
            content: Content {
                main: true,
                navigator: true,
            },
            rem: 16.,
            workspace_pinned: true,
            history_pinned: true,
        };
        panes.fit(request, &preferences);
        assert!(panes.workspace_docked);
        assert!(!panes.history_docked);
        request.width = 1000.;
        panes.fit(request, &preferences);
        assert!(!panes.workspace_docked);
        assert!(panes.history_docked);
        assert_eq!(preferences.navigator_width_rem, 20.);
        assert_eq!(preferences.history_width_rem, 20.);
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
                Side::History => panes.left + panes.conversation - panes.history_width,
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
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
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
                home.ensure_panel(Panel::Workspace, window, cx);
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
