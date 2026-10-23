//! Per-session presentation state. Saved comments stay in ConversationState.
use super::*;
use gpui_kit::component::Icon;
use gpui_kit::component::attachment::{
    Attachment as AttachmentView, AttachmentContent, AttachmentMedia,
};
use gpui_kit::component::input::Textarea;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{Selectable, StyledExt as _};
use gupi_conversation::quotes::QuoteAttachment;

actions!(gupi_quote, [SaveComment, CancelComment]);
pub(in crate::home) fn init(cx: &mut App) {
    for context in ["GupiQuoteComment", "GupiQuoteComment > Input"] {
        cx.bind_keys([
            KeyBinding::new("secondary-enter", SaveComment, Some(context)),
            KeyBinding::new("escape", CancelComment, Some(context)),
        ]);
    }
}
// Popover's public trigger contract carries its open state. AttachmentView
// remains the card; this wrapper only forwards that state and composition.
#[derive(IntoElement)]
struct Trigger {
    id: SharedString,
    body: AnyElement,
    open: bool,
}
impl Selectable for Trigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.open
    }
}
impl RenderOnce for Trigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().id(self.id).aria_expanded(self.open).child(self.body)
    }
}
struct Editing {
    id: String,
    input: Entity<TextareaState>,
    raw: Entity<TextareaState>,
    _subscription: Subscription,
}
pub(in crate::home) struct Cards {
    state: Entity<ConversationState>,
    key: String,
    composer: Entity<TextareaState>,
    focus: HashMap<String, FocusHandle>,
    editing: Option<Editing>,
    enabled: bool,
    _subscription: Subscription,
    _composer_subscription: Subscription,
}
impl Cards {
    pub(in crate::home) fn new(
        state: Entity<ConversationState>,
        key: String,
        composer: Entity<TextareaState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe(&state, |this, _, cx| {
            let ids: Vec<_> = this.quotes(cx).iter().map(|q| q.id.clone()).collect();
            this.focus.retain(|id, _| ids.contains(id));
            if !this.editable(cx)
                || this
                    .editing
                    .as_ref()
                    .is_some_and(|editing| !this.quotes(cx).iter().any(|q| q.id == editing.id))
            {
                this.editing = None;
            }
            cx.notify();
        });
        let composer_subscription = cx.subscribe(&composer, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) && this.editing.take().is_some() {
                cx.notify();
            }
        });
        Self {
            state,
            key,
            composer,
            focus: HashMap::new(),
            editing: None,
            enabled: true,
            _subscription: subscription,
            _composer_subscription: composer_subscription,
        }
    }
    pub(in crate::home) fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled != enabled {
            self.enabled = enabled;
            if !enabled {
                self.editing = None;
            }
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
                .auto_grow(2, 6)
                .placeholder(t(cx, "quote-comment-placeholder"));
            input.set_value(quote.comment.clone(), window, cx);
            input
        });
        let raw = cx.new(|cx| {
            let mut raw = TextareaState::new(window, cx).auto_grow(1, 6);
            raw.set_value(quote.selection.text.clone(), window, cx);
            raw
        });
        let subscription = cx.observe(&input, |_, _, cx| cx.notify());
        self.editing = Some(Editing {
            id: id.to_owned(),
            input,
            raw,
            _subscription: subscription,
        });
        self.editing
            .as_ref()
            .unwrap()
            .input
            .focus_handle(cx)
            .focus(window, cx);
        cx.notify();
    }
    fn close(&mut self, restore: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editing) = self.editing.take() {
            if restore && let Some(focus) = self.focus.get(&editing.id) {
                focus.focus(window, cx);
            }
            cx.notify();
        }
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editable(cx)
            && let Some(editing) = &self.editing
        {
            let id = editing.id.clone();
            let comment = editing.input.read(cx).value().to_string();
            self.state.update(cx, |state, cx| {
                state.edit_quote(&self.key, &id, comment, cx)
            });
        }
        self.close(true, window, cx);
    }
    fn remove(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable(cx) {
            return;
        }
        let quotes = self.quotes(cx);
        let next = quotes
            .iter()
            .position(|q| q.id == id)
            .and_then(|ix| {
                quotes
                    .get(ix + 1)
                    .or_else(|| ix.checked_sub(1).and_then(|ix| quotes.get(ix)))
            })
            .map(|q| q.id.clone());
        self.editing = None;
        self.state
            .update(cx, |state, cx| state.remove_quote(&self.key, id, cx));
        if let Some(focus) = next.and_then(|id| self.focus.get(&id)) {
            focus.focus(window, cx);
        } else {
            self.composer.focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }
    fn content(&self, quote: &QuoteAttachment, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(editing) = &self.editing else {
            return div().into_any_element();
        };
        let id = quote.id.clone();
        let height =
            (window.viewport_size().height - window.rem_size() * 2.).max(window.rem_size());
        let width = (window.viewport_size().width - window.rem_size()).max(window.rem_size());
        div()
            .v_flex()
            .gap_2()
            .w_80()
            .max_w(width)
            .max_h(height.min(window.rem_size() * 20.))
            .min_h_0()
            .key_context("GupiQuoteComment")
            .on_action(cx.listener(|this, _: &SaveComment, window, cx| this.save(window, cx)))
            .on_action(
                cx.listener(|this, _: &CancelComment, window, cx| this.close(true, window, cx)),
            )
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(quote.title.clone()),
            )
            .child(
                Textarea::new(&editing.raw)
                    .readonly(true)
                    .appearance(false)
                    .min_h_0()
                    .max_h_32()
                    .when(
                        matches!(quote.selection.origin, Origin::File { .. }),
                        |view| view.font_family(cx.theme().mono_font_family.clone()),
                    ),
            )
            .child(
                Textarea::new(&editing.input)
                    .aria_label(t(cx, "quote-comment-placeholder"))
                    .small()
                    .flex_none(),
            )
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .flex_none()
                    .child(
                        Button::new("delete-quote")
                            .ghost()
                            .danger()
                            .small()
                            .label(t(cx, "quote-delete"))
                            .on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.remove(&id, window, cx)
                                }),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("cancel-quote")
                            .ghost()
                            .small()
                            .label(t(cx, "action-cancel"))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.close(true, window, cx)),
                            ),
                    )
                    .child(
                        Button::new("save-quote")
                            .primary()
                            .small()
                            .label(t(cx, "quote-save"))
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
            .into_any_element()
    }
}
impl Cards {
    pub(in crate::home) fn render_card(
        &mut self,
        quote: std::sync::Arc<QuoteAttachment>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editable = self.editable(cx);
        let id = quote.id.clone();
        let focus = self
            .focus
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone();
        let open = self.editing.as_ref().is_some_and(|edit| edit.id == id);
        let mut args = FluentArgs::new();
        args.set("source", quote.short_title.clone());
        args.set(
            "comment",
            if quote.comment.is_empty() {
                "no"
            } else {
                "yes"
            },
        );
        let label = t_with_args(cx, "quote-card-label", &args);
        let delete_label = t_with_args(cx, "quote-delete-label", &args);
        let remove_id = id.clone();
        let preview = quote
            .selection
            .text
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        let open_id = id.clone();
        let trigger = div()
            .key_context("GupiQuoteAttachment")
            .relative()
            .flex_none()
            .child(
                AttachmentView::new()
                    .id(SharedString::from(format!("quote-card-{id}")))
                    .small()
                    .axis(Axis::Vertical)
                    .w_48()
                    .tooltip(quote.title.clone())
                    .media(
                        AttachmentMedia::new()
                            .w_full()
                            .h_12()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_xs()
                                    .line_clamp(3)
                                    .text_color(cx.theme().muted_foreground)
                                    .when(
                                        matches!(quote.selection.origin, Origin::File { .. }),
                                        |div| div.font_family(cx.theme().mono_font_family.clone()),
                                    )
                                    .child(preview),
                            ),
                    )
                    .content(
                        AttachmentContent::new().child(
                            Button::new(SharedString::from(format!("quote-open-{id}")))
                                .ghost()
                                .xsmall()
                                .w_full()
                                .min_w_0()
                                .track_focus(&focus)
                                .disabled(!editable)
                                .accessibility_label(label)
                                .on_click(cx.listener(move |this, event, window, cx| {
                                    // Pointer activation belongs to Popover's press
                                    // handler; don't reopen it on the same release.
                                    if !matches!(event, ClickEvent::Mouse(_)) {
                                        this.open(&open_id, window, cx);
                                    }
                                }))
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .min_w_0()
                                        .child(Icon::new(IconName::Quote).size_3().flex_none())
                                        .child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .child(quote.short_title.clone()),
                                        )
                                        .when(!quote.comment.is_empty(), |row| {
                                            row.child(
                                                Icon::new(IconName::MessageCircle)
                                                    .size_3()
                                                    .flex_none(),
                                            )
                                        }),
                                ),
                        ),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("quote-remove-{id}")))
                    .ghost()
                    .xsmall()
                    .icon(IconName::X)
                    .absolute()
                    .top_0()
                    .right_0()
                    .disabled(!editable)
                    .accessibility_label(delete_label.clone())
                    .tooltip(delete_label)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.remove(&remove_id, window, cx);
                    })),
            );
        if !editable {
            return trigger.into_any_element();
        }
        let owner = cx.entity();
        let editor_focus = self
            .editing
            .as_ref()
            .filter(|edit| edit.id == id)
            .map(|edit| edit.input.focus_handle(cx));
        Popover::new(SharedString::from(format!("quote-popover-{id}")))
            .anchor(Anchor::BottomCenter)
            .open(open)
            .trigger(Trigger {
                id: format!("quote-trigger-{}", quote.id).into(),
                body: trigger.into_any_element(),
                open,
            })
            .when_some(editor_focus, |popover, focus| popover.track_focus(&focus))
            .on_open_change(cx.listener(move |this, open, window, cx| {
                if *open {
                    this.open(&id, window, cx);
                } else if this
                    .editing
                    .as_ref()
                    .is_some_and(|editing| editing.id == id)
                {
                    this.close(true, window, cx);
                }
            }))
            .content(move |_, window, cx| {
                owner.update(cx, |this, cx| this.content(&quote, window, cx))
            })
            .into_any_element()
    }
}
