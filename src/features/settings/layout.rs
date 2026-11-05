use super::*;
use crate::features::settings::composition::{
    PageLayout, SettingsPage, SettingsRow, SettingsSection, control,
};
use gpui_kit::component::Selectable;
use gpui_kit::component::Sizable;
use gpui_kit::component::ThemeMode as Mode;
use gpui_kit::component::ThemeRegistry;
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::FluentBuilder;
use gupi_resources::pi_resources::Kind;

/// Gupi preferences and Pi configuration are separate settings areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Domain {
    Gupi,
    Pi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PageId {
    General,
    Appearance,
    Notifications,
    Keyboard,
    About,
    Connection,
    Conversation,
    Network,
    Tools,
    Models,
    Auth,
    Mcp,
    Packages,
    Skills,
    Prompts,
}
impl PageId {
    pub fn pages(domain: Domain) -> &'static [Self] {
        match domain {
            Domain::Gupi => &[
                Self::General,
                Self::Appearance,
                Self::Notifications,
                Self::Keyboard,
                Self::About,
            ],
            Domain::Pi => &[
                Self::Connection,
                Self::Conversation,
                Self::Network,
                Self::Tools,
                Self::Models,
                Self::Auth,
                Self::Mcp,
                Self::Packages,
                Self::Skills,
                Self::Prompts,
            ],
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::General | Self::Connection => 0,
            Self::Appearance | Self::Conversation => 1,
            Self::Notifications | Self::Network => 2,
            Self::Keyboard | Self::Tools => 3,
            Self::About | Self::Models => 4,
            Self::Auth => 5,
            Self::Mcp => 6,
            Self::Packages => 7,
            Self::Skills => 8,
            Self::Prompts => 9,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::General => "settings-general",
            Self::Appearance => "settings-appearance",
            Self::Notifications => "settings-notifications",
            Self::Keyboard => "settings-keyboard",
            Self::About => "settings-about",
            Self::Connection => "settings-connection",
            Self::Conversation => "settings-conversation",
            Self::Network => "settings-network",
            Self::Tools => "settings-tools",
            Self::Models => "settings-models",
            Self::Auth => "settings-auth",
            Self::Mcp => "settings-mcp",
            Self::Packages => "settings-packages",
            Self::Skills => "settings-skills",
            Self::Prompts => "settings-prompts",
        }
    }
}
#[cfg(test)]
pub(super) const KEYBOARD_PAGE: PageId = PageId::Keyboard;
#[cfg(test)]
pub(super) const DOMAIN_PAGES: [(Domain, usize); 2] = [(Domain::Gupi, 5), (Domain::Pi, 10)];
pub(super) const PI_PAGE: PageId = PageId::Connection;
pub(super) const CONVERSATION_PAGE: PageId = PageId::Conversation;
pub(super) const PACKAGES_PAGE: PageId = PageId::Packages;
pub(super) const MODELS_PAGE: PageId = PageId::Models;
pub(super) const AUTH_PAGE: PageId = PageId::Auth;

