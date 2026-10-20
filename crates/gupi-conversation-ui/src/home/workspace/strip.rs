use super::*;
use crate::chrome;
use gpui_kit::component::Icon;

const TAB_MIN_REM: f32 = 8.;
const TAB_MAX_REM: f32 = 16.;
const DRAG_SPACE_REM: f32 = 2.;

#[derive(Clone)]
struct TabDrag {
    session: String,
    id: Option<String>,
    title: String,
}
impl Render for TabDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .max_w(rems(TAB_MAX_REM))
            .rounded(cx.theme().radius)
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .truncate()
            .child(self.title.clone())
    }
}
impl HomeView {
    pub(in crate::home) fn toggle_workspace_maximized(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pane_layout.workspace_docked {
            self.pane_layout.maximized = !self.pane_layout.maximized;
            if self.pane_layout.maximized {
                self.pane_layout.overlay = None;
            }
            self.pane_layout.invalidate();
            self.fit_layout(window, cx);
            cx.notify();
        }
    }
    fn tab_title(&self, id: &Option<String>, cx: &App) -> (String, String) {
        match id.as_deref() {
            None => (t(cx, "files-title"), t(cx, "files-title")),
            Some(id) if id.starts_with(NEW_TAB) => {
                (t(cx, "workspace-new-tab"), t(cx, "workspace-new-tab"))
            }
            Some(id) => self
                .session_view()
                .and_then(|v| v.workspace.changes.iter().find(|tab| tab.id == id))
                .map(|tab| {
                    let title = tab
                        .title
                        .lines()
                        .next()
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            tab.timestamp
                                .and_then(|ms| {
                                    time::OffsetDateTime::from_unix_timestamp_nanos(
                                        i128::from(ms) * 1_000_000,
                                    )
                                    .ok()
                                })
                                .map(|date| {
                                    format!(
                                        "{} {:02}:{:02}",
                                        date.date(),
                                        date.hour(),
                                        date.minute()
                                    )
                                })
                                .unwrap_or_else(|| t(cx, "workspace-changes"))
                        });
                    let mut tooltip = tab.title.clone();
                    if let Some(date) = tab.timestamp.and_then(|ms| {
                        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
                            .ok()
                    }) {
                        tooltip.push_str(&format!(
                            "\n{} {:02}:{:02}",
                            date.date(),
                            date.hour(),
                            date.minute()
                        ));
                    }
                    let totals = tab.files.iter().filter_map(|file| file.stats()).fold(
                        None,
                        |sum, stats| {
                            let (a, b) = sum.unwrap_or((0, 0));
                            Some((a + stats.added(), b + stats.removed()))
                        },
                    );
                    if let Some((added, removed)) = totals {
                        tooltip.push_str(&format!("\n+{added} −{removed}"));
                    }
                    (title, tooltip)
                })
                .unwrap_or_else(|| (t(cx, "workspace-changes"), t(cx, "workspace-changes"))),
        }
    }
    fn reorder_workspace_tab(
        &mut self,
        drag: &TabDrag,
        target: Option<String>,
        after: bool,
        cx: &mut Context<Self>,
    ) {
        if self.shown_key.as_ref() != Some(&drag.session) || drag.id == target {
            return;
        }
        let Some(view) = self.views.get_mut(&drag.session) else {
            return;
        };
        let order = &mut view.workspace.order;
        let Some(from) = order.iter().position(|id| *id == drag.id) else {
            return;
        };
        let id = order.remove(from);
        let Some(to) = order.iter().position(|id| *id == target) else {
            order.insert(from, id);
            return;
        };
        order.insert(to + usize::from(after), id);
        view.workspace.drop_target = None;
        view.workspace.reveal();
        cx.notify();
    }
    pub(in crate::home) fn render_workspace_strip(
        &self,
        titlebar: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(view) = self.session_view() else {
            return div().into_any_element();
        };
        let order = &view.workspace.order;
        let mut tabs = gpui_kit::base::Tabs::new("workspace-tabs")
            .flex()
            .items_center()
            .min_w_0()
            .h_full()
            .gap_1()
            .overflow_x_scroll()
            .track_scroll(&view.workspace.scroll);
        for (ix, id) in order.iter().enumerate() {
            let selected = *id == view.workspace.active;
            let (title, tooltip) = self.tab_title(id, cx);
            let stable = id.clone().unwrap_or_else(|| "files".into());
            let focus = view
                .workspace
                .headers
                .get(id)
                .unwrap_or(&self.workspace_region)
                .clone()
                .tab_stop(selected);
            let close_id = stable.clone();
            let middle_id = stable.clone();
            let menu_id = stable.clone();
            let activation = id.clone();
            let key_id = id.clone();
            let hover_id = id.clone();
            let drop_id = id.clone();
            let owner = cx.weak_entity();
            let drag = TabDrag {
                session: self.shown_key.clone().unwrap_or_default(),
                id: id.clone(),
                title: title.clone(),
            };
            let group: SharedString = format!("workspace-tab-{stable}").into();
            let edge = view
                .workspace
                .drop_target
                .as_ref()
                .filter(|(target, _)| target == id)
                .map(|(_, after)| *after);
            let icon = match id.as_deref() {
                None => IconName::File,
                Some(id) if id.starts_with(NEW_TAB) => IconName::Plus,
                _ => IconName::FileDiff,
            };
            let drag_owner = cx.weak_entity();
            let selector = format!("workspace-tab-{stable}");
            let tab = gpui_kit::base::Tab::new(format!("workspace-tab-{stable}"))
                .debug_selector(move || selector.clone())
                .when(
                    cx.has_active_drag() && view.workspace.dragging.as_ref() == Some(id),
                    |tab| tab.opacity(0.5),
                )
                .group(group.clone())
                .track_focus(&focus)
                .tab_stop(selected)
                .selected(selected)
                .set_position(ix + 1, order.len())
                .accessibility_label(title.clone())
                .relative()
                .flex_none()
                .min_w(rems(TAB_MIN_REM))
                .max_w(rems(TAB_MAX_REM))
                .h_7()
                .pl_2()
                .pr_1()
                .gap_1p5()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(gpui_kit::transparent_black())
                .when(selected, |tab| {
                    tab.bg(cx.theme().secondary)
                        .border_color(cx.theme().border)
                        .text_color(cx.theme().foreground)
                })
                .when(!selected, |tab| {
                    tab.text_color(cx.theme().muted_foreground).hover(|s| {
                        s.bg(cx.theme().secondary.opacity(0.5))
                            .text_color(cx.theme().foreground)
                    })
                })
                .focus_visible(|s| s.border_color(cx.theme().ring))
                .when(cx.has_active_drag(), |tab| {
                    tab.when_some(edge, |tab, after| {
                        if after {
                            tab.border_r_2().border_color(cx.theme().primary)
                        } else {
                            tab.border_l_2().border_color(cx.theme().primary)
                        }
                    })
                })
                .child(Icon::new(icon).size_3p5().flex_none())
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_sm()
                        .child(title.clone()),
                )
                .child(
                    // Always laid out so the title never shifts when the close
                    // button appears on hover.
                    div()
                        .flex_none()
                        .when(!selected, |close| {
                            close
                                .opacity(0.)
                                .group_hover(group, |s| s.opacity(1.))
                                .in_focus(|s| s.opacity(1.))
                        })
                        .child(
                            Button::new(format!("close-workspace-{stable}"))
                                .ghost()
                                .xsmall()
                                .icon(IconName::X)
                                .accessibility_label(t(cx, "workspace-close-tab"))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.close_workspace_tabs(&close_id, 0, window, cx);
                                })),
                        ),
                )
                .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.select_workspace_tab(activation.clone(), window, cx);
                }))
                .on_mouse_up(
                    MouseButton::Middle,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_workspace_tabs(&middle_id, 0, window, cx);
                    }),
                )
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let Some(view) = this.shown_key.as_ref().and_then(|key| this.views.get(key))
                    else {
                        return;
                    };
                    let ix = view
                        .workspace
                        .order
                        .iter()
                        .position(|id| *id == key_id)
                        .unwrap_or(0);
                    let last = view.workspace.order.len().saturating_sub(1);
                    let next = match event.keystroke.key.as_str() {
                        "left" => Some(ix.saturating_sub(1)),
                        "right" => Some((ix + 1).min(last)),
                        "home" => Some(0),
                        "end" => Some(last),
                        "enter" | "space" => Some(ix),
                        _ => None,
                    };
                    if let Some(next) = next {
                        let id = view.workspace.order[next].clone();
                        this.select_workspace_tab(id.clone(), window, cx);
                        if let Some(focus) = this
                            .session_view()
                            .and_then(|v| v.workspace.headers.get(&id))
                        {
                            focus.focus(window, cx);
                        }
                        cx.stop_propagation();
                    }
                }))
                .on_drag(drag, move |drag, _, _, cx| {
                    let _ = drag_owner.update(cx, |this, cx| {
                        if let Some(view) = this.views.get_mut(&drag.session) {
                            view.workspace.dragging = Some(drag.id.clone());
                            cx.notify();
                        }
                    });
                    cx.new(|_| drag.clone())
                })
                .on_drag_move::<TabDrag>(cx.listener(
                    move |this, event: &DragMoveEvent<TabDrag>, _, cx| {
                        if event.bounds.contains(&event.event.position) {
                            let after = event.event.position.x > event.bounds.center().x;
                            if let Some(view) = this
                                .shown_key
                                .as_ref()
                                .and_then(|key| this.views.get_mut(key))
                            {
                                let next = Some((hover_id.clone(), after));
                                if view.workspace.drop_target != next {
                                    view.workspace.drop_target = next;
                                    cx.notify();
                                }
                            }
                        }
                    },
                ))
                .on_drop(cx.listener(move |this, drag: &TabDrag, _, cx| {
                    let after = this
                        .session_view()
                        .and_then(|v| v.workspace.drop_target.as_ref())
                        .is_some_and(|(id, after)| *id == drop_id && *after);
                    this.reorder_workspace_tab(drag, drop_id.clone(), after, cx);
                }))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .context_menu(move |mut menu, _, cx| {
                    for (mode, label) in [
                        (0, "workspace-close-tab"),
                        (1, "workspace-close-others"),
                        (2, "workspace-close-right"),
                    ] {
                        let owner = owner.clone();
                        let id = menu_id.clone();
                        menu = menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                            move |_, window, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.close_workspace_tabs(&id, mode, window, cx)
                                });
                            },
                        ));
                    }
                    menu
                });
            tabs = tabs.child(tab);
        }
        let all: Vec<_> = order
            .iter()
            .map(|id| (id.clone(), self.tab_title(id, cx).0))
            .collect();
        let active = view.workspace.active.clone();
        let owner = cx.weak_entity();
        let controls = h_flex()
            .gap_1()
            .flex_none()
            .child(chrome::control(
                "workspace-add-control",
                chrome::button("workspace-add")
                    .debug_selector(|| "workspace-add".into())
                    .icon(IconName::Plus)
                    .tooltip(t(cx, "workspace-new-tab"))
                    .accessibility_label(t(cx, "workspace-new-tab"))
                    .on_click(
                        cx.listener(|this, _, window, cx| this.new_workspace_tab(window, cx)),
                    ),
            ))
            .child(chrome::control(
                "workspace-all-control",
                chrome::button("workspace-all-tabs")
                    .icon(IconName::ChevronDown)
                    .accessibility_label(t(cx, "workspace-all-tabs"))
                    .tooltip(t(cx, "workspace-all-tabs"))
                    .dropdown_menu(move |mut menu, _, _cx| {
                        for (id, title) in &all {
                            let owner = owner.clone();
                            let id = id.clone();
                            menu = menu.item(
                                PopupMenuItem::new(title.clone())
                                    .checked(id == active)
                                    .on_click(move |_, window, cx| {
                                        let _ = owner.update(cx, |this, cx| {
                                            this.select_workspace_tab(id.clone(), window, cx)
                                        });
                                    }),
                            );
                        }
                        menu
                    }),
            ));
        let mut row = h_flex()
            .h_full()
            .min_w_0()
            .items_center()
            .px_2()
            .gap_1()
            .child(div().flex_initial().min_w_0().h_full().child(tabs))
            .child(controls)
            .child(div().h_full().min_w(rems(DRAG_SPACE_REM)).flex_1());
        if titlebar {
            row = row
                .child(chrome::control(
                    "workspace-maximize-control",
                    chrome::button("workspace-maximize")
                        .debug_selector(|| "workspace-maximize".into())
                        .icon(if self.pane_layout.maximized {
                            IconName::Minimize2
                        } else {
                            IconName::Maximize2
                        })
                        .tooltip(t(
                            cx,
                            if self.pane_layout.maximized {
                                "workspace-restore"
                            } else {
                                "workspace-maximize"
                            },
                        ))
                        .accessibility_label(t(
                            cx,
                            if self.pane_layout.maximized {
                                "workspace-restore"
                            } else {
                                "workspace-maximize"
                            },
                        ))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_workspace_maximized(window, cx)
                        })),
                ))
                .child(self.render_workspace_toggle(cx));
        }
        let _ = window;
        div()
            .h(chrome::HEIGHT)
            .flex_none()
            .min_w_0()
            .when(!titlebar, |strip| {
                strip.border_b_1().border_color(cx.theme().border)
            })
            .child(row)
            .into_any_element()
    }
}
