//! Rendering for the Conversation and Network pages.
use super::Load;
use super::Models;
use super::PiConfig;
use super::Support;
use super::Trust;
use super::fields::Field;
use super::fields::Kind;
use super::fields::Page;
use super::fields::Source;
use super::fields::display;
use super::fields::supported_levels;
use super::forms::Slot;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::h_flex;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::select::Select;
use gpui_kit::component::setting::SettingGroup;
use gpui_kit::component::setting::SettingItem;
use gpui_kit::component::setting::SettingPage;
use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_settings::Scope;
use gupi_settings::assets::IconName;
use gupi_settings::i18n::t;
use gupi_settings::i18n::t_with_args;
use serde_json::Value;

/// Groups per page, in order: title key and fields.
const CONVERSATION: &[(&str, &[Field])] = &[
    (
        "pi-group-model",
        &[
            Field::Model,
            Field::Thinking,
            Field::ModelThinking,
            Field::BudgetMinimal,
            Field::BudgetLow,
            Field::BudgetMedium,
            Field::BudgetHigh,
            Field::CacheWarming,
            Field::ModelScope,
        ],
    ),
    (
        "pi-group-context",
        &[
            Field::Compaction,
            Field::ReserveTokens,
            Field::KeepRecentTokens,
            Field::ModelOverrides,
        ],
    ),
    ("pi-group-queue", &[Field::Steering, Field::FollowUp]),
    ("pi-group-images", &[Field::AutoResize, Field::BlockImages]),
];
const NETWORK: &[(&str, &[Field])] = &[
    ("pi-group-proxy", &[Field::Proxy]),
    (
        "pi-group-retry",
        &[
            Field::Retry,
            Field::MaxRetries,
            Field::BaseDelay,
            Field::MaxDelay,
        ],
    ),
    (
        "pi-group-advanced",
        &[
            Field::Transport,
            Field::HttpTimeout,
            Field::WebsocketTimeout,
        ],
    ),
    (
        "pi-group-provider",
        &[
            Field::ProviderTimeout,
            Field::ProviderRetries,
            Field::ProviderDelay,
        ],
    ),
];

const TOOLS: &[(&str, &[Field])] = &[
    ("pi-group-resource-paths", &Field::RESOURCE_PATHS),
    (
        "pi-group-tools",
        &[
            Field::DefaultTools,
            Field::CodemodeMode,
            Field::CodemodeBudget,
        ],
    ),
    ("pi-group-shell", &[Field::ShellPath, Field::ShellPrefix]),
    ("pi-group-package-command", &[Field::NpmCommand]),
    (
        "pi-group-privacy-trust",
        &[Field::Telemetry, Field::ProjectTrust],
    ),
];

