//! Window-owned readers for recorded summaries and live tool details.
use super::actions::copy_button;
use super::activity::Tool;
use super::diff::DiffBlock;
use super::presentation::MarkerAction;
use super::presentation::activity_marker;
use super::presentation::code_fence;
use super::presentation::marker_trigger;
use super::presentation::tool_action;
use super::presentation::tool_title;
use super::tool_details;
use super::tool_details::Detail;
use super::tool_details::Section;
use super::tools::ToolProjection;
use super::*;
use crate::tool_icon::ToolIcon;
use gpui_kit::base::TextSelection;
use gpui_kit::component::StyledExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::text::TextView;
use gpui_kit::component::text::TextViewState;

struct Target {
    key: String,
    tool: String,
    preview: Option<String>,
}
struct TextBlock {
    source: String,
    view: Entity<TextViewState>,
    _subscription: Subscription,
}
struct DetailsView {
    tool: Option<Tool>,
    metadata: Vec<Detail>,
    sections: Vec<Section>,
    text: BTreeMap<String, TextBlock>,
    diffs: BTreeMap<String, DiffBlock>,
    tool_revision: Option<(u64, u64, bool)>,
    projection: ToolProjection,
    tool_source: Option<Rc<Tool>>,
    scroll: ScrollHandle,
    follow: bool,
    nested_expanded: bool,
    _subscription: Option<Subscription>,
}

fn code_source(language: &str, text: &str) -> String {
    // Each TextView contains one code block. EOF closes it for rendering;
    // omitting the closing fence keeps cumulative output append-compatible.
    // A longer fence in the payload changes the opener and correctly replaces it.
    format!("{}{language}\n{text}", code_fence(text))
}

impl HomeView {
    pub(super) fn summary_trigger(&self, message: &DisplayMessage, cx: &App) -> impl IntoElement {
        let text = message.text();
        let selector = format!("summary-details-{}", message.id);
        Button::new(format!("summary-details-{}", message.id))
            .debug_selector(move || selector.clone())
            .ghost()
            .small()
            .icon(IconName::FileText)
            .label(t(cx, "conversation-compaction"))
            .tooltip(t(cx, "message-details-open"))
            .on_click(move |_, window, cx| {
                let text = text.clone();
                let copy = text.clone();
                let view = cx.new(|cx| {
                    let mut view = DetailsView::empty();
                    view.sections.push(Section {
                        id: "summary",
                        label: "conversation-compaction",
                        parts: vec![Detail::Code {
                            language: String::new(),
                            text,
                        }],
                    });
                    view.sync_text(window, cx);
                    view
                });
                window.open_dialog(cx, move |dialog, window, cx| {
                    dialog
                        .title(
                            h_flex()
                                .gap_2()
                                .mr_8()
                                .child(t(cx, "conversation-compaction"))
                                .child(copy_button(
                                    format!("summary-copy-all-{}", view.entity_id().as_u64()),
                                    copy.clone(),
                                    t(cx, "message-details-copy-all"),
                                    window,
                                    cx,
                                )),
                        )
                        .width(
                            (window.rem_size() * 48.)
                                .min(window.viewport_size().width - window.rem_size() * 2.),
                        )
                        .on_ok(|_, _, _| false)
                        .content({
                            let view = view.clone();
                            move |content, _, _| content.min_h_0().child(view.clone())
                        })
                });
            })
    }

    pub(super) fn tool_trigger(&self, key: &str, tool: &Tool, cx: &App) -> impl IntoElement {
        let state = self.state.clone();
        let key = key.to_owned();
        let preview = self.views[&key].preview.clone();
        let tool = tool.clone();
        let title = tool_title(&tool, cx);
        let failed = tool.status == ToolStatus::Failed;
        let error = failed.then(|| {
            gupi_conversation::session_catalog::text_content(&tool.result)
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or_default()
                .to_owned()
        });
        let id = format!("tool-details-{}", tool.id);
        let marker = activity_marker(
            MarkerContent::new()
                .min_w_0()
                .whitespace_nowrap()
                .text_ellipsis()
                .text(title.clone()),
            Some(tool.kind().icon()),
            tool.status == ToolStatus::Running,
            failed,
            cx,
        );
        #[cfg(feature = "performance")]
        let marker =
            crate::performance::measure("tool_marker", tool.status == ToolStatus::Running, marker);
        let action: MarkerAction = Rc::new(move |window, cx| {
            let target = Target {
                key: key.clone(),
                tool: tool.id.clone(),
                preview: preview.clone(),
            };
            let view = cx.new(|cx| DetailsView::for_tool(&state, target, tool.clone(), window, cx));
            let title = tool.name.clone();
            window.open_dialog(cx, move |dialog, window, _| {
                dialog
                    .title(title.clone())
                    .width(
                        (window.rem_size() * 48.)
                            .min(window.viewport_size().width - window.rem_size() * 2.),
                    )
                    .on_ok(|_, _, _| false)
                    .content({
                        let view = view.clone();
                        move |content, _, _| content.min_h_0().child(view.clone())
                    })
            });
        });
        v_flex()
            .min_w_0()
            .child(marker_trigger(id, title, marker, None, Some(action), cx))
            .when_some(error.filter(|s| !s.is_empty()), |body, error| {
                body.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .truncate()
                        .child(error),
                )
            })
    }
}

