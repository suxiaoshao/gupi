//! Shared reading policy for recorded and worktree changes; source data stays in adapters.
use super::*;
use gpui_kit::component::menu::DropdownMenu;
use messages::diff::DiffBlock;

const SPLIT_MIN_REM: f32 = 48.;
#[derive(Clone, Copy)]
pub(super) struct Options {
    pub split: bool,
    pub wrap: bool,
    pub word: bool,
    pub collapsed: bool,
    pub expanded: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            split: false,
            wrap: false,
            word: true,
            collapsed: false,
            expanded: false,
        }
    }
}
impl Options {
    pub fn narrow(width: f32, rem: f32) -> bool {
        width < SPLIT_MIN_REM * rem
    }
    pub fn apply(&self, block: &DiffBlock, narrow: bool, cx: &mut App) {
        block.configure(
            self.split && !narrow,
            self.word,
            self.expanded,
            self.collapsed,
            cx,
        );
    }
}
#[derive(Clone, Copy)]
pub(super) enum Command {
    Mode,
    Wrap,
    Word,
    Collapse,
    Expand,
    ExpandFile,
    Context,
    CopyPath,
    CopyPatch,
    CopyContent,
    External,
}
impl HomeView {
    pub(super) fn reader_options(&self) -> Option<Options> {
        self.active_git()
            .map(|t| t.options)
            .or_else(|| self.active_changes().map(|t| t.options))
    }
    pub(super) fn reader_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let options = self.reader_options().unwrap_or_default();
        let owner = cx.weak_entity();
        let narrow = Options::narrow(self.pane_layout.main_width, f32::from(cx.theme().font_size));
        let (previous, next) = self.reader_neighbors();
        let mut row = h_flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border);
        for (forward, enabled, key, icon) in [
            (
                false,
                previous.is_some(),
                "diff-previous-file",
                IconName::ChevronLeft,
            ),
            (
                true,
                next.is_some(),
                "diff-next-file",
                IconName::ChevronRight,
            ),
        ] {
            row = row.child(
                Button::new(key)
                    .ghost()
                    .small()
                    .icon(icon)
                    .disabled(!enabled)
                    .tooltip(t(cx, key))
                    .accessibility_label(t(cx, key))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let paths = this.reader_neighbors();
                        if let Some(path) = if forward { paths.1 } else { paths.0 } {
                            this.select_reader_file(path, window, cx);
                        }
                    })),
            );
        }
        row = row
            .child(
                Button::new("diff-mode")
                    .ghost()
                    .small()
                    .selected(options.split)
                    .label(t(
                        cx,
                        if options.split && narrow {
                            "diff-unified-narrow"
                        } else if options.split {
                            "diff-split"
                        } else {
                            "diff-unified"
                        },
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reader_command(Command::Mode, window, cx)
                    })),
            )
            .child(
                Button::new("diff-wrap")
                    .ghost()
                    .small()
                    .selected(options.wrap)
                    .label(t(cx, "diff-wrap"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reader_command(Command::Wrap, window, cx)
                    })),
            )
            .child(
                Button::new("diff-more")
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .accessibility_label(t(cx, "diff-actions"))
                    .dropdown_menu(move |mut menu, _, cx| {
                        let Some(home) = owner.upgrade() else {
                            return menu;
                        };
                        let this = home.read(cx);
                        let options = this.reader_options().unwrap_or_default();
                        let entries = [
                            (Command::Word, "diff-word", options.word),
                            (Command::Context, "diff-context", options.expanded),
                            (Command::Collapse, "diff-collapse-all", false),
                            (Command::Expand, "diff-expand-all", false),
                            (Command::CopyPath, "diff-copy-path", false),
                            (Command::CopyPatch, "diff-copy-patch", false),
                            (Command::CopyContent, "diff-copy-content", false),
                            (Command::External, "diff-external", false),
                        ]
                        .map(|(command, key, checked)| {
                            (
                                command,
                                key,
                                checked,
                                this.reader_command_available(command, cx),
                            )
                        });
                        for (command, key, checked, enabled) in entries {
                            let owner = owner.clone();
                            menu = menu.item(
                                PopupMenuItem::new(t(cx, key))
                                    .checked(checked)
                                    .disabled(!enabled)
                                    .on_click(move |_, window, cx| {
                                        let _ = owner.update(cx, |home, cx| {
                                            home.reader_command(command, window, cx)
                                        });
                                    }),
                            );
                        }
                        menu
                    }),
            );
        row.into_any_element()
    }
    fn reader_neighbors(&self) -> (Option<String>, Option<String>) {
        let (paths, selected): (Vec<_>, _) = if let Some(tab) = self.active_git() {
            (
                tab.rows.iter().map(|r| r.path.clone()).collect(),
                tab.selected.as_ref(),
            )
        } else if let Some(tab) = self.active_changes() {
            (
                tab.visible_files()
                    .iter()
                    .map(|f| f.path().to_owned())
                    .collect(),
                tab.selected.as_ref(),
            )
        } else {
            return (None, None);
        };
        let Some(ix) = paths.iter().position(|p| Some(p) == selected) else {
            return (None, None);
        };
        (
            ix.checked_sub(1).and_then(|i| paths.get(i).cloned()),
            paths.get(ix + 1).cloned(),
        )
    }
    pub(super) fn select_reader_file(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_git().is_some() {
            self.select_git_file(path, window, cx);
        } else {
            self.select_change_file(path, window, cx);
        }
    }
    pub(super) fn apply_reader_options(&mut self, cx: &mut Context<Self>) {
        let narrow = Options::narrow(self.pane_layout.main_width, f32::from(cx.theme().font_size));
        if let Some(view) = self.shown_key.as_ref().and_then(|k| self.views.get_mut(k)) {
            if let Some(tab) = &view.workspace.git {
                for (path, body) in &tab.cache {
                    if let Some(block) = &body.block {
                        let mut options = tab.options;
                        if tab.unfolded.contains(path) {
                            options.collapsed = false;
                        }
                        options.apply(block, narrow, cx);
                    }
                }
            }
            for tab in &view.workspace.changes {
                for (path, body) in &tab.states {
                    let mut options = tab.options;
                    if tab.unfolded.contains(path) {
                        options.collapsed = false;
                    }
                    if let Some(block) = &body.combined {
                        options.apply(block, narrow, cx);
                    }
                    for block in body.operations.values() {
                        options.apply(block, narrow, cx);
                    }
                }
            }
        }
    }
}

