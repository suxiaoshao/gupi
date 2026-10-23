//! Per-session presentation state. Saved comments stay in ConversationState.
use super::*;
use gpui_kit::component::Icon;
use gpui_kit::component::Selectable;
use gpui_kit::component::attachment::{
    Attachment as AttachmentView, AttachmentContent, AttachmentMedia,
};
use gpui_kit::component::input::Textarea;
use gpui_kit::component::popover::Popover;
use gupi_conversation::quotes::QuoteAttachment;

actions!(gupi_quote, [SaveComment]);
pub(in crate::home) fn init(cx: &mut App) {
    for context in ["GupiQuoteComment", "GupiQuoteComment > Input"] {
        cx.bind_keys([KeyBinding::new("escape", SaveComment, Some(context))]);
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
    focus: FocusHandle,
    floating: Option<Bounds<Pixels>>,
    bounds: Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    _subscriptions: Vec<Subscription>,
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
                this.commit(cx);
            }
            cx.notify();
        });
        let composer_subscription = cx.subscribe(&composer, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.commit(cx);
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
                self.commit(cx);
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
        if !self.commit(cx) {
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
            input
        });
        let raw = cx.new(|cx| {
            let mut raw = TextareaState::new(window, cx).auto_grow(1, 8);
            raw.set_value(quote.selection.text.clone(), window, cx);
            raw
        });
        let focus = cx.focus_handle();
        let input_id = input.entity_id();
        let subscriptions = vec![
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.subscribe_in(&input, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { shift: false, .. }) {
                    this.save(window, cx);
                }
            }),
            cx.on_focus_out(&focus, window, move |this, _, _, cx| {
                if this
                    .editing
                    .as_ref()
                    .is_some_and(|editing| editing.input.entity_id() == input_id)
                {
                    this.commit(cx);
                }
            }),
        ];
        self.editing = Some(Editing {
            id: id.to_owned(),
            input,
            raw,
            focus,
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
    // Closing always commits to this session's attachment, even when another
    // session has become selected. Never route through the global current draft.
    pub(in crate::home) fn commit(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(editing) = &self.editing else {
            return true;
        };
        let comment = editing.input.read(cx).value().to_string();
        let id = editing.id.clone();
        let gone = !self.quotes(cx).iter().any(|quote| quote.id == id);
        let saved = gone
            || self.state.update(cx, |state, cx| {
                state.edit_quote(&self.key, &id, comment, cx)
            });
        if saved {
            self.editing = None;
            cx.notify();
        }
        saved
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.commit(cx) {
            self.composer.focus_handle(cx).focus(window, cx);
        }
    }
    pub(in crate::home) fn open_floating(
        &mut self,
        id: &str,
        anchor: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        div()
            .id("quote-comment-editor")
            .occlude()
            .key_context("GupiQuoteComment")
            .on_action(cx.listener(|this, _: &SaveComment, window, cx| this.save(window, cx)))
            .w_72()
            .max_w_full()
            .min_w_0()
            .flex_none()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_full()
            .shadow_md()
            .px_3()
            .py_1()
            .child(
                Textarea::new(&editing.input)
                    .aria_label(t(cx, "quote-comment-placeholder"))
                    .appearance(false)
                    .small(),
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
        let child = div()
            .track_focus(&editing.focus)
            .child(self.comment_input(cx));
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
                                this.commit(cx);
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
        let mut args = FluentArgs::new();
        args.set("source", quote.short_title.clone());
        v_flex()
            .track_focus(&editing.focus)
            .gap_2()
            .w_80()
            .max_w(width)
            .max_h(height)
            .min_h_0()
            .child(
                h_flex()
                    .gap_2()
                    .min_w_0()
                    .flex_none()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t_with_args(cx, "quote-preview-title", &args)),
                    )
                    .child(
                        Button::new("delete-quote")
                            .ghost()
                            .xsmall()
                            .icon(IconName::X)
                            .accessibility_label(t(cx, "quote-delete"))
                            .tooltip(t(cx, "quote-delete"))
                            .on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.remove(&id, window, cx)
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .min_h_0()
                    .border_l_2()
                    .border_color(cx.theme().border)
                    .pl_2()
                    .child(
                        Textarea::new(&editing.raw)
                            .readonly(true)
                            .appearance(false)
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .min_h_0()
                            .when(
                                matches!(quote.selection.origin, Origin::File { .. }),
                                |view| view.font_family(cx.theme().mono_font_family.clone()),
                            ),
                    ),
            )
            .child(self.comment_input(cx))
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
        let open = self
            .editing
            .as_ref()
            .is_some_and(|edit| edit.id == id && edit.floating.is_none());
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
            .take(if quote.comment.is_empty() { 3 } else { 1 })
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
                            .when(!quote.comment.is_empty(), |media| media.h_16())
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_flex()
                                    .w_full()
                                    .min_w_0()
                                    .gap_1()
                                    .when(!quote.comment.is_empty(), |column| {
                                        column.child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .line_clamp(2)
                                                .child(quote.comment.clone()),
                                        )
                                    })
                                    .child(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .text_xs()
                                            .line_clamp(if quote.comment.is_empty() {
                                                3
                                            } else {
                                                1
                                            })
                                            .text_color(cx.theme().muted_foreground)
                                            .when(!quote.comment.is_empty(), |div| {
                                                div.border_l_2()
                                                    .border_color(cx.theme().border)
                                                    .pl_1()
                                            })
                                            .when(
                                                matches!(
                                                    quote.selection.origin,
                                                    Origin::File { .. }
                                                ),
                                                |div| {
                                                    div.font_family(
                                                        cx.theme().mono_font_family.clone(),
                                                    )
                                                },
                                            )
                                            .child(preview),
                                    ),
                            ),
                    )
                    .content(
                        AttachmentContent::new().child(
                            h_flex()
                                .w_full()
                                .min_w_0()
                                .child(
                                    Button::new(SharedString::from(format!("quote-open-{id}")))
                                        .ghost()
                                        .xsmall()
                                        .flex_1()
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
                                                .child(
                                                    Icon::new(IconName::Quote).size_3().flex_none(),
                                                )
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .truncate()
                                                        .child(quote.short_title.clone()),
                                                ),
                                        ),
                                )
                                .child(
                                    Button::new(SharedString::from(format!("quote-remove-{id}")))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::X)
                                        .flex_none()
                                        .disabled(!editable)
                                        .accessibility_label(delete_label.clone())
                                        .tooltip(delete_label)
                                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                            cx.stop_propagation()
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            cx.stop_propagation();
                                            this.remove(&remove_id, window, cx);
                                        })),
                                ),
                        ),
                    ),
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
                    this.save(window, cx);
                }
            }))
            .content(move |_, window, cx| {
                owner.update(cx, |this, cx| this.content(&quote, window, cx))
            })
            .into_any_element()
    }
}
