use super::*;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::{
    form::{Field, Form},
    input::{Input, InputEvent, InputState},
};

pub(super) struct Editor {
    owner: Entity<AuthPage>,
    baseline: File,
    provider: Entity<InputState>,
    key: Entity<InputState>,
    fixed: bool,
    error: Option<Error>,
    _subscriptions: Vec<Subscription>,
}
impl Editor {
    pub(super) fn open(
        owner: &Entity<AuthPage>,
        entry: Option<Entry>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if owner.read(cx).saving.is_some() {
            return;
        }
        let Some(baseline) = owner.read(cx).file.clone() else {
            return;
        };
        let fixed = entry.is_some();
        let name = entry.map(|e| e.provider().to_owned()).unwrap_or_default();
        let editor = cx.new(|cx| {
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
            ];
            Self {
                owner: owner.clone(),
                baseline,
                provider,
                key,
                fixed,
                error: None,
                _subscriptions: subscriptions,
            }
        });
        window.open_dialog(cx, move |dialog, _, cx| {
            let submit = editor.clone();
            dialog
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

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let provider = self.provider.read(cx).value().to_string();
        if (!self.fixed && !pi_auth::valid_provider(&provider))
            || self.key.read(cx).value().is_empty()
        {
            self.error = Some(Error::Invalid);
            cx.notify();
            return;
        }
        let entry = self.baseline.entry(&provider);
        let file = self.baseline.clone();
        let owner = self.owner.clone();
        let key = self.key.read(cx).value().to_string();
        self.key
            .update(cx, |input, cx| input.set_value("", window, cx));
        window.close_dialog(cx);
        if matches!(entry.kind(), Kind::OAuth | Kind::Unknown) {
            window.open_dialog(cx, move |dialog, _, cx| {
                let save = (
                    owner.clone(),
                    file.clone(),
                    provider.clone(),
                    entry.revision(),
                    key.clone(),
                );
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("provider", provider.clone());
                dialog
                    .title(t_with_args(cx, "pi-auth-replace-title", &args))
                    .child(t(cx, "pi-auth-replace-help"))
                    .footer(super::super::dialog_buttons(
                        "pi-auth-replace-confirm",
                        false,
                        false,
                        cx,
                    ))
                    .on_ok(move |_, _, cx| {
                        save.0.update(cx, |this, cx| {
                            this.commit(
                                save.1.clone(),
                                save.2.clone(),
                                save.3.clone(),
                                Some(save.4.clone()),
                                cx,
                            )
                        });
                        true
                    })
            });
        } else {
            owner.update(cx, |this, cx| {
                this.commit(file, provider, entry.revision(), Some(key), cx)
            });
        }
    }
}
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let candidates = self.owner.read(cx).names.clone();
        let provider = self.provider.clone();
        let mut body = v_flex().gap_3().child(
            Form::new()
                .child(
                    Field::new().label(t(cx, "pi-auth-provider")).child(
                        Input::new(&self.provider)
                            .id("auth-provider")
                            .aria_label(t(cx, "pi-auth-provider"))
                            .readonly(self.fixed),
                    ),
                )
                .child(
                    Field::new().label(t(cx, "pi-auth-api-key")).child(
                        Input::new(&self.key)
                            .id("auth-key")
                            .aria_label(t(cx, "pi-auth-api-key")),
                    ),
                ),
        );
        if !self.fixed && !candidates.is_empty() {
            body = body.child(
                Button::new("auth-candidates")
                    .small()
                    .label(t(cx, "pi-auth-candidates"))
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
            );
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
        body.child(
            DialogFooter::new()
                .child(
                    Button::new("auth-cancel")
                        .label(t(cx, "action-cancel"))
                        .on_click(|_, window, cx| {
                            window
                                .dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx)
                        }),
                )
                .child(
                    Button::new("auth-save")
                        .primary()
                        .label(t(cx, "pi-settings-save"))
                        .disabled(self.key.read(cx).value().is_empty())
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                ),
        )
    }
}
