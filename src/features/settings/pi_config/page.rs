//! Rendering for the Conversation and Network pages.
use super::Load;
use super::Models;
use super::PiConfig;
use super::Support;
use super::Trust;
use super::fields::Edit;
use super::fields::Field;
use super::fields::Kind;
use super::fields::Page;
use super::fields::Source;
use super::fields::display;
use super::fields::parse_count;
use super::fields::supported_levels;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::h_flex;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::select::Select;
use gpui_kit::component::setting::SettingField;
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
    ("pi-group-model", &[Field::Model, Field::Thinking]),
    (
        "pi-group-context",
        &[
            Field::Compaction,
            Field::ReserveTokens,
            Field::KeepRecentTokens,
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
];

impl Field {
    fn label(self) -> &'static str {
        match self {
            Self::Model => "pi-field-model",
            Self::Thinking => "pi-field-thinking",
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
            Self::Model => "pi-field-model-help",
            Self::ReserveTokens => "pi-field-reserve-help",
            Self::KeepRecentTokens => "pi-field-keep-help",
            Self::AutoResize => "pi-field-auto-resize-help",
            Self::Proxy => "pi-field-proxy-help",
            _ => return None,
        })
    }
    fn id(self) -> &'static str {
        self.leaves()[self.leaves().len() - 1][self.leaves()[0].len() - 1]
    }
}

impl Page {
    fn groups(self) -> &'static [(&'static str, &'static [Field])] {
        match self {
            Self::Conversation => CONVERSATION,
            Self::Network => NETWORK,
        }
    }
    pub(in super::super) fn title(self) -> &'static str {
        match self {
            Self::Conversation => "settings-page-pi-conversation",
            Self::Network => "settings-page-pi-network",
        }
    }
}

fn level_label(level: &str, cx: &App) -> String {
    match level {
        "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" => {
            t(cx, &format!("conversation-thinking-{level}"))
        }
        other => other.to_owned(),
    }
}

