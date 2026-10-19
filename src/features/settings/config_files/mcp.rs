//! Settings-owned MCP operation state; no credential or raw CLI output is retained.
use super::*;
use gpui_kit::component::{dialog::DialogFooter, menu::PopupMenu};
use gupi_resources::pi_mcp::{
    self, Action, Cancellation, Eligibility, Outcome, Report, Server, ServerScope, State,
};

#[derive(Default)]
pub(super) struct Mount(pub Cancellation);
impl Drop for Mount {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
pub(super) struct Running {
    generation: u64,
    cancellation: Cancellation,
}
#[derive(Default)]
pub(super) struct Actions {
    pub(super) running: Option<Running>,
    generation: u64,
    report: Option<(Report, String)>,
    message: Option<&'static str>,
}
impl Actions {
    pub(super) fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancellation.cancel();
        }
    }
    pub(super) fn invalidate(&mut self) {
        self.cancel();
        self.generation += 1;
        self.report = None;
        self.message = None;
    }
}
fn status(server: Option<&Server>, cx: &App) -> String {
    match server.map(Server::state) {
        Some(State::Connected) => {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("count", server.unwrap().tools() as i64);
            t_with_args(cx, "pi-mcp-connected", &args)
        }
        Some(State::NeedsAuth) => t(cx, "pi-mcp-needs-auth"),
        Some(State::Failed) => t(cx, "pi-mcp-connection-failed"),
        Some(State::Disabled) => t(cx, "pi-mcp-disabled"),
        Some(_) => t(cx, "pi-mcp-disconnected"),
        None => t(cx, "pi-mcp-not-checked"),
    }
}
fn text(id: impl Into<ElementId>, value: String) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Label)
        .aria_label(value.clone())
        .child(value)
}
impl FilePages {
    pub(super) fn mcp_row_status(&self, name: &str, cx: &App) -> String {
        let scope = if matches!(self.scope, Scope::Global) {
            ServerScope::Global
        } else {
            ServerScope::Project
        };
        status(
            self.mcp_actions
                .report
                .as_ref()
                .and_then(|(report, _)| report.server(scope, name)),
            cx,
        )
    }
    pub(super) fn mcp_disabled_in_report(&self, name: &str) -> bool {
        let scope = if matches!(self.scope, Scope::Global) {
            ServerScope::Global
        } else {
            ServerScope::Project
        };
        self.mcp_actions
            .report
            .as_ref()
            .and_then(|(report, _)| report.server(scope, name))
            .is_some_and(|server| server.state() == State::Disabled)
    }
    pub(super) fn render_mcp_controls(
        &self,
        command: Option<String>,
        mount: Cancellation,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let busy = self.busy();
        let owner = cx.entity();
        let mut body = v_flex()
            .min_w_0()
            .w_full()
            .gap_2()
            .text_sm()
            .child(text("mcp-help", t(cx, "pi-mcp-help")).text_color(cx.theme().muted_foreground))
            .child(
                text("mcp-reconnect", t(cx, "pi-mcp-reconnect"))
                    .text_color(cx.theme().muted_foreground),
            )
            .when(matches!(self.scope, Scope::Global), |body| {
                body.child(text("mcp-global-cwd", t(cx, "pi-mcp-global-cwd")))
            })
            .when(self.dirty(), |body| {
                body.child(text("mcp-save-first", t(cx, "pi-mcp-save-first")))
            })
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("mcp-check")
                            .small()
                            .label(t(cx, "pi-mcp-check"))
                            .disabled(
                                busy || self.dirty()
                                    || self.loading.is_some()
                                    || self.mcp.file.is_none()
                                    || command.is_none(),
                            )
                            .on_click(move |_, window, cx| {
                                if let Some(command) = &command {
                                    owner.update(cx, |this, cx| {
                                        this.start_mcp(
                                            Action::Check,
                                            command.clone(),
                                            mount.clone(),
                                            window,
                                            cx,
                                        );
                                    });
                                }
                            }),
                    )
                    .when(self.mcp_actions.running.is_some(), |row| {
                        let owner = cx.entity();
                        row.child(text("mcp-running", t(cx, "pi-mcp-running")))
                            .child(
                                Button::new("mcp-cancel")
                                    .small()
                                    .label(t(cx, "action-cancel"))
                                    .on_click(move |_, _, cx| {
                                        owner.update(cx, |this, _| this.mcp_actions.cancel());
                                    }),
                            )
                    }),
            );
        if let Some(message) = self.mcp_actions.message {
            body = body.child(text("mcp-outcome", t(cx, message)));
        }
        body.into_any_element()
    }
    pub(super) fn render_mcp_results(&self, cx: &App) -> AnyElement {
        let mut body = v_flex().min_w_0().w_full().gap_2().text_sm();
        if let Some((report, time)) = &self.mcp_actions.report {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("time", time.as_str());
            body = body
                .child(
                    text(
                        "mcp-last-check",
                        t_with_args(cx, "pi-mcp-last-check", &args),
                    )
                    .font_weight(FontWeight::SEMIBOLD),
                )
                .when(report.has_configuration_errors(), |body| {
                    body.child(text("mcp-config-error", t(cx, "pi-mcp-config-error")))
                })
                .when(report.has_untrusted_project(), |body| {
                    body.child(text("mcp-untrusted", t(cx, "pi-mcp-untrusted")))
                });
            for scope in [ServerScope::Global, ServerScope::Project] {
                let servers: Vec<_> = report
                    .servers()
                    .iter()
                    .filter(|server| server.scope() == scope)
                    .collect();
                if servers.is_empty() {
                    continue;
                }
                let group = if scope == ServerScope::Global {
                    "global"
                } else {
                    "project"
                };
                body = body.child(
                    text(
                        SharedString::from(format!("mcp-report-{group}")),
                        t(
                            cx,
                            if scope == ServerScope::Global {
                                "pi-scope-global"
                            } else {
                                "pi-mcp-scope-project"
                            },
                        ),
                    )
                    .font_weight(FontWeight::MEDIUM),
                );
                for server in servers {
                    let identity = format!("mcp-report-{group}-{}", server.name());
                    body = body.child(
                        v_flex()
                            .min_w_0()
                            .gap_1()
                            .child(
                                text(
                                    SharedString::from(format!("{identity}-name")),
                                    server.name().to_owned(),
                                )
                                .truncate(),
                            )
                            .child(
                                h_flex()
                                    .min_w_0()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(text(
                                        SharedString::from(format!("{identity}-status")),
                                        status(Some(server), cx),
                                    ))
                                    .when(server.has_resources(), |row| {
                                        row.child(text(
                                            SharedString::from(format!("{identity}-resources")),
                                            t(cx, "pi-mcp-resources"),
                                        ))
                                    }),
                            ),
                    );
                }
            }
        }
        body.into_any_element()
    }
    #[expect(clippy::too_many_arguments)]
    pub(super) fn mcp_menu(
        mut menu: PopupMenu,
        entity: &Entity<Self>,
        name: &str,
        eligibility: Eligibility,
        command: Option<String>,
        mount: Cancellation,
        disabled: bool,
        cx: &App,
    ) -> PopupMenu {
        if eligibility != Eligibility::Eligible {
            return match eligibility {
                Eligibility::UntrustedProject => {
                    menu.item(PopupMenuItem::new(t(cx, "pi-mcp-untrusted")).disabled(true))
                }
                Eligibility::UnsupportedName => {
                    menu.item(PopupMenuItem::new(t(cx, "pi-mcp-unsupported-name")).disabled(true))
                }
                _ => menu,
            };
        }
        for login in [true, false] {
            let owner = entity.clone();
            let name = name.to_owned();
            let command = command.clone();
            let mount = mount.clone();
            menu = menu.item(
                PopupMenuItem::new(t(
                    cx,
                    if login {
                        "pi-mcp-login"
                    } else {
                        "pi-mcp-logout"
                    },
                ))
                .disabled(disabled)
                .on_click(move |_, window, cx| {
                    let owner = owner.clone();
                    let name = name.clone();
                    let command = command.clone();
                    let mount = mount.clone();
                    window.defer(cx, move |window, cx| {
                        if let Some(command) = command {
                            Self::mcp_dialog(&owner, name, login, command, mount, window, cx);
                        }
                    });
                }),
            );
        }
        menu.separator()
    }
    fn start_mcp(
        &mut self,
        action: Action,
        command: String,
        mount: Cancellation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        if self.busy() || self.dirty() || mount.is_cancelled() || self.loading.is_some() {
            return None;
        }
        self.mcp_actions.invalidate();
        let generation = self.mcp_actions.generation;
        let cancellation = mount.child();
        self.mcp_actions.running = Some(Running {
            generation,
            cancellation: cancellation.clone(),
        });
        let scope = self.scope.clone();
        let environment = gupi_pi_runtime::environment(cx);
        let is_login = matches!(action, Action::Login(_));
        let is_check = matches!(action, Action::Check);
        let worker = gpui_tokio::Tokio::spawn(cx, async move {
            let snapshot = environment.load(false).await;
            pi_mcp::run(
                command,
                action,
                scope,
                snapshot.variables().to_vec(),
                cancellation,
            )
            .await
        });
        // Detached owner task keeps stop/reap alive after page/window release.
        cx.spawn_in(window, async move |this, cx| {
            let result = worker.await.unwrap_or(Outcome::Failed);
            let next = this.update(cx, |this, cx| {
                if !this
                    .mcp_actions
                    .running
                    .as_ref()
                    .is_some_and(|r| r.generation == generation)
                {
                    return None;
                }
                this.mcp_actions.running = None;
                if this.mcp_actions.generation == generation && !mount.is_cancelled() {
                    match result {
                        Outcome::Checked(report) => {
                            this.mcp_actions.report =
                                Some((report, chrono::Local::now().format("%H:%M:%S").to_string()))
                        }
                        outcome => {
                            this.mcp_actions.message = Some(match outcome {
                                Outcome::Completed => "pi-mcp-completed",
                                Outcome::Cancelled => "pi-mcp-cancelled",
                                Outcome::TimedOut if is_check => "pi-mcp-check-timeout",
                                Outcome::TimedOut => "pi-mcp-timeout",
                                _ if is_login => "pi-mcp-login-failed",
                                _ if is_check => "pi-mcp-check-failed",
                                _ => "pi-mcp-failed",
                            })
                        }
                    }
                }
                cx.notify();
                this.continuation.take()
            });
            if let Ok(Some(next)) = next {
                let _ = cx.update(|window, cx| next(window, cx));
            }
        })
        .detach();
        cx.notify();
        Some(generation)
    }
    fn mcp_dialog(
        entity: &Entity<Self>,
        name: String,
        login: bool,
        command: String,
        mount: Cancellation,
        window: &mut Window,
        cx: &mut App,
    ) {
        if entity.read(cx).busy() || entity.read(cx).dirty() || mount.is_cancelled() {
            return;
        }
        let generation = if login {
            entity.update(cx, |this, cx| {
                this.start_mcp(
                    Action::Login(name.clone()),
                    command.clone(),
                    mount.clone(),
                    window,
                    cx,
                )
            })
        } else {
            None
        };
        if login && generation.is_none() {
            return;
        }
        let owner = entity.downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("name", name.as_str());
            let cancel_owner = owner.clone();
            let title = t_with_args(
                cx,
                if login {
                    "pi-mcp-login-title"
                } else {
                    "pi-mcp-logout-title"
                },
                &args,
            );
            let dialog = dialog.title(text("mcp-dialog-title", title)).child(text(
                "mcp-dialog-help",
                t(
                    cx,
                    if login {
                        "pi-mcp-login-help"
                    } else {
                        "pi-mcp-logout-help"
                    },
                ),
            ));
            if login {
                let active = owner.upgrade().is_some_and(|owner| {
                    owner
                        .read(cx)
                        .mcp_actions
                        .running
                        .as_ref()
                        .is_some_and(|r| Some(r.generation) == generation)
                });
                let message = owner
                    .upgrade()
                    .and_then(|owner| owner.read(cx).mcp_actions.message);
                dialog
                    .child(text(
                        "mcp-login-terminal",
                        t_with_args(cx, "pi-mcp-login-terminal", &args),
                    ))
                    .when_some(message, |dialog, key| {
                        dialog.child(text("mcp-login-outcome", t(cx, key)))
                    })
                    .on_close(move |_, _, cx| {
                        let _ = cancel_owner.update(cx, |this, _| {
                            if this
                                .mcp_actions
                                .running
                                .as_ref()
                                .is_some_and(|r| Some(r.generation) == generation)
                            {
                                this.mcp_actions.cancel();
                            }
                        });
                    })
                    .footer(
                        DialogFooter::new().child(
                            Button::new("mcp-login-close")
                                .label(t(
                                    cx,
                                    if active {
                                        "action-cancel"
                                    } else {
                                        "pi-mcp-close"
                                    },
                                ))
                                .on_click(|_, window, cx| {
                                    // Dispatching Cancel invokes the dialog's close hook.
                                    window.dispatch_action(
                                        Box::new(gpui_kit::component::dialog::Cancel),
                                        cx,
                                    );
                                }),
                        ),
                    )
            } else {
                let owner = owner.clone();
                let name = name.clone();
                let command = command.clone();
                let mount = mount.clone();
                dialog
                    .on_ok(move |_, window, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.start_mcp(
                                Action::Logout(name.clone()),
                                command.clone(),
                                mount.clone(),
                                window,
                                cx,
                            );
                        });
                        true
                    })
                    .footer(super::super::dialog_buttons(
                        "pi-mcp-logout",
                        false,
                        false,
                        super::super::DialogCommit::Danger,
                        cx,
                    ))
            }
        });
    }
}

#[cfg(test)]
mod tests;
