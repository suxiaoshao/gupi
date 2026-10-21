use super::actions::Kind;
use super::actions::Run;
use super::*;
use crate::chrome;
use gpui_kit::base::animation::EffectTransition;
use gpui_kit::base::animation::ease_in_out_cubic;
use gpui_kit::component::Selectable;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder as _;
use std::time::Duration;

impl HomeView {
    pub(super) fn render_titlebar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        #[cfg(feature = "performance")]
        let _span = tracing::debug_span!(target: "gupi::performance", "titlebar.render").entered();
        let state = self.state.read(cx);
        let temporary = state.is_temporary();
        let current = state.current();
        let title = current
            .map(|session| session.info().title())
            .filter(|title| !title.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| t(cx, "conversation-new"));
        window.set_window_title(&if temporary {
            format!("{title} — {} — Gupi", t(cx, "temporary-title"))
        } else {
            format!("{title} — Gupi")
        });
        let key = state.selected().clone();
        let has_actions = current.is_some_and(|session| {
            !session.info().path.as_os_str().is_empty() || session.has_instance()
        });
        let leading = chrome::leading_space(window);
        let target_width = if self.show_sidebar {
            px(self.pane_layout.left)
        } else {
            leading + px(72.)
        };
        let widths = window.use_keyed_state("titlebar-sidebar-width", cx, |_, _| {
            (target_width, target_width)
        });
        if self.resizing_sidebar() {
            widths.update(cx, |widths, _| *widths = (target_width, target_width));
        } else if widths.read(cx).1 != target_width {
            widths.update(cx, |widths, _| *widths = (widths.1, target_width));
        }
        let (from_width, to_width) = *widths.read(cx);
        let sidebar_label = t(
            cx,
            if self.show_sidebar {
                "conversation-hide-sidebar"
            } else {
                "conversation-show-sidebar"
            },
        );
        let left = h_flex()
            .id("titlebar-sidebar")
            .debug_selector(|| "titlebar-sidebar".into())
            .h_full()
            .flex_none()
            .pl(leading)
            .pr_3()
            .gap_1()
            .when(self.show_sidebar, |view| view.bg(cx.theme().sidebar))
            .when(!self.show_sidebar, |view| {
                view.pr_0().border_b_1().border_color(cx.theme().border)
            })
            .child(chrome::control(
                "sidebar-control",
                chrome::button("toggle-sidebar")
                    .icon(if self.show_sidebar {
                        gpui_kit::component::IconName::PanelLeftClose
                    } else {
                        gpui_kit::component::IconName::PanelLeftOpen
                    })
                    .tooltip_with_action(
                        sidebar_label.clone(),
                        &Run::new(Kind::Sidebar),
                        Some("Gupi"),
                    )
                    .accessibility_label(sidebar_label)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run_action(&Run::new(Kind::Sidebar), window, cx)
                    })),
            ))
            .when(!self.show_sidebar, |view| {
                view.child(chrome::control(
                    "new-conversation-control",
                    chrome::button("titlebar-new-conversation")
                        .icon(IconName::SquarePen)
                        .tooltip_with_action(
                            t(cx, "conversation-new"),
                            &Run::new(Kind::New),
                            Some("Gupi"),
                        )
                        .accessibility_label(t(cx, "conversation-new"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.new_conversation(window, cx);
                        })),
                ))
                .child(Separator::vertical().relative().w(px(1.)).h_5().ml_1())
            });
        let left = EffectTransition::new(Duration::from_millis(200))
            .ease(ease_in_out_cubic)
            .width(from_width, to_width)
            .apply(
                left,
                ElementId::NamedInteger(
                    "titlebar-sidebar-width".into(),
                    (from_width.as_f32().to_bits() as u64) << 32
                        | to_width.as_f32().to_bits() as u64,
                ),
            );
        let mut main = h_flex()
            .h_full()
            .flex_1()
            .min_w_0()
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            // Match the resize divider below: both start at the content edge.
            .when(self.show_sidebar, |view| view.border_l_1())
            .child(
                div()
                    .id("session-title")
                    .min_w_0()
                    .text_ellipsis()
                    .child(title.clone())
                    .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx)),
            );
        if let Some(key) = key.clone().filter(|_| has_actions) {
            let state = self.state.clone();
            let owner = cx.weak_entity();
            main = main.child(chrome::control(
                "session-menu-control",
                chrome::button("session-menu")
                    .icon(IconName::Ellipsis)
                    .tooltip(t(cx, "conversation-actions"))
                    .accessibility_label(t(cx, "conversation-actions"))
                    .dropdown_menu(move |menu, _, cx| {
                        navigation::session_menu(
                            menu,
                            key.clone(),
                            state.clone(),
                            owner.clone(),
                            cx,
                        )
                    }),
            ));
        }
        if let Some(head) = self.git.head() {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("name", head.name());
            let label = gupi_settings::i18n::t_with_args(
                cx,
                match head {
                    gupi_resources::git::Head::Branch(_) => "titlebar-git-branch",
                    gupi_resources::git::Head::Detached(_) => "titlebar-git-detached",
                },
                &args,
            );
            main = main.child(
                h_flex()
                    .id("session-git-head")
                    .debug_selector(|| "session-git-head".into())
                    .aria_label(label.clone())
                    .min_w_0()
                    .gap_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        gpui_kit::component::Icon::new(IconName::GitBranch)
                            .small()
                            .flex_shrink_0(),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .text_ellipsis()
                            .child(head.name().to_owned()),
                    )
                    .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx)),
            );
        }
        let history_bounds = self.panel_button_bounds[0].clone();

        main = main
            .child(div().flex_1())
            .child(
                div()
                    .track_focus(&self.history_button_focus)
                    .on_children_prepainted(move |bounds, _, _| {
                        history_bounds.set(bounds.first().copied())
                    })
                    .child(chrome::control(
                        "history-control",
                        chrome::button("toggle-history")
                            .icon(IconName::RotateCcwClock)
                            .selected(self.pane_layout.visible(panes::Panel::History))
                            .tooltip_with_action(
                                t(cx, "conversation-history"),
                                &Run::new(Kind::History),
                                Some("Gupi"),
                            )
                            .accessibility_label(t(cx, "conversation-history"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_panel(panes::Panel::History, window, cx)
                            })),
                    )),
            )
            .when(!self.pane_layout.workspace_docked, |row| {
                row.child(self.render_workspace_toggle(cx))
            });

        chrome::page_bar(cx).child(
            h_flex()
                .size_full()
                .min_w_0()
                .max_w(window.viewport_size().width)
                .child(left)
                .when(!self.pane_layout.maximized, |row| row.child(main))
                .when(self.pane_layout.workspace_docked, |row| {
                    row.child(
                        div()
                            .id("titlebar-workspace")
                            .debug_selector(|| "titlebar-workspace".into())
                            .h_full()
                            .min_w_0()
                            .border_l_1()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .when(self.pane_layout.maximized, |view| view.flex_1())
                            .when(!self.pane_layout.maximized, |view| {
                                view.flex_none().w(px(
                                    self.pane_layout.workspace_width + panes::geometry::DIVIDER
                                ))
                            })
                            .child(self.render_workspace_strip(true, window, cx)),
                    )
                }),
        )
    }
    pub(super) fn render_workspace_toggle(&self, cx: &mut Context<Self>) -> AnyElement {
        let bounds = self.panel_button_bounds[1].clone();
        div()
            .track_focus(&self.workspace_button_focus)
            .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
            .child(chrome::control(
                "workspace-control",
                chrome::button("toggle-workspace")
                    .icon(if self.pane_layout.visible(panes::Panel::Workspace) {
                        IconName::PanelRightClose
                    } else {
                        IconName::PanelRightOpen
                    })
                    .selected(self.pane_layout.visible(panes::Panel::Workspace))
                    .tooltip(t(cx, "files-panel"))
                    .accessibility_label(t(cx, "files-panel"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_panel(panes::Panel::Workspace, window, cx)
                    })),
            ))
            .into_any_element()
    }
}
