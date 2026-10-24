use super::*;
pub(super) mod geometry;
use geometry::{History, Workspace, clamp};

const LEFT_MIN_REM: f32 = 10.;
const LEFT_MAX_REM: f32 = 30.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Panel {
    History,
    Workspace,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Foreground {
    #[default]
    Conversation,
    Workspace,
}
#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub rem: f32,
    pub workspace_pinned: bool,
    pub history_pinned: bool,
}

#[derive(Default)]
pub(super) struct PaneLayout {
    container: Option<Request>,
    invalidated: bool,
    ratio: f32,
    pub left: f32,
    pub conversation: f32,
    pub work: f32,
    pub workspace_width: f32,
    pub history_width: f32,
    pub body_width: f32,
    pub body_origin: f32,
    pub workspace_docked: bool,
    pub history_docked: bool,
    pub history_overlay: bool,
    pub foreground: Foreground,
    pub maximized: bool,
    history_preference: f32,
}
impl PaneLayout {
    pub fn available(&self) -> f32 {
        self.container
            .map_or(0., |r| r.width - if r.sidebar { self.left } else { 0. })
    }
    pub fn needs_fit(&self, request: Request) -> bool {
        self.invalidated || self.container != Some(request)
    }
    pub fn fit(&mut self, request: Request, preferences: &layout::LayoutState) -> bool {
        if !self.needs_fit(request) {
            return false;
        }
        self.container = Some(request);
        self.invalidated = false;
        self.apply_preferences(preferences);
        self.allocate();
        true
    }
    fn apply_preferences(&mut self, preferences: &layout::LayoutState) {
        let Some(request) = self.container else {
            return;
        };
        self.left = clamp(
            preferences.sidebar_width_rem * request.rem,
            LEFT_MIN_REM * request.rem,
            (request.width - geometry::CONVERSATION_MIN_REM * request.rem)
                .min(LEFT_MAX_REM * request.rem),
        );
        self.history_preference = preferences.history_width_rem * request.rem;
        self.ratio = preferences.source_split_ratio;
    }
    fn restore_preferences(&mut self, preferences: &layout::LayoutState) {
        self.apply_preferences(preferences);
        self.allocate();
    }
    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }
    pub fn allocate(&mut self) {
        let Some(request) = self.container else {
            return;
        };
        let available = self.available();
        self.workspace_docked = request.workspace_pinned
            && !self.maximized
            && Workspace::dockable(available, request.rem);
        let workspace = if self.workspace_docked {
            Workspace::docked(available, self.ratio(), request.rem)
        } else if request.workspace_pinned
            && (self.maximized || self.foreground == Foreground::Workspace)
        {
            Workspace {
                width: available,
                ..Default::default()
            }
        } else {
            Workspace {
                conversation: available,
                ..Default::default()
            }
        };
        self.conversation = workspace.conversation;
        self.workspace_width = workspace.width;
        self.work = workspace.ratio_width;
        let history = History::new(
            self.conversation,
            self.history_preference,
            request.history_pinned,
            request.rem,
        );
        self.history_docked = request.history_pinned && history.dockable;
        if self.history_docked {
            self.history_overlay = false;
        }
        self.history_width = history.width;
        self.body_width = history.body;
        self.body_origin = history.origin;
    }
    pub fn dockable(&self, panel: Panel) -> bool {
        self.container.is_some_and(|r| match panel {
            Panel::Workspace => Workspace::dockable(self.available(), r.rem),
            Panel::History => {
                History::new(self.conversation, self.history_preference, true, r.rem).dockable
            }
        })
    }
    pub fn visible(&self, panel: Panel) -> bool {
        match panel {
            Panel::History => {
                self.conversation > 0. && (self.history_overlay || self.history_docked)
            }
            Panel::Workspace => self.workspace_width > 0.,
        }
    }
    pub fn ratio(&self) -> f32 {
        self.ratio
    }
    pub fn width(&self, side: Side) -> f32 {
        match side {
            Side::Left => self.left,
            Side::Right => unreachable!("navigator belongs to the workspace"),
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
                if self.workspace_docked && self.work > 0. {
                    self.ratio = clamp(
                        requested,
                        geometry::CONVERSATION_MIN_REM * r.rem,
                        self.work - geometry::WORKSPACE_MIN_REM * r.rem,
                    ) / self.work;
                }
            }
            Side::Left => {
                let reserve = if self.workspace_docked {
                    (geometry::CONVERSATION_MIN_REM + geometry::WORKSPACE_MIN_REM) * r.rem
                        + geometry::DIVIDER
                } else {
                    geometry::CONVERSATION_MIN_REM * r.rem
                };
                self.left = clamp(
                    requested,
                    LEFT_MIN_REM * r.rem,
                    (r.width - reserve).min(LEFT_MAX_REM * r.rem),
                );
            }
            Side::Right => unreachable!("navigator belongs to the workspace"),
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
    pub side: Side,
    // Allocated at drag start: preferences outside the window's constraints are clamped.
    start_width: f32,
}

