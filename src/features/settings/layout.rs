use super::*;
use gpui_kit::component::Selectable;
use gpui_kit::component::Sizable;
use gpui_kit::component::ThemeMode as Mode;
use gpui_kit::component::ThemeRegistry;
use gpui_kit::component::WindowExt;
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::SelectIndex;
use gpui_kit::component::setting::SettingField;
use gpui_kit::component::setting::SettingGroup;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::component::setting::SettingPage;
use gpui_kit::component::setting::Settings;
use gpui_kit::prelude::FluentBuilder;
use gupi_resources::pi_resources::Kind;

/// Gupi preferences and Pi configuration are separate settings areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Domain {
    Gupi,
    Pi,
}

// Page order within each domain, shared by the panel and navigation.
#[cfg(test)]
pub(super) const KEYBOARD_PAGE: usize = 3;
#[cfg(test)]
pub(super) const DOMAIN_PAGES: [(Domain, usize); 2] = [(Domain::Gupi, 5), (Domain::Pi, 6)];
pub(super) const PI_PAGE: usize = 0;
pub(super) const CONVERSATION_PAGE: usize = 1;
pub(super) const PACKAGES_PAGE: usize = 3;

impl SettingsView {
    /// Settings selection lives in the component's keyed state; a new key
    /// re-creates it with the requested page.
    pub(super) fn open_page(&mut self, domain: Domain, page_ix: usize, cx: &mut Context<Self>) {
        self.domain = domain;
        if domain == Domain::Pi {
            self.pi_config.update(cx, |config, cx| config.activate(cx));
        }
        self.navigation = (self.navigation.0 + 1, page_ix);
        cx.notify();
    }

