//! Ephemeral locations captured with a selection, never draft or history data.
use super::*;
use crate::home::messages::markdown::MarkdownState;
use gpui_kit::component::input::{
    RangeDecoration, RangeDecorationCollection, RangeDecorationStyle,
};
use gpui_kit::component::text::RenderedText;
use std::ops::Range;

#[derive(Clone, PartialEq)]
pub(in crate::home) enum Source {
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
    pub fn visible(&self, view: &SessionView, conversation: bool, file: bool) -> bool {
        match self {
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
