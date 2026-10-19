use super::*;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::{
    form::{Field, Form},
    input::{
        Input, InputEvent, InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton,
        InputState,
    },
};

pub(super) struct Editor {
    owner: Entity<AuthPage>,
    baseline: File,
    provider: Entity<InputState>,
    key: Entity<InputState>,
    fixed: bool,
    origin: Option<WeakFocusHandle>,
    fallback: WeakFocusHandle,
    error: Option<Error>,
    waiting: bool,
    confirming: bool,
    _subscriptions: Vec<Subscription>,
}
impl Editor {
    pub(super) fn open(
        owner: &Entity<AuthPage>,
        entry: Option<Entry>,
        fallback: WeakFocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        if owner.read(cx).saving.is_some() || owner.read(cx).loading.is_some() {
            return;
        }
        let Some(baseline) = owner.read(cx).file.clone() else {
            return;
        };
        let origin = window.focused(cx).map(|focus| focus.downgrade());
        let closed_origin = origin.clone();
        let closed_fallback = fallback.clone();
        let closed_owner = owner.downgrade();
        let fixed = entry.is_some();
        let name = entry.map(|e| e.provider().to_owned()).unwrap_or_default();
        let editor = cx.new(|cx: &mut Context<Self>| {
            let provider = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(name)
                    .placeholder(t(cx, "pi-auth-provider"))
            });
            let key = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder(t(cx, "pi-auth-key-placeholder"))
            });
            let subscriptions = vec![
                cx.subscribe(&provider, |_, _, _: &InputEvent, cx| cx.notify()),
                cx.subscribe(&key, |_, _, _: &InputEvent, cx| cx.notify()),
                cx.subscribe_in(
                    owner,
                    window,
                    |this, _, result: &WriteFinished, window, cx| {
                        if !this.waiting {
                            return;
                        }
                        this.waiting = false;
                        match result.0 {
                            Ok(()) => {
                                this.key
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                                window.close_dialog(cx);
                                AuthPage::restore_focus_if_lost(
                                    this.origin.clone(),
                                    this.fallback.clone(),
                                    this.owner.downgrade(),
                                    window,
                                    cx,
                                );
                            }
                            Err(error) => this.error = Some(error),
                        }
                        cx.notify();
                    },
                ),
            ];
            Self {
                owner: owner.clone(),
                baseline,
                provider,
                key,
                fixed,
                origin,
                fallback,
                error: None,
                waiting: false,
                confirming: false,
                _subscriptions: subscriptions,
            }
        });
        window.open_dialog(cx, move |dialog, _, cx| {
            let submit = editor.clone();
            let cancel = editor.clone();
            dialog
                .overlay_closable(false)
                .on_close({
                    let origin = closed_origin.clone();
                    let fallback = closed_fallback.clone();
                    let owner = closed_owner.clone();
                    move |_, window, cx| {
                        AuthPage::restore_focus_if_lost(
                            origin.clone(),
                            fallback.clone(),
                            owner.clone(),
                            window,
                            cx,
                        )
                    }
                })
                .close_button(!editor.read(cx).waiting)
                .on_cancel(move |_, _, cx| {
                    cancel.update(cx, |this, cx| {
                        if this.waiting {
                            return false;
                        }
                        if this.confirming {
                            this.confirming = false;
                            cx.notify();
                            return false;
                        }
                        true
                    })
                })
                .title(t(
                    cx,
                    if fixed {
                        "pi-auth-replace"
                    } else {
                        "pi-auth-add"
                    },
                ))
                .child(editor.clone())
                .on_ok(move |_, window, cx| {
                    submit.update(cx, |this, cx| this.save(window, cx));
                    false
                })
        });
    }

    fn save(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.waiting || self.owner.read(cx).saving.is_some() {
            return;
        }
        let provider = self.provider.read(cx).value().to_string();
        if (!self.fixed && !pi_auth::valid_provider(&provider))
            || self.key.read(cx).value().is_empty()
        {
            self.error = Some(Error::Invalid);
            cx.notify();
            return;
        }
        let entry = self.baseline.entry(&provider);
        if matches!(entry.kind(), Kind::OAuth | Kind::Unknown) && !self.confirming {
            self.confirming = true;
            self.error = None;
            cx.notify();
            return;
        }
        let key = self.key.read(cx).value().to_string();
        self.waiting = true;
        self.error = None;
        self.owner.update(cx, |this, cx| {
            this.commit(
                self.baseline.clone(),
                provider,
                entry.revision(),
                Some(key),
                cx,
            )
        });
        cx.notify();
    }
}
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let candidates = self.owner.read(cx).names.clone();
        let provider = self.provider.clone();
        let mut provider_input = InputGroup::new("auth-provider-group")
            .input(
                Input::new(&self.provider)
                    .id("auth-provider")
                    .aria_label(t(cx, "pi-auth-provider")),
            )
            .readonly(self.fixed || self.confirming)
            .disabled(self.waiting);
        if !self.fixed && !self.confirming && !candidates.is_empty() {
            provider_input = provider_input.addon(
                InputGroupAddon::new("candidates")
                    .align(InputGroupAddonAlignment::InlineEnd)
                    .child(
                        InputGroupButton::new("auth-candidates")
                            .small()
                            .ghost()
                            .icon(IconName::ChevronDown)
                            .accessibility_label(t(cx, "pi-auth-candidates"))
                            .disabled(self.waiting)
                            .dropdown_menu(move |menu, _, _| {
                                let mut menu = menu;
                                for name in &candidates {
                                    let input = provider.clone();
                                    let value = name.clone();
                                    menu = menu.item(PopupMenuItem::new(name.clone()).on_click(
                                        move |_, w, cx| {
                                            input.update(cx, |input, cx| {
                                                input.set_value(value.clone(), w, cx)
                                            })
                                        },
                                    ));
                                }
                                menu
                            }),
                    ),
            );
        }
        let mut body = v_flex().gap_3().child(
            Form::new()
                .child(
                    Field::new()
                        .label(t(cx, "pi-auth-provider"))
                        .child(provider_input),
                )
                .child(
                    Field::new().label(t(cx, "pi-auth-api-key")).child(
                        Input::new(&self.key)
                            .id("auth-key")
                            .aria_label(t(cx, "pi-auth-api-key"))
                            .readonly(self.confirming)
                            .disabled(self.waiting),
                    ),
                ),
        );
        if self.confirming {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("provider", self.provider.read(cx).value().to_string());
            body = body
                .child(t_with_args(cx, "pi-auth-replace-title", &args))
                .child(t(cx, "pi-auth-replace-help"));
        }
        if self
            .baseline
            .entry(self.provider.read(cx).value().as_str())
            .kind()
            != Kind::Missing
        {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-auth-set")),
            );
        }
        if let Some(error) = self.error {
            body = body.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
        }
        if self.waiting {
            body = body.child(t(cx, "pi-auth-saving"));
        }
        body.child(
            DialogFooter::new()
                .child(
                    Button::new("auth-cancel")
                        .label(t(cx, "action-cancel"))
                        .disabled(self.waiting)
                        .on_click(|_, window, cx| {
                            window
                                .dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx)
                        }),
                )
                .child(
                    Button::new("auth-save")
                        .primary()
                        .label(t(
                            cx,
                            if self.confirming {
                                "pi-auth-replace-confirm"
                            } else {
                                "pi-settings-save"
                            },
                        ))
                        .disabled(self.waiting || self.key.read(cx).value().is_empty())
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                ),
        )
    }
}
