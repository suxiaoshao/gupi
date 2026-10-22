//! Shared readonly operation renderer. Retained states change only with their source.
use super::edited_files::ChangeBody;
use super::presentation::code_fence;
use gpui_kit::component::diff::{
    Diff, DiffEvent, DiffFile, DiffInlineUnit, DiffLineRange, DiffMode, DiffState,
};
use gpui_kit::component::text::{TextView, TextViewState, TextViewStyle};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(in crate::home) struct DiffBlock {
    source: Source,
    height_rem: f32,
    reader_collapsed: Cell<bool>,
    state: Option<Entity<DiffState>>,
    fallback: Option<Entity<TextViewState>>,
    gutter_selection: Rc<RefCell<Option<DiffLineRange>>>,
    last_frame: Rc<Cell<u64>>,
    restore_pending: Rc<Cell<bool>>,
    _selection_subscriptions: Vec<Subscription>,
}
enum Source {
    Operation { body: Rc<ChangeBody>, path: String },
    Patch(String),
    Comparison,
}
impl DiffBlock {
    #[cfg(test)]
    pub(super) fn state_id(&self) -> Option<EntityId> {
        self.state.as_ref().map(Entity::entity_id)
    }
    #[cfg(test)]
    pub(super) fn saved_selection(&self) -> Option<DiffLineRange> {
        self.gutter_selection.borrow().clone()
    }

