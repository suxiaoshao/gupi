//! Inline diagnostic disclosure; the caller owns the error and recovery action.
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, collapsible::Collapsible, v_flex};
use gpui_kit::{prelude::*, *};
use gupi_settings::{assets::IconName, i18n::t};

/// Keeps raw diagnostics reachable by keyboard without making them the summary.
pub fn details(id: &'static str, error: &str) -> impl IntoElement + use<> {
    Details {
        id,
        error: error.to_owned(),
    }
}

#[derive(IntoElement)]
struct Details {
    id: &'static str,
    error: String,
}

impl RenderOnce for Details {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let expanded =
            window.use_keyed_state((ElementId::from(self.id), "expanded"), cx, |_, _| false);
        let open = *expanded.read(cx);
        let focus = window
            .use_keyed_state(self.id, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        Collapsible::new()
            .w_full()
            .min_w_0()
            .min_h_0()
            .max_h_full()
            .items_start()
            .open(open)
            .child(
                gpui_kit::base::Button::new(self.id)
                    .key_context("GupiErrorDetails")
                    .track_focus(&focus)
                    .accessibility_label(t(cx, "error-technical-details"))
                    .text_sm()
                    .h_6()
                    .flex_none()
                    .px_2()
                    .gap_1()
                    .rounded(cx.theme().radius)
                    .text_color(cx.theme().foreground)
                    .hover(|style| style.bg(cx.theme().secondary))
                    .border_1()
                    .border_color(cx.theme().ring.opacity(0.))
                    .focus_visible(|style| style.border_color(cx.theme().ring))
                    .child(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .small(),
                    )
                    .child(t(cx, "error-technical-details"))
                    .aria_expanded(open)
                    .on_click(move |_, window, cx| {
                        focus.focus(window, cx);
                        expanded.update(cx, |open, cx| {
                            *open = !*open;
                            cx.notify();
                        })
                    }),
            )
            .content(
                div()
                    .id((ElementId::from(self.id), "content"))
                    .test_support()
                    .w_full()
                    .min_w_0()
                    .min_h_0()
                    .max_h_48()
                    .overflow_y_scroll()
                    .whitespace_normal()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.error),
            )
            .into_any_element()
    }
}

pub(crate) fn message(
    id: &'static str,
    summary: &'static str,
    error: &str,
    cx: &App,
) -> AnyElement {
    v_flex()
        .w_full()
        .min_w_0()
        .min_h_0()
        .max_h_full()
        .gap_1()
        .child(
            div()
                .flex_none()
                .w_full()
                .min_w_0()
                .whitespace_normal()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(t(cx, summary)),
        )
        .child(details(id, error))
        .into_any_element()
}
