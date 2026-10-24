//! One-shot displayed selections become session-owned draft attachments.
use super::*;
pub(super) mod attachments;
pub(super) mod toolbar;
use fluent_bundle::FluentArgs;
use gpui_kit::component::diff::{DiffSide, DiffState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gupi_settings::i18n::t_with_args;

use gupi_conversation::quotes::QuoteAttachment;
pub(crate) use gupi_conversation::quotes::{FileKind, Origin, Selection};
#[derive(Clone, PartialEq)]
pub(crate) struct Snapshot {
    key: String,
    binding: u64,
    selection: Selection,
}
#[derive(Clone, PartialEq, Action)]
#[action(namespace = gupi, no_json)]
pub(crate) struct Quote(pub Option<Snapshot>);
#[derive(Clone, Copy, Debug)]
pub(crate) enum Unavailable {
    Selection,
    DiffLines,
    Composer,
}
impl Unavailable {
    pub fn menu_label(self, cx: &App) -> String {
        let mut args = FluentArgs::new();
        args.set(
            "reason",
            match self {
                Self::Selection => "selection",
                Self::DiffLines => "diff",
                Self::Composer => "composer",
            },
        );
        t_with_args(cx, "quote-disabled-menu", &args)
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Selection => "quote-select-text",
            Self::DiffLines => "quote-select-diff-lines",
            Self::Composer => "quote-composer-unavailable",
        }
    }
}
pub(crate) type Capture = Result<Snapshot, Unavailable>;

fn display_path(path: &str) -> String {
    path.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

fn attachment(selection: &Selection, cwd: &std::path::Path, cx: &App) -> QuoteAttachment {
    let mut args = FluentArgs::new();
    let title = match &selection.origin {
        Origin::Message { role } => {
            args.set("role", role.as_str());
            t_with_args(cx, "quote-message-title", &args)
        }
        Origin::File {
            path,
            start,
            end,
            kind,
            original,
            ..
        } => {
            let path = path.strip_prefix(cwd).unwrap_or(path).to_string_lossy();
            let path = display_path(&path);
            args.set("path", path);
            args.set("start", *start as i64);
            args.set("end", *end as i64);
            args.set("side", if *original { "original" } else { "modified" });
            t_with_args(
                cx,
                match kind {
                    FileKind::Source => "quote-file-title",
                    FileKind::Turn => "quote-turn-title",
                    FileKind::Staged => "quote-staged-title",
                    FileKind::Unstaged => "quote-unstaged-title",
                    FileKind::Untracked => "quote-untracked-title",
                },
                &args,
            )
        }
    };
    let short_title = match &selection.origin {
        Origin::Message { role } => {
            let mut args = FluentArgs::new();
            args.set("role", role.as_str());
            t_with_args(cx, "quote-message-short", &args)
        }
        Origin::File {
            path,
            start,
            end,
            kind,
            original,
            ..
        } => {
            let mut args = FluentArgs::new();
            args.set(
                "name",
                display_path(
                    &path
                        .file_name()
                        .unwrap_or(path.as_os_str())
                        .to_string_lossy(),
                ),
            );
            args.set("start", *start as i64);
            args.set("end", *end as i64);
            args.set("side", if *original { "original" } else { "modified" });
            t_with_args(
                cx,
                if *kind == FileKind::Source {
                    "quote-file-short"
                } else {
                    "quote-diff-short"
                },
                &args,
            )
        }
    };
    QuoteAttachment::new(
        selection.clone(),
        title,
        short_title,
        t(cx, "quote-comment-prefix"),
    )
}
impl HomeView {
    pub(crate) fn quote_allowed(&self, cx: &App) -> bool {
        let state = self.state.read(cx);
        let Some(session) = state.current() else {
            return false;
        };
        self.shown_key.as_ref() == state.selected().as_ref()
            && session.can_edit_draft()
            && session.pending_ui().is_empty()
            && self
                .shown_key
                .as_ref()
                .and_then(|key| self.views.get(key))
                .is_some_and(|v| {
                    v.preview
                        .as_deref()
                        .is_none_or(|id| session.history().on_current_path(id))
                })
    }
    pub(crate) fn bind_quote(
        &self,
        selection: Result<Selection, Unavailable>,
        cx: &App,
    ) -> Capture {
        if !self.quote_allowed(cx) {
            return Err(Unavailable::Composer);
        }
        let selection = selection?;
        if selection.text.is_empty() {
            return Err(Unavailable::Selection);
        }
        let state = self.state.read(cx);
        let session = state.current().unwrap();
        Ok(Snapshot {
            key: self.shown_key.clone().unwrap(),
            binding: session.binding(),
            selection,
        })
    }
    pub(crate) fn capture_quote(&self, window: &Window, cx: &App) -> Capture {
        let view = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get(key))
            .ok_or(Unavailable::Selection)?;
        let selection = if let Some(source) = view
            .source
            .as_ref()
            .filter(|source| source.read(cx).contains_focus(window, cx))
        {
            source.read(cx).quote_selection(cx)
        } else if let Some(selection) = self.workspace_quote(window, cx) {
            selection
        } else {
            view.markdown
                .quote_selection(window, cx)
                .ok_or(Unavailable::Selection)
        };
        self.bind_quote(selection, cx)
    }
    pub(crate) fn quote(&mut self, action: &Quote, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(snapshot) = &action.0 {
            self.add_quote_snapshot(snapshot, window, cx);
        }
    }
    fn quote_snapshot_current(&self, snapshot: &Snapshot, cx: &App) -> bool {
        self.quote_allowed(cx)
            && self.state.read(cx).selected().as_ref() == Some(&snapshot.key)
            && self
                .state
                .read(cx)
                .sessions()
                .get(&snapshot.key)
                .is_some_and(|s| s.binding() == snapshot.binding)
    }
    fn add_quote_snapshot(
        &mut self,
        snapshot: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if !self.quote_snapshot_current(snapshot, cx) {
            return None;
        }
        let quote = attachment(
            &snapshot.selection,
            &self.state.read(cx).sessions()[&snapshot.key].info().cwd,
            cx,
        );
        let id = quote.id.clone();
        let added = self
            .state
            .update(cx, |state, cx| state.add_quote(&snapshot.key, quote, cx));
        if !added {
            return None;
        }
        self.show_conversation(window, cx);
        if let Some(view) = self.views.get(&snapshot.key) {
            let session = &self.state.read(cx).sessions()[&snapshot.key];
            // All quote IDs share one visible slot, at the first quote's place.
            let index = session
                .attachment_order()
                .iter()
                .take_while(|id| !session.quotes().iter().any(|quote| &quote.id == *id))
                .filter(|id| {
                    session
                        .attachments()
                        .iter()
                        .any(|attachment| &attachment.id == *id)
                })
                .count();
            view.attachment_scroll.scroll_to_item(index);
        }
        Some(id)
    }
}