fn choice_label(field: Field, value: &str, cx: &App) -> String {
    match field {
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
            })
            .resettable(false)
            .title_suffix(move |window, cx| {
                header
                    .update(cx, |this, cx| this.render_header(page, window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            });
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
            let items = fields.iter().map(|field| {
                let field = *field;
                let weak = weak.clone();
                let mut item = SettingItem::new(
                    t(cx, field.label()),
                    SettingField::render(move |_, window, cx| {
                        weak.update(cx, |this, cx| this.render_field(field, window, cx))
                            .unwrap_or_else(|_| div().into_any_element())
                    }),
                )
                .keywords(
                    field
                        .leaves()
                        .iter()
                        .map(|leaf| leaf.join("."))
                        .chain(["pi".to_owned()])
                        .collect::<Vec<_>>(),
                );
                if let Some(help) = field.help() {
                    item = item.description(t(cx, help));
                }
                item
            });
            settings = settings.group(SettingGroup::new().title(t(cx, title)).items(items));
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
            .outline()
            .label(scope_label)
            .dropdown_caret(true)
            .accessibility_label(t(cx, "pi-scope-label"))
            .when_some(path.clone(), |this, path| {
                this.tooltip(path.display().to_string())
            })
            .disabled(self.is_saving())
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
        let dirty = self.has_page_drafts(page);
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
                        PopupMenuItem::new(t(cx, "pi-settings-reload")).on_click(
                            move |_, _, cx| reload.update(cx, |this, cx| this.reload(cx)),
                        ),
                    )
                }
            });
        let _ = window;
        h_flex()
            .gap_2()
            .items_center()
            .min_w_0()
            .child(self.render_scope(cx))
            .when(dirty && ready, |this| {
                this.child(
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
            })
            .child(more)
            .into_any_element()
    }

    fn render_field(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Load::Ready(files) = &self.files else {
            return div().into_any_element();
        };
        let project = matches!(self.scope, Scope::Project(_));
        if project && field.is_global_only() {
            return self.render_global_only(field, cx);
        }
        let draft = self.drafts.get(&field).cloned();
        let resolved = files.resolve(field, draft.as_ref());
        let saving = self.is_saving();
        let id = format!("pi-{}-{}", self.generation, field.id());
        let control = match field.kind() {
            Kind::Bool => {
                let checked = resolved.values[0].as_ref().and_then(Value::as_bool) == Some(true);
                Checkbox::new(SharedString::from(id))
                    .checked(checked)
                    .disabled(saving)
                    .accessibility_label(t(cx, field.label()))
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.set_draft(field, Edit::Set(vec![Value::Bool(*checked)]), cx)
                    }))
                    .into_any_element()
            }
            Kind::Choice(options) => {
                self.render_choice(field, options, &resolved.values, saving, cx)
            }
            Kind::Count | Kind::Text => {
                self.render_input(field, &id, &resolved.values, draft.as_ref(), window, cx)
            }
            Kind::Model => self.render_model(&resolved.values, saving, window, cx),
        };
        let mut lane = match resolved.source {
            Source::Default => t(cx, "pi-source-default"),
            Source::Global => t(cx, "pi-source-global"),
            Source::Project => t(cx, "pi-source-project"),
        };
        if draft.is_some() {
            lane = format!("{lane} · {}", t(cx, "pi-unsaved"));
        }
        let conflict = self.conflicts.contains(&field);
        let invalid = matches!(draft, Some(Edit::Invalid(_)));
        v_flex()
            .gap_1()
            .items_end()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(control)
                    .child(
                        div()
                            .w_24()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(lane),
                    )
                    .child(self.render_field_menu(
                        field,
                        resolved.overridden,
                        draft.as_ref(),
                        saving,
                        cx,
                    )),
            )
            .when(conflict || invalid, |this| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(t(
                    cx,
                    if conflict {
                        "pi-changed-on-disk"
                    } else {
                        "pi-invalid-count"
                    },
                )))
            })
            .into_any_element()
    }

    fn render_field_menu(
        &self,
        field: Field,
        overridden: bool,
        draft: Option<&Edit>,
        saving: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity = cx.entity();
        let project = matches!(self.scope, Scope::Project(_));
        let has_draft = draft.is_some();
        // After saving, would the edited scope hold a value?
        let set_here = match draft {
            Some(Edit::Set(_) | Edit::Invalid(_)) => true,
            Some(Edit::Remove) => false,
            None => overridden,
        };
        Button::new(SharedString::from(format!("pi-menu-{}", field.id())))
            .ghost()
            .xsmall()
            .icon(IconName::Ellipsis)
            .disabled(saving)
            .accessibility_label(t(cx, "pi-field-actions"))
            .dropdown_menu(move |menu, _, cx| {
                let mut menu = menu;
                if has_draft {
                    let entity = entity.clone();
                    menu = menu.item(PopupMenuItem::new(t(cx, "pi-menu-revert")).on_click(
                        move |_, _, cx| entity.update(cx, |this, cx| this.clear_draft(field, cx)),
                    ));
                }
                if !set_here {
                    let entity = entity.clone();
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
                            entity.update(cx, |this, cx| this.pin(field, cx))
                        }),
                    );
                } else {
                    let entity = entity.clone();
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
                            entity.update(cx, |this, cx| this.set_draft(field, Edit::Remove, cx))
                        }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn render_choice(
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
            match self.selected_model_levels() {
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
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.set_draft(
                                        field,
                                        Edit::Set(vec![Value::from(next.clone())]),
                                        cx,
                                    )
                                })
                            }),
                    )
                })
            })
            .into_any_element()
    }

    /// Levels for the effective default model, when its metadata is known.
    fn selected_model_levels(&self) -> Option<Vec<&'static str>> {
        let (Load::Ready(files), Models::Ready(models)) = (&self.files, &self.models) else {
            return None;
        };
        let resolved = files.resolve(Field::Model, self.drafts.get(&Field::Model));
        let [Some(Value::String(provider)), Some(Value::String(id))] = resolved.values.as_slice()
        else {
            return None;
        };
        models
            .iter()
            .find(|model| &model.provider == provider && &model.id == id)
            .map(supported_levels)
    }

    fn render_input(
        &self,
        field: Field,
        id: &str,
        values: &[Option<Value>],
        draft: Option<&Edit>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let shown = display(values[0].as_ref());
        let entity = cx.entity().downgrade();
        let state = window.use_keyed_state(SharedString::from(format!("{id}-input")), cx, {
            let shown = shown.clone();
            move |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(shown));
                let subscription = cx.subscribe(&input, move |_, input, event: &InputEvent, cx| {
                    if !matches!(event, InputEvent::Change) {
                        return;
                    }
                    let text = input.read(cx).value().to_string();
                    let _ = entity.update(cx, |this, cx| this.on_text(field, text, cx));
                });
                (input, subscription)
            }
        });
        let input = state.read(cx).0.clone();
        // Show external changes (load, discard, menu actions) unless the user
        // is still correcting invalid text.
        if !matches!(draft, Some(Edit::Invalid(_))) && input.read(cx).value().as_ref() != shown {
            input.update(cx, |input, cx| input.set_value(shown, window, cx));
        }
        Input::new(&input)
            .small()
            .w_40()
            .disabled(self.is_saving())
            .into_any_element()
    }

    /// Text typed into a number or text field.
    pub(super) fn on_text(&mut self, field: Field, text: String, cx: &mut Context<Self>) {
        let Load::Ready(files) = &self.files else {
            return;
        };
        let current = files.resolve(field, self.drafts.get(&field));
        // Programmatic updates echo the value already shown.
        if display(current.values[0].as_ref()) == text {
            if matches!(self.drafts.get(&field), Some(Edit::Invalid(_))) {
                self.clear_draft(field, cx);
            }
            return;
        }
        let edit = match field.kind() {
            Kind::Count => match parse_count(&text) {
                Some(value) => Edit::Set(vec![value]),
                None => Edit::Invalid(text),
            },
            _ if text.trim().is_empty() => Edit::Remove,
            _ => Edit::Set(vec![Value::from(text.trim().to_owned())]),
        };
        self.set_draft(field, edit, cx);
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
            Models::Loading => Some((t(cx, "pi-models-loading"), false)),
            Models::Failed(error) => {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("error", error.clone());
                Some((t_with_args(cx, "pi-models-failed", &args), true))
            }
            Models::Ready(_) if !known => Some((t(cx, "pi-field-model-unavailable"), true)),
            _ => None,
        };
        v_flex()
            .gap_1()
            .items_end()
            .child(
                Select::new(&self.model_picker)
                    .small()
                    .w_64()
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
                        .child(note)
                        .when(matches!(self.models, Models::Failed(_)), |this| {
                            this.child(
                                Button::new("pi-models-retry")
                                    .xsmall()
                                    .ghost()
                                    .label(t(cx, "pi-models-retry"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.load_models(window, cx)
                                    })),
                            )
                        }),
                )
            })
            .into_any_element()
    }

    /// A project row for a field Pi reads only from the global file.
    fn render_global_only(&self, field: Field, cx: &mut Context<Self>) -> AnyElement {
        let Load::Ready(files) = &self.files else {
            return div().into_any_element();
        };
        let value = files
            .global
            .get(field.leaves()[0])
            .map(|value| display(Some(value)))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| t(cx, "pi-value-unset"));
        let entity = cx.entity();
        h_flex()
            .gap_2()
            .items_center()
            .child(div().text_sm().truncate().max_w_64().child(value))
            .child(
                Button::new(SharedString::from(format!("pi-global-{}", field.id())))
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
    let scope = super::canonical_scope(scope);
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