    pub(in crate::home) fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        self.state
            .as_ref()
            .map(|state| state.read(cx).focus_handle(cx))
    }

    #[cfg(test)]
    pub(in crate::home) fn state(&self) -> Option<&Entity<DiffState>> {
        self.state.as_ref()
    }

    pub fn new(body: &Rc<ChangeBody>, path: &str, cx: &mut App) -> Self {
        let (text, files) = match body.as_ref() {
            ChangeBody::Patch { source, files } => (source.as_str(), Some(files.clone())),
            ChangeBody::Written { content } => (
                content.as_str(),
                Some(vec![DiffFile::unchanged(path.to_owned(), content)]),
            ),
            ChangeBody::Text { content } => (content.as_str(), None),
        };
        Self::prepare(
            Source::Operation {
                body: body.clone(),
                path: path.to_owned(),
            },
            text,
            files,
            cx,
        )
    }
    pub fn update(&mut self, body: &Rc<ChangeBody>, path: &str, cx: &mut App) {
        if self.matches(body, path) {
            return;
        }
        if let Source::Operation {
            body: previous,
            path: previous_path,
        } = &self.source
        {
            let equal = previous_path == path
                && match (previous.as_ref(), body.as_ref()) {
                    (ChangeBody::Patch { source: a, .. }, ChangeBody::Patch { source: b, .. }) => {
                        a == b
                    }
                    (ChangeBody::Written { content: a }, ChangeBody::Written { content: b }) => {
                        a == b
                    }
                    (ChangeBody::Text { content: a }, ChangeBody::Text { content: b }) => a == b,
                    _ => false,
                };
            if equal {
                self.source = Source::Operation {
                    body: body.clone(),
                    path: path.to_owned(),
                };
                return;
            }
        }
        let (text, files) = match body.as_ref() {
            ChangeBody::Patch { source, files } => (source.as_str(), Some(files.clone())),
            ChangeBody::Written { content } => (
                content.as_str(),
                Some(vec![DiffFile::unchanged(path.to_owned(), content)]),
            ),
            ChangeBody::Text { content } => (content.as_str(), None),
        };
        if let (Some(state), Some(files)) = (&self.state, files) {
            state.update(cx, |state, cx| state.set_files(files, cx));
            self.source = Source::Operation {
                body: body.clone(),
                path: path.to_owned(),
            };
            self.height_rem = (text.lines().count() as f32 * 1.25 + 3.5).clamp(6., 18.);
            *self.gutter_selection.borrow_mut() = None;
            self.restore_pending.set(false);
        } else {
            *self = Self::new(body, path, cx);
        }
    }
    pub fn patch(source: &str, cx: &mut App) -> Self {
        Self::prepare(
            Source::Patch(source.to_owned()),
            source,
            DiffFile::parse(source)
                .ok()
                .filter(|files| !files.is_empty()),
            cx,
        )
    }
    pub(in crate::home) fn comparison(files: Vec<DiffFile>, cx: &mut App) -> Self {
        Self::prepare(Source::Comparison, "", Some(files), cx)
    }
    fn prepare(identity: Source, source: &str, files: Option<Vec<DiffFile>>, cx: &mut App) -> Self {
        let state =
            files.map(|files| cx.new(|cx| DiffState::new(files, cx).with_mode(DiffMode::Unified)));
        let fallback = state.is_none().then(|| {
            cx.new(|cx| {
                TextViewState::markdown(&format!("{}text\n{source}", code_fence(source)), cx)
            })
        });
        let gutter_selection = Rc::new(RefCell::new(None));
        let mut _selection_subscriptions = Vec::new();
        if let Some(state) = &state {
            let selection = gutter_selection.clone();
            _selection_subscriptions.push(cx.observe(state, move |state, cx| {
                if let Some(range) = state.read(cx).selected_lines() {
                    *selection.borrow_mut() = Some(range);
                }
            }));
            let selection = gutter_selection.clone();
            _selection_subscriptions.push(cx.subscribe(state, move |_, event: &DiffEvent, _| {
                if let DiffEvent::SelectionChanged(Some(range)) = event {
                    *selection.borrow_mut() = Some(range.clone());
                }
            }));
        }
        Self {
            height_rem: (source.lines().count() as f32 * 1.25 + 3.5).clamp(6., 18.),
            reader_collapsed: Cell::new(false),
            gutter_selection,
            last_frame: Rc::default(),
            restore_pending: Rc::default(),
            _selection_subscriptions,
            source: identity,
            state,
            fallback,
        }
    }
    pub fn matches(&self, body: &Rc<ChangeBody>, path: &str) -> bool {
        matches!(&self.source, Source::Operation { body: previous, path: previous_path }
            if Rc::ptr_eq(previous, body) && previous_path == path)
    }
    pub fn matches_patch(&self, source: &str) -> bool {
        matches!(&self.source, Source::Patch(previous) if previous == source)
    }
    /// Preserve gutter selection across virtual unmount without claiming a second
    /// active window selection. Restore only when the original code body still owns focus.
    pub fn render_retained(
        &self,
        frame: Rc<Cell<u64>>,
        viewport: Rc<Cell<Option<Bounds<Pixels>>>>,
    ) -> AnyElement {
        let Some(state) = self.state.as_ref() else {
            return self.render();
        };
        let state = state.clone();
        let selection = self.gutter_selection.clone();
        let last = self.last_frame.clone();
        let pending = self.restore_pending.clone();
        let clicked_pending = pending.clone();
        let cancelled_pending = pending.clone();
        let clicked_selection = selection.clone();
        let cancelled_selection = selection.clone();
        let body_bounds = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let click_bounds = body_bounds.clone();
        div()
            .min_w_0()
            .child(self.render())
            .capture_any_mouse_down(move |event, _, _| {
                if event.button == MouseButton::Left
                    && click_bounds
                        .get()
                        .is_some_and(|bounds| bounds.contains(&event.position))
                {
                    clicked_selection.borrow_mut().take();
                    clicked_pending.set(false);
                }
            })
            .capture_key_down(move |event, _, _| {
                if event.keystroke.key.as_str() == "escape" {
                    cancelled_selection.borrow_mut().take();
                    cancelled_pending.set(false);
                }
            })
            .on_children_prepainted(move |bounds, window: &mut Window, cx: &mut App| {
                body_bounds.set(bounds.first().copied());
                let generation = frame.get();
                let visible =
                    bounds
                        .first()
                        .zip(viewport.get())
                        .is_some_and(|(bounds, viewport)| {
                            bounds.bottom() > viewport.top() && bounds.top() < viewport.bottom()
                        });
                if !visible {
                    return;
                }
                let remounted = generation > last.get().saturating_add(1);
                if remounted {
                    pending.set(true);
                }
                last.set(generation);
                if state.read(cx).selected_lines().is_some() {
                    pending.set(false);
                }
                if pending.get()
                    && state.read(cx).focus_handle(cx).is_focused(window)
                    && let Some(range) = selection.borrow().clone()
                {
                    pending.set(false);
                    let state = state.clone();
                    let pending = pending.clone();
                    let selection = selection.clone();
                    window.defer(cx, move |window, cx| {
                        if selection.borrow().as_ref() != Some(&range) {
                            return;
                        }
                        if state.read(cx).focus_handle(cx).is_focused(window)
                            && state.read(cx).selected_lines().is_none()
                        {
                            state.update(cx, |state, cx| state.set_selected_lines(Some(range), cx));
                        } else if state.read(cx).selected_lines().is_none() {
                            pending.set(true);
                        }
                    });
                }
            })
            .into_any_element()
    }
    /// The workspace shows one file per view and owns its header, so the
    /// component's per-file header (and its collapse control) is hidden.
    pub(in crate::home) fn configure(
        &self,
        split: bool,
        word: bool,
        expanded: bool,
        collapsed: bool,
        cx: &mut App,
    ) {
        self.reader_collapsed.set(collapsed);
        if let Some(state) = &self.state {
            state.update(cx, |state, cx| {
                state.set_mode(
                    if split {
                        DiffMode::Split
                    } else {
                        DiffMode::Unified
                    },
                    cx,
                );
                state.set_inline_unit(word.then_some(DiffInlineUnit::Word), cx);
                state.set_context_lines(if expanded { None } else { Some(3) }, cx);
                let paths: Vec<_> = state.files().iter().map(|f| f.path().to_string()).collect();
                for path in paths {
                    state.set_file_collapsed(&path, collapsed, cx);
                }
            });
        }
    }
    pub(in crate::home) fn copy_patch(&self) -> Option<String> {
        match &self.source {
            Source::Patch(patch) => Some(patch.clone()),
            Source::Comparison => None,
            Source::Operation { body, .. } => match body.as_ref() {
                ChangeBody::Patch { source, .. } => Some(source.clone()),
                _ => None,
            },
        }
    }
    pub(in crate::home) fn copy_content(&self) -> Option<String> {
        match &self.source {
            Source::Operation { body, .. } => match body.as_ref() {
                ChangeBody::Written { content } => Some(content.clone()),
                _ => None,
            },
            _ => None,
        }
    }
    pub(in crate::home) fn render_reader_bounded(&self, wrap: bool) -> AnyElement {
        div()
            .w_full()
            .min_w_0()
            .when(!self.reader_collapsed.get(), |body| {
                body.h(rems(self.height_rem))
            })
            .child(self.render_reader(wrap))
            .into_any_element()
    }
    pub(in crate::home) fn render_reader(&self, wrap: bool) -> AnyElement {
        if self.reader_collapsed.get() {
            return div().into_any_element();
        }
        if let Some(state) = &self.state {
            Diff::new(state)
                .soft_wrap(wrap)
                .header_visible(false)
                .size_full()
                .min_w_0()
                .min_h_0()
                .into_any_element()
        } else {
            div()
                .id(format!(
                    "workspace-diff-{}",
                    self.fallback.as_ref().unwrap().entity_id().as_u64()
                ))
                .size_full()
                .overflow_y_scroll()
                .when(!wrap, |body| body.overflow_x_scroll())
                .child(
                    TextView::new(self.fallback.as_ref().unwrap())
                        .selectable(true)
                        .style(TextViewStyle::default().code_block(if wrap {
                            StyleRefinement::default().whitespace_normal()
                        } else {
                            StyleRefinement::default().whitespace_nowrap()
                        })),
                )
                .into_any_element()
        }
    }
    pub fn render(&self) -> AnyElement {
        if let Some(state) = &self.state {
            Diff::new(state)
                .h(rems(self.height_rem))
                .w_full()
                .min_w_0()
                .into_any_element()
        } else {
            div()
                .id(format!(
                    "diff-fallback-{}",
                    self.fallback.as_ref().unwrap().entity_id().as_u64()
                ))
                .w_full()
                .min_w_0()
                .max_h(rems(self.height_rem))
                .overflow_y_scroll()
                .child(TextView::new(self.fallback.as_ref().unwrap()).selectable(true))
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use gpui_kit::component::{
        Root,
        diff::{DiffLineRange, DiffMode, DiffSide},
    };
    use gpui_kit::test::TestWindowExt as _;

    struct Host {
        first: DiffBlock,
        second: DiffBlock,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("diff-host")
                .test_support()
                .flex()
                .flex_col()
                .size_full()
                .child(div().id("first").test_support().child(self.first.render()))
                .child(
                    div()
                        .id("second")
                        .test_support()
                        .child(self.second.render()),
                )
        }
    }
    fn body(text: &str) -> Rc<ChangeBody> {
        let source = format!("--- same.rs\n+++ same.rs\n@@ -1 +1 @@\n-old\n+{text}\n");
        Rc::new(ChangeBody::Patch {
            files: DiffFile::parse(&source).unwrap(),
            source,
        })
    }
    struct ReaderHost {
        block: DiffBlock,
        wrap: bool,
    }
    impl Render for ReaderHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(220.)).child(
                div()
                    .id("reader-operation")
                    .debug_selector(|| "reader-operation".into())
                    .child(self.block.render_reader_bounded(self.wrap)),
            )
        }
    }
    #[gpui_kit::test]
    fn plaintext_reader_collapses_without_replacing_its_selection_model(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let mut host = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| ReaderHost {
                block: DiffBlock::new(
                    &Rc::new(ChangeBody::Text {
                        content: "long plaintext line ".repeat(50),
                    }),
                    "a.rs",
                    cx,
                ),
                wrap: false,
            });
            host = Some(view.clone());
            Root::new(view, window, cx)
        });
        let host = host.unwrap();
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let height = visual.debug_bounds("reader-operation").unwrap().size.height;
        assert!(height > px(0.));
        let id = visual.update(|_, cx| host.read(cx).block.fallback.as_ref().unwrap().entity_id());
        visual.update(|_, cx| {
            host.update(cx, |host, cx| {
                host.wrap = true;
                host.block.configure(false, false, true, true, cx);
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            visual.debug_bounds("reader-operation").unwrap().size.height,
            px(0.)
        );
        visual.update(|_, cx| {
            host.update(cx, |host, cx| {
                host.block.configure(false, false, true, false, cx);
                assert_eq!(host.block.fallback.as_ref().unwrap().entity_id(), id);
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            visual.debug_bounds("reader-operation").unwrap().size.height,
            height
        );
    }
    #[gpui_kit::test]
    fn second_operation_copies_its_source_and_unchanged_sync_retains_selection(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let mut host = None;
        let second_body = body("second");
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Host {
                first: DiffBlock::new(&body("first"), "same.rs", cx),
                second: DiffBlock::new(&second_body, "same.rs", cx),
            });
            host = Some(view.clone());
            Root::new(view, window, cx)
        });
        let host = host.unwrap();
        visual.update(|window, cx| {
            let second = host.read(cx).second.state.as_ref().unwrap().clone();
            second.update(cx, |state, cx| state.set_mode(DiffMode::Split, cx));
            window.render_frame(cx);
            window
                .within("second")
                .within(("new", 0usize))
                .click(("line", 0usize), cx);
            window.within("second").press("secondary-c", cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("second\n")
            );
            let id = second.entity_id();
            let selection = second.read(cx).selected_lines();
            host.update(cx, |host, cx| {
                let next = second_body.clone();
                if !host.second.matches(&next, "same.rs") {
                    host.second = DiffBlock::new(&next, "same.rs", cx);
                }
                assert_eq!(host.second.state.as_ref().unwrap().entity_id(), id);
            });
            assert_eq!(second.read(cx).selected_lines(), selection);
            let first = host.read(cx).first.state.as_ref().unwrap().clone();
            first.update(cx, |state, cx| {
                state.set_selected_lines(
                    Some(DiffLineRange::new("same.rs", DiffSide::Modified, 1, 1)),
                    cx,
                )
            });
            assert_eq!(first.read(cx).selected_text(cx), "first\n");
            assert_eq!(second.read(cx).selected_text(cx), "second\n");
        });
    }
    #[gpui_kit::test]
    fn malformed_patch_falls_back_and_written_content_is_neutral(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            let bad = DiffBlock::patch("not a patch", cx);
            assert!(bad.state.is_none() && bad.fallback.is_some());
            assert!(bad.matches_patch("not a patch"));
            assert!(!bad.matches_patch("different patch"));
            let body = Rc::new(ChangeBody::Written {
                content: "literal\n".into(),
            });
            let written = DiffBlock::new(&body, "existing.rs", cx);
            assert!(written.matches(&body.clone(), "existing.rs"));
            assert!(!written.matches(&body, "renamed.rs"));
            assert!(!written.matches(&Rc::new(body.as_ref().clone()), "existing.rs"));
            assert!(!written.matches_patch("literal\n"));
            let file = &written.state.as_ref().unwrap().read(cx).files()[0];
            assert_eq!(file.additions(), 0);
            assert_eq!(file.deletions(), 0);
            assert_eq!(
                file.status(),
                gpui_kit::component::diff::DiffFileStatus::Unchanged
            );
        });
    }
}