pub(crate) fn capture(
    owner: &WeakEntity<HomeView>,
    key: &str,
    selection: Result<Selection, Unavailable>,
    cx: &App,
) -> Capture {
    owner
        .read_with(cx, |home, cx| {
            if home.shown_key.as_deref() != Some(key) {
                return Err(Unavailable::Composer);
            }
            home.bind_quote(selection, cx)
        })
        .unwrap_or(Err(Unavailable::Composer))
}
pub(crate) fn menu(
    menu: PopupMenu,
    owner: WeakEntity<HomeView>,
    capture: Capture,
    text: String,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let reason = capture.as_ref().err().copied();
    let action = Quote(capture.ok());
    menu.item(
        PopupMenuItem::new(t(cx, "menu-copy"))
            .disabled(text.is_empty())
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new(t(cx, "quote-in-composer"))
            .disabled(reason.is_some())
            .on_click(move |_, window, cx| {
                let action = action.clone();
                let owner = owner.clone();
                window.defer(cx, move |window, cx| {
                    let _ = owner.update(cx, |home, cx| home.quote(&action, window, cx));
                });
            }),
    )
    .when_some(reason, |menu, reason| menu.label(t(cx, reason.key())))
}
pub(crate) fn diff_selection(
    state: &Entity<DiffState>,
    path: PathBuf,
    kind: FileKind,
    cx: &App,
) -> Result<Selection, Unavailable> {
    let state = state.read(cx);
    let text = state.selected_text(cx);
    if text.is_empty() {
        return Err(Unavailable::Selection);
    }
    let range = state
        .selected_lines()
        .filter(|r| r.side() == r.end_side())
        .ok_or(Unavailable::DiffLines)?;
    let language = files::directory::language(&path).to_owned();
    Ok(Selection {
        text,
        origin: Origin::File {
            path,
            start: range.start().min(range.end()),
            end: range.start().max(range.end()),
            language,
            kind,
            original: range.side() == DiffSide::Original,
        },
    })
}
pub(crate) fn diff_body(
    body: AnyElement,
    state: Option<&Entity<DiffState>>,
    path: PathBuf,
    kind: FileKind,
    owner: WeakEntity<HomeView>,
    key: String,
    fill: bool,
) -> AnyElement {
    let Some(state) = state.cloned() else {
        return body;
    };
    let events = toolbar::region(
        toolbar::Target::Diff {
            state: state.downgrade(),
            path: path.clone(),
            kind,
        },
        owner.clone(),
        key.clone(),
    );
    div()
        .relative()
        .id(format!("quote-diff-{}", state.entity_id().as_u64()))
        .w_full()
        .when(fill, |body| body.size_full())
        .min_h_0()
        .min_w_0()
        .child(body)
        .child(events)
        .context_menu(move |menu, window, cx| {
            let text = state.read(cx).selected_text(cx);
            self::menu(
                menu,
                owner.clone(),
                capture(
                    &owner,
                    &key,
                    diff_selection(&state, path.clone(), kind, cx),
                    cx,
                ),
                text,
                window,
                cx,
            )
        })
        .into_any_element()
}
