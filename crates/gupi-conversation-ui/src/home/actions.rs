use super::*;
use gupi_settings::commands as menus;
pub use gupi_settings::commands::Kind;
pub use gupi_settings::commands::Run;

actions!(
    gupi,
    [
        ConfirmExtension,
        CancelExtension,
        CloseNavigator,
        ReturnFromSource
    ]
);

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", CloseNavigator, Some("GupiNavigatorOverlay")),
        KeyBinding::new("escape", ReturnFromSource, Some("SourcePreview")),
        KeyBinding::new("enter", ConfirmExtension, Some("GupiExtension")),
        KeyBinding::new("escape", CancelExtension, Some("GupiExtension")),
    ]);
}
impl HomeView {
    pub fn action_enabled(&self, kind: Kind, cx: &App) -> bool {
        if kind.temporary_only() {
            return false;
        }
        let state = self.state.read(cx);
        let s = state.current();
        if state.is_temporary()
            && matches!(
                kind,
                Kind::QuickOpen
                    | Kind::Sidebar
                    | Kind::History
                    | Kind::OpenHistory
                    | Kind::ProjectFiles
                    | Kind::CloseSource
                    | Kind::FocusSource
                    | Kind::ShowConversation
                    | Kind::Scan
                    | Kind::Clone
                    | Kind::Export
                    | Kind::Close
                    | Kind::Reveal
                    | Kind::CopyPath
                    | Kind::Reconnect
            )
        {
            return false;
        }
        match kind {
            Kind::TemporaryActions
            | Kind::PasteAnswer
            | Kind::CopyTemporaryAnswer
            | Kind::RevealWorkspace
            | Kind::HideTemporary
            | Kind::TrashTemporary
            | Kind::TemporarySession(_) => false,
            Kind::Palette | Kind::QuickOpen | Kind::Settings | Kind::ShowMain | Kind::Quit => true,
            Kind::New | Kind::Sidebar => !state.draining(),
            Kind::Scan => !state.draining() && !state.scanning(),
            Kind::FocusInput | Kind::History | Kind::OpenHistory | Kind::ProjectFiles => {
                s.is_some()
            }
            Kind::CloseSource | Kind::FocusSource => self.source.is_some(),
            Kind::ShowConversation => {
                self.source.is_some() && self.pane_layout.single && self.source_active
            }
            Kind::Export => state
                .selected()
                .as_ref()
                .is_some_and(|key| state.can_export(key, cx)),
            Kind::Clone => state
                .selected()
                .as_ref()
                .is_some_and(|key| state.can_clone(key, cx)),
            Kind::CopyLastAnswer => s.and_then(|s| s.last_assistant_text()).is_some(),
            Kind::SessionInfo | Kind::Find => s.is_some(),
            Kind::Compact => state
                .selected()
                .as_ref()
                .is_some_and(|key| state.can_compact(key, cx)),
            Kind::Reconnect => state
                .selected()
                .as_ref()
                .is_some_and(|key| state.can_reconnect(key, cx)),
            Kind::Rename => {
                state
                    .selected()
                    .as_ref()
                    .is_some_and(|key| state.can_rename(key, cx))
                    && s.is_some_and(|s| !s.info().path.as_os_str().is_empty())
            }
            Kind::Delete => state
                .selected()
                .as_ref()
                .is_some_and(|key| state.can_delete(key)),
            Kind::Reveal | Kind::CopyPath => {
                s.is_some_and(|s| !s.info().path.as_os_str().is_empty())
            }
            Kind::Stop => {
                state.is_temporary() && s.is_none_or(|s| !s.busy())
                    || s.is_some_and(|s| {
                        s.busy() && !s.stopping() && (!s.is_command_running() || s.is_compacting())
                    })
            }
            Kind::Close => s.is_some_and(|s| s.has_instance() && !s.settings_busy()),
            Kind::Model => s.is_some_and(|s| !s.settings_busy()),
        }
    }
    pub fn run_action(&mut self, action: &Run, window: &mut Window, cx: &mut Context<Self>) {
        if action.0.temporary_only() {
            cx.propagate();
            return;
        }
        if action.0 == Kind::Stop
            && self.find.is_some()
            && !window.has_active_dialog(cx)
            && !self.has_image_preview(cx)
        {
            self.close_find(true, window, cx);
            return;
        }
        if self.has_image_preview(cx) {
            return;
        }
        if matches!(action.0, Kind::Palette | Kind::QuickOpen) && self.action_enabled(action.0, cx)
        {
            self.open_palette(action.0 == Kind::QuickOpen, window, cx);
            return;
        }
        if self.state.read(cx).is_temporary()
            && action.0 == Kind::Stop
            && !window.has_active_dialog(cx)
            && !self.state.read(cx).current().is_some_and(|s| s.busy())
        {
            crate::host::hide(window, cx);
            return;
        }
        if window.has_active_dialog(cx) || !self.action_enabled(action.0, cx) {
            return;
        }
        let key = self.state.read(cx).selected().clone();
        match action.0 {
            Kind::ShowConversation => {
                self.source_active = false;
                self.focus_composer(window, cx);
            }
            Kind::CloseSource => self.close_source(window, cx),
            Kind::FocusSource => {
                self.source_active = true;
                if let Some(source) = self.source.clone() {
                    source.update(cx, |s, cx| s.focus(window, cx));
                }
            }
            Kind::Find => {
                if let Some(source) = self.source.as_ref().filter(|s| {
                    (self.pane_layout.single && self.source_active)
                        || s.read(cx).contains_focus(window, cx)
                }) {
                    source.update(cx, |source, cx| source.find(window, cx));
                } else {
                    self.open_find(window, cx);
                }
            }
            Kind::New => self.new_conversation(window, cx),
            Kind::Settings => window.dispatch_action(Box::new(menus::ShowSettings), cx),
            Kind::ShowMain => window.dispatch_action(Box::new(menus::ShowMainWindow), cx),
            Kind::Quit => window.dispatch_action(Box::new(menus::Quit), cx),
            Kind::Sidebar => {
                self.show_sidebar = !self.show_sidebar;
                if !self.show_sidebar {
                    self.focus_composer(window, cx);
                }
            }
            Kind::History => self.toggle_history(window, cx),
            Kind::ProjectFiles => {
                self.files_tab = true;
                self.show_history = true;
                self.sync(false, window, cx);
                self.files.update(cx, |files, cx| files.focus(window, cx));
            }
            Kind::OpenHistory => {
                if !self.show_history || self.files_tab {
                    self.toggle_history(window, cx);
                }
            }
            Kind::CopyLastAnswer => {
                if let Some(text) = self
                    .state
                    .read(cx)
                    .current()
                    .and_then(|s| s.last_assistant_text())
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            Kind::SessionInfo => {
                if let Some(key) = key {
                    self.open_session_info(key, window, cx);
                }
            }
            Kind::Scan => self.state.update(cx, |s, cx| s.scan(cx)),
            Kind::FocusInput => self.focus_composer(window, cx),
            Kind::Model => {
                if let Some(view) = key.as_ref().and_then(|key| self.views.get(key)) {
                    view.model_picker
                        .update(cx, |picker, cx| picker.open(window, cx));
                }
            }
            Kind::Rename => {
                if let Some(key) = key {
                    self.rename_dialog(key, window, cx);
                }
            }
            Kind::Reveal | Kind::CopyPath => {
                if let Some(s) = self.state.read(cx).current() {
                    if action.0 == Kind::Reveal {
                        cx.reveal_path(&s.info().path);
                    } else {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            s.info().path.to_string_lossy().into_owned(),
                        ));
                    }
                }
            }
            kind => {
                if let Some(key) = key {
                    self.state.update(cx, |s, cx| match kind {
                        Kind::Reconnect => s.reconnect(&key, cx),
                        Kind::Export => s.export_html(&key, cx),
                        Kind::Clone => s.clone_session(&key, cx),
                        Kind::Compact => s.compact(&key, cx),
                        Kind::Stop => s.abort(&key, cx),
                        Kind::Close => s.close(&key, cx),
                        Kind::Delete => s.delete(&key, cx),
                        _ => {}
                    });
                }
            }
        }
        cx.notify();
    }
    pub fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Selection notifications are deferred. Resolve the selected page before
        // an explicit focus request so it cannot target the previous editor.
        if &self.shown_key != self.state.read(cx).selected() {
            self.sync(false, window, cx);
        }
        if let Some(s) = self.state.read(cx).current() {
            if let Some(pending) = s.pending_ui().front() {
                match &pending.request.method {
                    pi_rpc::protocol::UiMethod::Input { .. } => self
                        .extension_line
                        .update(cx, |input, cx| input.focus(window, cx)),
                    pi_rpc::protocol::UiMethod::Editor { .. } => self
                        .extension_input
                        .update(cx, |input, cx| input.focus(window, cx)),
                    pi_rpc::protocol::UiMethod::Select { .. } => {
                        if let Some(selection) = pending.selection.clone() {
                            selection.update(cx, |state, cx| state.focus_current_item(window, cx));
                        }
                    }
                    _ => self.extension_focus.focus(window, cx),
                }
                return;
            }
            self.input.update(cx, |input, cx| input.focus(window, cx));
        } else {
            self.focus_handle.focus(window, cx);
        }
    }
    pub(super) fn toggle_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_history = self.files_tab || !self.show_history;
        self.files_tab = false;
        self.sync(true, window, cx);
        if !self.show_history
            && let Some(view) = self.shown_key.as_ref().and_then(|key| self.views.get(key))
        {
            view.history_canvas
                .update(cx, |canvas, cx| canvas.clear_pointer(cx));
        }
        if self.show_history && self.history_view != HistoryView::Tree {
            self.history_list.update(cx, |list, cx| {
                if let Some(last) = list.delegate().rows.len().checked_sub(1) {
                    list.scroll_to_item(
                        gpui_kit::component::IndexPath::new(last),
                        ScrollStrategy::Bottom,
                        window,
                        cx,
                    );
                }
            });
        }
        if self.show_history {
            self.focus_navigator(window, cx);
        }
        cx.notify();
    }
}
