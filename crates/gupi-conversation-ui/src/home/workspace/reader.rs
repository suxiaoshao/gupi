//! Shared reading policy for recorded and worktree changes; source data stays in adapters.
use super::*;
use gpui_kit::component::menu::DropdownMenu;
use messages::diff::DiffBlock;

const SPLIT_MIN_REM: f32 = 48.;
/// Added and removed line counts in the theme's success and danger colors.
pub(super) fn line_stats(added: usize, removed: usize, cx: &App) -> Div {
    h_flex()
        .flex_none()
        .gap_1()
        .text_xs()
        .child(
            div()
                .text_color(cx.theme().success)
                .child(format!("+{added}")),
        )
        .child(
            div()
                .text_color(cx.theme().danger)
                .child(format!("−{removed}")),
        )
}
#[derive(Clone, Copy)]
pub(super) struct Options {
    pub split: bool,
    pub wrap: bool,
    pub word: bool,
    pub expanded: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            split: false,
            wrap: false,
            word: true,
            expanded: false,
        }
    }
}
impl Options {
    pub fn narrow(width: f32, rem: f32) -> bool {
        width < SPLIT_MIN_REM * rem
    }
    pub fn apply(&self, block: &DiffBlock, narrow: bool, collapsed: bool, cx: &mut App) {
        block.configure(
            self.split && !narrow,
            self.word,
            self.expanded,
            collapsed,
            cx,
        );
    }
}
#[derive(Clone, Copy)]
pub(super) enum Command {
    Unified,
    Split,
    Wrap,
    Word,
    ToggleFile,
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
    pub(super) fn reader_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let (previous, next) = self.reader_neighbors();
        let mut row = h_flex().flex_none().gap_1();
        for (forward, enabled, key, icon) in [
            (
                false,
                previous.is_some(),
                "diff-previous-file",
                IconName::ArrowUp,
            ),
            (true, next.is_some(), "diff-next-file", IconName::ArrowDown),
        ] {
            row = row.child(
                Button::new(key)
                    .ghost()
                    .small()
                    .icon(icon)
                    .disabled(!enabled)
                    .tooltip(t(cx, key))
                    .accessibility_label(t(cx, key))
                    .on_click(cx.listener(move |home, _, window, cx| {
                        let paths = home.reader_neighbors();
                        if let Some(path) = if forward { paths.1 } else { paths.0 } {
                            home.select_reader_file(path, window, cx);
                        }
                    })),
            );
        }
        let external = self.reader_command_available(Command::External, cx);
        let owner = cx.weak_entity();
        let actions_owner = owner.clone();
        let narrow = Options::narrow(
            self.workspace_layout.main_width,
            f32::from(cx.theme().font_size),
        );
        let options_label = t(
            cx,
            if narrow {
                "diff-reading-options-narrow"
            } else {
                "diff-reading-options"
            },
        );
        row.child(
            Button::new("diff-external")
                .ghost()
                .small()
                .icon(IconName::ExternalLink)
                .disabled(!external)
                .tooltip(t(
                    cx,
                    if external {
                        "diff-external"
                    } else {
                        "diff-unavailable-file"
                    },
                ))
                .accessibility_label(t(cx, "diff-external"))
                .on_click(cx.listener(|home, _, window, cx| {
                    home.reader_command(Command::External, window, cx)
                })),
        )
        .child(
            Button::new("diff-reading-options")
                .ghost()
                .small()
                .icon(IconName::SlidersHorizontal)
                .tooltip(options_label.clone())
                .accessibility_label(options_label)
                .dropdown_menu(move |menu, _, cx| {
                    let Some(home) = owner.upgrade() else {
                        return menu;
                    };
                    let entity = home.clone();
                    let home = home.read(cx);
                    let options = home.reader_options().unwrap_or_default();
                    let narrow = Options::narrow(
                        home.workspace_layout.main_width,
                        f32::from(cx.theme().font_size),
                    );
                    let split = options.split && !narrow;
                    menu.label(t(cx, "diff-view"))
                        .item(home.reader_menu_item(
                            Command::Unified,
                            "diff-unified",
                            !split,
                            &entity,
                            cx,
                        ))
                        .item(home.reader_menu_item(
                            Command::Split,
                            "diff-split",
                            split,
                            &entity,
                            cx,
                        ))
                        .separator()
                        .label(t(cx, "diff-display"))
                        .item(home.reader_menu_item(
                            Command::Word,
                            "diff-word",
                            options.word,
                            &entity,
                            cx,
                        ))
                        .item(home.reader_menu_item(
                            Command::Wrap,
                            "diff-wrap",
                            options.wrap,
                            &entity,
                            cx,
                        ))
                        .item(home.reader_menu_item(
                            Command::Context,
                            "diff-context",
                            options.expanded,
                            &entity,
                            cx,
                        ))
                }),
        )
        .child(
            Button::new("diff-more")
                .ghost()
                .small()
                .icon(IconName::Ellipsis)
                .tooltip(t(cx, "diff-actions"))
                .accessibility_label(t(cx, "diff-actions"))
                .dropdown_menu(move |menu, _, cx| {
                    let Some(home) = actions_owner.upgrade() else {
                        return menu;
                    };
                    let entity = home.clone();
                    let home = home.read(cx);
                    menu.item(home.reader_menu_item(
                        Command::CopyPath,
                        "diff-copy-path",
                        false,
                        &entity,
                        cx,
                    ))
                    .when(home.active_git().is_none(), |menu| {
                        menu.item(home.reader_menu_item(
                            Command::CopyPatch,
                            "diff-copy-patch",
                            false,
                            &entity,
                            cx,
                        ))
                    })
                    .item(home.reader_menu_item(
                        Command::CopyContent,
                        "diff-copy-content",
                        false,
                        &entity,
                        cx,
                    ))
                    .separator()
                    .item(home.reader_menu_item(
                        Command::External,
                        "diff-external",
                        false,
                        &entity,
                        cx,
                    ))
                }),
        )
        .into_any_element()
    }
    pub(super) fn reader_menu_item(
        &self,
        command: Command,
        key: &str,
        checked: bool,
        owner: &Entity<Self>,
        cx: &App,
    ) -> PopupMenuItem {
        let owner = owner.downgrade();
        PopupMenuItem::new(t(cx, key))
            .checked(checked)
            .disabled(!self.reader_command_available(command, cx))
            .on_click(move |_, window, cx| {
                let _ = owner.update(cx, |home, cx| home.reader_command(command, window, cx));
            })
    }
    pub(super) fn reader_fold_button(&self, cx: &mut Context<Self>) -> Button {
        let enabled = self.reader_can_fold();
        let collapsed = enabled && self.reader_collapsed();
        let label = t(
            cx,
            if collapsed {
                "diff-expand-file"
            } else {
                "diff-collapse-file"
            },
        );
        Button::new("diff-toggle-file")
            .ghost()
            .small()
            .icon(if collapsed {
                IconName::ChevronRight
            } else {
                IconName::ChevronDown
            })
            .disabled(!enabled)
            .tooltip(label.clone())
            .accessibility_label(label)
            .on_click(cx.listener(|home, _, window, cx| {
                home.reader_command(Command::ToggleFile, window, cx)
            }))
    }
    fn reader_can_fold(&self) -> bool {
        self.active_git().is_some_and(|tab| tab.can_fold())
            || self.active_changes().is_some_and(|tab| tab.can_fold())
    }
    fn reader_selected(&self) -> Option<&String> {
        self.active_git()
            .and_then(|t| t.selected.as_ref())
            .or_else(|| self.active_changes().and_then(|t| t.selected.as_ref()))
    }
    fn reader_collapsed(&self) -> bool {
        self.reader_selected().is_some_and(|id| {
            self.active_git().is_some_and(|t| t.collapsed.contains(id))
                || self
                    .active_changes()
                    .is_some_and(|t| t.collapsed.contains(id))
        })
    }
    fn reader_neighbors(&self) -> (Option<String>, Option<String>) {
        if let Some(tab) = self.active_git() {
            return tab.neighbors();
        }
        let (paths, selected): (Vec<_>, _) = if let Some(tab) = self.active_changes() {
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
        let narrow = Options::narrow(
            self.workspace_layout.main_width,
            f32::from(cx.theme().font_size),
        );
        if let Some(view) = self.shown_key.as_ref().and_then(|k| self.views.get_mut(k)) {
            if let Some(tab) = &view.workspace.git
                && let Some((path, body)) = &tab.body
                && let Some(block) = &body.block
            {
                tab.options
                    .apply(block, narrow, tab.collapsed.contains(path), cx);
            }
            for tab in &view.workspace.changes {
                for (path, body) in &tab.states {
                    let options = tab.options;
                    let collapsed = tab.collapsed.contains(path);
                    if let Some(block) = &body.combined {
                        options.apply(block, narrow, collapsed, cx);
                    }
                    for block in body.operations.values() {
                        options.apply(block, narrow, collapsed, cx);
                    }
                }
            }
        }
    }
}