impl SettingsView {
    /// Direct navigation owns page selection and clears a query that could hide it.
    pub(super) fn open_page(
        &mut self,
        domain: Domain,
        page: PageId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entering_pi = self.domain != domain && domain == Domain::Pi;
        self.domain = domain;
        self.navigation = page;
        self.query.clear();
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.settings_scroll.set_offset(Point::default());
        if entering_pi {
            self.pi_config.update(cx, |config, cx| config.activate(cx));
        }
        cx.notify();
    }
    pub(super) fn search_changed(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query;
        let query = self.query.trim().to_lowercase();
        let pages = self.settings_pages(cx);
        if !pages[self.navigation.index()].matches(&query)
            && let Some(ix) = pages.iter().position(|page| page.matches(&query))
        {
            self.navigation = PageId::pages(self.domain)[ix];
        }
        self.settings_scroll.set_offset(Point::default());
        cx.notify();
    }
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_changed(String::new(), cx);
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.search.focus_handle(cx).focus(window, cx);
    }

    pub(super) fn navigate_credentials(
        &mut self,
        page: CredentialPage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity().downgrade();
        let resources = self.resources.clone();
        let config = self.pi_config.clone();
        window.defer(cx, move |window, cx| {
            confirm_unsaved(
                resources,
                config,
                move |window, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.open_page(
                            Domain::Pi,
                            match page {
                                CredentialPage::Models => MODELS_PAGE,
                                CredentialPage::Auth => AUTH_PAGE,
                            },
                            window,
                            cx,
                        )
                    });
                },
                window,
                cx,
            );
        });
    }

    /// Switches area from the page bar. Leaving Pi asks about unsaved drafts.
    fn select_domain(&mut self, domain: Domain, window: &mut Window, cx: &mut Context<Self>) {
        if self.domain == domain {
            return;
        }
        if domain == Domain::Pi || !self.has_unsaved_pi(cx) {
            self.open_page(domain, PageId::pages(domain)[0], window, cx);
            return;
        }
        let this = cx.entity().downgrade();
        let (resources, pi_config) = (self.resources.clone(), self.pi_config.clone());
        // Deferred: the guards read this view, which is being updated here.
        window.defer(cx, move |window, cx| {
            confirm_unsaved(
                resources,
                pi_config,
                move |window, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.open_page(domain, PageId::pages(domain)[0], window, cx)
                    });
                },
                window,
                cx,
            );
        });
    }

    pub(crate) fn render_domain_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        use gpui_kit::component::tab::Tab;
        use gpui_kit::component::tab::TabBar;
        TabBar::new("settings-domain")
            .segmented()
            .small()
            .selected_index(usize::from(self.domain == Domain::Pi))
            .child(Tab::new().label(t(cx, "settings-domain-gupi")))
            .child(Tab::new().label(t(cx, "settings-domain-pi")))
            .on_click(cx.listener(|this, index: &usize, window, cx| {
                let domain = if *index == 1 {
                    Domain::Pi
                } else {
                    Domain::Gupi
                };
                this.select_domain(domain, window, cx);
            }))
            .into_any_element()
    }

    /// Opens the Pi conversation page scoped to a project directory.
    pub(crate) fn open_project_pi_settings(
        &mut self,
        cwd: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scope = match gupi_resources::pi_settings::Scope::Project(cwd).validated() {
            Ok(scope) => scope,
            Err(error) => {
                window.defer(cx, move |window, cx| {
                    window.push_notification(
                        gpui_kit::component::notification::Notification::error(error.to_string()),
                        cx,
                    );
                });
                return;
            }
        };
        let this = cx.entity().downgrade();
        let resources = self.resources.clone();
        // Unsaved editor text is settled first; the scope request then asks
        // about unsaved settings fields itself.
        window.defer(cx, move |window, cx| {
            resources::ResourcesView::confirm_leave(
                &resources,
                move |window, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.open_page(Domain::Pi, CONVERSATION_PAGE, window, cx);
                        let pi_config = this.pi_config.clone();
                        window.defer(cx, move |window, cx| {
                            pi_config::request_scope(&pi_config, scope, window, cx)
                        });
                    });
                },
                window,
                cx,
            )
        });
    }

    /// Returns the Pi area to the global scope. Unsaved edits keep their scope:
    /// they belong to it and are settled by the leave guard first.
    pub(crate) fn reset_pi_scope(&mut self, cx: &mut Context<Self>) {
        if self.has_unsaved_pi(cx) {
            return;
        }
        self.pi_config.update(cx, |config, cx| {
            config.set_scope(gupi_resources::pi_settings::Scope::Global, cx)
        });
    }

    /// Records the shown conversation's project; the scope menu lists it first.
    pub(crate) fn set_active_project(
        &mut self,
        cwd: Option<std::path::PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.pi_config
            .update(cx, |config, cx| config.set_active_project(cwd, cx));
    }

    /// Whether leaving must settle Pi drafts or an active settings/editor save.
    pub(crate) fn has_unsaved_pi(&self, cx: &App) -> bool {
        self.pi_config.read(cx).needs_leave_guard(cx)
            || self.resources.read(cx).needs_leave_guard(cx)
    }

    /// Runs `proceed` once unsaved Pi drafts are saved or discarded.
    pub(crate) fn confirm_leave(
        this: &Entity<Self>,
        proceed: impl FnOnce(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = this.read(cx);
        confirm_unsaved(
            view.resources.clone(),
            view.pi_config.clone(),
            proceed,
            window,
            cx,
        );
    }
    pub(super) fn settings_pages(&self, cx: &mut Context<Self>) -> Vec<SettingsPage> {
        let this = cx.entity().downgrade();
        let item = |key: &'static str,
                    aliases: &'static str,
                    render: fn(
            &mut SettingsView,
            &PageLayout,
            &mut Window,
            &mut Context<SettingsView>,
        ) -> AnyElement| {
            let this = this.clone();
            SettingsRow::content(move |layout, window, cx| {
                this.update(cx, |this, cx| render(this, layout, window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            })
            .keywords(vec![t(cx, key), aliases.to_owned()])
        };
        let page = |key: &'static str, items: Vec<SettingsRow>| {
            SettingsPage::new(t(cx, key))
                .icon(match key {
                    "settings-page-general" => IconName::Settings,
                    "settings-theme" => IconName::Sparkles,
                    "settings-page-pi" => IconName::Terminal,
                    "settings-page-keys" => IconName::Keyboard,
                    "settings-page-packages" => IconName::Puzzle,
                    "settings-page-skills" => IconName::BookOpen,
                    "settings-page-prompts" => IconName::FileText,
                    _ => IconName::Info,
                })
                .group(SettingsSection::new().items(items))
        };
        let general = SettingsPage::new(t(cx, "settings-page-general"))
            .icon(IconName::Settings)
            .group(
                SettingsSection::new()
                    .title(t(cx, "settings-language"))
                    .item({
                        let this = this.clone();
                        SettingsRow::new(
                            "settings-app-language",
                            t(cx, "settings-app-language"),
                            control(move |_, _, cx| {
                                this.update(cx, |this, cx| {
                                    Select::new(&this.language)
                                        .id("settings-language")
                                        .w(rems(12.))
                                        .accessibility_label(t(cx, "settings-app-language"))
                                        .cleanable(false)
                                        .disabled(this.controller.read(cx).busy(cx))
                                        .into_any_element()
                                })
                                .unwrap_or_else(|_| div().into_any_element())
                            }),
                        )
                        .description(t(cx, "settings-native-language-help"))
                        .keywords(["通用 general language locale 语言"])
                    }),
            )
            .group(
                SettingsSection::new()
                    .title(t(cx, "settings-config-heading"))
                    .item({
                        let this = this.clone();
                        SettingsRow::new(
                            "settings-config-location",
                            t(cx, "settings-config-location"),
                            control(move |options, _, cx| {
                                this.update(cx, |this, cx| this.render_config_actions(options, cx))
                                    .unwrap_or_else(|_| div().into_any_element())
                            }),
                        )
                        .description(
                            self.controller
                                .read(cx)
                                .path()
                                .map(|path| path.display().to_string())
                                .unwrap_or_default(),
                        )
                        .keywords(["通用 general config file 配置文件 reload 重新读取"])
                    }),
            );
        let general = general.group(
            SettingsSection::new()
                .title(t(cx, "settings-temporary-storage"))
                .item({
                    let controller = self.controller.clone();
                    SettingsRow::new(
                        "temporary-prewarm",
                        t(cx, "temporary-prewarm"),
                        control(move |_, _, cx| {
                            let checked = controller.read(cx).preferences(cx).prewarm_temporary;
                            let controller = controller.clone();
                            gpui_kit::component::switch::Switch::new("temporary-prewarm")
                                .accessibility_label(t(cx, "temporary-prewarm"))
                                .checked(checked)
                                .disabled(controller.read(cx).busy(cx))
                                .on_click(move |checked, _, cx| {
                                    controller.update(cx, |controller, cx| {
                                        controller.set_preference(
                                            PreferenceChange::PrewarmTemporary(*checked),
                                            cx,
                                        )
                                    });
                                })
                        }),
                    )
                    .description(t(cx, "temporary-prewarm-help"))
                    .keywords(["temporary prewarm 临时对话 预热"])
                })
                .item(
                    SettingsRow::new(
                        "temporary-clean-released",
                        t(cx, "temporary-clean-released"),
                        control(|_, _, cx| {
                            let owner = cx.try_global::<crate::app::temporary::Temporary>();
                            let busy = owner.is_some_and(|s| s.cleanup.is_some());
                            let result = owner.and_then(|s| s.cleanup_result.clone());
                            v_flex()
                                .items_start()
                                .gap_1()
                                .child(
                                    Button::new("temporary-clean-released")
                                        .small()
                                        .icon(IconName::Trash)
                                        .label(t(cx, "temporary-clean"))
                                        .loading(busy)
                                        .disabled(busy || owner.is_none())
                                        .on_click(|_, window, cx| {
                                            window.open_dialog(cx, |dialog, _, cx| {
                                                dialog
                                                    .title(t(cx, "temporary-clean-title"))
                                                    .child(t(cx, "temporary-clean-help"))
                                                    .footer(dialog_buttons(
                                                        "temporary-clean",
                                                        false,
                                                        false,
                                                        DialogCommit::Danger,
                                                        cx,
                                                    ))
                                                    .on_ok(|_, _, cx| {
                                                        crate::app::temporary::clean_released(cx);
                                                        true
                                                    })
                                            });
                                        }),
                                )
                                .children(result.map(|result| {
                                    div().text_sm().child(match result {
                                        Ok(count) => {
                                            format!("{}: {count}", t(cx, "temporary-cleaned"))
                                        }
                                        Err(error) => error,
                                    })
                                }))
                                .into_any_element()
                        }),
                    )
                    .description(t(cx, "temporary-clean-help"))
                    .keywords(["temporary cleanup 临时对话 清理"]),
                ),
        );
        #[cfg(target_os = "macos")]
        let general = general.group(SettingsSection::new().title(t(cx, "settings-permissions")).item(
            SettingsRow::new("settings-accessibility", t(cx, "settings-accessibility"), control(|_, _, cx| {
                h_flex().child(Button::new("accessibility-settings").small().label(t(cx, "settings-open-system-settings"))
                    .on_click(|_, _, cx| cx.open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")))
            })).description(t(cx, "settings-accessibility-help"))
        ));
        let notifications = self.notification_settings(cx);
        let appearance = SettingsPage::new(t(cx, "settings-theme"))
            .icon(IconName::Sparkles)

            .group(SettingsSection::new().title(t(cx, "setup-color-mode")).item({
                let this = this.clone();
                SettingsRow::new("settings-color-mode-label", t(cx, "settings-color-mode-label"),
                    control(move |_, _, cx| {
                        this.update(cx, |this, cx| this.render_mode(cx))
                            .unwrap_or_else(|_| div().into_any_element())
                    }),
                )
                .description(t(cx, "settings-mode-help"))
                .keywords([
                    "外观 appearance color mode dark light system 颜色模式 深色 浅色 跟随系统",
                ])
            }))
            .group(
                SettingsSection::new()
                    .title(t(cx, "settings-icon-theme"))
                    .description(t(cx, icon_help_key()))
                    .item(item(
                        "settings-icon-theme",
                        "icon logo dock 图标 标记 配色",
                        |this, _, _, cx| this.render_icon_themes(false, cx),
                    )),
            )
            .group(
                SettingsSection::new()
                    .title(t(cx, "light-themes"))
                    .description(theme_group_help(Mode::Light, cx))
                    .item(item(
                        "light-themes",
                        "外观 appearance light theme 浅色主题",
                        |this, layout, _, cx| this.render_theme_choices(Mode::Light, layout.width, cx),
                    )),
            )
            .group(
                SettingsSection::new()
                    .title(t(cx, "dark-themes"))
                    .description(theme_group_help(Mode::Dark, cx))
                    .item(item(
                        "dark-themes",
                        "外观 appearance dark theme 深色主题",
                        |this, layout, _, cx| this.render_theme_choices(Mode::Dark, layout.width, cx),
                    )),
            );
        let pi = page(
            "settings-page-pi",
            vec![item(
                "settings-pi-command",
                "pi executable path environment version 路径 环境 检查",
                |this, layout, _, cx| {
                    let probe = this.applied_pi.clone();
                    let command = this.controller.read(cx).preferences(cx).pi_command;
                    if !probe.read(cx).matches_command(command.as_deref()) {
                        cx.defer(move |cx| {
                            probe.update(cx, |probe, cx| probe.request(command, false, cx))
                        });
                    }
                    this.render_pi_settings(layout, cx)
                },
            )],
        );
        let keys = self.keys.clone();
        let keys_view = self.keys.read(cx);
        let filtering = keys_view.filtering(cx);
        let sessions_open = keys_view.sessions_open();
        let command_groups: Vec<_> = keys::Group::ALL
            .into_iter()
            .filter_map(|group| {
                let mut items = Vec::new();
                let mut sessions_added = false;
                for (index, command) in gupi_settings::keybindings::COMMANDS.iter().enumerate() {
                    if keys::Group::of(command.kind) != group {
                        continue;
                    }
                    let session = matches!(
                        command.kind,
                        gupi_settings::commands::Kind::TemporarySession(_)
                    );
                    if session && !filtering {
                        if !sessions_added {
                            items.push(keys::KeysView::sessions_item(&self.keys, cx));
                            sessions_added = true;
                        }
                        if !sessions_open {
                            continue;
                        }
                    }
                    if keys_view.matches(index, cx) {
                        items.push(keys::KeysView::item(&self.keys, index, cx));
                    }
                }
                (!items.is_empty()).then(|| {
                    SettingsSection::new()
                        .bindings()
                        .title(t(cx, group.label()))
                        .items(items)
                })
            })
            .collect();
        let keyboard = SettingsPage::new(t(cx, "settings-page-keys"))
            .icon(IconName::Keyboard)
            .group(keys::KeysView::actions(&keys))
            .groups(global_keys::GlobalKeys::groups(
                &self.global_keys,
                keys_view,
                cx,
            ))
            .groups((!command_groups.is_empty()).then(|| {
                SettingsSection::new().item(
                    SettingsRow::content(|_, _, cx| {
                        v_flex()
                            .w_full()
                            .min_w_0()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .pt_6()
                            .gap_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t(cx, "settings-key-app-heading")),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t(cx, "settings-key-app-help")),
                            )
                    })
                    .keywords(vec![
                        t(cx, "settings-key-app-heading"),
                        t(cx, "settings-key-app-help"),
                    ]),
                )
            }))
            .groups(command_groups);
        let resource_page = |key, kind, keywords: &str| {
            let resource_view = self.resources.downgrade();
            let section = |section, aliases: &str| {
                let resources = resource_view.clone();
                SettingsRow::content(move |layout, _, cx| {
                    resources
                        .update(cx, |view, cx| {
                            view.render_page(kind, section, layout.width, cx)
                        })
                        .unwrap_or_else(|_| div().into_any_element())
                })
                .keywords(vec![
                    t(cx, key),
                    aliases.to_owned(),
                    "refresh reload 刷新 重新读取".to_owned(),
                ])
            };
            let header = resource_view.clone();
            // Resource pages follow one shared Pi scope.
            let scope = self.pi_config.clone();
            let project = kind != Kind::Extension && self.resources.read(cx).in_project_scope();
            let mut page = SettingsPage::new(t(cx, key))
                .icon(match kind {
                    Kind::Extension => IconName::Puzzle,
                    Kind::Skill => IconName::BookOpen,
                    _ => IconName::FileText,
                })
                .scope(move |_, window, cx| {
                    scope.update(cx, |config, cx| config.render_scope(false, window, cx))
                })
                .actions(move |_, _, cx| {
                    header
                        .update(cx, |view, cx| view.render_header(kind, cx))
                        .unwrap_or_else(|_| div().into_any_element())
                });
            if self.resources.read(cx).has_status(kind, cx) {
                page = page.group(SettingsSection::new().item(section(
                    resources::Section::Overview,
                    "refresh reload 刷新 重新读取",
                )));
            }
            if kind == Kind::Extension {
                let resources = self.resources.clone();
                page = page.header(move |_, _, cx| {
                    resources.update(cx, |view, cx| {
                        h_flex()
                            .w_full()
                            .justify_end()
                            .child(view.render_packages_tabs(cx))
                    })
                });
                return page.group(SettingsSection::new()
                    .item(section(resources::Section::Packages,"package extension skill theme template install update remove source version 包 扩展 技能 主题 模板 安装 更新 移除 来源 版本")));
            }
            let inherited_title = t(cx, "settings-resource-group-inherited");
            if kind == Kind::Prompt {
                page = page
                    .group(
                        SettingsSection::new()
                            .title(t(cx, "settings-system-heading"))
                            .items(resources::ResourcesView::system_prompt_items(
                                &self.resources,
                                cx,
                            )),
                    )
                    .group(
                        SettingsSection::new()
                            .title(t(cx, "settings-template-heading"))
                            .item(section(resources::Section::Catalog, keywords))
                            .items(resources::ResourcesView::prompt_items(
                                &self.resources,
                                false,
                                cx,
                            )),
                    );
                if project {
                    page = page.group(SettingsSection::new().title(inherited_title).items(
                        resources::ResourcesView::prompt_items(&self.resources, true, cx),
                    ));
                }
                return page;
            }
            let mut own = SettingsSection::new();
            if project {
                own = own.title(t(cx, "settings-resource-group-project"));
            }
            page = page.group(
                own.item(section(resources::Section::Catalog, keywords))
                    .items(resources::ResourcesView::skill_items(
                        &self.resources,
                        false,
                        cx,
                    )),
            );
            if project {
                page = page.group(SettingsSection::new().title(inherited_title).items(
                    resources::ResourcesView::skill_items(&self.resources, true, cx),
                ));
            }
            page
        };
        let probe = self.applied_pi.read(cx);
        let unavailable = t(cx, "settings-about-unavailable");
        let mut about_items: Vec<_> = [
            (
                "settings-about-gupi",
                env!("CARGO_PKG_VERSION").to_owned(),
                "about gupi version 关于 版本",
            ),
            (
                "settings-about-pi",
                probe
                    .data()
                    .map(|p| p.version.clone())
                    .unwrap_or_else(|| unavailable.clone()),
                "about pi version 关于 版本",
            ),
            (
                "settings-about-path",
                probe
                    .data()
                    .map(|data| data.command.display().to_string())
                    .unwrap_or_else(|| unavailable.clone()),
                "about pi path executable 关于 路径",
            ),
            (
                "settings-about-status",
                if probe.is_running() {
                    t(cx, "startup-checking")
                } else if let Some(error) = probe.problem() {
                    t(cx, gupi_settings::i18n::ProbeFailureKey::key(error))
                } else if probe.data().is_some() {
                    t(cx, "home-ready")
                } else {
                    unavailable
                },
                "about pi status probe 关于 检查 状态",
            ),
        ]
        .into_iter()
        .map(|(key, value, aliases)| {
            let navigate = this.clone();
            SettingsRow::new(
                key,
                t(cx, key),
                control(move |_, _, cx| {
                    let tooltip = value.clone();
                    let value = div()
                        .id(key)
                        .max_w(rems(20.))
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .child(value.clone())
                        .tooltip(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tooltip.clone())
                                .build(window, cx)
                        });
                    h_flex().min_w_0().flex_wrap().gap_2().child(value).when(
                        key == "settings-about-status",
                        |row| {
                            let navigate = navigate.clone();
                            row.child(
                                Button::new("about-open-pi")
                                    .small()
                                    .outline()
                                    .label(t(cx, "settings-open-pi"))
                                    .on_click(move |_, window, cx| {
                                        navigate
                                            .update(cx, |this, cx| {
                                                this.open_page(Domain::Pi, PI_PAGE, window, cx)
                                            })
                                            .ok();
                                    }),
                            )
                        },
                    )
                }),
            )
            .keywords(vec![aliases])
        })
        .collect();
        let gupi_item = about_items.remove(0);
        let updates = self.updates.clone();
        let controller = self.controller.clone();
        let about = SettingsPage::new(t(cx, "settings-page-about"))
            .icon(IconName::Info)
            .group(
                SettingsSection::new()
                    .title("Gupi")
                    .item(gupi_item)
                    .item(
                        SettingsRow::content(move |layout, _, cx| {
                            updates.update(cx, |view, _| view.set_settings_axis(layout.axis));
                            updates.clone()
                        })
                        .framed(true)
                        .keywords(["updates version download 更新 版本 下载"]),
                    )
                    .item(
                        SettingsRow::new(
                            "updates-automatic",
                            t(cx, "updates-automatic"),
                            control(move |_, _, cx| {
                                let checked = controller
                                    .read(cx)
                                    .preferences(cx)
                                    .checks_updates_automatically();
                                let busy = controller.read(cx).busy(cx);
                                let controller = controller.clone();
                                gpui_kit::component::switch::Switch::new("updates-automatic")
                                    .accessibility_label(t(cx, "updates-automatic"))
                                    .checked(checked)
                                    .disabled(busy)
                                    .on_click(move |checked, _, cx| {
                                        controller.update(cx, |owner, cx| {
                                            owner.set_preference(
                                                PreferenceChange::AutoCheckUpdates(*checked),
                                                cx,
                                            )
                                        });
                                    })
                            }),
                        )
                        .description(t(cx, "updates-automatic-help")),
                    ),
            )
            .group(
                SettingsSection::new()
                    .title(t(cx, "settings-diagnostics"))
                    .item(
                        SettingsRow::new(
                            "settings-diagnostics",
                            t(cx, "settings-diagnostics"),
                            control(|_, _, cx| {
                                h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        Button::new("diagnostics-copy")
                                            .small()
                                            .label(t(cx, "menu-copy-diagnostics"))
                                            .on_click(|_, _, cx| {
                                                crate::app::menus::copy_diagnostics(cx)
                                            }),
                                    )
                                    .child(
                                        Button::new("diagnostics-logs")
                                            .small()
                                            .icon(IconName::FolderOpen)
                                            .label(t(cx, "menu-logs"))
                                            .on_click(|_, _, cx| crate::app::menus::show_logs(cx)),
                                    )
                            }),
                        )
                        .description(t(cx, "settings-diagnostics-help")),
                    ),
            )
            .group(
                SettingsSection::new()
                    .title("Pi")
                    .description(t(cx, "settings-about-help"))
                    .items(about_items),
            );
        let pages = match self.domain {
            Domain::Gupi => vec![general, appearance, notifications, keyboard, about],
            Domain::Pi => vec![
                pi.title(t(cx, "settings-page-connection")),
                pi_config::PiConfig::page(&self.pi_config, pi_config::Page::Conversation, cx),
                pi_config::PiConfig::page(&self.pi_config, pi_config::Page::Network, cx),
                pi_config::PiConfig::page(&self.pi_config, pi_config::Page::Tools, cx),
                super::config_files::FilePages::page(
                    &self.pi_config.read(cx).file_pages,
                    gupi_resources::pi_config_files::Kind::Models,
                    &self.pi_config,
                    cx,
                ),
                super::auth::AuthPage::page(&self.auth, cx),
                super::config_files::FilePages::page(
                    &self.pi_config.read(cx).file_pages,
                    gupi_resources::pi_config_files::Kind::Mcp,
                    &self.pi_config,
                    cx,
                ),
                resource_page(
                    "settings-page-packages",
                    Kind::Extension,
                    "package plugin extension install update remove source version path enable disable browse search npm 插件 扩展 包 安装 更新 移除 启停 浏览 搜索",
                ),
                resource_page(
                    "settings-page-skills",
                    Kind::Skill,
                    "skill name description source path create edit delete enable disable 技能 创建 编辑 删除 启停",
                ),
                resource_page(
                    "settings-page-prompts",
                    Kind::Prompt,
                    "prompt template 创建 编辑 删除 模板",
                ),
            ],
        };
        pages
            .into_iter()
            .zip(PageId::pages(self.domain))
            .map(|(page, id)| {
                let page = if matches!(id, PageId::Connection | PageId::Auth) {
                    let config = self.pi_config.clone();
                    page.scope(move |_, window, cx| {
                        config.update(cx, |config, cx| config.render_scope(true, window, cx))
                    })
                } else {
                    page
                };
                page.description(t(cx, &format!("{}-description", id.id())))
            })
            .collect()
    }
    pub(super) fn render_settings(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use gpui_kit::component::empty::{
            Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle,
        };
        use gpui_kit::component::resizable::{h_resizable, resizable_panel};
        use gpui_kit::component::scroll::ScrollableElement as _;
        use gpui_kit::component::sidebar::{SidebarItem as _, SidebarMenuItem};
        let pages = self.settings_pages(cx);
        let query = self.query.trim().to_lowercase();
        let matches = pages
            .iter()
            .map(|page| page.matches(&query))
            .collect::<Vec<_>>();
        let empty = !matches.iter().any(|value| *value);
        let selected = if matches[self.navigation.index()] || empty {
            self.navigation
        } else {
            PageId::pages(self.domain)[matches
                .iter()
                .position(|value| *value)
                .expect("matching page")]
        };
        let mut menu = v_flex().gap_1();
        for (ix, page) in pages.iter().enumerate() {
            if !empty && !matches[ix] {
                continue;
            }
            let id = PageId::pages(self.domain)[ix];
            let domain = self.domain;
            let item = SidebarMenuItem::new(page.title.clone())
                .active(selected == id)
                .when_some(page.icon.clone(), |item, icon| item.icon(icon))
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_page(domain, id, window, cx)),
                );
            menu = menu.child(item.render(
                SharedString::from(format!("settings-nav-{}", id.id())),
                window,
                cx,
            ));
        }
        let sidebar = v_flex()
            .id("settings-sidebar")
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .child(
                div().p_4().child(
                    Input::new(&self.search)
                        .aria_label(t(cx, "settings-search"))
                        .id("settings-search")
                        .prefix(gpui_kit::component::Icon::new(IconName::Search))
                        .cleanable(true),
                ),
            )
            .child(
                v_flex()
                    .id("settings-navigation")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.settings_nav_scroll)
                    .vertical_scrollbar(&self.settings_nav_scroll)
                    .child(div().px_2().pb_2().child(menu)),
            );
        let page = pages
            .into_iter()
            .nth(selected.index())
            .expect("settings destination");
        let scroll = self.settings_scroll.clone();
        let problem = self
            .controller
            .read(cx)
            .configuration()
            .read(cx, |op| op.problem().map(|p| p.key()));
        let error = self.error.as_deref().or(problem).map(|key| t(cx, key));
        let mut args = fluent_bundle::FluentArgs::new();
        args.set(
            "domain",
            t(
                cx,
                if self.domain == Domain::Gupi {
                    "settings-domain-gupi"
                } else {
                    "settings-domain-pi"
                },
            ),
        );
        args.set("query", self.query.clone());
        let empty_description =
            gupi_settings::i18n::t_with_args(cx, "settings-search-empty-description", &args);
        let empty_title = t(cx, "settings-search-empty");
        let clear = Button::new("settings-clear-search")
            .outline()
            .label(t(cx, "settings-clear-search"))
            .on_click(cx.listener(|this, _, window, cx| this.clear_search(window, cx)));
        let content = container_query(move |size, window, cx| {
            let rem = window.rem_size();
            let max_width = if matches!(
                selected,
                PageId::Keyboard
                    | PageId::Models
                    | PageId::Mcp
                    | PageId::Packages
                    | PageId::Skills
                    | PageId::Prompts
            ) {
                rem * 59.
            } else {
                rem * 52.
            };
            let width = (size.width - rem * 4.).max(Pixels::ZERO).min(max_width);
            let layout = PageLayout::new(width, rem);
            let content = if empty {
                Empty::new()
                    .min_h(rems(24.))
                    .header(
                        EmptyHeader::new()
                            .title(EmptyTitle::new().child(empty_title))
                            .description(EmptyDescription::new().child(empty_description)),
                    )
                    .content(EmptyContent::new().child(clear))
                    .into_any_element()
            } else {
                page.content(query, layout).into_any_element()
            };
            div()
                .id(selected.id())
                .relative()
                .size_full()
                .min_w_0()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .vertical_scrollbar(&scroll)
                .child(
                    v_flex()
                        .w_full()
                        .max_w(width)
                        .mx_auto()
                        .py_8()
                        .gap_4()
                        .when_some(error, |body, text| {
                            body.child(div().text_sm().text_color(cx.theme().danger).child(text))
                        })
                        .child(content),
                )
        });
        let rem = window.rem_size();
        h_resizable("settings-split")
            .with_state(&self.settings_split)
            .child(
                resizable_panel()
                    .size(rem * 15.625)
                    .size_range(rem * 10. ..rem * 22.5)
                    .child(sidebar),
            )
            .child(
                resizable_panel()
                    .size_range(rem * 24. ..Pixels::MAX)
                    .child(content),
            )
            .into_any_element()
    }

    pub(super) fn render_icon_themes(&self, heading: bool, cx: &Context<Self>) -> AnyElement {
        use gupi_settings::icons::IconTheme;
        let config = self.controller.read(cx).preferences(cx);
        let busy = self.controller.read(cx).busy(cx);
        let mut group = h_flex().w_full().items_start().flex_wrap().gap_3();
        for icon in IconTheme::ALL {
            let controller = self.controller.clone();
            let selected = config.icon_theme == icon;
            let hover_group: SharedString = format!("icon-preview-{}", icon.label()).into();
            group = group.child(
                div()
                    .id(hover_group.clone())
                    .group(hover_group.clone())
                    .w(rems(5.75))
                    .min_w_0()
                    .child(
                        Button::new(icon.label())
                            .custom(gpui_kit::component::button::ButtonCustomVariant::new(cx))
                            .w_full()
                            .h_auto()
                            .p_1()
                            .whitespace_normal()
                            .selected(selected)
                            .toggled(selected)
                            .disabled(busy)
                            .accessibility_label(t(cx, icon.label()))
                            .child(
                                v_flex()
                                    .w_full()
                                    .min_w_0()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .size(rems(4.75))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .border_1()
                                            .rounded_lg()
                                            .border_color(if selected {
                                                cx.theme().primary
                                            } else {
                                                cx.theme().transparent
                                            })
                                            .when(!selected, |preview| {
                                                preview.group_hover(hover_group, |style| {
                                                    style.border_color(cx.theme().muted_foreground)
                                                })
                                            })
                                            .child(
                                                img(SharedString::from(icon.preview()))
                                                    .size(rems(3.5)),
                                            ),
                                    )
                                    .child(preferences::preview_caption(
                                        t(cx, icon.label()).into(),
                                        selected,
                                        cx,
                                    )),
                            )
                            .on_click(move |_, _, cx| {
                                controller.update(cx, |owner, cx| {
                                    owner.set_preference(PreferenceChange::IconTheme(icon), cx)
                                })
                            }),
                    ),
            );
        }
        v_flex()
            .gap_2()
            .when(heading, |view| {
                view.child(t(cx, "settings-icon-theme")).child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, icon_help_key())),
                )
            })
            .child(group)
            .into_any_element()
    }
    fn render_mode(&self, cx: &Context<Self>) -> AnyElement {
        let modes = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];
        let current = self.controller.read(cx).preferences(cx).theme;
        gpui_kit::component::button::ButtonGroup::new("settings-mode")
            .outline()
            .small()
            .disabled(self.controller.read(cx).busy(cx))
            .children(
                [
                    ("theme-system", modes[0]),
                    ("theme-light", modes[1]),
                    ("theme-dark", modes[2]),
                ]
                .map(|(key, mode)| Button::new(key).label(t(cx, key)).selected(current == mode)),
            )
            .on_click(cx.listener(move |this, clicked: &Vec<usize>, _, cx| {
                if let Some(mode) = clicked.first().and_then(|index| modes.get(*index)) {
                    this.controller.update(cx, |c, cx| {
                        c.set_preference(PreferenceChange::Theme(*mode), cx)
                    })
                }
            }))
            .into_any_element()
    }
    fn render_theme_choices(&self, mode: Mode, width: Pixels, cx: &Context<Self>) -> AnyElement {
        let draft = self.controller.read(cx).preferences(cx);
        preferences::theme_grid_content(
            (
                if mode == Mode::Light {
                    "light-themes"
                } else {
                    "dark-themes"
                },
                mode,
                app_theme::theme_choices(ThemeRegistry::global(cx), mode, &[]),
            ),
            &draft,
            &self.controller,
            preferences::theme_columns(width.as_f32()),
            self.controller.read(cx).busy(cx),
            cx,
        )
    }
}

