use super::activity::Tool;
use super::viewport::ActivityViewport;
use super::*;
use crate::foundation::i18n::t_with_args;
use crate::foundation::tool_presentation::ToolKind;
use fluent_bundle::FluentArgs;
use gpui_kit::component::{
    Icon,
    collapsible::Collapsible,
    marker::{Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant},
    tooltip::Tooltip,
};
use gpui_kit::prelude::FluentBuilder;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Run,
    Group,
    Tool,
}

pub(super) type MarkerAction = Rc<dyn Fn(&mut Window, &mut App)>;

pub(super) fn activity_marker(
    content: MarkerContent,
    icon: Option<IconName>,
    loading: bool,
    failed: bool,
    cx: &App,
) -> Marker {
    Marker::new()
        .with_variant(MarkerVariant::Plain)
        .loading(loading)
        .with_loading_style(MarkerLoadingStyle::Shimmer)
        .min_w_0()
        .gap_1()
        .when(failed, |marker| marker.text_color(cx.theme().danger))
        .when_some(icon, |marker, icon| {
            marker.icon(MarkerIcon::new().child(Icon::new(icon).size_4()))
        })
        .content(content)
}

pub(super) fn marker_trigger(
    id: String,
    title: String,
    marker: Marker,
    expanded: Option<bool>,
    action: Option<MarkerAction>,
    cx: &App,
) -> impl IntoElement {
    let selector = id.clone();
    div()
        .id(id.clone())
        .debug_selector(move || selector.clone())
        .test_support()
        .group(SharedString::from(format!("disclosure-{id}")))
        .w_full()
        .min_w_0()
        .when(expanded.is_none(), |trigger| {
            let label = t(cx, "message-details-open");
            trigger.tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        })
        .when_some(action, |trigger, action| {
            let click = action.clone();
            let keyboard = action.clone();
            trigger
                .role(Role::Button)
                .aria_label(title)
                .when_some(expanded, |trigger, open| trigger.aria_expanded(open))
                .focusable()
                .tab_stop(true)
                .focus_visible(|style| style.bg(cx.theme().muted))
                .hover(|style| style.text_color(cx.theme().foreground))
                .on_click(move |_, window, cx| click(window, cx))
                .on_key_down(move |event, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        cx.stop_propagation();
                        keyboard(window, cx);
                    }
                })
                .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    action(window, cx)
                })
        })
        .child(marker)
}
pub(super) struct Disclosure {
    title: String,
    clock: Option<Entity<super::super::progress::ProcessClock>>,
    icon: Option<IconName>,
    level: Level,
    loading: bool,
    failed: bool,
    locked: bool,
}
impl Disclosure {
    pub fn run(title: String) -> Self {
        Self {
            title,
            clock: None,
            icon: None,
            level: Level::Run,
            loading: false,
            failed: false,
            locked: false,
        }
    }
    pub fn resource(title: String) -> Self {
        Self {
            level: Level::Group,
            ..Self::run(title)
        }
    }
    pub fn clock(mut self, clock: Option<Entity<super::super::progress::ProcessClock>>) -> Self {
        self.clock = clock;
        self
    }
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }
}

