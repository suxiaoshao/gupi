use super::*;
use gpui_kit::component::Sizable;
use gpui_kit::component::ThemeMode as Mode;
use gpui_kit::component::form::field;
use gpui_kit::component::form::v_form;
use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::component::select::Select;
use gpui_kit::prelude::FluentBuilder;
use gupi_settings::i18n::ProbeFailureKey;
use gupi_settings::theme;

#[derive(Clone)]
pub(super) struct LanguageItem {
    value: AppLanguage,
    title: SharedString,
    keywords: &'static str,
}
impl SearchableListItem for LanguageItem {
    type Value = AppLanguage;
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(SharedString::from(format!(
                "language-option-{:?}",
                self.value
            )))
            .role(Role::Label)
            .aria_label(self.title.clone())
            .child(self.title.clone())
    }
    fn value(&self) -> &AppLanguage {
        &self.value
    }
    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query) || self.keywords.contains(&query)
    }
}
pub(super) fn language_items(cx: &App) -> SearchableVec<LanguageItem> {
    SearchableVec::new(vec![
        LanguageItem {
            value: AppLanguage::System,
            title: t(cx, "language-system").into(),
            keywords: "system auto",
        },
        LanguageItem {
            value: AppLanguage::English,
            title: t(cx, "language-english").into(),
            keywords: "en english 英语 英文",
        },
        LanguageItem {
            value: AppLanguage::Chinese,
            title: t(cx, "language-chinese").into(),
            keywords: "zh zh-cn chinese simplified 简体中文 简体 中文 汉语",
        },
        LanguageItem {
            value: AppLanguage::TraditionalChinese,
            title: t(cx, "language-traditional-chinese").into(),
            keywords: "zh-tw zh-hant traditional chinese 繁体中文 繁體中文",
        },
        LanguageItem {
            value: AppLanguage::Japanese,
            title: t(cx, "language-japanese").into(),
            keywords: "ja japanese 日本語 日语 日文",
        },
        LanguageItem {
            value: AppLanguage::Korean,
            title: t(cx, "language-korean").into(),
            keywords: "ko korean 한국어 조선어 韩语 韓語",
        },
        LanguageItem {
            value: AppLanguage::German,
            title: t(cx, "language-german").into(),
            keywords: "de german deutsch 德语 德文",
        },
        LanguageItem {
            value: AppLanguage::French,
            title: t(cx, "language-french").into(),
            keywords: "fr french français francais 法语 法文",
        },
        LanguageItem {
            value: AppLanguage::Spanish,
            title: t(cx, "language-spanish").into(),
            keywords: "es spanish español espanol 西班牙语 西语",
        },
        LanguageItem {
            value: AppLanguage::BrazilianPortuguese,
            title: t(cx, "language-portuguese-brazil").into(),
            keywords: "pt pt-br portuguese português portugues brasil brazilian 巴西葡萄牙语",
        },
    ])
}
impl SettingsView {
    pub(super) fn render_language(&self, cx: &Context<Self>) -> AnyElement {
        v_flex()
            .gap_3()
            .child(
                v_form().child(
                    field().label(t(cx, "settings-language")).child(
                        Select::new(&self.language)
                            .id("settings-language")
                            .accessibility_label(t(cx, "settings-app-language"))
                            .w_full()
                            .when(self.controller.read(cx).is_onboarding(cx), |control| {
                                control.large()
                            })
                            .cleanable(false)
                            .disabled(self.controller.read(cx).busy(cx))
                            .search_placeholder(t(cx, "setup-search-language")),
                    ),
                ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child({
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set(
                            "language",
                            gupi_settings::i18n::system_language_autonym(cx).to_owned(),
                        );
                        gupi_settings::i18n::t_with_args(cx, "setup-system-language", &args)
                    }),
            )
            .into_any_element()
    }
    pub(super) fn probe_matches(&self, cx: &App) -> bool {
        let command = self.pi_command(cx);
        self.draft_pi.read(cx).matches_command(command.as_deref())
    }
    pub(super) fn probe_ready(&self, cx: &App) -> bool {
        let command = self.pi_command(cx);
        self.draft_pi
            .read(cx)
            .ready_for(command.as_deref())
            .is_some()
    }
    pub(super) fn render_pi(&self, cx: &Context<Self>) -> AnyElement {
        let pi = self.draft_pi.read(cx);
        let matching = self.probe_matches(cx);
        let busy = self.controller.read(cx).busy(cx)
            || self
                .resources
                .read(cx)
                .controller
                .read(cx)
                .mutation()
                .is_running();
        let onboarding = self.controller.read(cx).is_onboarding(cx);
        let dirty = self.controller.read(cx).pi_form().read(cx).is_dirty();
        let label = t(cx, "settings-pi-command");
        let state = (!onboarding).then(|| {
            t(
                cx,
                if dirty {
                    "settings-pi-draft"
                } else {
                    "settings-pi-saved-tag"
                },
            )
        });
        let input = if onboarding {
            v_flex()
                .child(
                    v_form().child(
                        field()
                            .label_fn(move |_, _| {
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(label.clone())
                                    .children(state.clone().map(|state| {
                                        gpui_kit::component::tag::Tag::secondary()
                                            .outline()
                                            .small()
                                            .child(state)
                                    }))
                            })
                            .description(t(cx, "settings-path-help"))
                            .child(
                                Input::new(self.pi_input(cx))
                                    .when(self.controller.read(cx).is_onboarding(cx), |input| {
                                        input.large()
                                    })
                                    .disabled(busy),
                            ),
                    ),
                )
                .into_any_element()
        } else {
            v_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "settings-path-help")),
                )
                .child(Input::new(self.pi_input(cx)).disabled(busy))
                .children(state.map(|state| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(state)
                }))
                .into_any_element()
        };
        let mut view = v_flex().gap_4().child(input);
        let actions = h_flex()
            .w_full()
            .gap_2()
            .flex_wrap()
            .when(!onboarding, |actions| actions.justify_end())
            .child(
                Button::new("check-draft-pi")
                    .icon(IconName::RotateCw)
                    .loading(pi.is_running())
                    .label(t(
                        cx,
                        if pi.is_running() {
                            "startup-checking"
                        } else {
                            "setup-check-pi"
                        },
                    ))
                    .disabled(busy || pi.is_running())
                    .on_click(cx.listener(|this, _, _, cx| {
                        match PiSettings::new(this.pi_command(cx)).normalized() {
                            Ok(config) => {
                                this.error = None;
                                this.draft_pi
                                    .update(cx, |pi, cx| pi.request(config.command, true, cx));
                            }
                            Err(error) => this.error = Some(error),
                        }
                        cx.notify();
                    })),
            );
        view = view.child(actions.when(!onboarding, |actions| {
            actions.child(
                Button::new("save-pi")
                    .primary()
                    .label(t(cx, "settings-save-pi"))
                    .disabled(busy || !dirty)
                    .loading(self.controller.read(cx).is_running(cx))
                    .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
            )
        }));
        if !onboarding {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(
                        cx,
                        if dirty {
                            "settings-pi-unsaved"
                        } else {
                            "settings-pi-saved"
                        },
                    )),
            );
        }
        if matching && !pi.is_running() {
            if let Some(problem) = pi.problem() {
                view = view.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(t(cx, problem.key()))
                        .child(div().text_xs().child(problem.to_string())),
                );
            } else if self.probe_ready(cx)
                && let Some(data) = pi.data()
            {
                view = view.child(
                    v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(t(cx, "home-ready"))
                                .child(data.version.clone()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(data.command.display().to_string()),
                        ),
                );
            }
        }
        view.into_any_element()
    }
}