#[cfg(test)]
fn find_tool(state: &ConversationState, target: &Target) -> Option<Tool> {
    let session = state.sessions().get(&target.key)?;
    ToolProjection::default()
        .sync(session, target.preview.as_deref())
        .tool(&target.tool)
        .cloned()
}

impl DetailsView {
    fn for_tool(
        state: &Entity<ConversationState>,
        target: Target,
        tool: Tool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self::empty();
        view.tool_revision = state
            .read(cx)
            .sessions()
            .get(&target.key)
            .map(|session| session.tool_projection_revision());
        if let Some(session) = state.read(cx).sessions().get(&target.key) {
            view.tool_source = view
                .projection
                .sync(session, target.preview.as_deref())
                .shared_tool(&target.tool)
                .cloned();
        }
        view.follow = tool.status == ToolStatus::Running;
        view.set_tool(tool, window, cx);
        view._subscription = Some(cx.subscribe_in(
            state,
            window,
            move |this, state, event: &ConversationEvent, window, cx| {
                let ConversationEvent::Changed(changes) = event else {
                    return;
                };
                if !changes.bodies.contains(&target.key)
                    && !changes.sessions.contains_key(&target.key)
                {
                    return;
                }
                let state = state.read(cx);
                let Some(session) = state.sessions().get(&target.key) else {
                    return;
                };
                let revision = Some(session.tool_projection_revision());
                let tool = this
                    .projection
                    .sync(session, target.preview.as_deref())
                    .shared_tool(&target.tool)
                    .cloned();
                if revision == this.tool_revision {
                    return;
                }
                this.tool_revision = revision;
                if let Some(tool) = tool {
                    if this
                        .tool_source
                        .as_ref()
                        .is_some_and(|old| Rc::ptr_eq(old, &tool))
                    {
                        return;
                    }
                    this.tool_source = Some(tool.clone());
                    this.set_tool(tool.as_ref().clone(), window, cx);
                }
            },
        ));
        view
    }
    fn empty() -> Self {
        Self {
            tool: None,
            metadata: vec![],
            sections: vec![],
            text: BTreeMap::new(),
            diffs: BTreeMap::new(),
            tool_revision: None,
            projection: ToolProjection::default(),
            tool_source: None,
            scroll: ScrollHandle::new(),
            follow: false,
            nested_expanded: false,
            _subscription: None,
        }
    }
    fn set_tool(&mut self, tool: Tool, window: &mut Window, cx: &mut Context<Self>) {
        if self.tool.as_ref() == Some(&tool) {
            return;
        }
        let document = tool_details::document(&tool);
        self.metadata = document.metadata;
        self.sections = document.sections;
        self.tool = Some(tool);
        self.sync_text(window, cx);
        cx.notify();
    }
    fn sync_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut keep = HashSet::new();
        let mut keep_diffs = HashSet::new();
        for section in &self.sections {
            // Pi content blocks have ordered positions rather than individual IDs.
            for (index, part) in section.parts.iter().enumerate() {
                let Detail::Code { language, text } = part else {
                    continue;
                };
                let id = format!("{}-{index}", section.id);
                if language == "diff" {
                    keep_diffs.insert(id.clone());
                    if self
                        .diffs
                        .get(&id)
                        .is_none_or(|block| !block.matches_patch(text))
                    {
                        self.diffs.insert(id, DiffBlock::patch(text, cx));
                    }
                    continue;
                }
                keep.insert(id.clone());
                let source = if section.id == "summary" {
                    text.clone()
                } else {
                    code_source(language, text)
                };
                if let Some(block) = self.text.get_mut(&id) {
                    if source != block.source {
                        block.view.update(cx, |view, cx| {
                            if let Some(delta) = source.strip_prefix(&block.source) {
                                view.push_str(delta, cx);
                            } else {
                                view.set_text(&source, cx);
                            }
                        });
                        block.source = source;
                    }
                } else {
                    let view = cx.new(|cx| TextViewState::markdown(&source, cx));
                    let subscription = cx.observe_in(&view, window, |this, _, window, cx| {
                        if this.follow && !TextSelection::has_selection(window, cx) {
                            this.scroll.scroll_to_bottom();
                        }
                        cx.notify();
                    });
                    self.text.insert(
                        id,
                        TextBlock {
                            source,
                            view,
                            _subscription: subscription,
                        },
                    );
                }
            }
        }
        self.text.retain(|id, _| keep.contains(id));
        self.diffs.retain(|id, _| keep_diffs.contains(id));
    }
    fn render_nested(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        use fluent_bundle::FluentArgs;
        use gpui_kit::base::{SelectableText, TestSupportExt as _};
        use gupi_settings::i18n::t_with_args;
        let nested = &self.tool.as_ref()?.nested;
        if nested.calls.is_empty() && nested.complete != Some(false) {
            return None;
        }
        let start = if self.nested_expanded {
            0
        } else {
            nested.calls.len().saturating_sub(8)
        };
        let mut section = v_flex().min_w_0().flex_none().gap_2().child(
            h_flex()
                .justify_between()
                .gap_2()
                .child(div().font_semibold().child(t(cx, "tool-nested-calls")))
                .when(nested.calls.len() > 8, |header| {
                    header.child(
                        Button::new("nested-calls-toggle")
                            .debug_selector(|| "nested-calls-toggle".into())
                            .ghost()
                            .small()
                            .icon(if self.nested_expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .label(t(
                                cx,
                                if self.nested_expanded {
                                    "tool-nested-recent"
                                } else {
                                    "tool-nested-show-all"
                                },
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.nested_expanded = !this.nested_expanded;
                                cx.notify();
                            })),
                    )
                }),
        );
        if nested.complete == Some(false) {
            section = section.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "tool-nested-incomplete")),
            );
        }
        for call in &nested.calls[start..] {
            let mut args = FluentArgs::new();
            args.set("name", call.name.clone());
            args.set(
                "state",
                match call.status {
                    ToolStatus::Running => "running",
                    ToolStatus::Complete => "complete",
                    ToolStatus::Failed => "failed",
                    ToolStatus::Unfinished => "unfinished",
                },
            );
            let selector = format!("nested-call-{}", call.id);
            let mut row = v_flex()
                .id(SharedString::from(selector.clone()))
                .test_support()
                .debug_selector(move || selector.clone())
                .min_w_0()
                .flex_none()
                .gap_1()
                .text_sm()
                .child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(
                            div()
                                .when(call.status == ToolStatus::Failed, |label| {
                                    label.text_color(cx.theme().danger)
                                })
                                .child(t_with_args(cx, "conversation-tool-action-other", &args)),
                        )
                        .when_some(call.duration_ms, |header, duration| {
                            let mut args = FluentArgs::new();
                            args.set("milliseconds", duration.to_string());
                            header.child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t_with_args(cx, "tool-nested-duration", &args)),
                            )
                        }),
                );
            if let Some(args) = &call.args {
                row = row.child(SelectableText::new(
                    SharedString::from(format!("nested-args-{}", call.id)),
                    serde_json::to_string_pretty(args).unwrap_or_default(),
                ));
            } else if call.arguments_omitted {
                row = row.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "tool-nested-arguments-omitted")),
                );
            }
            if let Some(error) = &call.error {
                row = row.child(
                    div()
                        .text_color(cx.theme().danger)
                        .child(SelectableText::new(
                            SharedString::from(format!("nested-error-{}", call.id)),
                            error.clone(),
                        )),
                );
            }
            section = section.child(row);
        }
        Some(section.into_any_element())
    }
    fn update_follow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.follow = self.tool.is_some()
            && (self.scroll.max_offset().y + self.scroll.offset().y).abs() <= px(1.)
            && !TextSelection::has_selection(window, cx);
    }
}
impl Render for DetailsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex().min_w_0().gap_4();
        if let Some(tool) = &self.tool {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(if tool.status == ToolStatus::Failed {
                        cx.theme().danger
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(tool_action(tool, cx)),
            );
        }
        for detail in &self.metadata {
            let text = match detail {
                Detail::Path(value) | Detail::Query(value) => value.clone(),
                Detail::Field(label, value) => format!("{} {value}", t(cx, label)),
                _ => unreachable!(),
            };
            body = body.child(div().text_sm().child(text));
        }
        if let Some(nested) = self.render_nested(cx) {
            body = body.child(nested);
        }
        for section in &self.sections {
            let mut content = v_flex().min_w_0().flex_none().gap_2();
            if section.id != "summary" {
                let copy = section.copy_text();
                content = content.child(
                    h_flex()
                        .gap_2()
                        .child(div().font_semibold().child(t(cx, section.label)))
                        .when(!copy.is_empty(), |head| {
                            head.child(copy_button(
                                format!("detail-{}-copy-{}", cx.entity_id().as_u64(), section.id),
                                copy,
                                t(cx, "conversation-copy"),
                                window,
                                cx,
                            ))
                        }),
                );
            }
            for (index, part) in section.parts.iter().enumerate() {
                content = match part {
                    Detail::Code { language, .. } if language == "diff" => {
                        content.child(self.diffs[&format!("{}-{index}", section.id)].render())
                    }
                    Detail::Code { .. } => content.child(
                        TextView::new(&self.text[&format!("{}-{index}", section.id)].view)
                            .selectable(true),
                    ),
                    Detail::Image { mime, data } => content.child(tool_details::ToolImage {
                        id: format!("detail-image-{}-{index}", section.id),
                        mime: mime.clone(),
                        data: data.clone(),
                    }),
                    Detail::Notice(label) => content.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(cx, label)),
                    ),
                    Detail::Status(label, value) => {
                        content.child(div().text_sm().child(match value {
                            Some(value) => format!("{} {value}", t(cx, label)),
                            None => t(cx, label),
                        }))
                    }
                    _ => unreachable!(),
                };
            }
            body = body.child(content);
        }
        body.id("message-details-body")
            .debug_selector(|| "message-details-body".into())
            .max_h((window.rem_size() * 32.).min(window.viewport_size().height * 0.65))
            .relative()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .vertical_scrollbar(&self.scroll)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.follow = false),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|_, _, window, cx| {
                    cx.defer_in(window, |this, window, cx| this.update_follow(window, cx))
                }),
            )
            .on_scroll_wheel(cx.listener(|_, _, window, cx| {
                cx.defer_in(window, |this, window, cx| this.update_follow(window, cx))
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::DetailsView;
    use super::Target;
    use super::code_source;
    use super::find_tool;
    use gpui_kit::AppContext;
    use gpui_kit::TestAppContext;
    use gpui_kit::component::Root;
    use gupi_conversation::conversation::Changes;
    use gupi_conversation::conversation::ConversationEvent;
    use gupi_conversation::conversation::ConversationState;
    use gupi_conversation::conversation::Session;
    use gupi_conversation::conversation::ToolActivity;
    use gupi_conversation::conversation::ToolExecution;
    use serde_json::json;

    #[gpui_kit::test]
    fn cumulative_code_output_appends_and_keeps_literal_fences(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let mut source = code_source("text", "first");
        let view = cx.new(|cx| super::TextViewState::markdown(&source, cx));
        cx.run_until_parked();
        for payload in [
            "first\nsecond",
            "first\nsecond\n",
            "first\nsecond\n```rust\nlet a = 1;\n```",
        ] {
            let next = code_source("text", payload);
            if !payload.contains('`') {
                assert!(next.starts_with(&source), "ordinary output must append");
            } else {
                assert!(
                    !next.starts_with(&source),
                    "a longer delimiter needs replacement"
                );
            }
            view.update(cx, |view, cx| {
                if let Some(delta) = next.strip_prefix(&source) {
                    view.push_str(delta, cx);
                } else {
                    view.set_text(&next, cx);
                }
            });
            source = next;
            cx.run_until_parked();
            view.update(cx, |view, cx| {
                view.select_all(cx);
                // Markdown's rendered code selection terminates the last line.
                let expected = if payload.ends_with('\n') {
                    payload.to_owned()
                } else {
                    format!("{payload}\n")
                };
                assert_eq!(view.selected_text(), expected);
                view.clear_selection(cx);
            });
        }
        assert!(!code_source("rust", "first").starts_with(&code_source("text", "first")));
        assert!(!code_source("text", "replacement").starts_with(&source));
    }

    fn session(output: &str) -> Session {
        let mut session = Session::from_rpc_messages(&[
            json!({"type":"message_end", "message":{"role":"user", "timestamp":1, "content":"test"}}),
            json!({"type":"message_end", "message":{"role":"assistant", "timestamp":2, "stopReason":"toolUse", "content":[{"type":"toolCall", "id":"call", "name":"bash", "arguments":{"command":"printf hello"}}]}}),
        ]);
        session
            .tools_for_test()
            .push(std::sync::Arc::new(ToolActivity::new(
                "call".into(),
                "bash".into(),
                json!({"command":"printf hello"}),
                ToolExecution::Running(json!({"content":[{"type":"text", "text":output}]})),
            )));
        session
    }
    fn target() -> Target {
        Target {
            key: "source".into(),
            tool: "tool-call".into(),
            preview: None,
        }
    }
    #[gpui_kit::test]
    fn nested_parent_details_default_to_eight_and_expand_recorded_calls(cx: &mut TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_pi_runtime::init(cx);
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            let mut session = session("first");
            session.receive_message_for_test(
                "message_end",
                &json!({"message":{
                    "role":"toolResult", "toolCallId":"call", "toolName":"bash", "timestamp":3,
                    "isError":false, "content":"parent result", "nestedCalls":{
                        "complete":false, "calls":(0..10).map(|index|json!({
                            "id":format!("recorded-{index}"),"name":format!("nested_{index}"),
                            "status":"unfinished", "argumentsBytes":9000
                        })).collect::<Vec<_>>()
                    }
                }}),
            );
            state.sessions_for_test().insert("source".into(), session);
        });
        let mut reader = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let tool = find_tool(state.read(cx), &target()).unwrap();
            let view = cx.new(|cx| DetailsView::for_tool(&state, target(), tool, window, cx));
            reader = Some(view.clone());
            Root::new(view, window, cx)
        });
        let reader = reader.unwrap();
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("nested-call-recorded-0").is_none());
            assert!(window.try_find("nested-call-recorded-1").is_none());
            for index in 2..10 {
                assert!(
                    window
                        .try_find(format!("nested-call-recorded-{index}"))
                        .is_some()
                );
            }
            window.click("nested-calls-toggle", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("nested-call-recorded-0").is_some());
            assert!(reader.read(cx).nested_expanded);
            window.click("nested-calls-toggle", cx);
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("nested-call-recorded-0").is_none());
            assert!(!reader.read(cx).nested_expanded);
        });
    }

    #[gpui_kit::test]
    fn tool_reader_updates_only_its_source_and_survives_message_identity_changes(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::host::install_headless(cx);
            gupi_settings::i18n::apply(Default::default(), cx);
            gupi_pi_runtime::init(cx);
        });
        let state = cx.new(|cx| ConversationState::new("unused-pi".into(), cx));
        state.update(cx, |state, _| {
            state
                .sessions_for_test()
                .insert("source".into(), session("first"));
        });
        let mut reader = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let tool = find_tool(state.read(cx), &target()).unwrap();
            let view = cx.new(|cx| DetailsView::for_tool(&state, target(), tool, window, cx));
            reader = Some(view.clone());
            Root::new(view, window, cx)
        });
        let reader = reader.unwrap();
        visual.run_until_parked();
        let text = visual.update(|_, cx| reader.read(cx).text["output-0"].view.clone());
        state.update(visual, |state, cx| {
            let mut next = session("first\nsecond");
            // Reconciliation can replace live message IDs with persisted entry IDs.
            for message in next.live_for_test() {
                message.id = format!("entry-{}", message.id);
            }
            state.sessions_for_test().insert("source".into(), next);
            cx.emit(ConversationEvent::Changed({
                let mut record = Changes::default();
                record.bodies = ["other".into()].into();
                record
            }));
        });
        visual.run_until_parked();
        visual.update(|_, cx| assert_eq!(reader.read(cx).sections[1].copy_text(), "first"));
        state.update(visual, |_, cx| {
            cx.emit(ConversationEvent::Changed({
                let mut record = Changes::default();
                record.bodies = ["source".into()].into();
                record
            }))
        });
        visual.run_until_parked();
        visual.update(|_, cx| {
            let reader = reader.read(cx);
            assert_eq!(reader.sections[1].copy_text(), "first\nsecond");
            assert_eq!(reader.text["output-0"].view, text);
            assert!(!state.read(cx).sessions()["source"].has_instance());
        });
        visual.update(|window, cx| {
            window.draw(cx).clear(cx);
            text.update(cx, |text, cx| text.select_all(cx));
            reader.update(cx, |reader, cx| {
                reader.update_follow(window, cx);
                assert!(!reader.follow, "selecting output must pause following");
            });
        });
        let weak = reader.downgrade();
        visual.update(|window, _| window.remove_window());
        drop(reader);
        visual.run_until_parked();
        assert!(weak.upgrade().is_none());
    }
}