impl Field {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Extensions => "pi-field-resource-extensions",
            Self::Skills => "pi-field-resource-skills",
            Self::Prompts => "pi-field-resource-prompts",
            Self::Themes => "pi-field-resource-themes",
            Self::DefaultTools => "pi-field-default-tools",
            Self::CodemodeMode => "pi-field-codemode-mode",
            Self::CodemodeBudget => "pi-field-codemode-budget",
            Self::ShellPath => "pi-field-shell-path",
            Self::ShellPrefix => "pi-field-shell-prefix",
            Self::NpmCommand => "pi-field-npm-command",
            Self::Telemetry => "pi-field-telemetry",
            Self::ProjectTrust => "pi-field-project-trust",
            Self::ModelThinking => "pi-field-model-thinking",
            Self::ModelOverrides => "pi-field-model-overrides",
            Self::BudgetMinimal => "pi-field-budget-minimal",
            Self::BudgetLow => "pi-field-budget-low",
            Self::BudgetMedium => "pi-field-budget-medium",
            Self::BudgetHigh => "pi-field-budget-high",
            Self::CacheWarming => "pi-field-cache-warming",
            Self::Transport => "pi-field-transport",
            Self::HttpTimeout => "pi-field-http-timeout",
            Self::WebsocketTimeout => "pi-field-websocket-timeout",
            Self::ProviderTimeout => "pi-field-provider-timeout",
            Self::ProviderRetries => "pi-field-provider-retries",
            Self::ProviderDelay => "pi-field-provider-delay",
            Self::Model => "pi-field-model",
            Self::Thinking => "pi-field-thinking",
            Self::ModelScope => "pi-model-scope",
            Self::Compaction => "pi-field-compaction",
            Self::ReserveTokens => "pi-field-reserve",
            Self::KeepRecentTokens => "pi-field-keep",
            Self::Steering => "pi-field-steering",
            Self::FollowUp => "pi-field-follow-up",
            Self::AutoResize => "pi-field-auto-resize",
            Self::BlockImages => "pi-field-block-images",
            Self::Proxy => "pi-field-proxy",
            Self::Retry => "pi-field-retry",
            Self::MaxRetries => "pi-field-max-retries",
            Self::BaseDelay => "pi-field-base-delay",
            Self::MaxDelay => "pi-field-max-delay",
        }
    }
    fn help(self) -> Option<&'static str> {
        Some(match self {
            Self::Extensions | Self::Skills | Self::Prompts | Self::Themes => return None,
            Self::CodemodeMode | Self::CodemodeBudget => return None,
            Self::BudgetMinimal | Self::BudgetLow | Self::BudgetMedium | Self::BudgetHigh => {
                "pi-thinking-budget-help"
            }
            Self::CacheWarming => "pi-cache-warming-help",
            Self::Telemetry => "pi-telemetry-help",
            Self::ProviderRetries => "pi-provider-retry-help",
            Self::NpmCommand => "pi-npm-help",
            Self::Model => "pi-field-model-help",
            Self::ReserveTokens => "pi-field-reserve-help",
            Self::KeepRecentTokens => "pi-field-keep-help",
            Self::AutoResize => "pi-field-auto-resize-help",
            Self::Proxy => "pi-field-proxy-help",
            _ => return None,
        })
    }
    pub(super) fn id(self) -> &'static str {
        if self == Self::ProviderRetries {
            return "provider-maxRetries";
        }
        self.leaves()[self.leaves().len() - 1][self.leaves()[0].len() - 1]
    }
}

impl Page {
    fn groups(self) -> &'static [(&'static str, &'static [Field])] {
        match self {
            Self::Conversation => CONVERSATION,
            Self::Network => NETWORK,
            Self::Tools => TOOLS,
        }
    }
    pub(in super::super) fn title(self) -> &'static str {
        match self {
            Self::Conversation => "settings-page-pi-conversation",
            Self::Network => "settings-page-pi-network",
            Self::Tools => "settings-page-pi-tools",
        }
    }
}

pub(super) fn level_label(level: &str, cx: &App) -> String {
    match level {
        "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" => {
            t(cx, &format!("conversation-thinking-{level}"))
        }
        other => other.to_owned(),
    }
}

fn choice_label(field: Field, value: &str, cx: &App) -> String {
    match field {
        Field::ProjectTrust => t(
            cx,
            match value {
                "ask" => "pi-choice-ask",
                "always" => "pi-choice-always",
                "never" => "pi-choice-never",
                _ => return value.to_owned(),
            },
        ),
        Field::CodemodeMode => t(
            cx,
            match value {
                "on" => "pi-choice-codemode-on",
                "only" => "pi-choice-codemode-only",
                _ => return value.to_owned(),
            },
        ),
        Field::CacheWarming => match value {
            "off" => level_label("off", cx),
            "streaming" => t(cx, "pi-choice-streaming"),
            "idle" => t(cx, "pi-choice-idle"),
            _ => value.to_owned(),
        },
        Field::Transport if value == "auto" => t(cx, "pi-choice-auto"),
        Field::Thinking => level_label(value, cx),
        _ => match value {
            "one-at-a-time" => t(cx, "pi-delivery-one"),
            "all" => t(cx, "pi-delivery-all"),
            other => other.to_owned(),
        },
    }
}

