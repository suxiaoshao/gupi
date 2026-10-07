use super::actions::Kind;
use super::actions::Run;
use super::*;
use crate::chrome;
use gpui_kit::base::animation::EffectTransition;
use gpui_kit::base::animation::ease_in_out_cubic;
use gpui_kit::component::Icon;
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
        let loading = current.is_some_and(|s| s.core_read().running() || s.is_reconnecting());
        let can_refresh = key.as_ref().is_some_and(|key| state.can_reconnect(key, cx));
        let can_export = key.as_ref().is_some_and(|key| state.can_export(key, cx));
        let exporting = current.is_some_and(|session| session.is_exporting());
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
            .child(Icon::new(IconName::Folder).small())
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
        main = main
            .child(div().flex_1())
            .when(!temporary, |row| {
                row.child(chrome::control(
                    "export-control",
                    chrome::button("export-session")
                        .icon(IconName::Download)
                        .loading(exporting)
                        .disabled(!can_export)
                        .tooltip_with_action(
                            t(cx, "conversation-export"),
                            &Run::new(Kind::Export),
                            Some("Gupi"),
                        )
                        .accessibility_label(t(cx, "conversation-export"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.run_action(&Run::new(Kind::Export), window, cx);
                        })),
                ))
                .child(chrome::control(
                    "refresh-control",
                    chrome::button("refresh-session")
                        .icon(IconName::RefreshCw)
                        .loading(loading)
                        .disabled(!can_refresh)
                        .tooltip_with_action(
                            t(cx, "conversation-refresh-current"),
                            &Run::new(Kind::Reconnect),
                            Some("Gupi"),
                        )
                        .accessibility_label(t(cx, "conversation-refresh-current"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(key) = &key {
                                this.state.update(cx, |state, cx| state.reconnect(key, cx));
                            }
                        })),
                ))
            })
            .child(chrome::control(
                "history-control",
                chrome::button("toggle-history")
                    .icon(IconName::PanelRight)
                    .tooltip(t(cx, "files-panel"))
                    .accessibility_label(t(cx, "files-panel"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.show_history {
                            this.close_navigator(window, cx);
                        } else {
                            this.show_history = true;
                            this.sync(false, window, cx);
                            this.focus_overlay_navigator(window, cx);
                        }
                        cx.notify();
                    })),
            ));
        chrome::page_bar(cx).child(h_flex().size_full().min_w_0().child(left).child(main))
    }
}