impl SettingsView {
    /// Settings Pi page: the saved path's latest check first, then the editable path.
    pub(super) fn render_pi_settings(
        &self,
        layout: &crate::features::settings::composition::PageLayout,
        cx: &Context<Self>,
    ) -> AnyElement {
        let applied = self.applied_pi.read(cx);
        use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
        let (color, status) = if applied.is_running() {
            (cx.theme().muted_foreground, t(cx, "startup-checking"))
        } else if let Some(problem) = applied.problem() {
            (cx.theme().danger, t(cx, problem.key()))
        } else if let Some(data) = applied.data() {
            (
                cx.theme().success,
                format!("{} · {}", t(cx, "home-ready"), data.version),
            )
        } else {
            (
                cx.theme().muted_foreground,
                t(cx, "settings-about-unavailable"),
            )
        };
        let path = applied
            .data()
            .map(|data| data.command.display().to_string())
            .unwrap_or_default();
        v_flex()
            .w_full()
            .gap_6()
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t(cx, "settings-pi-current")),
                    )
                    .child(
                        GroupBox::new()
                            .fill()
                            .content_style(StyleRefinement::default().px_5().py_0().gap_0())
                            .child(crate::features::settings::composition::settings_row(
                                layout,
                                t(cx, "settings-pi-saved-connection"),
                                Some(path.into()),
                                h_flex()
                                    .id("settings-pi-status")
                                    .gap_2()
                                    .min_w_0()
                                    .child(div().size_2().rounded_full().bg(color))
                                    .child(div().text_sm().child(status)),
                                false,
                                cx,
                            )),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t(cx, "settings-pi-command")),
                    )
                    .child(
                        GroupBox::new()
                            .fill()
                            .content_style(StyleRefinement::default().px_5().py_4())
                            .child(self.render_pi(cx)),
                    ),
            )
            .into_any_element()
    }
}