struct DragPreview;
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl HomeView {
    fn pane_width(&self, side: Side) -> f32 {
        if side == Side::Right {
            self.workspace_layout.navigator_width
        } else {
            self.pane_layout.width(side)
        }
    }
    fn drag_pane_to(&mut self, side: Side, x: f32, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.pane_layout.visible(Panel::Workspace);
        if side == Side::Right {
            let requested = f32::from(window.viewport_size().width) - x;
            self.workspace_layout = workspace::layout::Layout::new(
                self.pane_layout.workspace_width,
                f32::from(window.rem_size()),
                self.workspace_content(),
                false,
                requested,
            );
        } else {
            self.pane_layout.drag_to(side, x);
            self.fit_workspace_layout(window, cx);
        }
        if visible != self.pane_layout.visible(Panel::Workspace) {
            self.sync_worktree(window, cx);
        }
    }
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
                    start_width: this.pane_width(side),
                });
                this.drag_pane_to(side, f32::from(window.mouse_position().x), window, cx);
                cx.notify();
            });
            cx.new(|_| DragPreview)
        })
    }

    pub(super) fn interrupt_pane_drag(&mut self, window: &mut Window, cx: &mut App) {
        if self.pane_drag.take().is_some() {
            if !self.state.read(cx).is_temporary() {
                self.pane_layout
                    .restore_preferences(cx.global::<layout::LayoutState>());
                self.fit_workspace_layout(window, cx);
            }
            cx.stop_active_drag(window);
        }
    }

    pub(super) fn finish_pane_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.pane_drag.take() else {
            return;
        };
        let width = self.pane_width(drag.side);
        // Temporary windows keep transient allocation only; never change main-window preferences.
        if self.state.read(cx).is_temporary() {
            cx.stop_active_drag(window);
            cx.notify();
            return;
        }
        if (width - drag.start_width).abs() <= 0.01 {
            // A drag held at a clamp keeps the wider preference for a larger window.
            self.pane_layout
                .restore_preferences(cx.global::<layout::LayoutState>());
            self.fit_workspace_layout(window, cx);
            self.sync_worktree(window, cx);
            cx.stop_active_drag(window);
            cx.notify();
            return;
        }
        match drag.side {
            Side::Left => self.save_layout(Some(width), None, window, cx),
            Side::Right => self.save_layout(None, Some(width), window, cx),
            Side::Source => {
                cx.global_mut::<layout::LayoutState>().source_split_ratio =
                    self.pane_layout.ratio();
                self.save_layout(None, None, window, cx);
            }
            Side::History => {
                cx.global_mut::<layout::LayoutState>().history_width_rem =
                    width / f32::from(window.rem_size());
                self.save_layout(None, None, window, cx);
            }
        }
        self.pane_layout
            .restore_preferences(cx.global::<layout::LayoutState>());
        self.fit_workspace_layout(window, cx);
        self.sync_worktree(window, cx);
        cx.stop_active_drag(window);
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
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if !phase.bubble() || event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            let _ = owner.update(cx, |this, cx| {
                if let Some(drag) = &this.pane_drag {
                    this.drag_pane_to(drag.side, f32::from(event.position.x), window, cx);
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
            let _ = owner.update(cx, |this, cx| {
                if let Some(drag) = &this.pane_drag {
                    this.drag_pane_to(drag.side, f32::from(event.position.x), window, cx);
                    this.finish_pane_drag(window, cx);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::test::TestWindowExt as _;
    struct Fixture {
        _directory: tempfile::TempDir,
    }
    impl Global for Fixture {}
    fn setup(
        cx: &mut gpui_kit::TestAppContext,
    ) -> (
        gpui_kit::Entity<super::HomeView>,
        &mut gpui_kit::VisualTestContext,
    ) {
        setup_with_preferences(cx, layout::LayoutState::default())
    }

    fn setup_with_preferences(
        cx: &mut gpui_kit::TestAppContext,
        preferences: layout::LayoutState,
    ) -> (
        gpui_kit::Entity<super::HomeView>,
        &mut gpui_kit::VisualTestContext,
    ) {
        setup_initialized(cx, preferences, false, |_, _, _| {})
    }

    fn setup_initialized(
        cx: &mut gpui_kit::TestAppContext,
        preferences: layout::LayoutState,
        temporary: bool,
        initialize: impl FnOnce(&Entity<HomeView>, &mut Window, &mut App),
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
            let directory = tempfile::tempdir().unwrap();
            cx.set_global(super::super::TestLayoutDirectory(directory.path().into()));
            cx.set_global(Fixture {
                _directory: directory,
            });
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_settings::keybindings::apply(&Default::default(), cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(preferences);
        });
        let state = cx.new(|cx| {
            if temporary {
                ConversationState::temporary("unused-pi".into(), cx)
            } else {
                ConversationState::new("unused-pi".into(), cx)
            }
        });
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
            initialize(&view, window, cx);
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        let home = home.unwrap();
        visual.simulate_resize(size(px(1728.), px(800.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        (home, visual)
    }

    // Opening the palette captures the source before focus changes; adding a
    // session attachment must leave the existing editor content untouched.
    #[gpui_kit::test]
    fn quote_palette_adds_attachment_without_editing_existing_draft(cx: &mut TestAppContext) {
        use gpui_kit::base::input::InlineToken;
        use gupi_conversation::history::DisplayMessage;
        let (home, visual) = setup_initialized(
            cx,
            layout::LayoutState::default(),
            false,
            |home, window, cx| {
                gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
                home.update(cx, |home, cx| {
                    home.state.update(cx, |state, cx| {
                        let session = state.sessions_for_test().get_mut("empty").unwrap();
                        *session.binding_for_test() = 1;
                        *session.state_for_test() = Some(serde_json::from_value(serde_json::json!({
                            "sessionId":"quote-fixture","isStreaming":false,"isCompacting":false
                        })).unwrap());
                        session.mark_transcript_message_for_test();
                        session.live_for_test().push(DisplayMessage::new(
                            "quote-answer".into(),
                            serde_json::json!({"role":"assistant","stopReason":"stop","content":[{"type":"text","text":"Selected **answer**."}]}),
                        ));
                        *session.content_revision_for_test() += 1;
                        session.attachments_for_test().push(
                            gupi_conversation::attachments::Attachment::file("note.txt".into(), 0),
                        );
                        gupi_conversation::conversation::test_support::notify_session("empty", cx);
                    });
                    home.sync(false, window, cx);
                    home.input.update(cx, |input, cx| {
                        input.set_value("draft ", window, cx);
                        input.replace_range_with_token(
                            6..6,
                            InlineToken::new("file-fixture", "@note.txt"),
                            window,
                            cx,
                        ).unwrap();
                        input.set_selected_range(0..5, cx);
                    });
                });
            },
        );
        visual.run_until_parked();
        visual.update(|window, cx| window.render_frame(cx));
        let (original, original_selection, captured) = visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                let view = &home.views[home.shown_key.as_ref().unwrap()];
                let markdown = view.markdown.get(
                    "empty-text-quote-answer",
                    "Selected **answer**.".into(),
                    view.scroller.downgrade(),
                    cx,
                );
                let text_view = markdown.read(cx).view.clone();
                text_view.update(cx, |text, cx| {
                    text.select_all(cx);
                    text.focus_handle().focus(window, cx);
                });
                let captured = home.capture_quote(window, cx).unwrap();
                let original = home.input.read(cx).content().clone();
                let original_selection = home.input.read(cx).selected_range();
                home.open_commands(false, window, cx);
                (original, original_selection, captured)
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.input("Quote in composer", cx);
            window.press("enter", cx);
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            home.update(cx, |home, cx| {
                assert_eq!(home.input.read(cx).content(), original);
                assert_eq!(home.input.read(cx).selected_range(), original_selection);
                let session = home.state.read(cx).current().unwrap();
                assert_eq!(session.quotes().len(), 1);
                assert_eq!(session.quotes()[0].selection.text, "Selected answer.\n");
                assert_eq!(home.input.read(cx).content().tokens(), original.tokens());
                assert_eq!(
                    home.state.read(cx).current().unwrap().attachments().len(),
                    1
                );
            })
        });
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                assert_eq!(home.input.read(cx).content(), original);
                home.quote(&quote::Quote(Some(captured.clone())), window, cx);
                home.state.update(cx, |state, _| {
                    *state
                        .sessions_for_test()
                        .get_mut("empty")
                        .unwrap()
                        .binding_for_test() += 1;
                });
                home.quote(&quote::Quote(Some(captured)), window, cx);
                assert_eq!(home.input.read(cx).content(), original);
                assert_eq!(home.state.read(cx).current().unwrap().quotes().len(), 2);
            })
        });
        let id = visual.update(|_, cx| {
            home.read(cx).state.read(cx).current().unwrap().quotes()[0]
                .id
                .clone()
        });
        visual.update(|window, cx| {
            assert_eq!(
                window
                    .find("quote-list-open")
                    .label()
                    .unwrap()
                    .replace(['\u{2068}', '\u{2069}'], ""),
                "2 quotes"
            );
            home.read(cx).input.focus_handle(cx).focus(window, cx);
            window.hover("quote-list-open", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert!(window.try_find(format!("quote-edit-{id}")).is_some());
            assert!(
                home.read(cx)
                    .input
                    .focus_handle(cx)
                    .contains_focused(window, cx)
            );
            window.dispatch_event(
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: Keystroke::parse("escape").unwrap(),
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert!(window.try_find(format!("quote-edit-{id}")).is_none());
            assert!(
                home.read(cx)
                    .input
                    .focus_handle(cx)
                    .contains_focused(window, cx)
            );
            window.click("quote-list-open", cx);
            window.click(format!("quote-edit-{id}"), cx);
            window.input("Why this order?", cx);
            window.press("enter", cx);
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert_eq!(
                home.read(cx).state.read(cx).current().unwrap().quotes()[0].comment,
                "Why this order?"
            );
        });
        visual.update(|window, cx| {
            window.click("quote-list-open", cx);
            window.click(format!("quote-edit-{id}"), cx);
            window.press("secondary-a", cx);
            window.input("Save on Escape", cx);
            window.press("escape", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                home.read(cx).state.read(cx).current().unwrap().quotes()[0].comment,
                "Save on Escape"
            );
            window.click(format!("quote-edit-{id}"), cx);
            window.press("secondary-a", cx);
            window.input("Keep this when preparation takes over", cx);
            let state = home.read(cx).state.clone();
            state.update(cx, |state, cx| {
                assert!(state.begin_preparation("empty", cx));
            });
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            let state = home.read(cx).state.clone();
            assert_eq!(
                state.read(cx).current().unwrap().quotes()[0].comment,
                "Keep this when preparation takes over"
            );
            state.update(cx, |state, cx| state.finish_preparation("empty", None, cx));
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.click("quote-list-open", cx);
            window.click(format!("quote-remove-{id}"), cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                home.read(cx)
                    .state
                    .read(cx)
                    .current()
                    .unwrap()
                    .quotes()
                    .len(),
                1
            );
            assert_eq!(
                window
                    .find("quote-list-open")
                    .label()
                    .unwrap()
                    .replace(['\u{2068}', '\u{2069}'], ""),
                "1 quote"
            );
            window.click("quote-clear", cx);
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert!(
                home.read(cx)
                    .state
                    .read(cx)
                    .current()
                    .unwrap()
                    .quotes()
                    .is_empty()
            );
            assert_eq!(home.read(cx).input.read(cx).content(), original);
        });
    }

    #[gpui_kit::test]
    fn quote_toolbar_adds_pointer_selection_and_hides_on_scroll(cx: &mut TestAppContext) {
        use gupi_conversation::history::DisplayMessage;
        let text = "Select this displayed answer to discuss its ordering.";
        let (home, visual) = setup_initialized(
            cx,
            layout::LayoutState::default(),
            false,
            |home, window, cx| {
                home.update(cx, |home, cx| {
                home.state.update(cx, |state, cx| {
                    let session = state.sessions_for_test().get_mut("empty").unwrap();
                    *session.state_for_test() = Some(serde_json::from_value(serde_json::json!({"sessionId":"tools", "isStreaming":false, "isCompacting":false})).unwrap());
                    session.mark_transcript_message_for_test();
                    session.live_for_test().push(DisplayMessage::new("quote-tools".into(), serde_json::json!({"role":"assistant", "stopReason":"stop", "content":[{"type":"text", "text":text}]})));
                    *session.content_revision_for_test() += 1;
                    gupi_conversation::conversation::test_support::notify_session("empty", cx);
                });
                home.sync(false, window, cx);
            });
            },
        );
        visual.run_until_parked();
        let view = visual.update(|window, cx| {
            window.activate_window();
            home.update(cx, |home, cx| {
                let view = &home.views["empty"];
                view.markdown
                    .get(
                        "empty-text-quote-tools",
                        text.into(),
                        view.scroller.downgrade(),
                        cx,
                    )
                    .read(cx)
                    .view
                    .clone()
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            let bounds = view.read(cx).bounds();
            // Native events can all arrive before TextView paints its public
            // selected text. Do not let the helper draw between pointer events.
            let start = point(bounds.left() + px(2.), bounds.center().y);
            let end = point(bounds.left() + px(160.), bounds.center().y);
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: start,
                    pressed_button: None,
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
            window.render_frame(cx);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    position: start,
                    button: MouseButton::Left,
                    click_count: 1,
                    ..Default::default()
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: end,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    position: end,
                    button: MouseButton::Left,
                    click_count: 1,
                    ..Default::default()
                }),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_ne!(window.find("quote-selection-add").disabled(), Some(true));
            window.click("quote-selection-add", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                home.read(cx)
                    .state
                    .read(cx)
                    .current()
                    .unwrap()
                    .quotes()
                    .len(),
                1
            );
            assert!(home.read(cx).input.read(cx).value().is_empty());
            assert!(view.read(cx).selected_text().is_empty());
            assert!(
                home.read(cx).views["empty"]
                    .quote_attachments
                    .read(cx)
                    .floating()
            );
            window.input("Pointer comment", cx);
            window.press("enter", cx);
            let bounds = view.read(cx).bounds();
            window.drag(
                point(bounds.left() + px(2.), bounds.center().y),
                point(bounds.left() + px(160.), bounds.center().y),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                home.read(cx).state.read(cx).current().unwrap().quotes()[0].comment,
                "Pointer comment"
            );
            assert_ne!(window.find("quote-selection-add").disabled(), Some(true));
            window.scroll(
                "quote-selection-add",
                ScrollDelta::Pixels(point(px(0.), px(-20.))),
                cx,
            );
            assert!(window.try_find("quote-selection-add").is_none());
        });
    }

    #[gpui_kit::test]
    fn quote_diff_uses_real_path_and_single_side_lines(cx: &mut TestAppContext) {
        use super::super::messages::diff::DiffBlock;
        use gpui_kit::component::diff::{DiffLineRange, DiffSide};
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.state.update(cx, |state, _| {
                    *state
                        .sessions_for_test()
                        .get_mut("empty")
                        .unwrap()
                        .state_for_test() = Some(
                        serde_json::from_value(serde_json::json!({
                            "sessionId":"diff-fixture","isStreaming":false,"isCompacting":false
                        }))
                        .unwrap(),
                    );
                });
                gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
                let block = DiffBlock::patch(
                    "--- a/file\n+++ b/file\n@@ -1,2 +1,2 @@\n-old\n+new\n context\n",
                    cx,
                );
                let diff = block.state().unwrap();
                diff.update(cx, |state, cx| {
                    state.set_selected_lines(
                        Some(DiffLineRange::new("file", DiffSide::Modified, 1, 2)),
                        cx,
                    )
                });
                let selection =
                    quote::diff_selection(diff, "src/real.rs".into(), quote::FileKind::Staged, cx)
                        .unwrap();
                let capture = home.bind_quote(Ok(selection), cx).unwrap();
                home.quote(&quote::Quote(Some(capture)), window, cx);
                let quotes = home.state.read(cx).current().unwrap().quotes();
                let text = quotes[0].formatted().replace(['\u{2068}', '\u{2069}'], "");
                assert!(text.starts_with("Git · Staged — Content at selection: src/real.rs, modified side, lines 1–2\n```rust\n"), "{text}");
                assert!(text.ends_with("new\ncontext\n```"), "{text}");
                diff.update(cx, |state, cx| {
                    state.set_selected_lines(
                        Some(
                            DiffLineRange::new("file", DiffSide::Original, 1, 1)
                                .with_end_side(DiffSide::Modified),
                        ),
                        cx,
                    )
                });
                assert!(matches!(
                    quote::diff_selection(diff, "src/real.rs".into(), quote::FileKind::Staged, cx),
                    Err(quote::Unavailable::DiffLines)
                ));
                assert!(home.input.read(cx).value().is_empty());
                assert_eq!(home.state.read(cx).current().unwrap().quotes().len(), 1);
            })
        });
    }

    #[gpui_kit::test]
    fn saved_source_ratio_is_applied_before_first_frame(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("state.toml");
        let mut preferences = layout::LayoutState::default();
        preferences.source_split_ratio = 0.55;
        layout::save(&path, &preferences).unwrap();
        let (home, visual) =
            setup_initialized(cx, layout::load(&path), false, |home, window, cx| {
                home.update(cx, |home, cx| {
                    home.show_sidebar = false;
                    home.open_source("/tmp/gupi-first-frame.rs".into(), false, window, cx);
                    home.views
                        .get_mut("empty")
                        .unwrap()
                        .workspace
                        .navigator_hidden = true;
                    home.ensure_panel(Panel::Workspace, window, cx);
                    assert!(home.pane_layout.workspace_docked);
                    assert_eq!(home.pane_layout.ratio(), 0.55);
                    assert_eq!(cx.global::<layout::LayoutState>().source_split_ratio, 0.55);
                });
                // The persisted ratio is installed before Root can paint HomeView's first frame.
            });
        visual.update(|window, cx| {
            let panes = &home.read(cx).pane_layout;
            let expected = (0.55 * panes.work).clamp(
                geometry::CONVERSATION_MIN_REM * f32::from(window.rem_size()),
                panes.work - geometry::WORKSPACE_MIN_REM * f32::from(window.rem_size()),
            );
            assert!((panes.conversation - expected).abs() < 0.01);
            assert!((panes.conversation - 0.4 * panes.work).abs() > 1.);
        });
    }

    #[gpui_kit::test]
    fn temporary_pane_completion_keeps_memory_only_widths(cx: &mut TestAppContext) {
        let preferences = layout::LayoutState::default();
        setup_initialized(cx, preferences, true, |home, window, cx| {
            let before = cx.global::<layout::LayoutState>().clone();
            home.update(cx, |home, cx| {
                home.pane_layout.history_width = 360.;
                home.pane_layout.ratio = 0.65;
                home.pane_drag = Some(Drag {
                    side: Side::History,
                    start_width: 320.,
                });
                home.finish_pane_drag(window, cx);
                assert_eq!(home.pane_layout.history_width, 360.);
                assert!(home.layout_save.is_none());
                home.pane_drag = Some(Drag {
                    side: Side::Source,
                    start_width: 0.,
                });
                home.interrupt_pane_drag(window, cx);
                assert_eq!(home.pane_layout.ratio(), 0.65);
                assert!(home.layout_save.is_none());
            });
            assert_eq!(
                cx.global::<layout::LayoutState>().history_width_rem,
                before.history_width_rem
            );
            assert_eq!(
                cx.global::<layout::LayoutState>().source_split_ratio,
                before.source_split_ratio
            );
        });
    }

    #[gpui_kit::test]
    fn compact_list_preserves_manual_intent_and_selection_returns_to_main(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(1000.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                home.select_workspace_tab(None, window, cx);
                home.open_source("/missing-layout-fixture.rs".into(), true, window, cx);
                assert!(home.workspace_layout.compact);
                assert!(!home.workspace_layout.navigator_visible());
                let before = (
                    home.pane_layout.conversation,
                    home.pane_layout.workspace_width,
                );
                home.toggle_workspace_navigator(window, cx);
                assert!(home.temporary_list());
                assert_eq!(home.workspace_layout.main_width, 0.);
                assert_eq!(
                    home.workspace_layout.navigator_width,
                    home.pane_layout.workspace_width
                );
                home.open_source("/other-layout-fixture.rs".into(), false, window, cx);
                assert!(!home.temporary_list());
                assert!(home.workspace_layout.main_width > 0.);
                assert!(!home.views["empty"].workspace.navigator_hidden);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width
                    )
                );
                home.set_files_navigator_hidden(true, cx);
            })
        });
        visual.simulate_resize(size(px(2200.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.fit_layout(window, cx);
                assert!(!home.workspace_layout.navigator_visible());
                home.toggle_workspace_navigator(window, cx);
                assert!(home.workspace_layout.split);
                let before = (home.pane_layout.conversation, home.pane_layout.ratio());
                let x = f32::from(window.viewport_size().width)
                    - home.workspace_layout.navigator_width
                    - 40.;
                home.drag_pane_to(Side::Right, x, window, cx);
                assert_eq!(
                    before,
                    (home.pane_layout.conversation, home.pane_layout.ratio())
                );
            })
        });
    }
    #[gpui_kit::test]
    fn focus_source_clears_only_files_temporary_list_across_tabs(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.simulate_resize(size(px(1000.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                home.open_source("/missing-layout-fixture.rs".into(), true, window, cx);
                home.toggle_workspace_navigator(window, cx);
                assert!(home.temporary_list());
                home.open_git_changes(window, cx);
                home.toggle_workspace_navigator(window, cx);
                assert!(home.temporary_list());
                home.run_action(&actions::Run::new(actions::Kind::FocusSource), window, cx);
                assert!(home.workspace_layout.main_width > 0.);
                assert!(!home.temporary_list());
            })
        });
        visual.run_until_parked();
        assert!(visual.debug_bounds("source-pane").is_some());
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                assert!(home.source().unwrap().read(cx).contains_focus(window, cx));
                home.select_workspace_tab(Some("workspace:git".into()), window, cx);
                assert!(
                    home.temporary_list(),
                    "the originating Git tab keeps its own temporary list"
                );
            })
        });
    }
    #[gpui_kit::test]
    fn launcher_git_and_files_share_outer_rectangles(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                let before = (
                    home.pane_layout.conversation,
                    home.pane_layout.workspace_width,
                    home.pane_layout.history_docked,
                );
                home.select_workspace_tab(None, window, cx);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width,
                        home.pane_layout.history_docked
                    )
                );
                home.open_source("/missing-layout-fixture.rs".into(), false, window, cx);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width,
                        home.pane_layout.history_docked
                    )
                );
                home.toggle_workspace_navigator(window, cx);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width,
                        home.pane_layout.history_docked,
                    )
                );
                home.open_git_changes(window, cx);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width,
                        home.pane_layout.history_docked
                    )
                );
                home.new_workspace_tab(window, cx);
                assert_eq!(
                    before,
                    (
                        home.pane_layout.conversation,
                        home.pane_layout.workspace_width,
                        home.pane_layout.history_docked
                    )
                );
            })
        });
    }
    #[gpui_kit::test]
    fn shrinking_keeps_composer_focus_and_launcher_outer_divider_is_draggable(
        cx: &mut TestAppContext,
    ) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                home.focus_composer(window, cx);
                home.layout_save.take();
            })
        });
        let ratio = visual.update(|_, cx| home.read(cx).pane_layout.ratio());
        let end = start_drag(&home, visual, Side::Source, 40.);
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.update(|_, cx| {
            assert_ne!(home.read(cx).pane_layout.ratio(), ratio);
            assert!(home.read(cx).layout_save.is_some());
        });
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.focus_composer(window, cx);
                home.layout_save.take();
            })
        });
        visual.simulate_resize(size(px(600.), px(800.)));
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert!(home.read(cx).pane_layout.conversation > 0.);
            assert!(!home.read(cx).pane_layout.visible(Panel::Workspace));
            assert!(
                home.read(cx)
                    .input
                    .read(cx)
                    .focus_handle(cx)
                    .contains_focused(window, cx)
            );
            assert!(home.read(cx).layout_save.is_none());
        });
    }
    #[test]
    fn outer_geometry_is_finite_for_zoom_sidebar_and_tiny_windows() {
        for rem in [10., 14., 24., 40.] {
            for width in [0., 0.5, 100., 700., 1600.] {
                for sidebar in [false, true] {
                    for sidebar_rem in [10., 20., 30.] {
                        let mut preferences = layout::LayoutState::default();
                        preferences.workspace_pinned = true;
                        preferences.sidebar_width_rem = sidebar_rem;
                        preferences.source_split_ratio = 0.65;
                        let mut panes = PaneLayout::default();
                        panes.fit(
                            Request {
                                width,
                                sidebar,
                                rem,
                                workspace_pinned: true,
                                history_pinned: false,
                            },
                            &preferences,
                        );
                        assert!(
                            [
                                panes.left,
                                panes.conversation,
                                panes.workspace_width,
                                panes.body_width,
                                panes.body_origin
                            ]
                            .into_iter()
                            .all(|n| n.is_finite() && n >= 0.)
                        );
                        assert!(panes.conversation + panes.workspace_width <= width + 0.001);
                        assert_eq!(preferences.source_split_ratio, 0.65);
                    }
                }
            }
        }
    }
    #[gpui_kit::test]
    fn single_region_return_and_recovery_preserve_preferences(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.ensure_panel(Panel::Workspace, window, cx);
                home.layout_save.take();
            })
        });
        visual.simulate_resize(size(px(600.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.fit_layout(window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                assert!(!home.pane_layout.workspace_docked);
                assert_eq!(home.pane_layout.conversation, 0.);
                home.layout_save.take();
                home.show_conversation(window, cx);
                assert!(!home.pane_layout.visible(Panel::Workspace));
                assert!(cx.global::<layout::LayoutState>().workspace_pinned);
                assert!(home.layout_save.is_none());
            })
        });
        visual.simulate_resize(size(px(1728.), px(800.)));
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.fit_layout(window, cx);
                assert!(home.pane_layout.workspace_docked);
                assert!(home.layout_save.is_none());
            })
        });
    }
    #[test]
    fn resize_does_not_write_preferences_and_history_can_redock() {
        let mut preferences = layout::LayoutState::default();
        preferences.workspace_pinned = true;
        preferences.history_pinned = true;
        preferences.history_width_rem = 15.;
        let mut panes = PaneLayout::default();
        let mut request = Request {
            width: 1400.,
            sidebar: false,
            rem: 16.,
            workspace_pinned: true,
            history_pinned: true,
        };
        panes.fit(request, &preferences);
        assert!(panes.workspace_docked);
        request.width = 700.;
        panes.fit(request, &preferences);
        assert!(!panes.workspace_docked);
        assert!(panes.history_docked);
        assert_eq!(preferences.source_split_ratio, 0.4);
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
                Side::Right => {
                    f32::from(window.viewport_size().width)
                        - home.read(cx).workspace_layout.navigator_width
                }
                Side::Source => panes.left + panes.conversation,
                Side::History => panes.left + panes.conversation - panes.history_width,
            }
        });
        let start = point(px(x), px(300.));
        let end = point(px(x + delta), px(300.));
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        // Start through the rendered handle before continuing across the window.
        visual.simulate_mouse_move(
            point(px(x + delta / 2.), px(300.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.update(|window, cx| window.draw(cx).clear(cx));
        visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        visual.update(|_, cx| assert!(home.read(cx).pane_drag.is_some()));
        end
    }

    fn queued_sidebar_drag(
        home: &Entity<HomeView>,
        visual: &mut VisualTestContext,
        moves: &[f32],
        escape: bool,
        release_delta: f32,
    ) {
        let before = visual.update(|_, cx| home.read(cx).pane_layout.left);
        let start = point(px(before), px(250.));
        visual.update(|window, cx| {
            let preference = cx.global::<layout::LayoutState>().sidebar_width_rem;
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    position: start,
                    button: MouseButton::Left,
                    ..Default::default()
                }),
                cx,
            );
            for delta in moves {
                window.dispatch_event(
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position: point(px(before + delta), px(250.)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    }),
                    cx,
                );
            }
            assert!(home.read(cx).pane_layout.left > before + 90.);
            assert!(home.read(cx).pane_drag.is_some());
            assert!(home.read(cx).layout_save.is_none());
            assert_eq!(
                cx.global::<layout::LayoutState>().sidebar_width_rem,
                preference
            );
            if escape {
                window.dispatch_event(
                    PlatformInput::KeyDown(KeyDownEvent {
                        keystroke: Keystroke::parse("escape").unwrap(),
                        is_held: false,
                        prefer_character_input: false,
                    }),
                    cx,
                );
                assert!(home.read(cx).pane_drag.is_none());
                assert!(!cx.has_active_drag());
                assert!(home.read(cx).layout_save.is_some());
                // Keep the first save alive while checking stale events cannot enqueue another.
                let _save = home.update(cx, |home, _| home.layout_save.take());
                window.dispatch_event(
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position: point(px(before + 120.), px(250.)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Modifiers::default(),
                    }),
                    cx,
                );
                window.dispatch_event(
                    PlatformInput::MouseUp(MouseUpEvent {
                        position: point(px(before + 120.), px(250.)),
                        button: MouseButton::Left,
                        ..Default::default()
                    }),
                    cx,
                );
                assert!(home.read(cx).layout_save.is_none());
            } else {
                window.dispatch_event(
                    PlatformInput::MouseUp(MouseUpEvent {
                        position: point(px(before + release_delta), px(250.)),
                        button: MouseButton::Left,
                        ..Default::default()
                    }),
                    cx,
                );
                assert!(home.read(cx).layout_save.is_some());
            }
            assert!(home.read(cx).pane_drag.is_none());
            assert!(!cx.has_active_drag());
            let committed_delta = if escape { 100. } else { release_delta };
            assert!((home.read(cx).pane_layout.left - (before + committed_delta)).abs() < 0.01);
            assert_eq!(
                cx.global::<layout::LayoutState>().sidebar_width_rem * f32::from(window.rem_size()),
                home.read(cx).pane_layout.left,
            );
        });
        visual.run_until_parked();
    }

    #[gpui_kit::test]
    fn sidebar_drag_without_intermediate_frame_commits_resize(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        queued_sidebar_drag(&home, visual, &[50., 100.], false, 100.);
    }

    #[gpui_kit::test]
    fn single_move_sidebar_drag_without_intermediate_frame_commits_resize(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        queued_sidebar_drag(&home, visual, &[100.], false, 100.);
    }

    #[gpui_kit::test]
    fn queued_escape_commits_once_before_stale_move_and_release(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        queued_sidebar_drag(&home, visual, &[100.], true, 120.);
    }

    #[gpui_kit::test]
    fn queued_release_commits_final_pointer_position(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        queued_sidebar_drag(&home, visual, &[50., 100.], false, 120.);
    }

    #[gpui_kit::test]
    fn divider_click_without_movement_does_not_own_escape(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let x = visual.update(|_, cx| home.read(cx).pane_layout.left);
        let position = point(px(x), px(300.));
        visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        visual.update(|window, cx| window.draw(cx).clear(cx));
        visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
            let home = home.read(cx);
            assert!(home.pane_drag.is_none());
            assert!(home.layout_save.is_none());
        });
    }

    #[gpui_kit::test]
    fn clamped_drag_owns_escape_without_persisting(cx: &mut TestAppContext) {
        let mut preferences = layout::LayoutState::default();
        preferences.sidebar_width_rem = LEFT_MIN_REM;
        let (home, visual) = setup_with_preferences(cx, preferences);
        let before = visual.update(|_, cx| home.read(cx).pane_layout.left);
        let end = start_drag(&home, visual, Side::Left, -40.);
        visual.simulate_keystrokes("escape");
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.update(|_, cx| {
            let home = home.read(cx);
            assert_eq!(home.pane_layout.left, before);
            assert!(home.pane_drag.is_none());
            assert!(!cx.has_active_drag());
            assert!(home.layout_save.is_none());
        });
    }

    fn navigator_room(panes: &PaneLayout, window: &Window) -> f32 {
        panes.workspace_width
            - geometry::DIVIDER
            - workspace::layout::MAIN_MIN_REM * f32::from(window.rem_size())
    }

    #[gpui_kit::test]
    fn drag_held_at_clamp_keeps_wider_preference(cx: &mut TestAppContext) {
        let mut preferences = layout::LayoutState::default();
        preferences.navigator_width_rem = 200.;
        let (home, visual) = setup_with_preferences(cx, preferences);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
            })
        });
        visual.run_until_parked();
        let (before, preference) = visual.update(|_, cx| {
            (
                home.read(cx).workspace_layout.navigator_width,
                cx.global::<layout::LayoutState>().navigator_width_rem * 16.,
            )
        });
        assert!(before < preference);
        // Opening the workspace pins it; only the drag's own save matters here.
        let _ = visual.update(|_, cx| home.update(cx, |home, _| home.layout_save.take()));
        let end = start_drag(&home, visual, Side::Right, -40.);
        visual.update(|_, cx| cx.global_mut::<layout::LayoutState>().source_split_ratio = 0.58);
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.update(|window, cx| {
            let home = home.read(cx);
            // The navigator stays clamped inside the workspace the new ratio leaves.
            assert_eq!(
                home.workspace_layout.navigator_width,
                before.min(navigator_room(&home.pane_layout, window))
            );
            assert_eq!(home.pane_layout.ratio(), 0.58);
            assert_eq!(
                cx.global::<layout::LayoutState>().navigator_width_rem * 16.,
                preference
            );
            assert!(home.pane_drag.is_none());
            assert!(home.layout_save.is_none());
            assert_eq!(cx.global::<layout::LayoutState>().navigator_width_rem, 200.);
        });
    }

    #[gpui_kit::test]
    fn tab_drag_reflow_ends_tab_without_acquiring_pane_ownership(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.select_workspace_tab(None, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                home.layout_save.take();
            });
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("workspace-add", cx);
        });
        visual.run_until_parked();
        let start = visual.debug_bounds("workspace-tab-files").unwrap().center();
        let end = start + point(px(20.), px(0.));
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        visual.simulate_resize(size(px(1800.), px(800.)));
        visual.run_until_parked();
        visual.update(|_, cx| {
            assert!(!cx.has_active_drag());
            assert!(!home.read(cx).workspace_tab_drag_active());
            assert!(home.read(cx).pane_drag.is_none());
            assert!(home.read(cx).layout_save.is_none());
        });
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    }

    #[gpui_kit::test]
    async fn escape_and_release_commit_dragged_sizes(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Modifiers, MouseButton};
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                home.layout_save.take();
            })
        });
        visual.run_until_parked();
        let committed =
            visual.update(|_, cx| cx.global::<layout::LayoutState>().navigator_width_rem);
        let end = start_drag(&home, visual, Side::Right, 40.);
        visual.update(|_, cx| {
            assert_eq!(
                cx.global::<layout::LayoutState>().navigator_width_rem,
                committed
            )
        });
        let width = visual.update(|_, cx| home.read(cx).workspace_layout.navigator_width);
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
        let save = visual.update(|_, cx| home.update(cx, |home, _| home.layout_save.take()));
        // Events from the finished gesture must not resize or persist again.
        visual.simulate_mouse_move(
            end + point(px(10.), px(0.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.update(|_, cx| {
            let home = home.read(cx);
            assert!(home.pane_drag.is_none());
            assert!(home.layout_save.is_none());
            assert_eq!(home.workspace_layout.navigator_width, width);
        });
        if let Some(save) = save {
            save.await;
        }
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
                assert!(home.read(cx).layout_save.is_some());
                assert_eq!(cx.global::<layout::LayoutState>().source_split_ratio, ratio);
                assert_eq!(home.read(cx).pane_layout.ratio(), ratio);
            });
            let save = visual.update(|_, cx| home.update(cx, |home, _| home.layout_save.take()));
            if escape {
                visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
                visual.update(|_, cx| {
                    assert!(home.read(cx).pane_drag.is_none());
                    assert!(home.read(cx).layout_save.is_none());
                });
            }
            if let Some(save) = save {
                save.await;
            }
            let path = visual
                .update(|_, cx| super::HomeView::layout_directory(cx).unwrap())
                .join("state.toml");
            let saved = layout::load(&path);
            assert!((saved.source_split_ratio - ratio).abs() < 0.0001);
            assert_eq!(
                saved.navigator_width_rem,
                visual.update(|window, _| width / f32::from(window.rem_size()))
            );
        }
        let directory = visual.update(|_, cx| super::HomeView::layout_directory(cx).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[gpui_kit::test]
    fn interrupted_navigator_drag_restores_committed_width_without_saving(cx: &mut TestAppContext) {
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                home.layout_save.take();
            });
        });
        visual.run_until_parked();
        let before = visual.update(|_, cx| home.read(cx).workspace_layout.navigator_width);
        let end = start_drag(&home, visual, Side::Right, -30.);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                assert!(home.workspace_layout.navigator_width > before);
                cx.global_mut::<layout::LayoutState>().source_split_ratio = 0.61;
                home.interrupt_pane_drag(window, cx);
                assert_eq!(
                    home.workspace_layout.navigator_width,
                    before.min(navigator_room(&home.pane_layout, window))
                );
                assert_eq!(home.pane_layout.ratio(), 0.61);
                assert_eq!(
                    cx.global::<layout::LayoutState>().navigator_width_rem
                        * f32::from(window.rem_size()),
                    before
                );
            });
        });
        visual.simulate_mouse_move(
            end + point(px(-10.), px(0.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.update(|window, cx| {
            assert!(home.read(cx).pane_drag.is_none());
            assert_eq!(
                home.read(cx).workspace_layout.navigator_width,
                before.min(navigator_room(&home.read(cx).pane_layout, window))
            );
            assert!(home.read(cx).layout_save.is_none());
        });
    }

    #[gpui_kit::test]
    fn layout_change_interrupts_source_drag_without_saving(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{px, size};
        let (home, visual) = setup(cx);
        visual.update(|window, cx| {
            home.update(cx, |home, cx| {
                home.open_source("/tmp/gupi-resize-fixture.rs".into(), false, window, cx);
                home.ensure_panel(Panel::Workspace, window, cx);
                home.layout_save.take();
            })
        });
        visual.run_until_parked();
        for initial in [0.4, 0.45] {
            visual.update(|_, cx| {
                home.update(cx, |home, cx| {
                    cx.global_mut::<layout::LayoutState>().source_split_ratio = initial;
                    home.pane_layout.invalidate();
                    cx.notify();
                })
            });
            let end = start_drag(&home, visual, Side::Source, 80.);
            visual.simulate_resize(size(px(1800.), px(800.)));
            visual.update(|window, cx| {
                window.draw(cx).clear(cx);
                let home = home.read(cx);
                assert!(home.pane_drag.is_none());
                assert_eq!(home.pane_layout.ratio, initial);
                assert!(home.layout_save.is_none());
                let panes = &home.pane_layout;
                let expected = (initial * panes.work).clamp(
                    24. * f32::from(window.rem_size()),
                    panes.work - 20. * f32::from(window.rem_size()),
                );
                assert!((panes.conversation - expected).abs() < 0.01);
            });
            visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            visual.update(|_, cx| assert!(home.read(cx).layout_save.is_none()));
            visual.simulate_resize(size(px(1728.), px(800.)));
        }
    }
}