impl PiConfig {
    /// Builds a settings page. Rows read this entity when rendered.
    pub(in super::super) fn page(this: &Entity<Self>, page: Page, cx: &App) -> SettingPage {
        let weak = this.downgrade();
        let header = weak.clone();
        let mut settings = SettingPage::new(t(cx, page.title()))
            .icon(match page {
                Page::Conversation => IconName::MessageSquare,
                Page::Network => IconName::Globe,
                Page::Tools => IconName::Terminal,
            })
            .resettable(false)
            .group(
                SettingGroup::new()
                    .border_0()
                    .p_0()
                    .item(SettingItem::render(move |_, window, cx| {
                        header
                            .update(cx, |this, cx| this.render_header(page, window, cx))
                            .unwrap_or_else(|_| div().into_any_element())
                    })),
            );
        let config = this.read(cx);
        let status = config.status(cx);
        if status.is_some() || config.error.is_some() {
            let weak = weak.clone();
            settings = settings.group(
                SettingGroup::new().item(
                    SettingItem::render(move |_, _, cx| {
                        weak.update(cx, |this, cx| this.render_status(cx))
                            .unwrap_or_else(|_| div().into_any_element())
                    })
                    .keywords(["pi settings 设置 status"]),
                ),
            );
        }
        if status.is_some() {
            return settings;
        }
        for (title, fields) in page.groups() {
            let items = fields
                .iter()
                .filter(|field| {
                    !matches!(
                        field,
                        Field::BudgetLow | Field::BudgetMedium | Field::BudgetHigh
                    )
                })
                .map(|field| {
                    let field = *field;
                    let weak = weak.clone();
                    SettingItem::render(move |_, window, cx| {
                        weak.update(cx, |this, cx| match field {
                            Field::BudgetMinimal => this.render_budgets(window, cx),
                            Field::ModelScope => this.render_model_scope(window, cx),
                            Field::ModelThinking | Field::ModelOverrides => {
                                this.render_model_map(field, window, cx)
                            }
                            _ => this.render_field(field, window, cx),
                        })
                        .unwrap_or_else(|_| div().into_any_element())
                    })
                    .keywords(
                        (if field == Field::BudgetMinimal {
                            vec![
                                Field::BudgetMinimal,
                                Field::BudgetLow,
                                Field::BudgetMedium,
                                Field::BudgetHigh,
                            ]
                        } else {
                            vec![field]
                        })
                        .into_iter()
                        .flat_map(|field| {
                            field
                                .leaves()
                                .iter()
                                .map(|leaf| leaf.join("."))
                                .chain([t(cx, field.label())])
                        })
                        .collect::<Vec<_>>(),
                    )
                });
            let mut group = SettingGroup::new().title(t(cx, title)).items(items);
            if *title == "pi-group-tools" {
                group = group.description(t(cx, "pi-codemode-help"));
            }
            if *title == "pi-group-resource-paths" {
                group = group.description(t(cx, "pi-resource-paths-help"));
            }
            settings = settings.group(group);
        }
        settings
    }

    /// A message that replaces the fields while they cannot be edited.
    fn status(&self, cx: &App) -> Option<String> {
        match self.support(cx) {
            Support::Ready => {}
            Support::Checking => return Some(t(cx, "startup-checking")),
            Support::Unavailable => return Some(t(cx, "pi-settings-unavailable")),
            Support::Unsupported(version) => {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("version", version);
                return Some(t_with_args(cx, "pi-settings-unsupported", &args));
            }
        }
        if let Err(error) = &self.agent {
            return Some(error.clone());
        }
        match &self.files {
            Load::Ready(_) => None,
            Load::Loading => Some(t(cx, "pi-settings-loading")),
            Load::Failed(error) => Some(error.clone()),
        }
    }

