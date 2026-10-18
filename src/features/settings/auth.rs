//! Global Pi credentials, committed independently of settings drafts.
mod editor;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_auth::{
    self, Check, CheckReason, CheckStatus, Entry, Error, File, Kind, Revision,
};
use gupi_settings::{
    assets::IconName,
    config::ConfigController,
    i18n::{t, t_with_args},
};
use std::{collections::HashMap, path::PathBuf};

struct WriteFinished(Result<(), Error>);
impl EventEmitter<WriteFinished> for AuthPage {}

pub(super) struct AuthPage {
    agent: Option<PathBuf>,
    file: Option<File>,
    names: Vec<String>,
    error: Option<Error>,
    loading: Option<Task<()>>,
    saving: Option<Task<()>>,
    checks: HashMap<String, Option<Check>>,
    checking: HashMap<String, Task<()>>,
    controller: Entity<ConfigController>,
}
impl AuthPage {
    pub(super) fn new(controller: Entity<ConfigController>) -> Self {
        Self {
            #[cfg(not(test))]
            agent: gupi_resources::pi_resources::agent_dir().ok(),
            #[cfg(test)]
            agent: None,
            file: None,
            names: vec![],
            error: None,
            loading: None,
            saving: None,
            checks: HashMap::new(),
            checking: HashMap::new(),
            controller,
        }
    }
    #[cfg(test)]
    pub(super) fn set_agent_for_test(&mut self, agent: PathBuf) {
        self.agent = Some(agent);
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        let Some(agent) = self.agent.clone() else {
            self.error = Some(Error::Read);
            cx.notify();
            return;
        };
        let worker = cx.background_spawn(async move {
            let auth = File::load(&agent.join("auth.json"));
            let models = gupi_resources::pi_config_files::File::load(
                gupi_resources::pi_config_files::Kind::Models,
                &agent.join("models.json"),
            )
            .and_then(|f| f.names())
            .unwrap_or_default();
            (auth, models)
        });
        self.loading = Some(cx.spawn(async move |this, cx| {
            let (auth, models) = worker.await;
            let _ = this.update(cx, |this, cx| {
                this.loading = None;
                match auth {
                    Ok(file) => {
                        this.names = file
                            .entries()
                            .iter()
                            .map(|e| e.provider().to_owned())
                            .collect();
                        for name in models {
                            if !this.names.contains(&name) {
                                this.names.push(name)
                            }
                        }
                        this.file = Some(file);
                        this.error = None;
                    }
                    Err(e) => {
                        this.file = None;
                        this.error = Some(e)
                    }
                }
                this.checks.clear();
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn commit(
        &mut self,
        file: File,
        provider: String,
        revision: Revision,
        key: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() {
            return;
        }
        let worker =
            cx.background_spawn(async move { file.commit(&provider, &revision, key.as_deref()) });
        self.saving = Some(cx.spawn(async move |this, cx| {
            let result = worker.await;
            let _ = this.update(cx, |this, cx| {
                this.saving = None;
                let outcome = result.as_ref().map(|_| ()).map_err(|error| *error);
                match result {
                    Ok(file) => {
                        this.file = Some(file);
                        this.reload(cx)
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.emit(WriteFinished(outcome));
                cx.notify()
            });
        }));
        self.error = None;
        cx.notify();
    }
    fn check(&mut self, provider: String, cx: &mut Context<Self>) {
        if self.checking.contains_key(&provider) {
            return;
        }
        let command = self
            .controller
            .read(cx)
            .configuration()
            .read(cx, |op| {
                op.data()
                    .and_then(|d| d.configured())
                    .map(|c| c.pi_executable().to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "pi".into());
        let environment = gupi_pi_runtime::environment(cx);
        let name = provider.clone();
        let worker = gpui_tokio::Tokio::spawn(cx, async move {
            let snapshot = environment.load(false).await;
            pi_auth::check(&command, &name, snapshot.variables()).await
        });
        let name = provider.clone();
        let task = cx.spawn(async move |this, cx| {
            let result = worker.await.ok().flatten();
            let _ = this.update(cx, |this, cx| {
                this.checking.remove(&name);
                this.checks.insert(name, result);
                cx.notify()
            });
        });
        self.checking.insert(provider, task);
        cx.notify();
    }
    pub(super) fn page(entity: &Entity<Self>, cx: &App) -> SettingPage {
        let e = entity.clone();
        SettingPage::new(t(cx,"pi-auth-title")).icon(IconName::KeyRound).resettable(false).group(
            SettingGroup::new().item(SettingItem::render(move |_,_,cx|e.update(cx,|this,cx|this.render_page(cx))).keywords(["auth credentials API key OAuth login logout provider 认证 凭据 密钥 登录 注销"]))
        )
    }
    fn render_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.file.is_none() && self.error.is_none() && self.loading.is_none() {
            let e = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = e.update(cx, |this, cx| this.reload(cx));
            });
        }
        let busy = self.loading.is_some() || self.saving.is_some();
        let add = cx.entity();
        let reload = add.clone();
        let mut body = v_flex().w_full().max_w(rems(56.)).gap_2().child(
            h_flex()
                .w_full()
                .items_start()
                .gap_2()
                .child(
                    v_flex()
                        .flex_1()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t(cx, "pi-files-providers")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "pi-auth-help")),
                        ),
                )
                .child(
                    Button::new("auth-add")
                        .small()
                        .label(t(cx, "pi-auth-add"))
                        .disabled(busy || self.file.is_none())
                        .on_click(move |_, w, cx| editor::Editor::open(&add, None, w, cx)),
                )
                .child(
                    Button::new("auth-reload")
                        .small()
                        .ghost()
                        .icon(IconName::RotateCw)
                        .accessibility_label(t(cx, "pi-settings-reload"))
                        .disabled(busy)
                        .on_click(move |_, _, cx| reload.update(cx, |this, cx| this.reload(cx))),
                ),
        );
        if let Some(error) = self.error {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
        }
        if busy {
            return body.child(t(cx, "pi-settings-loading")).into_any_element();
        }
        let Some(file) = &self.file else {
            return body.into_any_element();
        };
        if self.names.is_empty() {
            body = body.child(
                div()
                    .py_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-auth-empty")),
            );
        }
        if file
            .entries()
            .iter()
            .any(|entry| entry.kind() == Kind::OAuth)
        {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-auth-oauth-help")),
            );
        }
        for name in &self.names {
            let entry = file.entry(name);
            let owner = cx.entity();
            let open = (owner.clone(), entry.clone());
            let menu = (owner.clone(), entry.clone());
            let mut description = t(cx, kind_key(entry.kind()));
            if entry.kind() != Kind::Missing {
                description = format!("{description} · {}", t(cx, "pi-auth-set"));
            }
            let mut content =
                v_flex().w_full().gap_1().child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            Button::new(SharedString::from(format!("auth-row-{name}")))
                                .small()
                                .ghost()
                                .flex_1()
                                .min_w_0()
                                .accessibility_label(name.clone())
                                .child(
                                    h_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_left()
                                                .truncate()
                                                .font_family(cx.theme().mono_font_family.clone())
                                                .child(name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(description),
                                        ),
                                )
                                .on_click(move |_, w, cx| {
                                    Self::open_actions(&open.0, open.1.clone(), w, cx)
                                }),
                        )
                        .child(
                            Button::new(SharedString::from(format!("auth-menu-{name}")))
                                .xsmall()
                                .ghost()
                                .icon(IconName::Ellipsis)
                                .accessibility_label(t(cx, "pi-field-actions"))
                                .dropdown_menu(move |popup, _, cx| {
                                    let edit = menu.clone();
                                    let check = menu.clone();
                                    let remove = menu.clone();
                                    let login = menu.clone();
                                    let popup = if menu.1.kind() != Kind::OAuth {
                                        popup.item(
                                            PopupMenuItem::new(t(
                                                cx,
                                                if menu.1.kind() == Kind::Missing {
                                                    "pi-auth-add"
                                                } else {
                                                    "pi-auth-replace"
                                                },
                                            ))
                                            .on_click(move |_, w, cx| {
                                                editor::Editor::open(
                                                    &edit.0,
                                                    Some(edit.1.clone()),
                                                    w,
                                                    cx,
                                                )
                                            }),
                                        )
                                    } else {
                                        popup.item(
                                            PopupMenuItem::new(t(cx, "pi-auth-copy-login"))
                                                .on_click(move |_, w, cx| {
                                                    Self::copy_login(&login.1, w, cx)
                                                }),
                                        )
                                    };
                                    let popup = popup.item(
                                        PopupMenuItem::new(t(cx, "pi-auth-check"))
                                            .disabled(
                                                menu.0
                                                    .read(cx)
                                                    .checking
                                                    .contains_key(menu.1.provider()),
                                            )
                                            .on_click(move |_, _, cx| {
                                                check.0.update(cx, |this, cx| {
                                                    this.check(check.1.provider().into(), cx)
                                                })
                                            }),
                                    );
                                    if menu.1.kind() != Kind::Missing {
                                        popup.item(
                                            PopupMenuItem::new(t(
                                                cx,
                                                if menu.1.kind() == Kind::OAuth {
                                                    "pi-auth-logout"
                                                } else {
                                                    "pi-auth-remove"
                                                },
                                            ))
                                            .on_click(move |_, w, cx| {
                                                Self::confirm_remove(
                                                    &remove.0,
                                                    remove.1.clone(),
                                                    w,
                                                    cx,
                                                )
                                            }),
                                        )
                                    } else {
                                        popup
                                    }
                                }),
                        ),
                );
            if let Some(expires) = entry
                .expires()
                .and_then(|v| chrono::DateTime::from_timestamp_millis(v as i64))
            {
                let text = if expires < chrono::Utc::now() {
                    t(cx, "pi-auth-expired")
                } else {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set(
                        "time",
                        expires
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M")
                            .to_string(),
                    );
                    t_with_args(cx, "pi-auth-expires", &args)
                };
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(text),
                );
            }
            if self.checking.contains_key(name) {
                content = content.child(t(cx, "pi-auth-checking"));
            }
            if let Some(result) = self.checks.get(name) {
                content = content.child(Self::check_text(*result, cx));
            }
            body = body.child(content);
        }
        body.into_any_element()
    }
    fn check_text(check: Option<Check>, cx: &App) -> AnyElement {
        let mut body = v_flex().gap_1().text_sm();
        let key = match check.map(|c| c.status()) {
            Some(CheckStatus::Ready) => "pi-auth-ready",
            Some(CheckStatus::NotReady) => "pi-auth-not-ready",
            Some(CheckStatus::Invalid) => "pi-auth-invalid",
            None => "pi-auth-check-failed",
        };
        body = body.child(t(cx, key));
        if let Some(check) = check {
            if let Some(reason) = check.reason() {
                body = body.child(t(
                    cx,
                    match reason {
                        CheckReason::ProviderNotFound => "pi-auth-provider-not-found",
                        CheckReason::NotConfigured => "pi-auth-not-configured",
                        CheckReason::NotAvailable => "pi-auth-not-available",
                        CheckReason::InvalidState => "pi-auth-invalid-state",
                    },
                ));
            }
            if let Some(kind) = check.kind() {
                body = body.child(t(cx, kind_key(kind)));
            }
        }
        body.into_any_element()
    }
    fn open_actions(owner: &Entity<Self>, entry: Entry, window: &mut Window, cx: &mut App) {
        let owner = owner.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let edit = (owner.clone(), entry.clone());
            let remove = edit.clone();
            let check = edit.clone();
            let login = entry.clone();
            let mut body = v_flex().gap_2().child(t(cx, kind_key(entry.kind())));
            if entry.kind() != Kind::OAuth {
                body = body.child(
                    Button::new("auth-edit")
                        .label(t(
                            cx,
                            if entry.kind() == Kind::Missing {
                                "pi-auth-add"
                            } else {
                                "pi-auth-replace"
                            },
                        ))
                        .on_click(move |_, w, cx| {
                            w.close_dialog(cx);
                            editor::Editor::open(&edit.0, Some(edit.1.clone()), w, cx)
                        }),
                );
            }
            if entry.kind() == Kind::OAuth {
                body = body.child(
                    Button::new("auth-login")
                        .label(t(cx, "pi-auth-copy-login"))
                        .on_click(move |_, w, cx| Self::copy_login(&login, w, cx)),
                );
            }
            body = body.child(
                Button::new("auth-check")
                    .label(t(cx, "pi-auth-check"))
                    .disabled(owner.read(cx).checking.contains_key(entry.provider()))
                    .on_click(move |_, w, cx| {
                        w.close_dialog(cx);
                        check
                            .0
                            .update(cx, |this, cx| this.check(check.1.provider().into(), cx))
                    }),
            );
            if entry.kind() != Kind::Missing {
                body = body.child(
                    Button::new("auth-delete")
                        .danger()
                        .label(t(
                            cx,
                            if entry.kind() == Kind::OAuth {
                                "pi-auth-logout"
                            } else {
                                "pi-auth-remove"
                            },
                        ))
                        .on_click(move |_, w, cx| {
                            w.close_dialog(cx);
                            Self::confirm_remove(&remove.0, remove.1.clone(), w, cx)
                        }),
                );
            }
            dialog.title(entry.provider().to_owned()).child(body)
        });
    }
    fn copy_login(entry: &Entry, window: &mut Window, cx: &mut App) {
        let addressable = pi_auth::addressable_provider(entry.provider());
        let command = if addressable {
            format!("/login {}", entry.provider())
        } else {
            "/login".into()
        };
        cx.write_to_clipboard(ClipboardItem::new_string(command));
        window.push_notification(
            t(
                cx,
                if addressable {
                    "pi-auth-login-help"
                } else {
                    "pi-auth-login-select-help"
                },
            ),
            cx,
        );
    }
    fn confirm_remove(owner: &Entity<Self>, entry: Entry, window: &mut Window, cx: &mut App) {
        let Some(file) = owner.read(cx).file.clone() else {
            return;
        };
        let owner = owner.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let save = (owner.clone(), entry.clone(), file.clone());
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("provider", entry.provider().to_owned());
            dialog
                .title(t_with_args(
                    cx,
                    if entry.kind() == Kind::OAuth {
                        "pi-auth-logout-title"
                    } else {
                        "pi-auth-remove-title"
                    },
                    &args,
                ))
                .child(t(cx, "pi-auth-remove-help"))
                .when(entry.has_env(), |d| d.child(t(cx, "pi-auth-remove-env")))
                .footer(super::dialog_buttons(
                    if entry.kind() == Kind::OAuth {
                        "pi-auth-logout-confirm"
                    } else {
                        "pi-auth-remove-confirm"
                    },
                    false,
                    false,
                    cx,
                ))
                .on_ok(move |_, _, cx| {
                    save.0.update(cx, |this, cx| {
                        this.commit(
                            save.2.clone(),
                            save.1.provider().into(),
                            save.1.revision(),
                            None,
                            cx,
                        )
                    });
                    true
                })
        });
    }
}
fn kind_key(kind: Kind) -> &'static str {
    match kind {
        Kind::ApiKey => "pi-auth-api-key",
        Kind::OAuth => "pi-auth-oauth",
        Kind::Unknown => "pi-auth-unknown",
        Kind::Missing => "pi-auth-missing",
    }
}
fn error_key(error: Error) -> &'static str {
    match error {
        Error::Read => "pi-auth-read-error",
        Error::Parse => "pi-auth-parse-error",
        Error::Busy => "pi-auth-busy",
        Error::Conflict => "pi-auth-conflict",
        Error::Write => "pi-auth-write-error",
        Error::Invalid => "pi-auth-input-error",
    }
}

#[cfg(test)]
mod tests;
