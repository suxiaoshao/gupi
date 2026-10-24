//! Per-session presentation state. Saved comments stay in ConversationState.
use super::*;
use gpui_kit::base::{Popup, SelectableText, StyledExt as _};
use gpui_kit::component::attachment::AttachmentActions;
use gpui_kit::component::input::{
    InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton, Textarea,
};
use gpui_kit::component::{Icon, tooltip::Tooltip};
use gupi_conversation::quotes::QuoteAttachment;

actions!(gupi_quote, [CancelComment, CloseQuotes]);
pub(in crate::home) fn init(cx: &mut App) {
    for context in ["GupiQuoteComment", "GupiQuoteComment > Input"] {
        cx.bind_keys([KeyBinding::new("escape", CancelComment, Some(context))]);
    }
    cx.bind_keys([KeyBinding::new(
        "escape",
        CloseQuotes,
        Some("GupiQuoteAttachment"),
    )]);
}
struct Editing {
    id: String,
    input: Entity<TextareaState>,
    floating: Option<Bounds<Pixels>>,
    bounds: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    _subscriptions: Vec<Subscription>,
}
pub(in crate::home) struct Attachments {
    state: Entity<ConversationState>,
    key: String,
    composer: Entity<TextareaState>,
    trigger_focus: FocusHandle,
    list_focus: FocusHandle,
    list_open: bool,
    pinned: bool,
    hover_suppressed: bool,
    trigger_hovered: bool,
    list_hovered: bool,
    close_task: Option<Task<()>>,
    scroll: ScrollHandle,
    editing: Option<Editing>,
    enabled: bool,
    _subscription: Subscription,
}
impl Attachments {
    pub(in crate::home) fn new(
        state: Entity<ConversationState>,
        key: String,
        composer: Entity<TextareaState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe(&state, |this, _, cx| {
            if this.quotes(cx).is_empty() {
                this.list_open = false;
                this.trigger_hovered = false;
                this.list_hovered = false;
                this.close_task = None;
            }
            if this
                .editing
                .as_ref()
                .is_some_and(|editing| !this.quotes(cx).iter().any(|q| q.id == editing.id))
            {
                this.editing = None;
            }
            cx.notify();
        });
        Self {
            state,
            key,
            composer,
            trigger_focus: cx.focus_handle(),
            list_focus: cx.focus_handle(),
            list_open: false,
            pinned: false,
            hover_suppressed: false,
            trigger_hovered: false,
            list_hovered: false,
            close_task: None,
            scroll: ScrollHandle::new(),
            editing: None,
            enabled: true,
            _subscription: subscription,
        }
    }
    pub(in crate::home) fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled != enabled {
            self.enabled = enabled;
            cx.notify();
        }
    }
    fn quotes<'a>(&self, cx: &'a App) -> &'a [std::sync::Arc<QuoteAttachment>] {
        self.state
            .read(cx)
            .sessions()
            .get(&self.key)
            .map(|s| s.quotes())
            .unwrap_or_default()
    }
    fn editable(&self, cx: &App) -> bool {
        let state = self.state.read(cx);
        self.enabled
            && state.selected().as_ref() == Some(&self.key)
            && state
                .sessions()
                .get(&self.key)
                .is_some_and(|s| s.can_edit_draft() && s.pending_ui().is_empty())
    }
    pub(in crate::home) fn open(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable(cx) {
            return;
        }
        if let Some(editing) = self.editing.as_ref().filter(|edit| edit.id == id) {
            editing.input.focus_handle(cx).focus(window, cx);
            return;
        }
        let Some(quote) = self.quotes(cx).iter().find(|q| q.id == id).cloned() else {
            return;
        };
        let input = cx.new(|cx| {
            let mut input = TextareaState::new(window, cx)
                .auto_grow(1, 4)
                .submit_on_enter(true)
                .placeholder(t(cx, "quote-comment-placeholder"));
            input.set_value(quote.comment.clone(), window, cx);
            let end = quote.comment.len();
            input.set_selected_range(end..end, cx);
            input
        });
        let subscriptions = vec![
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.subscribe_in(&input, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { shift: false, .. }) {
                    this.save(window, cx);
                }
            }),
        ];
        self.editing = Some(Editing {
            id: id.to_owned(),
            input,
            floating: None,
            bounds: Rc::default(),
            _subscriptions: subscriptions,
        });
        self.editing
            .as_ref()
            .unwrap()
            .input
            .focus_handle(cx)
            .focus(window, cx);
        cx.notify();
    }
    /// End an editing session without modifying the saved attachment.
    fn discard(&mut self, cx: &mut Context<Self>) {
        if self.editing.take().is_some() {
            cx.notify();
        }
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editing) = &self.editing else {
            return;
        };
        if !self.editable(cx) {
            return;
        }
        let comment = editing.input.read(cx).value().to_string();
        let id = editing.id.clone();
        let floating = self.floating();
        if self.state.update(cx, |state, cx| {
            state.edit_quote(&self.key, &id, comment, cx)
        }) {
            self.discard(cx);
            if floating || !self.list_open {
                self.composer.focus_handle(cx).focus(window, cx);
            } else {
                self.list_focus.focus(window, cx);
            }
        }
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let floating = self.floating();
        self.discard(cx);
        if floating || !self.list_open {
            self.composer.focus_handle(cx).focus(window, cx);
        } else {
            self.list_focus.focus(window, cx);
        }
        cx.notify();
    }
    pub(in crate::home) fn open_floating(
        &mut self,
        id: &str,
        anchor: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list_open = false;
        self.close_task = None;
        self.open(id, window, cx);
        if let Some(editing) = &mut self.editing {
            editing.floating = Some(anchor);
        }
        cx.notify();
    }
    fn comment_input(&self, cx: &Context<Self>) -> AnyElement {
        let Some(editing) = &self.editing else {
            return div().into_any_element();
        };
        let Some((index, quote)) = self
            .quotes(cx)
            .iter()
            .enumerate()
            .find(|(_, quote)| quote.id == editing.id)
        else {
            return div().into_any_element();
        };
        let mut args = FluentArgs::new();
        args.set("number", (index + 1) as i64);
        args.set("source", quote.short_title.clone());
        let delete_label = t_with_args(cx, "quote-delete-number", &args);
        let id = editing.id.clone();
        v_flex()
            .id("quote-comment-editor")
            // Popup already occludes the inline card. A second occluder would
            // mask the list's hover hitbox while the pointer is inside it.
            .when(editing.floating.is_some(), |card| card.occlude())
            .key_context("GupiQuoteComment")
            .on_action(cx.listener(|this, _: &CancelComment, window, cx| this.cancel(window, cx)))
            .map(|card| {
                if editing.floating.is_some() {
                    card.w_72()
                } else {
                    card.w_full()
                }
            })
            .max_w_full()
            .min_w_0()
            .flex_none()
            .child(
                InputGroup::new("quote-comment-group")
                    .bg(cx.theme().popover)
                    .input(
                        Textarea::new(&editing.input)
                            .aria_label(t(cx, "quote-comment-placeholder"))
                            .small(),
                    )
                    .addon(
                        InputGroupAddon::new("quote-comment-footer")
                            .align(InputGroupAddonAlignment::BlockEnd)
                            .child(
                                InputGroupButton::new("quote-comment-delete")
                                    .icon(IconName::Trash)
                                    .disabled(!self.editable(cx))
                                    .accessibility_label(delete_label.clone())
                                    .tooltip(delete_label)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.remove(&id, window, cx)
                                    })),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .justify_end()
                                    .child(
                                        InputGroupButton::new("quote-comment-cancel")
                                            .secondary()
                                            .outline()
                                            .label(t(cx, "action-cancel"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.cancel(window, cx)
                                            })),
                                    )
                                    .child(
                                        InputGroupButton::new("quote-comment-save")
                                            .primary()
                                            .label(t(cx, "action-save"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save(window, cx)
                                            })),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
    pub(in crate::home) fn floating(&self) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|editing| editing.floating.is_some())
    }
    pub(in crate::home) fn render_floating(&self, cx: &Context<Self>) -> AnyElement {
        let Some(editing) = &self.editing else {
            return div().into_any_element();
        };
        let Some(anchor) = editing.floating else {
            return div().into_any_element();
        };
        let bounds = editing.bounds.clone();
        let child = self.comment_input(cx);
        let owner = cx.weak_entity();
        let input_id = editing.input.entity_id();
        canvas(
            move |_, window, cx| {
                let gap = window.rem_size() * 0.5;
                let viewport = window.viewport_size();
                let mut child = div()
                    .max_w((viewport.width - gap * 2.).max(px(0.)))
                    .child(child)
                    .into_any_element();
                let measured = child.layout_as_root(
                    size(AvailableSpace::MaxContent, AvailableSpace::MinContent),
                    window,
                    cx,
                );
                let origin = point(
                    (anchor.center().x - measured.width / 2.)
                        .clamp(gap, (viewport.width - gap - measured.width).max(gap)),
                    anchor
                        .top()
                        .clamp(gap, (viewport.height - gap - measured.height).max(gap)),
                );
                bounds.set(Some(Bounds::new(origin, measured)));
                child.prepaint_at(origin, window, cx);
                (child, bounds)
            },
            move |_, (mut child, bounds), window, cx| {
                child.paint(window, cx);
                window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                    if phase.capture()
                        && !bounds
                            .get()
                            .is_some_and(|bounds| bounds.contains(&event.position))
                    {
                        let _ = owner.update(cx, |this, cx| {
                            if this
                                .editing
                                .as_ref()
                                .is_some_and(|editing| editing.input.entity_id() == input_id)
                            {
                                this.discard(cx);
                            }
                        });
                    }
                });
            },
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }
    pub(in crate::home) fn dismiss_on_escape(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.list_open || self.editing.is_some() {
            return false;
        }
        // Escape must remain closed while the pointer is still on the pill.
        self.hover_suppressed = self.trigger_hovered;
        self.close_list(window, cx);
        true
    }
    fn close_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.discard(cx);
        self.list_open = false;
        self.pinned = false;
        self.trigger_hovered = false;
        self.list_hovered = false;
        self.close_task = None;
        if self.list_focus.contains_focused(window, cx) {
            self.trigger_focus.focus(window, cx);
        }
        cx.notify();
    }
    fn hover(&mut self, trigger: bool, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if trigger {
            if !hovered {
                self.hover_suppressed = false;
            }
            self.trigger_hovered = hovered;
        } else {
            self.list_hovered = hovered;
        }
        self.close_task = None;
        if hovered
            && !self.hover_suppressed
            && !self.floating()
            && self.enabled
            && !self.quotes(cx).is_empty()
        {
            if !self.list_open {
                self.list_open = true;
                self.pinned = false;
                cx.notify();
            }
        } else if self.list_open && !self.trigger_hovered && !self.list_hovered && !self.pinned {
            self.close_task = Some(cx.spawn_in(window, async move |owner, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let _ = owner.update_in(cx, |this, window, cx| {
                    if this.list_open && !this.trigger_hovered && !this.list_hovered && !this.pinned
                    {
                        this.close_list(window, cx);
                    }
                });
            }));
        }
    }
    fn open_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quotes(cx).is_empty() || !self.enabled {
            return;
        }
        self.list_open = true;
        self.pinned = true;
        self.hover_suppressed = false;
        self.close_task = None;
        self.list_focus.focus(window, cx);
        cx.notify();
    }
    fn remove(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable(cx) {
            return;
        }
        self.state
            .update(cx, |state, cx| state.remove_quote(&self.key, id, cx));
        if self.quotes(cx).is_empty() {
            self.list_open = false;
            self.close_task = None;
        }
        if self.list_open {
            self.list_focus.focus(window, cx);
        } else {
            self.composer.focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable(cx) {
            return;
        }
        self.editing = None;
        let ids: Vec<_> = self
            .quotes(cx)
            .iter()
            .map(|quote| quote.id.clone())
            .collect();
        self.state.update(cx, |state, cx| {
            for id in ids {
                state.remove_quote(&self.key, &id, cx);
            }
        });
        self.list_open = false;
        self.close_task = None;
        self.composer.focus_handle(cx).focus(window, cx);
        cx.notify();
    }
    fn content(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let width = (window.viewport_size().width - window.rem_size() * 2.).max(px(0.));
        let height = (window.viewport_size().height - window.rem_size() * 2.).max(px(0.));
        let editable = self.editable(cx);
        let rows: Vec<_> = self
            .quotes(cx)
            .to_vec()
            .into_iter()
            .enumerate()
            .map(|(index, quote)| {
                let id = quote.id.clone();
                let editing = self
                    .editing
                    .as_ref()
                    .filter(|editing| editing.id == id && editing.floating.is_none());
                let mut args = FluentArgs::new();
                args.set("number", (index + 1) as i64);
                args.set("source", quote.short_title.clone());
                let label = t_with_args(
                    cx,
                    if matches!(quote.selection.origin, Origin::Message { .. }) {
                        "quote-selected-text"
                    } else {
                        "quote-list-source"
                    },
                    &args,
                );
                let text = window
                    .use_keyed_state(
                        SharedString::from(format!("quote-list-raw-{id}")),
                        cx,
                        // A selected line can include its newline and the next line's
                        // indentation. Omit trailing blank preview rows; the frozen snapshot
                        // used for sending and persistence remains untouched.
                        |_, _| SharedString::from(quote.selection.text.trim_end().to_owned()),
                    )
                    .read(cx)
                    .clone();
                let edit_id = id.clone();
                let remove_id = id.clone();
                v_flex()
                    .id(SharedString::from(format!("quote-row-{id}")))
                    .gap_1()
                    .py_2()
                    .min_w_0()
                    .when(index > 0, |row| {
                        row.border_t_1().border_color(cx.theme().border)
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .items_start()
                            .min_w_0()
                            .child(
                                div()
                                    .text_xs()
                                    .line_height(relative(1.25))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{}.", index + 1)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("quote-source-{id}")))
                                            .text_xs()
                                            .line_height(relative(1.25))
                                            .text_color(cx.theme().muted_foreground)
                                            .truncate()
                                            .tooltip({
                                                let title = quote.title.clone();
                                                move |window, cx| {
                                                    Tooltip::new(title.clone()).build(window, cx)
                                                }
                                            })
                                            .child(label),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("quote-preview-{id}")))
                                            .test_support()
                                            .line_clamp(3)
                                            .text_ellipsis()
                                            .flex_none()
                                            .text_sm()
                                            .line_height(relative(1.25))
                                            .min_w_0()
                                            .text_color(cx.theme().foreground)
                                            .child(SelectableText::new(
                                                SharedString::from(format!("quote-text-{id}")),
                                                text,
                                            )),
                                    )
                                    .when_some(editing, |row, _| row.child(self.comment_input(cx)))
                                    .when(editing.is_none() && !quote.comment.is_empty(), |row| {
                                        row.child(
                                            div()
                                                .text_xs()
                                                .line_height(relative(1.25))
                                                .text_color(cx.theme().muted_foreground)
                                                .child(t(cx, "quote-user-comment")),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "quote-comment-{id}"
                                                )))
                                                .test_support()
                                                .text_sm()
                                                .line_height(relative(1.25))
                                                .text_color(cx.theme().foreground)
                                                .child(quote.comment.clone()),
                                        )
                                    }),
                            )
                            .when(editing.is_none(), |row| {
                                row.child(
                                    Button::new(SharedString::from(format!("quote-edit-{id}")))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Pencil)
                                        .disabled(!editable)
                                        .accessibility_label(t_with_args(
                                            cx,
                                            "quote-edit-label",
                                            &args,
                                        ))
                                        .tooltip(t(cx, "quote-edit-comment"))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open(&edit_id, window, cx)
                                        })),
                                )
                                .child(
                                    Button::new(SharedString::from(format!("quote-remove-{id}")))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Trash)
                                        .disabled(!editable)
                                        .accessibility_label(t_with_args(
                                            cx,
                                            "quote-delete-number",
                                            &args,
                                        ))
                                        .tooltip(t_with_args(cx, "quote-delete-number", &args))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.remove(&remove_id, window, cx)
                                        })),
                                )
                            }),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .id("quote-list")
            .key_context("GupiQuoteAttachment")
            .track_focus(&self.list_focus)
            .on_action(cx.listener(|this, _: &CloseQuotes, window, cx| this.close_list(window, cx)))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close_list(window, cx)))
            // This tracks pointer presence, not transient keyboard modality.
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.hover(false, *hovered, window, cx)
            }))
            .w_80()
            .max_w(width)
            .max_h(height.min(window.rem_size() * 24.))
            .min_h_0()
            .bg(cx.theme().popover)
            // InputGroup addons are muted/medium by default. The popup is its
            // own surface: restore normal text here, not on the nested input.
            .text_color(cx.theme().foreground)
            .font_normal()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .shadow_lg()
            .px_3()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(rows)
            .into_any_element()
    }
    pub(in crate::home) fn render_pill(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut args = FluentArgs::new();
        args.set("count", self.quotes(cx).len() as i64);
        let label = t_with_args(cx, "quote-count", &args);
        let editable = self.editable(cx);
        let trigger = h_flex()
            .id("quote-pill")
            .key_context("GupiQuoteAttachment")
            .relative()
            .flex_none()
            .self_start()
            .h_6()
            .pl_2()
            .pr_0p5()
            .gap_1()
            .rounded_full()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .text_sm()
            .text_color(cx.theme().foreground)
            .hover(|style| style.bg(cx.theme().muted))
            // This tracks pointer presence, not transient keyboard modality.
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.hover(true, *hovered, window, cx)
            }))
            .child(Icon::new(IconName::MessageCircle).size_3())
            .child(label.clone())
            // One whole-pill trigger below the independently interactive action,
            // matching Attachment's click-layer/actions composition.
            .child(
                gpui_kit::base::Button::new("quote-list-open")
                    .track_focus(&self.trigger_focus)
                    .accessibility_label(label)
                    .aria_expanded(self.list_open)
                    .absolute()
                    .inset_0()
                    .rounded_full()
                    .border_1()
                    .border_color(cx.theme().ring.opacity(0.))
                    .focus_visible(|style| style.border_color(cx.theme().ring))
                    .on_click(cx.listener(|this, _, window, cx| this.open_list(window, cx))),
            )
            .child(
                AttachmentActions::new().child(
                    Button::new("quote-clear")
                        .ghost()
                        .xsmall()
                        .size_5()
                        .rounded_full()
                        .icon(IconName::X)
                        .disabled(!editable)
                        .accessibility_label(t(cx, "quote-clear"))
                        .tooltip(t(cx, "quote-clear"))
                        .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
                ),
            );
        let mut popup = Popup::new("quote-list-popup", trigger)
            .anchor(Anchor::BottomLeft)
            .offset(window.rem_size() * 0.25);
        if self.list_open {
            popup = popup.content(self.content(window, cx));
        }
        popup.into_any_element()
    }
}
