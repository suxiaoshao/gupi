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
use crate::features::settings::composition::{
    PageLayout, SettingsPage, SettingsRow, SettingsSection, settings_row,
};
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
    pub(in super::super) fn page(this: &Entity<Self>, page: Page, cx: &App) -> SettingsPage {
        let weak = this.downgrade();
        let header = weak.clone();
        let scope = weak.clone();
        let actions = weak.clone();
        let config = this.read(cx);
        let show_header = config.has_page_drafts(page, cx)
            || config.status(cx).is_some()
            || config.error.is_some()
            || matches!(
                (&config.scope, &config.trust),
                (
                    Scope::Project(_),
                    Some(Trust::Untrusted | Trust::Unknown(_))
                )
            );
        let mut settings = SettingsPage::new(t(cx, page.title()))
            .icon(match page {
                Page::Conversation => IconName::MessageSquare,
                Page::Network => IconName::Globe,
                Page::Tools => IconName::Terminal,
            })
            .scope(move |_, window, cx| {
                scope
                    .update(cx, |this, cx| this.render_scope(false, window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            })
            .actions(move |_, _, cx| {
                actions
                    .update(cx, |this, cx| this.render_file_actions(cx))
                    .unwrap_or_else(|_| div().into_any_element())
            });
        if show_header {
            settings = settings.header(move |layout, window, cx| {
                header
                    .update(cx, |this, cx| {
                        let actions = this.render_header(page, layout, window, cx);
                        v_flex()
                            .gap_3()
                            .child(actions)
                            .when(this.status(cx).is_some() || this.error.is_some(), |view| {
                                view.child(this.render_status(cx))
                            })
                            .into_any_element()
                    })
                    .unwrap_or_else(|_| div().into_any_element())
            });
        }
        let config = this.read(cx);
        let status = config.status(cx);
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
                    SettingsRow::content(move |layout, window, cx| {
                        weak.update(cx, |this, cx| match field {
                            Field::BudgetMinimal => this.render_budgets(layout, window, cx),
                            Field::ModelScope => this.render_model_scope(window, cx),
                            Field::ModelThinking | Field::ModelOverrides => {
                                this.render_model_map(field, window, cx)
                            }
                            _ => this.render_field(field, layout, window, cx),
                        })
                        .unwrap_or_else(|_| div().into_any_element())
                    })
                    .framed(!matches!(
                        field,
                        Field::BudgetMinimal
                            | Field::ModelScope
                            | Field::ModelThinking
                            | Field::ModelOverrides
                            | Field::Extensions
                            | Field::Skills
                            | Field::Prompts
                            | Field::Themes
                    ))
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
            let mut group = SettingsSection::new().title(t(cx, title)).items(items);
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
        let failed = matches!(self.files, Load::Failed(_))
            || (!self.conflicts.is_empty() || !self.refresh_conflicts.is_empty());
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

    pub(in super::super) fn render_scope(
        &mut self,
        global_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let scopes = self.scope_choices();
        let mut items = scopes
            .iter()
            .map(|scope| super::ScopeItem {
                value: Some(scope.clone()),
                title: match scope {
                    Scope::Global => t(cx, "pi-scope-global"),
                    Scope::Project(cwd) => project_label(cwd, &scopes),
                }
                .into(),
            })
            .collect::<Vec<_>>();
        items.push(super::ScopeItem {
            value: None,
            title: t(cx, "pi-scope-choose-folder").into(),
        });
        if items != self.scope_items {
            self.scope_items = items.clone();
            self.global_select.update(cx, |picker, cx| {
                picker.set_items(
                    super::SearchableVec::new(vec![items[0].clone()]),
                    window,
                    cx,
                );
                picker.set_selected_index(
                    Some(gpui_kit::component::IndexPath::default()),
                    window,
                    cx,
                );
            });
            self.scope_select.update(cx, |picker, cx| {
                picker.set_items(super::SearchableVec::new(items), window, cx)
            });
        }
        if self.scope_select.read(cx).selected_value() != Some(&Some(self.scope.clone())) {
            self.scope_select.update(cx, |picker, cx| {
                picker.set_selected_value(&Some(self.scope.clone()), window, cx)
            });
        }
        Select::new(if global_only {
            &self.global_select
        } else {
            &self.scope_select
        })
        .id("pi-scope")
        .small()
        .w(rems(9.))
        .accessibility_label(t(cx, "pi-scope-label"))
        .disabled(global_only || self.is_saving() || self.file_pages.read(cx).busy())
        .into_any_element()
    }

    pub(in super::super) fn render_trust(&self, cx: &mut Context<Self>) -> AnyElement {
        let entity = cx.entity();
        let trust = match (&self.scope, &self.trust) {
            (Scope::Project(_), Some(Trust::Untrusted)) => {
                Some((t(cx, "pi-trust-untrusted"), true))
            }
            (Scope::Project(_), Some(Trust::Unknown(error))) => Some((error.clone(), true)),
            _ => None,
        };
        let trust = trust.map(|(status, can_trust)| {
            h_flex()
                .w_full()
                .min_w_0()
                .flex_wrap()
                .gap_2()
                .items_start()
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(rems(12.))
                        .gap_1()
                        .child(
                            div()
                                .id("pi-trust-status")
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(status),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(t(cx, "pi-trust-untrusted-help")),
                        ),
                )
                .when(can_trust, |row| {
                    row.child(
                        Button::new("pi-trust")
                            .small()
                            .label(t(cx, "pi-trust-action"))
                            .loading(self.trusting.is_some())
                            .disabled(self.trusting.is_some())
                            .on_click(move |_, window, cx| {
                                PiConfig::confirm_trust(&entity, window, cx)
                            }),
                    )
                })
                .into_any_element()
        });
        v_flex().min_w_0().children(trust).into_any_element()
    }

    fn render_header(
        &mut self,
        page: Page,
        layout: &PageLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dirty = self.has_page_drafts(page, cx);
        let saving = self.is_saving();
        let ready = matches!(self.files, Load::Ready(_)) && self.status(cx).is_none();
        let _ = (layout, window);
        let trust = self.render_trust(cx);
        let actions = h_flex()
            .w_full()
            .justify_end()
            .flex_wrap()
            .gap_2()
            .items_center()
            .when(dirty && ready, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "settings-key-unsaved")),
                )
                .child(
                    Button::new("pi-settings-discard")
                        .small()
                        .label(t(cx, "pi-settings-discard"))
                        .disabled(saving)
                        .on_click(cx.listener(move |this, _, _, cx| this.discard(Some(page), cx))),
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
            });
        v_flex()
            .w_full()
            .min_w_0()
            .gap_3()
            .child(trust)
            .child(actions)
            .into_any_element()
    }

    fn render_file_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        let path = self.agent.as_ref().ok().map(|agent| self.scope.file(agent));
        h_flex()
            .gap_1()
            .child(
                Button::new("pi-settings-open")
                    .ghost()
                    .small()
                    .icon(IconName::FolderOpen)
                    .tooltip(t(cx, "pi-settings-open-file"))
                    .accessibility_label(t(cx, "pi-settings-open-file"))
                    .disabled(self.is_saving() || path.as_ref().is_none_or(|p| !p.exists()))
                    .on_click(move |_, _, cx| {
                        if let Some(path) = &path {
                            cx.open_with_system(path);
                        }
                    }),
            )
            .child(
                Button::new("pi-settings-reload")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .tooltip(t(cx, "pi-settings-reload"))
                    .accessibility_label(t(cx, "pi-settings-reload"))
                    .disabled(self.is_saving())
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
            .into_any_element()
    }

    fn render_field(
        &mut self,
        field: Field,
        layout: &PageLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Load::Ready(_) = &self.files else {
            return div().into_any_element();
        };
        let project = matches!(self.scope, Scope::Project(_));

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
        let mut descriptions = description.into_iter().collect::<Vec<_>>();
        if field == Field::DefaultTools {
            descriptions.push(t(cx, "pi-tools-startup-help"));
        }
        if field == Field::Telemetry
            && let Some(enabled) = super::advanced::telemetry(
                std::env::var_os("PI_TELEMETRY")
                    .as_deref()
                    .map(|value| value.to_str().unwrap_or("")),
            )
        {
            descriptions.push(t(
                cx,
                if enabled {
                    "pi-telemetry-env-on"
                } else {
                    "pi-telemetry-env-off"
                },
            ));
        }
        let conflict = self.conflicts.contains(&field) || self.refresh_conflicts.contains(&field);
        let invalid = self.invalid(field, cx);
        let expanded = field.is_resource_paths()
            || matches!(
                field,
                Field::ShellPath | Field::ShellPrefix | Field::NpmCommand | Field::DefaultTools
            );
        let input_width = if field.kind() == Kind::Count {
            layout.width.min(rems(8.).to_pixels(window.rem_size()))
        } else {
            layout.content_width()
        };
        let mut input = v_flex()
            .min_w_0()
            .gap_1()
            .items_end()
            .when(expanded, |input| input.w_full().items_start())
            .when(
                !expanded && !matches!(field.kind(), Kind::Bool | Kind::Timeout),
                |input| input.w(input_width),
            )
            .child(control)
            .child(self.render_field_source(field, cx));
        if conflict || invalid {
            input = input.child(div().text_xs().text_color(cx.theme().danger).child(t(
                cx,
                if self.refresh_conflicts.contains(&field) {
                    "pi-settings-refresh-conflict"
                } else if conflict {
                    "pi-changed-on-disk"
                } else if field.is_resource_paths() {
                    "pi-resource-unsupported"
                } else if field == Field::NpmCommand {
                    "pi-npm-invalid"
                } else {
                    "pi-invalid-count"
                },
            )));
        }
        div()
            .id(SharedString::from(format!("pi-field-{}", field.id())))
            .w_full()
            .min_w_0()
            .child(settings_row(
                layout,
                t(cx, field.label()),
                (!descriptions.is_empty()).then(|| descriptions.join("\n").into()),
                input,
                expanded,
                cx,
            ))
            .into_any_element()
    }

    fn field_source(&self, field: Field, cx: &App) -> Source {
        if field.is_global_only() && matches!(self.scope, Scope::Project(_)) {
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
        }
    }

    fn render_budgets(
        &mut self,
        _layout: &PageLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        const BUDGETS: [Field; 4] = [
            Field::BudgetMinimal,
            Field::BudgetLow,
            Field::BudgetMedium,
            Field::BudgetHigh,
        ];
        let mut controls = div().w_full().min_w_0().grid().grid_cols(2).gap_4();
        for field in BUDGETS {
            let input = self.input(field, window, cx);
            controls = controls.child(
                v_flex()
                    .flex_1()
                    .min_w(rems(6.))
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, field.label())),
                    )
                    .child(
                        div().w(rems(8.)).max_w_full().child(
                            Input::new(&input)
                                .small()
                                .min_w_0()
                                .aria_label({
                                    let mut args = fluent_bundle::FluentArgs::new();
                                    args.set("level", t(cx, field.label()));
                                    t_with_args(cx, "pi-budget-input", &args)
                                })
                                .disabled(self.is_saving()),
                        ),
                    )
                    .child(self.render_field_source(field, cx))
                    .when(
                        self.invalid(field, cx)
                            || (self.conflicts.contains(&field)
                                || self.refresh_conflicts.contains(&field)),
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
        v_flex()
            .id("pi-thinking-budgets")
            .w_full()
            .min_w_0()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(t(cx, "pi-thinking-budgets")),
            )
            .child(controls)
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "pi-thinking-budget-help")),
            )
            .into_any_element()
    }

    pub(super) fn restore_default(&mut self, field: Field, cx: &mut Context<Self>) {
        if self.is_saving() || (field.is_global_only() && matches!(self.scope, Scope::Project(_))) {
            return;
        }
        self.set_values(field, vec![Slot::Unset; field.leaves().len()], cx);
        if field.is_resource_paths() {
            self.resource_inputs.remove(&field);
        }
    }

    pub(super) fn render_field_source(&self, field: Field, cx: &mut Context<Self>) -> AnyElement {
        let project = matches!(self.scope, Scope::Project(_));
        let values = self.snapshot(cx).value(field);
        let has_local = values.iter().any(|value| !matches!(value, Slot::Unset));
        let readonly = project && field.is_global_only()
            || values
                .iter()
                .any(|value| matches!(value, Slot::Unreadable(_)))
            || field == Field::Telemetry
                && super::advanced::telemetry(
                    std::env::var_os("PI_TELEMETRY")
                        .as_deref()
                        .map(|v| v.to_str().unwrap_or("")),
                )
                .is_some();
        h_flex()
            .min_w_0()
            .flex_wrap()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(super::map_controls::origin_label(
                        self.field_source(field, cx),
                        self.pending(field, cx),
                        cx,
                    )),
            )
            .when(has_local && !readonly, |row| {
                row.child(
                    Button::new(SharedString::from(format!("pi-restore-{}", field.id())))
                        .ghost()
                        .xsmall()
                        .icon(IconName::RotateCcw)
                        .label(t(
                            cx,
                            if project {
                                "pi-menu-inherit"
                            } else {
                                "pi-menu-default"
                            },
                        ))
                        .accessibility_label(format!(
                            "{}: {}",
                            t(cx, field.label()),
                            t(
                                cx,
                                if project {
                                    "pi-menu-inherit"
                                } else {
                                    "pi-menu-default"
                                }
                            )
                        ))
                        .disabled(self.is_saving())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.restore_default(field, cx)),
                        ),
                )
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
            .w_full()
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
pub(super) fn choose_project_folder(entity: &Entity<PiConfig>, window: &mut Window, cx: &mut App) {
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