    fn render_status(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let failed = matches!(self.files, Load::Failed(_)) || !self.conflicts.is_empty();
        let message = self.status(cx).or_else(|| self.error.clone());
        let danger = failed || self.error.is_some();
        h_flex()
            .gap_2()
            .items_center()
            .w_full()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .when(danger, |this| this.text_color(cx.theme().danger))
                    .children(message),
            )
            .when(failed, |this| {
                this.child(
                    Button::new("pi-settings-reload-status")
                        .small()
                        .icon(IconName::RotateCw)
                        .label(t(cx, "pi-settings-reload"))
                        .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                )
            })
            .into_any_element()
    }

    /// The scope menu and project trust status, shared by every scoped Pi page.
    pub(in super::super) fn render_scope(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let entity = cx.entity();
        let scope_label = match &self.scope {
            Scope::Global => t(cx, "pi-scope-global"),
            Scope::Project(cwd) => cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| cwd.display().to_string()),
        };
        let path = self.agent.as_ref().ok().map(|agent| self.scope.file(agent));
        let scopes = self.scope_choices();
        let current = self.scope.clone();
        let scope_menu = Button::new("pi-scope")
            .small()
            .min_w_0()
            .max_w(rems(16.))
            .outline()
            .label(scope_label)
            .dropdown_caret(true)
            .accessibility_label(t(cx, "pi-scope-label"))
            .when_some(path.clone(), |this, path| {
                this.tooltip(path.display().to_string())
            })
            .disabled(self.is_saving() || self.file_pages.read(cx).busy())
            .dropdown_menu({
                let entity = entity.clone();
                move |menu, _, cx| {
                    let menu = scopes.iter().fold(menu, |menu, scope| {
                        let label = match scope {
                            Scope::Global => t(cx, "pi-scope-global"),
                            Scope::Project(cwd) => project_label(cwd, &scopes),
                        };
                        let entity = entity.clone();
                        let scope = scope.clone();
                        menu.item(
                            PopupMenuItem::new(label)
                                .checked(scope == current)
                                .on_click(move |_, window, cx| {
                                    request_scope(&entity, scope.clone(), window, cx);
                                }),
                        )
                    });
                    let entity = entity.clone();
                    menu.separator().item(
                        PopupMenuItem::new(t(cx, "pi-scope-choose-folder")).on_click(
                            move |_, window, cx| choose_project_folder(&entity, window, cx),
                        ),
                    )
                }
            });
        let trust = match (&self.scope, &self.trust) {
            (Scope::Project(_), Some(Trust::Untrusted)) => {
                Some((t(cx, "pi-trust-untrusted"), true))
            }
            (Scope::Project(_), Some(Trust::Unknown(error))) => Some((error.clone(), true)),
            _ => None,
        };
        h_flex()
            .w_full()
            .min_w_0()
            .flex_wrap()
            .gap_2()
            .items_center()
            .min_w_0()
            .child(scope_menu)
            .when_some(trust, |this, (status, can_trust)| {
                this.child(
                    div()
                        .id("pi-trust-status")
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .min_w_0()
                        .tooltip({
                            let help = t(cx, "pi-trust-untrusted-help");
                            move |window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(help.clone())
                                    .build(window, cx)
                            }
                        })
                        .child(status),
                )
                .when(can_trust, |this| {
                    this.child(
                        Button::new("pi-trust")
                            .small()
                            .label(t(cx, "pi-trust-action"))
                            .loading(self.trusting.is_some())
                            .disabled(self.trusting.is_some())
                            .on_click({
                                let entity = entity.clone();
                                move |_, window, cx| PiConfig::confirm_trust(&entity, window, cx)
                            }),
                    )
                })
            })
            .into_any_element()
    }

    fn render_header(
        &mut self,
        page: Page,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity = cx.entity();
        let path = self.agent.as_ref().ok().map(|agent| self.scope.file(agent));
        let dirty = self.has_page_drafts(page, cx);
        let saving = self.is_saving();
        let ready = matches!(self.files, Load::Ready(_)) && self.status(cx).is_none();
        let more = Button::new("pi-settings-more")
            .ghost()
            .small()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-settings-file-actions"))
            .dropdown_menu({
                let entity = entity.clone();
                move |menu, _, cx| {
                    let open = path.clone();
                    let reload = entity.clone();
                    menu.item(
                        PopupMenuItem::new(t(cx, "pi-settings-open-file"))
                            .disabled(open.as_ref().is_none_or(|path| !path.exists()))
                            .on_click(move |_, _, cx| {
                                if let Some(path) = &open {
                                    cx.open_with_system(path);
                                }
                            }),
                    )
                    .item(
                        PopupMenuItem::new(t(cx, "pi-settings-reload"))
                            .disabled(entity.read(cx).is_saving())
                            .on_click(move |_, _, cx| {
                                reload.update(cx, |this, cx| this.reload(cx))
                            }),
                    )
                }
            });
        let _ = window;
        v_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(self.render_scope(cx))
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .when(dirty && ready, |this| {
                        this.child(
                            Button::new("pi-settings-discard")
                                .small()
                                .label(t(cx, "pi-settings-discard"))
                                .disabled(saving)
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.discard(Some(page), cx)),
                                ),
                        )
                        .child(
                            Button::new("pi-settings-save")
                                .small()
                                .primary()
                                .label(t(cx, "pi-settings-save"))
                                .loading(saving)
                                .disabled(saving)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.save(Some(page), None, window, cx)
                                })),
                        )
                    })
                    .child(more),
            )
            .into_any_element()
    }

    fn render_field(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Load::Ready(_) = &self.files else {
            return div().into_any_element();
        };
        let project = matches!(self.scope, Scope::Project(_));

        let pending = self.pending(field, cx);
        let resolved = self.resolved(field, cx);
        let saving = self.is_saving();
        let id = format!("pi-{}-{}", self.generation, field.id());
        let control = if project && field.is_global_only() {
            self.render_global_only(field, cx)
        } else {
            match field.kind() {
                Kind::Bool => {
                    let checked =
                        resolved.values[0].as_ref().and_then(Value::as_bool) == Some(true);
                    Checkbox::new(SharedString::from(id))
                        .checked(checked)
                        .disabled(saving)
                        .accessibility_label(t(cx, field.label()))
                        .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                            this.set_values(field, vec![Slot::Set(Value::Bool(*checked))], cx)
                        }))
                        .into_any_element()
                }
                Kind::Choice(options) => {
                    self.render_choice(field, options, &resolved.values, saving, cx)
                }
                Kind::RawList => self.render_resource_paths(field, window, cx),
                Kind::List => self.render_tools_chips(window, cx),
                Kind::ThinkingMap | Kind::CompactionMap => unreachable!("map block"),
                Kind::Timeout => self.render_timeout(field, window, cx),
                Kind::Count | Kind::Text => self.render_input(field, window, cx),
                Kind::ModelScope => self.render_model_scope(window, cx),
                Kind::Model => self.render_model(&resolved.values, saving, window, cx),
            }
        };
        let description = if field == Field::DefaultTools {
            Some(self.tools_preview(cx))
        } else {
            field.help().map(|key| t(cx, key))
        };
        let mut label = v_flex()
            .gap_1()
            .flex_1()
            .min_w_0()
            .child(self.field_label(field, cx));
        if let Some(description) = description {
            label = label.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            );
        }
        if field == Field::DefaultTools {
            label = label.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-tools-startup-help")),
            );
        }
        if field == Field::Telemetry
            && let Some(enabled) = super::advanced::telemetry(
                std::env::var_os("PI_TELEMETRY")
                    .as_deref()
                    .map(|s| s.to_str().unwrap_or("")),
            )
        {
            label = label.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if enabled {
                            "pi-telemetry-env-on"
                        } else {
                            "pi-telemetry-env-off"
                        },
                    )),
            );
        }
        let conflict = self.conflicts.contains(&field);
        let invalid = self.invalid(field, cx);
        let top_aligned = field.is_resource_paths();
        let stacked = matches!(field.kind(), Kind::Count | Kind::Timeout);

        v_flex()
            .w_full()
            .max_w(rems(56.))
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .when(stacked, |row| row.flex_col().items_start())
                    .when(top_aligned, |row| row.items_start())
                    .w_full()
                    .child(label)
                    .child(
                        h_flex()
                            .min_w_0()
                            .w_80()
                            .max_w_full()
                            .when(stacked, |row| row.w_full())
                            .flex_shrink_0()
                            .justify_end()
                            .child(control),
                    )
                    .child(if project && field.is_global_only() {
                        div().w_6().flex_shrink_0().into_any_element()
                    } else {
                        if field.is_resource_paths() {
                            self.render_resource_paths_menu(field, cx)
                        } else {
                            self.render_field_menu(field, pending, saving, cx)
                        }
                    }),
            )
            .when(conflict || invalid, |this| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(t(
                    cx,
                    if conflict {
                        "pi-changed-on-disk"
                    } else if field.is_resource_paths() {
                        "pi-resource-unsupported"
                    } else if field == Field::NpmCommand {
                        "pi-npm-invalid"
                    } else {
                        "pi-invalid-count"
                    },
                )))
            })
            .into_any_element()
    }

    pub(super) fn field_label(&self, field: Field, cx: &App) -> AnyElement {
        let source = if field.is_global_only() && matches!(self.scope, Scope::Project(_)) {
            match &self.files {
                Load::Ready(files)
                    if field
                        .leaves()
                        .iter()
                        .any(|leaf| files.global.get(leaf).is_some()) =>
                {
                    Source::Global
                }
                _ => Source::Default,
            }
        } else {
            self.resolved(field, cx).source
        };
        h_flex()
            .gap_1()
            .flex_wrap()
            .child(t(cx, field.label()))
            .when(source != Source::Default, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!(
                            "· {}",
                            t(
                                cx,
                                if source == Source::Global {
                                    "pi-source-global"
                                } else {
                                    "pi-source-project"
                                }
                            )
                        )),
                )
            })
            .when(self.pending(field, cx), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("· {}", t(cx, "pi-unsaved"))),
                )
            })
            .into_any_element()
    }

    fn render_budgets(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        const BUDGETS: [Field; 4] = [
            Field::BudgetMinimal,
            Field::BudgetLow,
            Field::BudgetMedium,
            Field::BudgetHigh,
        ];
        let mut controls = h_flex().w_80().flex_shrink_0().gap_1();
        for field in BUDGETS {
            let input = self.input(field, window, cx);
            controls = controls.child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.field_label(field, cx)),
                    )
                    .child(
                        Input::new(&input)
                            .small()
                            .min_w_0()
                            .aria_label({
                                let mut args = fluent_bundle::FluentArgs::new();
                                args.set("level", t(cx, field.label()));
                                t_with_args(cx, "pi-budget-input", &args)
                            })
                            .disabled(self.is_saving()),
                    )
                    .when(
                        self.invalid(field, cx) || self.conflicts.contains(&field),
                        |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().danger)
                                    .child(t(cx, "pi-invalid-count")),
                            )
                        },
                    ),
            );
        }
        h_flex()
            .w_full()
            .max_w(rems(56.))
            .gap_2()
            .items_center()
            .child(
                v_flex()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .child(t(cx, "pi-thinking-budgets"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, "pi-thinking-budget-help")),
                    ),
            )
            .child(controls)
            .child(self.render_fields_menu(
                "budgets",
                BUDGETS.to_vec(),
                BUDGETS.iter().any(|field| self.pending(*field, cx)),
                self.is_saving(),
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn render_field_menu(
        &self,
        field: Field,
        pending: bool,
        saving: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_fields_menu(field.id(), vec![field], pending, saving, cx)
    }

    /// One menu for fields shown together; each action applies to the fields it fits.
    fn render_fields_menu(
        &self,
        id: &str,
        fields: Vec<Field>,
        pending: bool,
        saving: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity = cx.entity();
        let project = matches!(self.scope, Scope::Project(_));
        let has_draft = pending;
        let snapshot = self.snapshot(cx);
        // After saving, would the edited scope hold a value?
        let (set_here, unset_here): (Vec<Field>, Vec<Field>) =
            fields.iter().copied().partition(|field| {
                snapshot
                    .value(*field)
                    .iter()
                    .any(|value| !matches!(value, Slot::Unset))
            });
        Button::new(SharedString::from(format!("pi-menu-{id}")))
            .w_6()
            .flex_shrink_0()
            .ghost()
            .xsmall()
            .icon(IconName::Ellipsis)
            .disabled(saving)
            .accessibility_label(t(cx, "pi-field-actions"))
            .dropdown_menu(move |menu, _, cx| {
                let mut menu = menu;
                if has_draft {
                    let entity = entity.clone();
                    let fields = fields.clone();
                    menu = menu.item(PopupMenuItem::new(t(cx, "pi-menu-revert")).on_click(
                        move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                for field in &fields {
                                    this.clear_draft(*field, cx);
                                }
                            })
                        },
                    ));
                }
                if !unset_here.is_empty() {
                    let entity = entity.clone();
                    let unset_here = unset_here.clone();
                    menu = menu.item(
                        PopupMenuItem::new(t(
                            cx,
                            if project {
                                "pi-menu-pin-project"
                            } else {
                                "pi-menu-pin-global"
                            },
                        ))
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                for field in &unset_here {
                                    this.pin(*field, cx);
                                }
                            })
                        }),
                    );
                }
                if !set_here.is_empty() {
                    let entity = entity.clone();
                    let set_here = set_here.clone();
                    menu = menu.item(
                        PopupMenuItem::new(t(
                            cx,
                            if project {
                                "pi-menu-inherit"
                            } else {
                                "pi-menu-default"
                            },
                        ))
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                for field in &set_here {
                                    this.set_values(
                                        *field,
                                        vec![Slot::Unset; field.leaves().len()],
                                        cx,
                                    )
                                }
                            })
                        }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    pub(super) fn render_choice(
        &self,
        field: Field,
        options: &'static [&'static str],
        values: &[Option<Value>],
        saving: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = values[0]
            .as_ref()
            .and_then(Value::as_str)
            .map(str::to_owned);
        let mut choices: Vec<String> = if field == Field::Thinking {
            match self.selected_model_levels(cx) {
                Some(levels) => levels.into_iter().map(str::to_owned).collect(),
                None => options.iter().map(|o| (*o).to_owned()).collect(),
            }
        } else {
            options.iter().map(|o| (*o).to_owned()).collect()
        };
        // A stored value the model does not support stays visible and selected.
        let unsupported = current
            .as_ref()
            .filter(|value| !choices.contains(value))
            .cloned();
        if let Some(value) = &unsupported {
            choices.push(value.clone());
        }
        let label = |value: &str, cx: &App| {
            let text = choice_label(field, value, cx);
            if unsupported.as_deref() == Some(value) && field == Field::Thinking {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("level", text);
                t_with_args(cx, "pi-thinking-unsupported", &args)
            } else {
                text
            }
        };
        let selected = current
            .as_deref()
            .map(|value| label(value, cx))
            .unwrap_or_default();
        let entries: Vec<(String, String)> =
            choices.iter().map(|c| (c.clone(), label(c, cx))).collect();
        let entity = cx.entity();
        Button::new(SharedString::from(format!("pi-choice-{}", field.id())))
            .outline()
            .small()
            .w_80()
            .label(selected)
            .dropdown_caret(true)
            .disabled(saving)
            .accessibility_label(t(cx, field.label()))
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                entries.iter().fold(menu, |menu, (value, label)| {
                    let entity = entity.clone();
                    let next = value.clone();
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(current.as_deref() == Some(value.as_str()))
                            .on_click(move |_, window, cx| {
                                if field == Field::ProjectTrust && next == "always" {
                                    PiConfig::confirm_always(&entity, window, cx);
                                } else {
                                    entity.update(cx, |this, cx| {
                                        this.set_values(
                                            field,
                                            vec![Slot::Set(Value::from(next.clone()))],
                                            cx,
                                        )
                                    });
                                }
                            }),
                    )
                })
            })
            .into_any_element()
    }

    /// Levels for the effective default model, when its metadata is known.
    fn selected_model_levels(&self, cx: &App) -> Option<Vec<&'static str>> {
        let (Load::Ready(_), Models::Ready(models)) = (&self.files, &self.models) else {
            return None;
        };
        let resolved = self.resolved(Field::Model, cx);
        let [Some(Value::String(provider)), Some(Value::String(id))] = resolved.values.as_slice()
        else {
            return None;
        };
        models
            .iter()
            .find(|model| &model.provider == provider && &model.id == id)
            .map(supported_levels)
    }

    pub(super) fn render_input(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input = self.input(field, window, cx);
        Input::new(&input)
            .id(SharedString::from(format!("pi-input-{}", field.id())))
            .small()
            .w_full()
            .aria_label(t(cx, field.label()))
            .disabled(self.is_saving())
            .into_any_element()
    }

    fn render_model(
        &mut self,
        values: &[Option<Value>],
        saving: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if matches!(self.models, Models::Idle) {
            cx.defer_in(window, |this, window, cx| {
                if matches!(this.models, Models::Idle) {
                    this.load_models(window, cx);
                }
            });
        }
        let key = match values {
            [Some(Value::String(provider)), Some(Value::String(model))] => {
                Some((provider.clone(), model.clone()))
            }
            _ => None,
        };
        let raw = match values {
            [None, None] => t(cx, "pi-field-model-auto"),
            _ => values
                .iter()
                .map(|value| display(value.as_ref()))
                .collect::<Vec<_>>()
                .join("/"),
        };
        // Project the effective model once per change; an unknown model leaves
        // the picker empty, which must not trigger another projection.
        if self.synced_model.as_ref() != Some(&key) {
            self.model_picker.update(cx, |picker, cx| match &key {
                Some(key) => picker.set_selected_value(key, window, cx),
                None => picker.set_selected_index(None, window, cx),
            });
            self.synced_model = Some(key.clone());
        }
        let known = key.is_none() || self.model_picker.read(cx).selected_value().is_some();
        let note = match &self.models {
            Models::Ready(_) if !known => Some((t(cx, "pi-field-model-unavailable"), true)),
            _ => None,
        };
        v_flex()
            .w_full()
            .gap_1()
            .items_end()
            .child(
                Select::new(&self.model_picker)
                    .small()
                    .w_full()
                    .placeholder(raw)
                    .search_placeholder(t(cx, "conversation-model-search"))
                    .accessibility_label(t(cx, "pi-field-model"))
                    .disabled(saving || !matches!(self.models, Models::Ready(_))),
            )
            .when_some(note, |this, (note, failed)| {
                this.child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .text_xs()
                        .text_color(if failed {
                            cx.theme().danger
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(note),
                )
            })
            .into_any_element()
    }

    /// A project row for a field Pi reads only from the global file.
    pub(super) fn render_global_only(&self, field: Field, cx: &mut Context<Self>) -> AnyElement {
        let Load::Ready(files) = &self.files else {
            return div().into_any_element();
        };
        let global = super::fields::Files {
            global: files.global.clone(),
            project: None,
        };
        let value = global.resolve(field, None).values[0]
            .as_ref()
            .map(|value| {
                value
                    .as_str()
                    .map(|v| choice_label(field, v, cx))
                    .unwrap_or_else(|| display(Some(value)))
            })
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| t(cx, "pi-value-unset"));
        let entity = cx.entity();
        h_flex()
            .gap_2()
            .items_center()
            .child(div().text_sm().truncate().max_w_64().child(value))
            .child(
                Button::new(SharedString::from(format!("pi-global-{}", field.id())))
                    .ghost()
                    .small()
                    .label(t(cx, "pi-edit-in-global"))
                    .on_click(move |_, window, cx| {
                        request_scope(&entity, Scope::Global, window, cx)
                    }),
            )
            .into_any_element()
    }
}

