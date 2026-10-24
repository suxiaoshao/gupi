//! Direct Pi bash records are Shell rows, never synthetic agent tool calls.
use super::presentation::code_fence;
use super::viewport::ActivityViewport;
use super::*;
use fluent_bundle::FluentArgs;
use gpui_kit::component::Icon;
use gupi_settings::i18n::t_with_args;

const PREVIEW_LINES: usize = 80;

pub(super) fn plain_output(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.next() {
                Some('[') => {
                    for ch in chars.by_ref() {
                        if ('@'..='~').contains(&ch) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(ch) = chars.next() {
                        if ch == '\u{7}' || ch == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if ch == '\r' {
            if chars.peek() != Some(&'\n') {
                output.push('\n');
            }
        } else if ch == '\n' || ch == '\t' || !ch.is_control() {
            output.push(ch);
        }
    }
    output
}
fn preview_tail(output: &str) -> String {
    let mut lines = output.lines().rev().take(PREVIEW_LINES).collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
}
fn code(text: &str) -> String {
    format!("{}text\n{text}", code_fence(text))
}

pub(super) fn find_sources(row: &str, message: &DisplayMessage) -> Vec<super::super::find::Source> {
    [
        (
            "command",
            message.value["command"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        ),
        (
            "output",
            plain_output(message.value["output"].as_str().unwrap_or_default()),
        ),
    ]
    .into_iter()
    .filter(|(_, text)| !text.is_empty())
    .map(|(part, text)| super::super::find::Source {
        id: format!("shell-{}-{part}", message.id),
        row: row.to_owned(),
        text,
        process: Some(format!("shell-output-{}", message.id)),
    })
    .collect()
}
impl HomeView {
    pub(super) fn render_shell(
        &self,
        key: &str,
        row: &ChatRow,
        message: &DisplayMessage,
        cx: &App,
    ) -> AnyElement {
        let value = &message.value;
        let running = value["shellRunning"] == true;
        let stopping = value["shellStopping"] == true;
        let command = value["command"].as_str().unwrap_or_default();
        let output = plain_output(value["output"].as_str().unwrap_or_default());
        let status = if running {
            t(
                cx,
                if stopping {
                    "shell-stopping"
                } else {
                    "shell-running"
                },
            )
        } else if let Some(error) = value["shellError"].as_str() {
            error.to_owned()
        } else if value["cancelled"] == true {
            t(cx, "shell-cancelled")
        } else if let Some(exit) = value["exitCode"].as_i64() {
            let mut args = FluentArgs::new();
            args.set("code", exit);
            t_with_args(cx, "shell-exit-code", &args)
        } else {
            t(cx, "shell-completed")
        };
        let failed = value["shellError"].is_string()
            || value["exitCode"].as_i64().is_some_and(|exit| exit != 0);
        let mut header = h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .items_center()
            .child(Icon::new(IconName::Terminal).size_4())
            .child(t(cx, "shell-title"))
            .child(
                div()
                    .text_xs()
                    .text_color(if failed {
                        cx.theme().danger
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(status),
            );
        if running {
            let state = self.state.clone();
            let target = key.to_owned();
            header = header.child(
                Button::new(format!("shell-stop-{}", message.id))
                    .small()
                    .ghost()
                    .ml_auto()
                    .icon(IconName::Square)
                    .label(t(cx, "shell-stop"))
                    .accessibility_label(t(cx, "shell-stop"))
                    .disabled(
                        self.state
                            .read(cx)
                            .sessions()
                            .get(key)
                            .is_none_or(|s| s.shell_stopping()),
                    )
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, cx| state.stop_shell(&target, cx))
                    }),
            );
        }
        let mut content = v_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .when(value["excludeFromContext"] == true, |content| {
                content.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "shell-excluded")),
                )
            })
            .child(self.text_view(
                key,
                &row.id,
                format!("shell-{}-command", message.id),
                code(command),
                cx,
            ));
        let mut preview_truncated = false;
        if !output.is_empty() {
            let expanded = self.views[key]
                .process_open
                .get(&format!("shell-full-{}", message.id))
                .copied()
                .unwrap_or(false);
            let long = output.lines().count() > PREVIEW_LINES;
            preview_truncated = long && !expanded;
            let preview = if preview_truncated {
                preview_tail(&output)
            } else {
                output.clone()
            };
            let body = ActivityViewport::new(
                format!("shell-scroll-{}", message.id),
                self.text_view(
                    key,
                    &row.id,
                    format!("shell-{}-output", message.id),
                    code(&preview),
                    cx,
                )
                .into_any_element(),
            );
            content = content.child(self.fold(
                (key, &row.id),
                &format!("shell-output-{}", message.id),
                Disclosure::resource(t(cx, "tool-detail-output")),
                true,
                body.into_any_element(),
                cx,
            ));
            if long {
                let owner = self.history_list.read(cx).delegate().owner.clone();
                let key = key.to_owned();
                let row_id = row.id.clone();
                let id = format!("shell-full-{}", message.id);
                content = content.child(
                    Button::new(id.clone())
                        .ghost()
                        .small()
                        .label(t(
                            cx,
                            if expanded {
                                "shell-show-less"
                            } else {
                                "shell-show-all"
                            },
                        ))
                        .on_click(move |_, _, cx| {
                            let _ = owner.update(cx, |home, cx| {
                                home.toggle_process(&key, &row_id, &id, expanded, cx)
                            });
                        }),
                );
            }
        }
        if value["shellDisplayTruncated"] == true || preview_truncated {
            content = content.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "shell-display-truncated")),
            );
        }
        if value["truncated"] == true {
            content = content.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "tool-detail-truncated")),
            );
        }
        if let Some(path) = value["fullOutputPath"].as_str() {
            let path = PathBuf::from(path);
            let owner = self.history_list.read(cx).delegate().owner.clone();
            content = content.child(
                Button::new(format!("shell-full-output-{}", message.id))
                    .small()
                    .ghost()
                    .label(t(cx, "shell-open-full-output"))
                    .on_click(move |_, window, cx| {
                        let _ = owner.update(cx, |home, cx| {
                            if home.state.read(cx).is_temporary() {
                                cx.open_with_system(&path);
                            } else {
                                home.open_source(path.clone(), true, window, cx);
                                home.ensure_panel(
                                    super::super::panes::Panel::Workspace,
                                    window,
                                    cx,
                                );
                            }
                        });
                    }),
            );
        }
        Message::new()
            .header(MessageHeader::new().content_inset(false).child(header))
            .content(MessageContent::new().child(content))
            .footer(
                MessageFooter::new()
                    .content_inset(false)
                    .child(actions::MessageActions {
                        id: format!("shell-copy-{key}-{}", message.id),
                        message: message.clone(),
                        text: format!("{command}\n\n{output}"),
                        before_copy: None,
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plain_shell_output_keeps_text_and_newlines_without_terminal_controls() {
        let output = (0..100)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let tail = super::preview_tail(&output);
        assert!(tail.starts_with("line 20\n"));
        assert!(tail.ends_with("line 99"));
        assert_eq!(
            super::plain_output("\x1b[31mred\x1b[0m\r\nnext\rline\x1b]0;title\x07!"),
            "red\nnext\nline!"
        );
        assert_eq!(
            super::plain_output("\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
    }
    #[gpui_kit::test]
    fn recorded_shell_rows_render_expand_copy_and_reveal_find(cx: &mut gpui_kit::TestAppContext) {
        use crate::home::HomeView;
        use gpui_kit::component::Root;
        use gpui_kit::test::TestWindowExt;
        use gpui_kit::{AppContext, px, size};
        use gupi_conversation::conversation::{ConversationState, Session};
        use serde_json::json;
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            app_theme::init(cx);
            gupi_settings::theme::init(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gpui_tokio::init(cx);
            gupi_pi_runtime::init(cx);
            cx.set_global(gupi_settings::layout::LayoutState::default());
        });
        let output = (0..100)
            .map(|line| format!("output line {line}\n"))
            .collect::<String>();
        let state = cx.new(|cx| ConversationState::temporary("unused".into(), cx));
        state.update(cx, |state, _| {
            let session = Session::from_rpc_messages(&[json!({"type":"message_end","message":{
                "role":"bashExecution","command":"printf fixture","output":output,"timestamp":1,
                "exitCode":7,"cancelled":false,"truncated":true,"excludeFromContext":true
            }})]);
            state.sessions_for_test().insert("a".into(), session);
            *state.selected_for_test() = Some("a".into());
        });
        let mut home = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| HomeView::with_state(state.clone(), window, cx));
            home = Some(view.clone());
            Root::new(view, window, cx)
        });
        visual.simulate_resize(size(px(900.), px(900.)));
        visual.run_until_parked();
        let home = home.unwrap();
        let id = visual.update(|window, cx| {
            window.render_frame(cx);
            let row = &home.read(cx).views["a"].rows[0];
            assert!(matches!(row.kind, super::RowKind::Shell(_)));
            assert_eq!(row.find_sources().len(), 2);
            row.id.clone()
        });
        visual.update(|window, cx| {
            window.click(format!("shell-full-{id}"), cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            assert_eq!(
                home.read(cx).views["a"]
                    .process_open
                    .get(&format!("shell-full-{id}")),
                Some(&true)
            );
            let scroller = home.read(cx).views["a"].scroller.clone();
            scroller.update(cx, |s, cx| s.scroll_to_end(cx));
            window.render_frame(cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.click(format!("copy-shell-copy-a-{id}"), cx);
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some(format!("printf fixture\n\n{output}"))
            );
            let row = &home.read(cx).views["a"].rows[0];
            let mut open = Default::default();
            row.reveal_find(&mut open);
            assert_eq!(open.get(&format!("shell-output-{id}")), Some(&true));
            assert_eq!(open.get(&format!("shell-full-{id}")), Some(&true));
        });
    }
}
