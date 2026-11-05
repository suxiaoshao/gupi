//! Global Pi credentials, committed independently of settings drafts.
use crate::features::settings::composition::{SettingsPage, SettingsRow, SettingsSection};
mod editor;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::group_box::GroupBoxVariants as _;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
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
impl EventEmitter<super::CredentialPage> for AuthPage {}

pub(super) struct AuthPage {
    agent: Option<PathBuf>,
    file: Option<File>,
    names: Vec<String>,
    model_names: Vec<String>,
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
            model_names: vec![],
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
                this.model_names = models.clone();
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
        if self.saving.is_some() || self.loading.is_some() {
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
        if !pi_auth::checkable_provider(&provider) || self.checking.contains_key(&provider) {
            return;
        }
        let command = self
            .controller
            .read(cx)
            .configuration()
            .read(cx, |op| {
                op.data()
                    .and_then(|d| d.configured())
                    .map(|c| c.pi_executable().as_os_str().to_owned())
            })
            .unwrap_or_else(|| "pi".into());
        let environment = gupi_pi_runtime::environment(cx);
        let name = provider.clone();
        let worker = gpui_tokio::Tokio::spawn(cx, async move {
            let variables = async move { environment.load(false).await.variables() };
            pi_auth::check(&command, &name, variables).await
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
    pub(super) fn page(entity: &Entity<Self>, cx: &App) -> SettingsPage {
        let e = entity.clone();
        let actions = entity.clone();
        SettingsPage::new(t(cx,"pi-auth-title")).icon(IconName::KeyRound)
            .actions(move |_, _, cx| actions.update(cx, |this, cx| Button::new("auth-reload").ghost().small().icon(IconName::RotateCw)
                .tooltip(t(cx, "pi-settings-reload")).accessibility_label(t(cx, "pi-settings-reload"))
                .disabled(this.loading.is_some() || this.saving.is_some())
                .on_click(cx.listener(|this, _, _, cx| this.reload(cx)))))
            .group(
            SettingsSection::new().item(SettingsRow::content(move |layout,window,cx|e.update(cx,|this,cx|this.render_page(layout,window,cx))).keywords(["auth credentials API key OAuth login logout provider 认证 凭据 密钥 登录 注销"]))
        )
    }
    fn render_page(
        &mut self,
        layout: &crate::features::settings::composition::PageLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.file.is_none() && self.error.is_none() && self.loading.is_none() {
            let e = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = e.update(cx, |this, cx| this.reload(cx));
            });
        }
        let busy = self.loading.is_some() || self.saving.is_some();
        let add_focus = window
            .use_keyed_state("auth-add-focus", cx, |_, cx| {
                cx.focus_handle().tab_stop(true)
            })
            .read(cx)
            .clone();
        let add_trigger = add_focus.clone();
        let add = cx.entity();
        let mut body =
            v_flex()
                .w_full()
                .min_w_0()
                .max_w(rems(56.))
                .gap_2()
                .child(super::management_header(
                    div().into_any_element(),
                    None,
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("auth-add")
                                .track_focus(&add_focus)
                                .small()
                                .label(t(cx, "pi-auth-add"))
                                .disabled(busy || self.file.is_none())
                                .on_click(move |_, w, cx| {
                                    add_trigger.focus(w, cx);
                                    editor::Editor::open(&add, None, add_trigger.downgrade(), w, cx)
                                }),
                        )
                        .into_any_element(),
                ));
        if busy {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if self.saving.is_some() {
                            "pi-auth-saving"
                        } else {
                            "pi-settings-loading"
                        },
                    )),
            );
        }
        for key in ["pi-auth-help", "pi-auth-models-help"] {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, key)),
            );
        }
        if let Some(error) = self.error {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
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
            let row_focus = window
                .use_keyed_state(
                    SharedString::from(format!("auth-row-focus-{name}")),
                    cx,
                    |_, cx| cx.focus_handle().tab_stop(true),
                )
                .read(cx)
                .clone();
            let menu_focus = window
                .use_keyed_state(
                    SharedString::from(format!("auth-menu-focus-{name}")),
                    cx,
                    |_, cx| cx.focus_handle().tab_stop(true),
                )
                .read(cx)
                .clone();
            let entry = file.entry(name);
            let mut description = t(cx, kind_key(entry.kind()));
            if entry.kind() != Kind::Missing {
                description = format!("{description} · {}", t(cx, "pi-auth-set"));
            }
            let mut info = v_flex()
                .min_w_0()
                .gap_1()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(description);
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
                info = info.child(text);
            }
            if self.checking.contains_key(name) {
                info = info.child(t(cx, "pi-auth-checking"));
            }
            if let Some(result) = self.checks.get(name) {
                info = info.child(Self::check_text(*result, cx));
            }
            let primary = (
                cx.entity(),
                entry.clone(),
                row_focus.clone(),
                add_focus.clone(),
            );
            let check = (cx.entity(), name.clone());
            let menu = (cx.entity(), entry.clone(), menu_focus.clone());
            let checkable = pi_auth::checkable_provider(name);
            let actions =
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(SharedString::from(format!("auth-row-{name}")))
                            .track_focus(&row_focus)
                            .small()
                            .outline()
                            .label(t(cx, Self::primary_label(entry.kind())))
                            .accessibility_label(Self::row_label(&entry, cx))
                            .disabled(busy)
                            .on_click(move |_, window, cx| {
                                primary.2.focus(window, cx);
                                Self::activate_primary(
                                    &primary.0,
                                    primary.1.clone(),
                                    primary.3.downgrade(),
                                    window,
                                    cx,
                                );
                            }),
                    )
                    .child(
                        Button::new(SharedString::from(format!("auth-check-{name}")))
                            .small()
                            .ghost()
                            .label(t(cx, "pi-auth-check"))
                            .loading(self.checking.contains_key(name))
                            .disabled(busy || !checkable || self.checking.contains_key(name))
                            .when(!checkable, |button| {
                                button.tooltip(t(cx, "pi-auth-check-unavailable"))
                            })
                            .on_click(move |_, _, cx| {
                                check
                                    .0
                                    .update(cx, |this, cx| this.check(check.1.clone(), cx))
                            }),
                    )
                    .child(
                        Button::new(SharedString::from(format!("auth-menu-{name}")))
                            .track_focus(&menu_focus)
                            .small()
                            .ghost()
                            .icon(IconName::Ellipsis)
                            .tooltip(format!("{}: {}", t(cx, "pi-field-actions"), name))
                            .accessibility_label(t(cx, "pi-field-actions"))
                            .disabled(busy)
                            .dropdown_menu(move |popup, _, cx| {
                                let navigate = menu.0.clone();
                                let remove = menu.clone();
                                popup
                                    .when(
                                        menu.0
                                            .read(cx)
                                            .model_names
                                            .iter()
                                            .any(|name| name == menu.1.provider()),
                                        |popup| {
                                            popup.item(
                                                PopupMenuItem::new(t(cx, "pi-auth-edit-models"))
                                                    .on_click(move |_, _, cx| {
                                                        navigate.update(cx, |_, cx| {
                                                            cx.emit(super::CredentialPage::Models)
                                                        })
                                                    }),
                                            )
                                        },
                                    )
                                    .when(menu.1.kind() != Kind::Missing, |popup| {
                                        popup.item(
                                            PopupMenuItem::new(t(
                                                cx,
                                                if menu.1.kind() == Kind::OAuth {
                                                    "pi-auth-logout"
                                                } else {
                                                    "pi-auth-remove"
                                                },
                                            ))
                                            .on_click(move |_, window, cx| {
                                                let remove = remove.clone();
                                                window.defer(cx, move |window, cx| {
                                                    remove.2.focus(window, cx);
                                                    Self::confirm_remove(
                                                        &remove.0, remove.1, window, cx,
                                                    );
                                                });
                                            }),
                                        )
                                    })
                            }),
                    );
            let title = super::provider_name(name, cx);
            body = body.child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap_3()
                    .mt_4()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(
                        gpui_kit::component::group_box::GroupBox::new()
                            .fill()
                            .content_style(StyleRefinement::default().px_5().py_0().gap_0())
                            .child(crate::features::settings::composition::settings_row(
                                layout, info, None, actions, false, cx,
                            )),
                    ),
            );
        }
        body.into_any_element()
    }
    fn restore_focus_if_lost(
        origin: Option<WeakFocusHandle>,
        fallback: WeakFocusHandle,
        owner: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Root owns the usual restore. Only an expired origin needs our fallback;
        // in that case Root has not scheduled a competing animation restore.
        if origin.and_then(|origin| origin.upgrade()).is_some() {
            return;
        }
        window.defer(cx, move |window, cx| {
            if owner.upgrade().is_none() || window.has_active_dialog(cx) {
                return;
            }
            if let Some(fallback) = fallback.upgrade() {
                // Add is the stable logical fallback when the original row vanished.
                fallback.focus(window, cx);
            }
        });
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
    fn primary_label(kind: Kind) -> &'static str {
        match kind {
            Kind::Missing => "pi-auth-add",
            Kind::OAuth => "pi-auth-copy-login",
            Kind::ApiKey | Kind::Unknown => "pi-auth-replace",
        }
    }
    fn row_label(entry: &Entry, cx: &App) -> String {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("provider", super::provider_label(entry.provider(), cx));
        args.set("action", t(cx, Self::primary_label(entry.kind())));
        t_with_args(cx, "pi-auth-row-action", &args)
    }
    fn activate_primary(
        owner: &Entity<Self>,
        entry: Entry,
        fallback: WeakFocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        if entry.kind() == Kind::OAuth {
            Self::copy_login(&entry, window, cx);
        } else {
            editor::Editor::open(owner, Some(entry), fallback, window, cx);
        }
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
            args.set("provider", super::provider_label(entry.provider(), cx));
            dialog
                .title(
                    div()
                        .when(entry.provider().is_empty(), |view| view.italic())
                        .child(t_with_args(
                            cx,
                            if entry.kind() == Kind::OAuth {
                                "pi-auth-logout-title"
                            } else {
                                "pi-auth-remove-title"
                            },
                            &args,
                        )),
                )
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
                    crate::features::settings::DialogCommit::Danger,
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