impl HomeView {
    fn reader_copy(&self, command: Command) -> Option<String> {
        if let Some(tab) = self.active_git() {
            let entry = tab.selected_entry()?;
            if matches!(command, Command::CopyPath) {
                return entry.path().to_str().map(str::to_owned);
            }
            let loaded = tab.current_body()?;
            match command {
                Command::CopyPatch => None,
                Command::CopyContent => loaded.copy_content().map(str::to_owned),
                _ => None,
            }
        } else {
            let tab = self.active_changes()?;
            match command {
                Command::CopyPath => tab.selected.clone(),
                Command::CopyPatch => tab.combined_block()?.copy_patch(),
                Command::CopyContent => tab.combined_block()?.copy_content(),
                _ => None,
            }
        }
    }
    fn reader_external_path(&self, cx: &App) -> Option<PathBuf> {
        if let Some(tab) = self.active_git() {
            return tab.external_path();
        }
        let tab = self.active_changes()?;
        if !tab.in_scope {
            return None;
        }
        let path = tab.selected.as_ref()?;
        if path.contains('\0') {
            return None;
        }
        let session = self.state.read(cx).current()?;
        Some(session.info().cwd.join(path))
    }
    pub(super) fn reader_command_available(&self, command: Command, cx: &App) -> bool {
        match command {
            Command::ToggleFile => self.reader_can_fold(),
            Command::Split => !Options::narrow(
                self.workspace_layout.main_width,
                f32::from(cx.theme().font_size),
            ),
            Command::External => self.reader_external_path(cx).is_some(),
            Command::CopyPath | Command::CopyPatch | Command::CopyContent => {
                self.reader_copy(command).is_some()
            }
            Command::Context => {
                self.active_git().is_some_and(|t| {
                    t.current_body()
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
                if let Some(path) = self.reader_external_path(cx) {
                    resource::open_external(path, window, cx);
                }
            } else if let Some(text) = self.reader_copy(command) {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            return;
        }
        let (options, collapsed, selected) = if let Some(tab) = self.git_mut() {
            (&mut tab.options, &mut tab.collapsed, &tab.selected)
        } else if let Some(view) = self.shown_key.as_ref().and_then(|k| self.views.get_mut(k))
            && let Some(tab) = view
                .workspace
                .changes
                .iter_mut()
                .find(|t| Some(&t.id) == view.workspace.active.as_ref())
        {
            (&mut tab.options, &mut tab.collapsed, &tab.selected)
        } else {
            return;
        };
        match command {
            Command::Unified => options.split = false,
            Command::Split => options.split = true,
            Command::Wrap => options.wrap = !options.wrap,
            Command::Word => options.word = !options.word,
            Command::Context => options.expanded = !options.expanded,
            Command::ToggleFile => {
                if let Some(selected) = selected
                    && !collapsed.remove(selected)
                {
                    collapsed.insert(selected.clone());
                }
            }
            _ => {}
        }
        self.apply_reader_options(cx);
        cx.notify();
        let _ = window;
    }
}
