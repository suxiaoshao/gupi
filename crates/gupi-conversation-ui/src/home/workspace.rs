use super::*;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tab::{Tab, TabBar};

impl HomeView {
    fn session_view(&self) -> Option<&SessionView> {
        self.shown_key.as_ref().and_then(|key| self.views.get(key))
    }
    pub(super) fn files(&self) -> Option<Entity<files::Files>> {
        self.session_view().map(|view| view.files.clone())
    }
    pub(super) fn source(&self) -> Option<Entity<source::Preview>> {
        self.session_view().and_then(|view| view.source.clone())
    }
    pub(super) fn source_active(&self) -> bool {
        self.session_view().is_some_and(|view| view.source_active)
    }
    pub(super) fn set_source_active(&mut self, active: bool) {
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.source_active = active;
        }
    }
    pub(super) fn open_source(
        &mut self,
        path: PathBuf,
        keyboard: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = self
            .shown_key
            .clone()
            .filter(|key| self.views.contains_key(key))
        else {
            return;
        };
        self.set_source_active(true);
        self.pane_layout.fit_workspace(
            f32::from(window.viewport_size().width),
            self.show_sidebar,
            self.show_history,
            true,
            f32::from(window.rem_size()),
            cx.global::<layout::LayoutState>(),
        );
        let overlay = self.pane_layout.overlay;
        if let Some(source) = self.source().filter(|s| s.read(cx).path == path) {
            if overlay {
                self.show_history = false;
                self.sync(false, window, cx);
            }
            if keyboard {
                source.update(cx, |s, cx| s.focus(window, cx));
            } else if overlay {
                source.update(cx, |s, cx| s.focus_container(window, cx));
            }
            cx.notify();
            return;
        }
        let root = self.state.read(cx).sessions()[&key].info().cwd.clone();
        let preview =
            cx.new(|cx| source::Preview::new(path.clone(), Some(root), keyboard, window, cx));
        if overlay {
            self.show_history = false;
            self.sync(false, window, cx);
            preview.update(cx, |s, cx| s.focus_container(window, cx));
        }
        self.files()
            .unwrap()
            .update(cx, |files, cx| files.set_open_path(Some(path), cx));
        let preview_key = key.clone();
        let subscription = cx.subscribe_in(
            &preview,
            window,
            move |this, _, event: &source::Event, window, cx| {
                if this.shown_key.as_ref() != Some(&preview_key) {
                    return;
                }
                match event {
                    source::Event::Close => this.close_source(window, cx),
                    source::Event::Return => this.return_from_source(window, cx),
                    source::Event::Reveal => {
                        let path = this.source().as_ref().map(|s| s.read(cx).path.clone());
                        this.files_tab = true;
                        this.show_history = true;
                        this.sync(false, window, cx);
                        if let Some(path) = path {
                            this.files()
                                .unwrap()
                                .update(cx, |files, cx| files.reveal(path, window, cx));
                        }
                    }
                }
                cx.notify();
            },
        );
        let view = self.views.get_mut(&key).unwrap();
        view.source_subscription = Some(subscription);
        view.source = Some(preview);
        cx.notify();
    }
    pub(super) fn return_from_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_history
            && self.files_tab
            && !self.pane_layout.overlay
            && !self.pane_layout.single
        {
            self.files()
                .unwrap()
                .update(cx, |files, cx| files.focus_open(window, cx));
        } else {
            self.focus_composer(window, cx);
        }
    }
    pub(super) fn close_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self
            .source()
            .as_ref()
            .is_some_and(|s| s.read(cx).contains_focus(window, cx));
        if focused {
            self.return_from_source(window, cx);
        }
        if let Some(view) = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
        {
            view.source = None;
            view.source_subscription = None;
            view.source_active = false;
            view.files
                .update(cx, |files, cx| files.set_open_path(None, cx));
        }
        cx.notify();
    }
    pub(super) fn close_navigator(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_history = false;
        if let Some(view) = self.shown_key.as_ref().and_then(|key| self.views.get(key)) {
            view.history_canvas
                .update(cx, |canvas, cx| canvas.clear_pointer(cx));
        }
        self.sync(false, window, cx);
        cx.notify();
    }
    pub(super) fn focus_overlay_navigator(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pane_layout.fit_workspace(
            f32::from(window.viewport_size().width),
            self.show_sidebar,
            self.show_history,
            self.source().is_some(),
            f32::from(window.rem_size()),
            cx.global::<layout::LayoutState>(),
        );
        if self.show_history && self.pane_layout.overlay {
            self.focus_navigator(window, cx);
        }
    }
    pub(super) fn focus_navigator(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.files_tab {
            if let Some(files) = self.files() {
                files.update(cx, |files, cx| files.focus(window, cx));
            }
        } else {
            self.history_focus_handle(cx).focus(window, cx);
        }
    }
    pub(super) fn save_source_split(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).is_temporary() {
            return;
        }
        self.source_split_touched = true;
        let ratio = self.pane_layout.ratio();
        let previous = self.source_save.take();
        let directory = Self::layout_directory(cx);
        self.source_save = Some(cx.spawn(async move |_, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let result = cx
                .background_spawn(async move {
                    directory.and_then(|dir| {
                        gupi_settings::source_split::save(&dir.join("source-split.toml"), ratio)
                    })
                })
                .await;
            if let Err(error) = result {
                tracing::warn!(%error,"source split save failed");
            }
        }));
    }
    pub(super) fn render_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let conversation = v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .children(self.find.clone())
            .child(self.render_messages(window, cx));
        let Some(source) = self.source() else {
            return conversation
                .child(self.render_composer(window, cx))
                .into_any_element();
        };
        if self.pane_layout.single {
            let busy = self.state.read(cx).current().is_some_and(|s| s.busy());
            let label = t(
                cx,
                if busy {
                    "files-conversation-busy"
                } else {
                    "files-conversation"
                },
            );
            let tabs = TabBar::new("workspace-tabs")
                .segmented()
                .small()
                .selected_index(usize::from(self.source_active()))
                .child(
                    Tab::new()
                        .label(label)
                        .when(busy, |tab| tab.prefix(Spinner::new().small())),
                )
                .child(Tab::new().label(t(cx, "files-source")))
                .on_click(cx.listener(|this, ix: &usize, window, cx| {
                    this.set_source_active(*ix == 1);
                    if this.source_active() {
                        if let Some(source) = this.source() {
                            source.update(cx, |s, cx| s.focus(window, cx));
                        }
                    } else {
                        this.focus_composer(window, cx);
                    }
                    cx.notify();
                }));
            return v_flex()
                .size_full()
                .min_h_0()
                .min_w_0()
                .child(
                    h_flex()
                        .h_10()
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(tabs),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .child(if self.source_active() {
                            source.into_any_element()
                        } else {
                            conversation.into_any_element()
                        }),
                )
                .child(
                    v_flex()
                        .flex_none()
                        .w_full()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .when(self.source_active(), |view| {
                            view.child(
                                div()
                                    .px_5()
                                    .pt_2()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t(cx, "files-conversation")),
                            )
                        })
                        .child(self.render_composer(window, cx)),
                )
                .into_any_element();
        }
        h_flex()
            .size_full()
            .min_w_0()
            .items_stretch()
            .child(
                div()
                    .relative()
                    .w(px(self.pane_layout.conversation))
                    .flex_none()
                    .h_full()
                    .min_w_0()
                    .child(conversation.child(self.render_composer(window, cx))),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .child(source)
                    .child(self.pane_handle(panes::Side::Source, cx)),
            )
            .into_any_element()
    }
}