impl HomeView {
    fn reader_copy(&self, command: Command, cx: &App) -> Option<String> {
        if let Some(tab) = self.active_git() {
            let entry = tab.selected_entry()?;
            if matches!(command, Command::CopyPath) {
                return Some(entry.path().display().to_string());
            }
            if matches!(command, Command::External) {
                return tab.external_path().map(|p| p.display().to_string());
            }
            let loaded = tab.selected.as_ref().and_then(|p| tab.cache.get(p))?;
            match command {
                Command::CopyPatch => loaded.content.patch().map(str::to_owned),
                Command::CopyContent => {
                    (if entry.kind() == gupi_resources::git::ChangeKind::Deleted {
                        loaded.content.original()
                    } else {
                        loaded.content.modified()
                    })
                    .map(str::to_owned)
                }
                Command::External => tab.external_path().map(|p| p.display().to_string()),
                _ => None,
            }
        } else {
            let tab = self.active_changes()?;
            match command {
                Command::CopyPath => tab.selected.clone(),
                Command::CopyPatch => tab.combined_block()?.copy_patch(),
                Command::CopyContent => tab.combined_block()?.copy_content(),
                Command::External => tab.external.as_ref().map(|p| p.display().to_string()),
                _ => {
                    let _ = cx;
                    None
                }
            }
        }
    }
    pub(super) fn reader_command_available(&self, command: Command, cx: &App) -> bool {
        match command {
            Command::CopyPath | Command::CopyPatch | Command::CopyContent | Command::External => {
                self.reader_copy(command, cx).is_some()
            }
            Command::Context => {
                self.active_git().is_some_and(|t| {
                    t.selected
                        .as_ref()
                        .and_then(|p| t.cache.get(p))
                        .is_some_and(|b| b.content.original().is_some())
                }) || self
                    .active_changes()
                    .and_then(|t| t.combined_block())
                    .and_then(|b| b.copy_content())
                    .is_some()
            }
            _ => true,
        }
    }
    pub(super) fn reader_command(
        &mut self,
        command: Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.reader_command_available(command, cx) {
            return;
        }
        if matches!(
            command,
            Command::CopyPath | Command::CopyPatch | Command::CopyContent | Command::External
        ) {
            if matches!(command, Command::External) {
                if let Some(path) = self.active_git().and_then(|t| t.external_path()) {
                    cx.open_with_system(&path);
                } else if let Some(path) = self.active_changes().and_then(|t| t.external.as_ref()) {
                    cx.open_with_system(path);
                }
            } else if let Some(text) = self.reader_copy(command, cx) {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            return;
        }
        let (options, unfolded, selected) = if let Some(tab) = self.git_mut() {
            (&mut tab.options, &mut tab.unfolded, &tab.selected)
        } else if let Some(view) = self.shown_key.as_ref().and_then(|k| self.views.get_mut(k))
            && let Some(tab) = view
                .workspace
                .changes
                .iter_mut()
                .find(|t| Some(&t.id) == view.workspace.active.as_ref())
        {
            (&mut tab.options, &mut tab.unfolded, &tab.selected)
        } else {
            return;
        };
        match command {
            Command::Mode => options.split = !options.split,
            Command::Wrap => options.wrap = !options.wrap,
            Command::Word => options.word = !options.word,
            Command::Context => options.expanded = !options.expanded,
            Command::Collapse => {
                options.collapsed = true;
                unfolded.clear();
            }
            Command::Expand => {
                options.collapsed = false;
                unfolded.clear();
            }
            Command::ExpandFile => {
                if let Some(selected) = selected {
                    unfolded.insert(selected.clone());
                }
            }
            _ => {}
        }
        self.apply_reader_options(cx);
        cx.notify();
        let _ = window;
    }
}
