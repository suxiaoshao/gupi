//! One-shot displayed selections; the normal composer owns edits and undo.
use super::*;
use fluent_bundle::FluentArgs;
use gpui_kit::component::diff::{DiffSide, DiffState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gupi_settings::i18n::t_with_args;

#[derive(Clone, PartialEq)]
pub(crate) enum Origin {
    Message {
        role: String,
    },
    File {
        path: PathBuf,
        start: usize,
        end: usize,
        language: String,
        kind: FileKind,
        original: bool,
    },
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FileKind {
    Source,
    Turn,
    Staged,
    Unstaged,
    Untracked,
}
#[derive(Clone, PartialEq)]
pub(crate) struct Selection {
    pub origin: Origin,
    pub text: String,
}
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

impl Selection {
    fn format(&self, cwd: &std::path::Path, cx: &App) -> String {
        let mut args = FluentArgs::new();
        let title = match &self.origin {
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
                let path: String = path
                    .chars()
                    .flat_map(|c| {
                        if c.is_control() {
                            c.escape_default().collect::<Vec<_>>()
                        } else {
                            vec![c]
                        }
                    })
                    .collect();
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
        match &self.origin {
            Origin::Message { .. } => format!(
                "{title}\n{}\n\n",
                self.text
                    .split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Origin::File { language, .. } => {
                let fence = messages::code_fence(&self.text);
                let language = if matches!(language.as_str(), "text" | "plaintext") {
                    ""
                } else {
                    language
                };
                format!(
                    "{title}\n{fence}{language}\n{}{separator}{fence}\n\n",
                    self.text,
                    separator = if self.text.ends_with('\n') { "" } else { "\n" }
                )
            }
        }
    }
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
        let Some(snapshot) = &action.0 else {
            return;
        };
        if !self.quote_allowed(cx) {
            return;
        }
        let state = self.state.read(cx);
        if state.selected().as_ref() != Some(&snapshot.key)
            || state
                .sessions()
                .get(&snapshot.key)
                .is_none_or(|s| s.binding() != snapshot.binding)
        {
            return;
        }
        let text = snapshot
            .selection
            .format(&state.sessions()[&snapshot.key].info().cwd, cx);
        let inserted = self.input.update(cx, |input, cx| {
            if input.marked_text_range(window, cx).is_some() {
                return false;
            }
            let value = input.value();
            let separator = if value.is_empty() || value.ends_with("\n\n") {
                ""
            } else if value.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let appended = format!("{separator}{text}");
            let end = value.len();
            input.set_selected_range(end..end, cx);
            input.insert(appended, window, cx);
            true
        });
        if inserted {
            self.show_conversation(window, cx);
            self.focus_composer(window, cx);
        }
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
    div()
        .id(format!("quote-diff-{}", state.entity_id().as_u64()))
        .w_full()
        .when(fill, |body| body.size_full())
        .min_h_0()
        .min_w_0()
        .child(body)
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
