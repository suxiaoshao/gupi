//! Ephemeral locations captured with a selection, never draft or history data.
use super::*;
use crate::home::messages::markdown::MarkdownState;
use gpui_kit::component::diff::DiffLineRange;
use gpui_kit::component::input::{
    RangeDecoration, RangeDecorationCollection, RangeDecorationStyle,
};
use gpui_kit::component::text::RenderedText;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

/// A live DiffBlock owns this data identity. Presentation updates never advance it.
pub(in crate::home) struct DiffOwner {
    state: WeakEntity<DiffState>,
    revision: u64,
    active: Rc<RefCell<Option<DiffLineRange>>>,
}
impl DiffOwner {
    pub fn new(state: &Entity<DiffState>, active: Rc<RefCell<Option<DiffLineRange>>>) -> Self {
        Self {
            state: state.downgrade(),
            revision: 0,
            active,
        }
    }
    pub fn invalidate(&mut self) {
        self.revision += 1;
        self.active.borrow_mut().take();
    }
    pub fn capture(
        &self,
        owner: WeakEntity<Self>,
        path: PathBuf,
        kind: FileKind,
        cx: &App,
    ) -> LocatedSelection {
        let Some(state) = self.state.upgrade() else {
            return (Err(Unavailable::Selection), None);
        };
        let selection = diff_selection(&state, path, kind, cx);
        let source = selection.as_ref().ok().and_then(|_| self.source(owner, cx));
        (selection, source)
    }
    pub fn source(&self, owner: WeakEntity<Self>, cx: &App) -> Option<Source> {
        let range = self
            .state
            .read_with(cx, |state, _| state.selected_lines())
            .ok()??;
        if range.side() != range.end_side() {
            return None;
        }
        let range = DiffLineRange::new(
            range.path().clone(),
            range.side(),
            range.start().min(range.end()),
            range.start().max(range.end()),
        );
        Some(Source::Diff(DiffSource {
            owner,
            revision: self.revision,
            range,
        }))
    }
}

#[derive(Clone)]
pub(in crate::home) struct DiffSource {
    owner: WeakEntity<DiffOwner>,
    revision: u64,
    range: DiffLineRange,
}
impl PartialEq for DiffSource {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.revision == other.revision && self.range == other.range
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::home) enum Source {
    Diff(DiffSource),
    Message {
        state: WeakEntity<MarkdownState>,
        revision: u64,
        rendered: RenderedText,
        range: Range<usize>,
    },
    File {
        state: WeakEntity<source::Preview>,
        revision: u64,
        range: Range<usize>,
    },
}

impl Source {
    pub fn valid(&self, cx: &App) -> bool {
        match self {
            Self::Diff(source) => source
                .owner
                .read_with(cx, |owner, _| {
                    owner.revision == source.revision && owner.state.upgrade().is_some()
                })
                .unwrap_or(false),
            Self::Message {
                state,
                revision,
                rendered,
                ..
            } => state
                .read_with(cx, |state, cx| state.quote_current(*revision, rendered, cx))
                .unwrap_or(false),
            Self::File {
                state, revision, ..
            } => state
                .read_with(cx, |state, cx| state.quote_editor(*revision, cx).is_some())
                .unwrap_or(false),
        }
    }
    pub fn visible(
        &self,
        view: &SessionView,
        conversation: bool,
        file: bool,
        workspace: bool,
    ) -> bool {
        match self {
            // The actual renderer paints only while this live source is mounted.
            Self::Diff(_) => workspace,
            Self::Message { .. } => conversation,
            Self::File { state, .. } => {
                file && view
                    .source
                    .as_ref()
                    .is_some_and(|preview| preview.entity_id() == state.entity_id())
            }
        }
    }
}

pub(in crate::home) struct Active {
    source: Source,
    decorations: Option<RangeDecorationCollection>,
}
impl Active {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            decorations: None,
        }
    }
    pub fn paint(&mut self, visible: bool, cx: &mut App) {
        match &self.source {
            Source::Diff(source) => {
                let _ = source.owner.update(cx, |owner, cx| {
                    let next = (visible && owner.revision == source.revision)
                        .then(|| source.range.clone());
                    if *owner.active.borrow() != next {
                        *owner.active.borrow_mut() = next;
                        let _ = owner.state.update(cx, |_, cx| cx.notify());
                    }
                });
            }
            Source::Message {
                state,
                revision,
                rendered,
                range,
            } => {
                let _ = state.update(cx, |state, cx| {
                    state.set_quote_highlight(
                        visible.then(|| (*revision, rendered.clone(), range.clone())),
                        cx,
                    );
                });
            }
            Source::File {
                state,
                revision,
                range,
            } => {
                let editor = visible
                    .then(|| {
                        state
                            .read_with(cx, |state, cx| state.quote_editor(*revision, cx))
                            .ok()
                            .flatten()
                    })
                    .flatten();
                if let Some(editor) = editor {
                    if self.decorations.is_none() {
                        let collection = editor.update(cx, |editor, cx| {
                            editor.create_range_decorations_collection(vec![], cx)
                        });
                        let _ = state.update(cx, |state, _| {
                            state.quote_decoration = Some(collection.clone())
                        });
                        self.decorations = Some(collection);
                    }
                    self.decorations.as_ref().unwrap().set(
                        vec![
                            RangeDecoration::new(range.clone())
                                .with_style(RangeDecorationStyle::Fill)
                                .with_color(cx.theme().warning.opacity(0.2)),
                        ],
                        cx,
                    );
                } else if let Some(collection) = self.decorations.take() {
                    collection.clear(cx);
                    collection.dispose(cx);
                    let _ = state.update(cx, |state, _| state.quote_decoration = None);
                }
            }
        }
    }
    pub fn clear(&mut self, cx: &mut App) {
        self.paint(false, cx);
    }
}