impl HomeView {
    pub(super) fn fold(
        &self,
        location: (&str, &str),
        id: &str,
        heading: Disclosure,
        default: bool,
        content: AnyElement,
        cx: &App,
    ) -> AnyElement {
        let (key, row) = location;
        let open = heading.locked
            || self
                .views
                .get(key)
                .and_then(|v| v.process_open.get(id))
                .copied()
                .unwrap_or(default);
        let owner = self.history_list.read(cx).delegate().owner.clone();
        let key = key.to_owned();
        let id = id.to_owned();
        let target = id.clone();
        let row = row.to_owned();
        let action = Rc::new(move |_: &mut Window, cx: &mut App| {
            let _ = owner.update(cx, |this, cx| {
                this.toggle_process(&key, &row, &target, open, cx)
            });
        });
        let group = SharedString::from(format!("disclosure-{id}"));
        let tool = heading.level == Level::Tool;
        let arrow = div()
            .flex_none()
            .child(
                Icon::new(if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size_3(),
            )
            .when(tool, |arrow| {
                arrow
                    .opacity(0.)
                    .group_hover(group.clone(), |s| s.opacity(1.))
            });
        let marker_content = MarkerContent::new()
            .min_w_0()
            .whitespace_nowrap()
            .text_ellipsis()
            .map(|content| {
                if let Some(clock) = heading.clock {
                    content.child(clock)
                } else {
                    content.text(heading.title.clone())
                }
            });
        let marker = activity_marker(
            marker_content,
            heading.icon,
            heading.loading && heading.level != Level::Run,
            heading.failed && heading.level == Level::Tool,
            cx,
        )
        .with_variant(if heading.level == Level::Run {
            MarkerVariant::Border
        } else {
            MarkerVariant::Plain
        })
        .when(!heading.locked, |m| m.child(arrow));
        let trigger = marker_trigger(
            id,
            heading.title,
            marker,
            Some(open),
            (!heading.locked).then_some(action as MarkerAction),
            cx,
        );
        v_flex()
            .w_full()
            .min_w_0()
            .when(open, |s| s.gap_1())
            .child(trigger)
            .child(
                Collapsible::new()
                    .w_full()
                    .min_w_0()
                    .open(open)
                    .content(content),
            )
            .into_any_element()
    }

    pub(super) fn render_block(
        &self,
        key: &str,
        row: &str,
        block: ActivityBlock<'_>,
        current: bool,
        cx: &App,
    ) -> AnyElement {
        match block {
            ActivityBlock::Message(Activity::Text { id, text, .. }) => Message::new()
                .content(
                    MessageContent::new().child(
                        self.text_view(key, row, id.clone(), text.clone())
                            .stream_fade(),
                    ),
                )
                .into_any_element(),
            ActivityBlock::Message(_) => unreachable!("Only assistant prose separates groups"),
            ActivityBlock::Group { id, items } => {
                if let [item] = items {
                    return self.render_activity(key, row, item, cx);
                }
                let tools: Vec<_> = items
                    .iter()
                    .filter_map(|a| {
                        if let Activity::Tool(tool) = a {
                            Some(tool)
                        } else {
                            None
                        }
                    })
                    .collect();
                let running_tool = current
                    .then(|| {
                        tools
                            .iter()
                            .rev()
                            .find(|tool| tool.status == ToolStatus::Running)
                    })
                    .flatten();
                let thinking = items.iter().any(|item| {
                    matches!(
                        item,
                        Activity::Text {
                            thinking: true,
                            running: true,
                            ..
                        }
                    )
                });
                let title = if let Some(tool) = running_tool {
                    tool_title(tool, cx)
                } else if thinking {
                    t(cx, "conversation-thinking-running")
                } else if tools.is_empty() {
                    t(cx, "conversation-thinking-content")
                } else {
                    group_summary(&tools, cx)
                };
                let heading = Disclosure {
                    title,
                    clock: None,
                    icon: tools
                        .iter()
                        .any(|tool| tool.status == ToolStatus::Failed)
                        .then_some(IconName::CircleAlert),
                    level: Level::Group,
                    loading: running_tool.is_some() || thinking,
                    failed: tools.iter().any(|tool| tool.status == ToolStatus::Failed),
                    locked: false,
                };
                let content = v_flex()
                    .w_full()
                    .min_w_0()
                    .gap_1()
                    .children(
                        items
                            .iter()
                            .map(|item| self.render_activity(key, row, item, cx)),
                    )
                    .into_any_element();
                self.fold(
                    (key, row),
                    &id,
                    heading,
                    false,
                    ActivityViewport::new(format!("{key}-{id}"), content).into_any_element(),
                    cx,
                )
            }
        }
    }

    fn render_activity(&self, key: &str, row: &str, item: &Activity, cx: &App) -> AnyElement {
        match item {
            Activity::Tool(tool) => self.render_tool(key, tool, cx),
            Activity::Text {
                id, text, running, ..
            } => self.fold(
                (key, row),
                id,
                Disclosure {
                    clock: None,
                    title: t(
                        cx,
                        if *running {
                            "conversation-thinking-running"
                        } else {
                            "conversation-thinking-content"
                        },
                    ),
                    icon: Some(IconName::Brain),
                    level: Level::Tool,
                    loading: *running,
                    failed: false,
                    locked: text.is_empty(),
                },
                false,
                div()
                    .pl_6()
                    .child(
                        self.text_view(key, row, id.clone(), text.clone())
                            .stream_fade(),
                    )
                    .into_any_element(),
                cx,
            ),
        }
    }

    fn render_tool(&self, key: &str, tool: &Tool, cx: &App) -> AnyElement {
        self.tool_trigger(key, tool, cx).into_any_element()
    }
}

pub(super) fn tool_action(tool: &Tool, cx: &App) -> String {
    let mut args = FluentArgs::new();
    args.set(
        "state",
        match tool.status {
            ToolStatus::Running => "running",
            ToolStatus::Complete => "complete",
            ToolStatus::Failed => "failed",
            ToolStatus::Unfinished => "unfinished",
        },
    );
    args.set("name", tool.name.clone());
    t_with_args(cx, tool.kind().action_key(), &args)
}

pub(super) fn tool_title(tool: &Tool, cx: &App) -> String {
    let mut args = FluentArgs::new();
    args.set("action", tool_action(tool, cx));
    args.set(
        "summary",
        tool.summary().unwrap_or_default().replace('\n', " "),
    );
    if tool.kind() == ToolKind::Shell {
        args.set(
            "shell",
            if tool.name == "powershell" {
                "PowerShell"
            } else {
                "Bash"
            },
        );
        t_with_args(cx, "conversation-shell-line", &args)
            .trim()
            .to_owned()
    } else {
        t_with_args(cx, "conversation-tool-line", &args)
            .trim()
            .to_owned()
    }
}

fn group_summary(tools: &[&Tool], cx: &App) -> String {
    let categories = [
        (ToolKind::Skill, "conversation-tool-group-skill"),
        (ToolKind::Read, "conversation-tool-group-read"),
        (ToolKind::Search, "conversation-tool-group-search"),
        (ToolKind::Shell, "conversation-tool-group-bash"),
        (ToolKind::Edit, "conversation-tool-group-edit"),
        (ToolKind::Other, "conversation-tool-group"),
    ];
    let category = |tool: &&Tool| match tool.kind() {
        ToolKind::List => ToolKind::Search,
        ToolKind::Write => ToolKind::Edit,
        kind => kind,
    };
    let mut parts = vec![];
    for (kind, key) in categories {
        let count = tools.iter().filter(|tool| category(tool) == kind).count();
        if count == 0 {
            continue;
        }
        let mut args = FluentArgs::new();
        args.set("count", count);
        parts.push(t_with_args(cx, key, &args));
    }
    let failures: BTreeSet<_> = tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::Failed)
        .map(|tool| tool_action(tool, cx))
        .collect();
    parts.extend(failures);
    // Independent category labels, not fragments of a translated sentence.
    parts.join(" · ")
}
pub(super) fn code_fence(text: &str) -> String {
    // Tool output may contain Markdown fences itself.
    "`".repeat(
        text.split(|c| c != '`')
            .map(str::len)
            .max()
            .unwrap_or(0)
            .max(2)
            + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::{MarkerAction, activity_marker, group_summary, marker_trigger, tool_action};
    use crate::features::home::messages::activity::{Tool, ToolStatus};
    use gpui_kit::{
        AppContext, Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
        Styled, TestAppContext, Window,
        component::{Root, marker::MarkerContent},
        div, px, size,
    };
    use std::{cell::Cell, rc::Rc};

    struct MarkerFixture(Rc<Cell<usize>>);

    impl Render for MarkerFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let activations = self.0.clone();
            let action: MarkerAction = Rc::new(move |_, _| activations.set(activations.get() + 1));
            div().w(px(500.)).child(marker_trigger(
                "test-marker".into(),
                "Run command".into(),
                activity_marker(
                    MarkerContent::new().child(
                        div()
                            .id("marker-label")
                            .debug_selector(|| "marker-label".into())
                            .child("Run command"),
                    ),
                    None,
                    false,
                    false,
                    cx,
                ),
                None,
                Some(action),
                cx,
            ))
        }
    }

    #[gpui_kit::test]
    fn marker_is_left_aligned_and_activates_with_keyboard_and_pointer(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::foundation::i18n::apply(Default::default(), cx);
        });
        let activations = Rc::new(Cell::new(0));
        let (_, visual) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| MarkerFixture(activations.clone()));
            Root::new(view, window, cx)
        });
        visual.simulate_resize(size(px(600.), px(200.)));
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        assert!(visual.debug_bounds("marker-label").unwrap().left() < px(30.));
        visual.update(|window, cx| {
            window.focus_next(cx);
            window.draw(cx).clear(cx);
        });
        visual.simulate_keystrokes("enter");
        visual.simulate_keystrokes("space");
        assert_eq!(activations.get(), 2);
        let bounds = visual.debug_bounds("test-marker").unwrap();
        visual.simulate_click(bounds.center(), Modifiers::default());
        assert_eq!(activations.get(), 3);
    }

    #[gpui_kit::test]
    fn group_summary_identifies_failure_without_calling_successful_tools_failed(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| crate::foundation::i18n::apply(Default::default(), cx));
        let tool = |id: &str, status| Tool {
            id: id.into(),
            entries: vec![],
            name: "bash".into(),
            args: None,
            result: serde_json::Value::Null,
            status,
        };
        let complete = tool("complete", ToolStatus::Complete);
        let failed = tool("failed", ToolStatus::Failed);
        cx.update(|cx| {
            let summary = group_summary(&[&complete, &failed], cx);
            assert!(summary.contains(&tool_action(&failed, cx)));
            assert!(!summary.contains(&tool_action(&complete, cx)));
            assert!(!group_summary(&[&complete], cx).contains(&tool_action(&failed, cx)));
        });
    }
}