// Build an equal-width grid: choose columns from the available width,
// then let grid tracks share all remaining space after the gaps.
pub(super) fn theme_columns(width: f32) -> u16 {
    if !width.is_finite() || width <= 178. {
        return 1;
    }
    let columns = ((width + 12.) / (178. + 12.)).floor().max(1.) as u16;
    let tile_width = (width - 12. * (columns - 1) as f32) / columns as f32;
    let next_width = (width - 12. * columns as f32) / (columns + 1) as f32;
    if tile_width > 220. && next_width >= 178. * 0.95 {
        columns + 1
    } else {
        columns
    }
}

pub(super) fn theme_grid(
    group: (&'static str, Mode, Vec<app_theme::ThemeChoice>),
    draft: &AppConfig,
    controller: &Entity<ConfigController>,
    busy: bool,
) -> AnyElement {
    super::theme_grid::ThemeGrid {
        group,
        draft: draft.clone(),
        controller: controller.clone(),
        busy,
    }
    .into_any_element()
}

pub(super) fn theme_grid_content(
    group: (&'static str, Mode, Vec<app_theme::ThemeChoice>),
    draft: &AppConfig,
    controller: &Entity<ConfigController>,
    columns: u16,
    busy: bool,
    cx: &App,
) -> AnyElement {
    use gpui_kit::component::Selectable as _;
    let (id, mode, choices) = group;
    let selected_id = theme::selected_id(draft, mode);
    div()
        .id(id)
        .w_full()
        .grid()
        .grid_cols(columns)
        .gap_3()
        .children(choices.into_iter().enumerate().map(|(index, choice)| {
            let selected = choice.id == selected_id;
            // These colors are the preview's data, not an application palette.
            let colors = app_theme::preview_theme(&choice.config).colors;
            let name = if app_theme::is_system_accent_material_you_theme_id(&choice.id) {
                SharedString::from(t(cx, "setup-system-accent"))
            } else {
                choice.name
            };
            let controller = controller.clone();
            let theme_id = choice.id;
            let hover_group: SharedString = format!("preview-{id}-{theme_id}").into();
            div()
                .id(hover_group.clone())
                .group(hover_group.clone())
                .min_w_0()
                .child(
                    Button::new(SharedString::from(format!("{id}-{theme_id}")))
                        .custom(gpui_kit::component::button::ButtonCustomVariant::new(cx))
                        .w_full()
                        .h_auto()
                        .p_1()
                        .whitespace_normal()
                        .selected(selected)
                        .toggled(selected)
                        .disabled(busy)
                        .accessibility_label(name.clone())
                        .debug_selector(move || format!("{id}-tile-{index}"))
                        .child(
                            v_flex()
                                .w_full()
                                .min_w_0()
                                .gap_2()
                                .child(
                                    div()
                                        .w_full()
                                        .p_0p5()
                                        .border_1()
                                        .rounded_lg()
                                        .border_color(if selected {
                                            cx.theme().primary
                                        } else {
                                            cx.theme().border
                                        })
                                        .when(!selected, |preview| {
                                            preview.group_hover(hover_group, |style| {
                                                style.border_color(cx.theme().muted_foreground)
                                            })
                                        })
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .h(rems(6.))
                                                .rounded_md()
                                                .overflow_hidden()
                                                .bg(colors.background)
                                                .text_color(colors.foreground)
                                                .items_stretch()
                                                .child(
                                                    v_flex()
                                                        .w(relative(0.25))
                                                        .flex_shrink_0()
                                                        .p_2()
                                                        .py_4()
                                                        .gap_2()
                                                        .bg(colors.sidebar)
                                                        .child(
                                                            div().h_1().w_full().rounded_sm().bg(
                                                                colors.foreground.opacity(0.16),
                                                            ),
                                                        )
                                                        .child(
                                                            div()
                                                                .h_1()
                                                                .w_full()
                                                                .rounded_sm()
                                                                .bg(colors.primary.opacity(0.7)),
                                                        )
                                                        .child(
                                                            div().h_1().w_full().rounded_sm().bg(
                                                                colors.foreground.opacity(0.16),
                                                            ),
                                                        ),
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .justify_center()
                                                        .items_start()
                                                        .p_2()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .child("Aa 中文"),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(
                                                                    colors.foreground.opacity(0.7),
                                                                )
                                                                .child(t(
                                                                    cx,
                                                                    "settings-theme-preview",
                                                                )),
                                                        )
                                                        .child(
                                                            div()
                                                                .h_1p5()
                                                                .w_8()
                                                                .rounded_sm()
                                                                .bg(colors.primary),
                                                        ),
                                                ),
                                        ),
                                )
                                .child(preview_caption(name, selected, cx)),
                        )
                        .on_click(move |_, _, cx| {
                            controller.update(cx, |owner, cx| {
                                owner.set_preference(
                                    match mode {
                                        Mode::Light => {
                                            PreferenceChange::LightTheme(Some(theme_id.clone()))
                                        }
                                        Mode::Dark => {
                                            PreferenceChange::DarkTheme(Some(theme_id.clone()))
                                        }
                                    },
                                    cx,
                                )
                            })
                        }),
                )
        }))
        .into_any_element()
}

/// A full caption is part of the selection surface, outside the preview image.
pub(super) fn preview_caption(name: SharedString, selected: bool, cx: &App) -> AnyElement {
    h_flex()
        .w_full()
        .min_w_0()
        .justify_center()
        .items_start()
        .gap_1()
        .text_xs()
        .when(selected, |caption| {
            caption.font_weight(FontWeight::SEMIBOLD)
        })
        .when(selected, |caption| {
            caption.child(
                gpui_kit::component::Icon::new(IconName::Check)
                    .size_3()
                    .flex_shrink_0()
                    .text_color(cx.theme().primary),
            )
        })
        .child(
            div()
                .min_w_0()
                .whitespace_normal()
                .text_center()
                .child(name),
        )
        .into_any_element()
}