impl SettingsView {
    fn notification_settings(&self, cx: &Context<Self>) -> SettingsPage {
        use gupi_settings::notifications::CompletionMode;
        use gupi_settings::notifications::Preferences;
        let controller = self.controller.clone();
        type Toggle = (&'static str, fn(&mut Preferences) -> &mut bool);
        let toggle = |(label, field): Toggle| {
            let controller = controller.clone();
            SettingsRow::new(
                label,
                t(cx, label),
                control(move |_, _, cx| {
                    let mut p = controller.read(cx).preferences(cx).notifications;
                    let controller = controller.clone();
                    gpui_kit::component::switch::Switch::new(label)
                        .accessibility_label(t(cx, label))
                        .checked(*field(&mut p))
                        .disabled(controller.read(cx).busy(cx))
                        .on_click(move |checked, _, cx| {
                            let mut p = controller.read(cx).preferences(cx).notifications;
                            *field(&mut p) = *checked;
                            controller.update(cx, |c, cx| {
                                c.set_preference(PreferenceChange::Notifications(p), cx)
                            });
                        })
                }),
            )
            .keywords(["notifications background 提醒 通知 后台"])
        };
        let background = SettingsSection::new()
            .title(t(cx, "settings-notification-background-heading"))
            .description(t(cx, "settings-notification-help"))
            .items(
                [
                    (
                        "settings-notification-waiting",
                        (|p: &mut Preferences| &mut p.waiting) as fn(&mut Preferences) -> &mut bool,
                    ),
                    ("settings-notification-attention", |p: &mut Preferences| {
                        &mut p.attention
                    }),
                    ("settings-notification-failures", |p: &mut Preferences| {
                        &mut p.failures
                    }),
                    ("settings-notification-plugins", |p: &mut Preferences| {
                        &mut p.plugins
                    }),
                ]
                .map(toggle),
            );
        let completion = SettingsSection::new()
            .title(t(cx, "settings-notification-completion-heading"))
            .item(
                SettingsRow::new(
                    "settings-notification-completion",
                    t(cx, "settings-notification-completion"),
                    control(move |_, _, cx| {
                        let modes = [
                            CompletionMode::Off,
                            CompletionMode::Background,
                            CompletionMode::Always,
                        ];
                        let value = controller.read(cx).preferences(cx).notifications.completion;
                        let controller = controller.clone();
                        gpui_kit::component::radio::RadioGroup::horizontal(
                            "notification-completion",
                        )
                        .children([
                            t(cx, "notification-off"),
                            t(cx, "notification-background"),
                            t(cx, "notification-always"),
                        ])
                        .selected_index(modes.iter().position(|m| *m == value))
                        .disabled(controller.read(cx).busy(cx))
                        .on_click(move |index, _, cx| {
                            let mut p = controller.read(cx).preferences(cx).notifications;
                            p.completion = modes[*index];
                            controller.update(cx, |c, cx| {
                                c.set_preference(PreferenceChange::Notifications(p), cx)
                            });
                        })
                    }),
                )
                .keywords(["notifications completion 回答完成 通知"]),
            );
        let page = SettingsPage::new(t(cx, "settings-notifications"))
            .icon(IconName::Bell)
            .group(background)
            .group(completion);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let page = page.group(
            SettingsSection::new()
                .title(t(cx, "settings-permissions"))
                .item(
                    SettingsRow::new("settings-notification-permission-open", t(cx, "settings-notification-permission-open"),
                        control(|_, _, cx| {
                            Button::new("notification-system-settings")
                                .small()
                                .label(t(cx, "settings-open-system-settings"))
                                .on_click(|_, _, cx| {
                                    #[cfg(target_os = "macos")]
                                    cx.open_url(
                                        "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
                                    );
                                    #[cfg(target_os = "windows")]
                                    cx.open_url("ms-settings:notifications");
                                })
                        }),
                    )
                    .description(t(cx, notification_permission_help_key()))
                    .keywords(["notifications permission system settings 通知 权限 系统设置"]),
                ),
        );
        page
    }
}

fn icon_help_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "settings-icon-theme-help"
    } else {
        "settings-icon-theme-help-other"
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn notification_permission_help_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "settings-notification-permission-help"
    } else {
        "settings-notification-permission-help-windows"
    }
}

// The applied theme mode tells which grid is in effect, including System mode.
fn theme_group_help(mode: Mode, cx: &App) -> String {
    let active = cx.theme().mode == mode;
    t(
        cx,
        match (mode, active) {
            (Mode::Light, true) => "settings-light-help-active",
            (Mode::Light, false) => "settings-light-help",
            (Mode::Dark, true) => "settings-dark-help-active",
            (Mode::Dark, false) => "settings-dark-help",
        },
    )
}

/// Settles unsaved resource editor text, then unsaved settings fields, then
/// runs `proceed`. Each file saves on its own; a failed save stops the chain.
fn confirm_unsaved(
    resources: Entity<resources::ResourcesView>,
    pi_config: Entity<pi_config::PiConfig>,
    proceed: impl FnOnce(&mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    resources::ResourcesView::confirm_leave(
        &resources,
        move |window, cx| pi_config::PiConfig::confirm_leave(&pi_config, proceed, window, cx),
        window,
        cx,
    );
}