    /// Switches area from the page bar. Leaving Pi asks about unsaved drafts.
    fn select_domain(&mut self, domain: Domain, window: &mut Window, cx: &mut Context<Self>) {
        if self.domain == domain {
            return;
        }
        if domain == Domain::Pi || !self.has_unsaved_pi(cx) {
            self.open_page(domain, 0, cx);
            return;
        }
        let this = cx.entity().downgrade();
        let (resources, pi_config) = (self.resources.clone(), self.pi_config.clone());
        // Deferred: the guards read this view, which is being updated here.
        window.defer(cx, move |window, cx| {
            confirm_unsaved(
                resources,
                pi_config,
                move |_, cx| {
                    let _ = this.update(cx, |this, cx| this.open_page(domain, 0, cx));
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
        let this = cx.entity().downgrade();
        let resources = self.resources.clone();
        // Unsaved editor text is settled first; the scope request then asks
        // about unsaved settings fields itself.
        window.defer(cx, move |window, cx| {
            resources::ResourcesView::confirm_leave(
                &resources,
                move |window, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.open_page(Domain::Pi, CONVERSATION_PAGE, cx);
                        let pi_config = this.pi_config.clone();
                        window.defer(cx, move |window, cx| {
                            pi_config::request_scope(
                                &pi_config,
                                gupi_resources::pi_settings::Scope::Project(cwd),
                                window,
                                cx,
                            )
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

    /// Whether any Pi settings field or resource editor holds unsaved changes.
    pub(crate) fn has_unsaved_pi(&self, cx: &App) -> bool {
        self.pi_config.read(cx).has_unsaved() || self.resources.read(cx).has_unsaved(cx)
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
    pub(super) fn settings_panel(&self, cx: &mut Context<Self>) -> Settings {
        let this = cx.entity().downgrade();
        let item = |key: &'static str,
                    aliases: &'static str,
                    render: fn(
            &mut SettingsView,
            &mut Window,
            &mut Context<SettingsView>,
        ) -> AnyElement| {
            let this = this.clone();
            SettingItem::render(move |_, window, cx| {
                this.update(cx, |this, cx| render(this, window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            })
            .keywords(vec![t(cx, key), aliases.to_owned()])
        };
        let page = |key: &'static str, items: Vec<SettingItem>| {
            SettingPage::new(t(cx, key))
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
                .resettable(false)
                .group(SettingGroup::new().items(items))
        };
        let general = SettingPage::new(t(cx, "settings-page-general"))
            .icon(IconName::Settings)
            .resettable(false)
            .group(SettingGroup::new().title(t(cx, "settings-language")).item({
                let this = this.clone();
                SettingItem::new(
                    t(cx, "settings-app-language"),
                    SettingField::render(move |_, _, cx| {
                        this.update(cx, |this, cx| {
                            gpui_kit::component::combobox::Combobox::new(&this.language)
                                .w(px(220.))
                                .cleanable(false)
                                .disabled(this.controller.read(cx).busy(cx))
                                .into_any_element()
                        })
                        .unwrap_or_else(|_| div().into_any_element())
                    }),
                )
                .description(t(cx, "settings-native-language-help"))
                .keywords(["通用 general language locale 语言"])
            }))
            .group(
                SettingGroup::new()
                    .title(t(cx, "settings-config-heading"))
                    .item({
                        let this = this.clone();
                        SettingItem::new(
                            t(cx, "settings-config-location"),
                            SettingField::render(move |options, _, cx| {
                                this.update(cx, |this, cx| this.render_config_actions(options, cx))
                                    .unwrap_or_else(|_| div().into_any_element())
                            }),
                        )
                        .keywords(["通用 general config file 配置文件 reload 重新读取"])
                    }),
            );
        let general = general.group(
            SettingGroup::new()
                .title(t(cx, "settings-temporary-storage"))
                .item(
                    SettingItem::new(
                        t(cx, "temporary-clean-released"),
                        SettingField::render(|_, _, cx| {
                            let owner = cx.try_global::<crate::app::temporary::Temporary>();
                            let busy = owner.is_some_and(|s| s.cleanup.is_some());
                            let result = owner.and_then(|s| s.cleanup_result.clone());
                            v_flex()
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
                                                    .title(t(cx, "temporary-clean-released"))
                                                    .child(t(cx, "temporary-clean-help"))
                                                    .footer(dialog_buttons(
                                                        "temporary-clean",
                                                        false,
                                                        false,
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
        let general = general.group(SettingGroup::new().title(t(cx, "settings-permissions")).item(
            SettingItem::new(t(cx, "settings-accessibility"), SettingField::render(|_, _, cx| {
                Button::new("accessibility-settings").small().label(t(cx, "settings-open-system-settings"))
                    .on_click(|_, _, cx| cx.open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"))
            })).description(t(cx, "settings-accessibility-help"))
        ));
        let notifications = self.notification_settings(cx);
        let appearance = SettingPage::new(t(cx, "settings-theme"))
            .icon(IconName::Sparkles)
            .resettable(false)
            .group(SettingGroup::new().title(t(cx, "setup-color-mode")).item({
                let this = this.clone();
                SettingItem::new(
                    t(cx, "settings-color-mode-label"),
                    SettingField::render(move |_, _, cx| {
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
                SettingGroup::new()
                    .title(t(cx, "settings-icon-theme"))
                    .description(t(cx, icon_help_key()))
                    .item(item(
                        "settings-icon-theme",
                        "icon logo dock 图标 标记 配色",
                        |this, _, cx| this.render_icon_themes(false, cx),
                    )),
            )
            .group(
                SettingGroup::new()
                    .title(t(cx, "light-themes"))
                    .description(theme_group_help(Mode::Light, cx))
                    .item(item(
                        "light-themes",
                        "外观 appearance light theme 浅色主题",
                        |this, _, cx| this.render_theme_choices(Mode::Light, cx),
                    )),
            )
            .group(
                SettingGroup::new()
                    .title(t(cx, "dark-themes"))
                    .description(theme_group_help(Mode::Dark, cx))
                    .item(item(
                        "dark-themes",
                        "外观 appearance dark theme 深色主题",
                        |this, _, cx| this.render_theme_choices(Mode::Dark, cx),
                    )),
            );
        let pi = page(
            "settings-page-pi",
            vec![item(
                "settings-pi-command",
                "pi executable path environment version 路径 环境 检查",
                |this, _, cx| {
                    let probe = this.applied_pi.clone();
                    let command = this.controller.read(cx).preferences(cx).pi_command;
                    if !probe.read(cx).matches_command(command.as_deref()) {
                        cx.defer(move |cx| {
                            probe.update(cx, |probe, cx| probe.request(command, false, cx))
                        });
                    }
                    this.render_pi_settings(cx)
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
                (!items.is_empty())
                    .then(|| SettingGroup::new().title(t(cx, group.label())).items(items))
            })
            .collect();
        let keyboard = SettingPage::new(t(cx, "settings-page-keys"))
            .icon(IconName::Keyboard)
            .resettable(false)
            .group(keys::KeysView::actions(&keys, &self.global_keys))
            .groups(global_keys::GlobalKeys::groups(
                &self.global_keys,
                keys_view,
                cx,
            ))
            .groups(command_groups);
        let resource_page = |key, kind, keywords: &str| {
            let resource_view = self.resources.downgrade();
            let section = |section, aliases: &str| {
                let resources = resource_view.clone();
                SettingItem::render(move |_, _, cx| {
                    resources
                        .update(cx, |view, cx| view.render_page(kind, section, cx))
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
            let mut page = SettingPage::new(t(cx, key))
                .resettable(false)
                .icon(match kind {
                    Kind::Extension => IconName::Puzzle,
                    Kind::Skill => IconName::BookOpen,
                    _ => IconName::FileText,
                })
                .title_suffix(move |_, cx| {
                    let scope = scope.update(cx, |config, cx| config.render_scope(cx));
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(scope)
                        .child(
                            header
                                .update(cx, |view, cx| view.render_header(kind, cx))
                                .unwrap_or_else(|_| div().into_any_element()),
                        )
                        .into_any_element()
                });
            if self.resources.read(cx).has_status(kind, cx) {
                page = page.group(SettingGroup::new().item(section(
                    resources::Section::Overview,
                    "refresh reload 刷新 重新读取",
                )));
            }
            if kind == Kind::Extension {
                return page.group(SettingGroup::new()
                    .item(section(resources::Section::Packages,"package extension skill theme template install update remove source version 包 扩展 技能 主题 模板 安装 更新 移除 来源 版本")));
            }
            let inherited_title = t(cx, "settings-resource-group-inherited");
            if kind == Kind::Prompt {
                page = page
                    .group(
                        SettingGroup::new()
                            .title(t(cx, "settings-system-heading"))
                            .items(resources::ResourcesView::system_prompt_items(
                                &self.resources,
                                cx,
                            )),
                    )
                    .group(
                        SettingGroup::new()
                            .title(t(cx, "settings-template-heading"))
                            .item(section(resources::Section::Catalog, keywords))
                            .items(resources::ResourcesView::prompt_items(
                                &self.resources,
                                false,
                                cx,
                            )),
                    );
                if project {
                    page = page.group(SettingGroup::new().title(inherited_title).items(
                        resources::ResourcesView::prompt_items(&self.resources, true, cx),
                    ));
                }
                return page;
            }
            let mut own = SettingGroup::new();
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
                page = page.group(SettingGroup::new().title(inherited_title).items(
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
            SettingItem::new(
                t(cx, key),
                SettingField::render(move |_, _, _| {
                    let tooltip = value.clone();
                    div()
                        .id(key)
                        .max_w(px(320.))
                        .text_sm()
                        .truncate()
                        .child(value.clone())
                        .tooltip(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tooltip.clone())
                                .build(window, cx)
                        })
                }),
            )
            .keywords(vec![aliases])
        })
        .collect();
        let gupi_item = about_items.remove(0);
        let updates = self.updates.clone();
        let controller = self.controller.clone();
        let about = SettingPage::new(t(cx, "settings-page-about"))
            .icon(IconName::Info)
            .resettable(false)
            .group(
                SettingGroup::new().title("Gupi").item(gupi_item).item(
                    SettingItem::new(
                        t(cx, "settings-diagnostics"),
                        SettingField::render(|_, _, cx| {
                            h_flex()
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
                SettingGroup::new()
                    .title(t(cx, "updates-title"))
                    .item(
                        SettingItem::render(move |_, _, _| updates.clone())
                            .keywords(["updates version download 更新 版本 下载"]),
                    )
                    .item(
                        SettingItem::new(
                            t(cx, "updates-automatic"),
                            SettingField::render(move |_, _, cx| {
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
                SettingGroup::new()
                    .title("Pi")
                    .description(t(cx, "settings-about-help"))
                    .items(about_items)
                    .item({
                        let this = this.clone();
                        SettingItem::new(
                            t(cx, "settings-page-pi"),
                            SettingField::render(move |_, _, cx| {
                                let this = this.clone();
                                Button::new("about-open-pi")
                                    .small()
                                    .label(t(cx, "settings-open-pi"))
                                    .on_click(move |_, _, cx| {
                                        this.update(cx, |this, cx| {
                                            this.open_page(Domain::Pi, PI_PAGE, cx)
                                        })
                                        .ok();
                                    })
                            }),
                        )
                        .keywords(["about pi settings open 关于 打开 Pi 设置"])
                    }),
            );
        let id = match self.domain {
            Domain::Gupi => "gupi-settings",
            Domain::Pi => "pi-settings",
        };
        let settings = Settings::new((id, self.navigation.0))
            .default_selected_index(SelectIndex {
                page_ix: self.navigation.1,
                group_ix: None,
            })
            .with_group_variant(GroupBoxVariant::Normal);
        match self.domain {
            Domain::Gupi => settings
                .page(general)
                .page(appearance)
                .page(notifications)
                .page(keyboard)
                .page(about),
            Domain::Pi => settings
                .page(pi.title(t(cx, "settings-page-connection")))
                .page(pi_config::PiConfig::page(&self.pi_config, pi_config::Page::Conversation, cx))
                .page(pi_config::PiConfig::page(&self.pi_config, pi_config::Page::Network, cx))
                .page(resource_page("settings-page-packages",Kind::Extension,"package plugin extension install update remove source version path enable disable browse search npm 插件 扩展 包 安装 更新 移除 启停 浏览 搜索"))
                .page(resource_page("settings-page-skills",Kind::Skill,"skill name description source path create edit delete enable disable 技能 创建 编辑 删除 启停"))
                .page(resource_page("settings-page-prompts",Kind::Prompt,"prompt template 创建 编辑 删除 模板")),
        }
    }
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self.settings_panel(cx);
        let problem = self
            .controller
            .read(cx)
            .configuration()
            .read(cx, |op| op.problem().map(|p| p.key()));
        v_flex()
            .size_full()
            .min_h_0()
            .children(
                self.error
                    .as_deref()
                    .or(problem)
                    .map(|key| div().text_color(cx.theme().danger).child(t(cx, key))),
            )
            .child(div().flex_1().min_h_0().child(panel))
            .into_any_element()
    }
    pub(super) fn render_icon_themes(&self, heading: bool, cx: &Context<Self>) -> AnyElement {
        use gupi_settings::icons::IconTheme;
        let config = self.controller.read(cx).preferences(cx);
        let busy = self.controller.read(cx).busy(cx);
        let mut group = gpui_kit::base::RadioGroup::new("icon-themes")
            .aria_label(t(cx, "settings-icon-theme"))
            .axis(Axis::Horizontal)
            .flex()
            .flex_wrap()
            .gap_2();
        for icon in IconTheme::ALL {
            let controller = self.controller.clone();
            let selected = config.icon_theme == icon;
            group = group.child(
                gpui_kit::base::Radio::new(icon.label())
                    .checked(selected)
                    .disabled(busy)
                    .accessibility_label(t(cx, icon.label()))
                    .p_2()
                    .w_24()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(if selected {
                        cx.theme().primary
                    } else {
                        cx.theme().border
                    })
                    .focus(|s| s.border_color(cx.theme().ring))
                    .on_change(move |_, _, _, cx| {
                        controller.update(cx, |owner, cx| {
                            owner.set_preference(PreferenceChange::IconTheme(icon), cx)
                        });
                    })
                    .child(
                        v_flex()
                            .items_center()
                            .gap_1()
                            .child(img(SharedString::from(icon.preview())).size_10())
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(div().text_sm().child(t(cx, icon.label())))
                                    .when(selected, |row| {
                                        row.child(
                                            gpui_kit::component::Icon::new(IconName::Check)
                                                .size_4(),
                                        )
                                    }),
                            ),
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
    fn render_theme_choices(&self, mode: Mode, cx: &Context<Self>) -> AnyElement {
        let draft = self.controller.read(cx).preferences(cx);
        preferences::theme_grid(
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
            self.controller.read(cx).busy(cx),
        )
    }
}

impl SettingsView {
    fn notification_settings(&self, cx: &Context<Self>) -> SettingPage {
        use gupi_settings::notifications::CompletionMode;
        use gupi_settings::notifications::Preferences;
        let controller = self.controller.clone();
        type Toggle = (&'static str, fn(&mut Preferences) -> &mut bool);
        let toggle = |(label, field): Toggle| {
            let controller = controller.clone();
            SettingItem::new(
                t(cx, label),
                SettingField::render(move |_, _, cx| {
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
        let background = SettingGroup::new()
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
        let completion = SettingGroup::new()
            .title(t(cx, "settings-notification-completion-heading"))
            .item(
                SettingItem::new(
                    t(cx, "settings-notification-completion"),
                    SettingField::render(move |_, _, cx| {
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
        let page = SettingPage::new(t(cx, "settings-notifications"))
            .icon(IconName::Bell)
            .resettable(false)
            .group(background)
            .group(completion);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let page = page.group(
            SettingGroup::new()
                .title(t(cx, "settings-permissions"))
                .item(
                    SettingItem::new(
                        t(cx, "settings-notification-permission-open"),
                        SettingField::render(|_, _, cx| {
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