/// Changes scope after confirming unsaved drafts.
pub(in super::super) fn request_scope(
    entity: &Entity<PiConfig>,
    scope: Scope,
    window: &mut Window,
    cx: &mut App,
) {
    let scope = match scope.validated() {
        Ok(scope) => scope,
        Err(error) => {
            window.push_notification(
                gpui_kit::component::notification::Notification::error(error.to_string()),
                cx,
            );
            return;
        }
    };
    if entity.read(cx).scope == scope {
        return;
    }
    let target = entity.clone();
    PiConfig::confirm_leave(
        entity,
        move |_, cx| target.update(cx, |this, cx| this.set_scope(scope, cx)),
        window,
        cx,
    );
}

/// A project's folder name, with its location when another project shares it.
fn project_label(cwd: &std::path::Path, scopes: &[Scope]) -> String {
    let name = |path: &std::path::Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string())
    };
    let label = name(cwd);
    let shared = scopes
        .iter()
        .filter(|scope| matches!(scope, Scope::Project(other) if name(other) == label))
        .count()
        > 1;
    match cwd.parent() {
        Some(parent) if shared => format!("{label} — {}", parent.display()),
        _ => label,
    }
}

/// Picks any existing folder as the project scope. Choosing a folder neither
/// creates `.pi` nor trusts the folder.
fn choose_project_folder(entity: &Entity<PiConfig>, window: &mut Window, cx: &mut App) {
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some(t(cx, "pi-scope-choose-folder-action").into()),
    });
    let entity = entity.downgrade();
    window
        .spawn(cx, async move |cx| {
            let Ok(Ok(Some(paths))) = prompt.await else {
                return;
            };
            let Some(folder) = paths.into_iter().next().filter(|path| path.is_dir()) else {
                return;
            };
            let _ = cx.update(|window, cx| {
                if let Some(entity) = entity.upgrade() {
                    request_scope(&entity, Scope::Project(folder), window, cx);
                }
            });
        })
        .detach();
}
