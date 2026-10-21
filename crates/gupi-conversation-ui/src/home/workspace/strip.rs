use super::*;
use crate::chrome;
use gpui_kit::component::button::ButtonCustomVariant;
use gpui_kit::component::tab::{Tab, TabBar};
use std::cell::RefCell;

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
                    let timestamp = tab.timestamp.and_then(local_timestamp);
                    let title = tab
                        .title
                        .lines()
                        .next()
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            timestamp
                                .clone()
                                .unwrap_or_else(|| t(cx, "workspace-changes"))
                        });
                    let mut tooltip = tab.title.clone();
                    if let Some(timestamp) = timestamp {
                        tooltip.push_str(&format!("\n{timestamp}"));
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
        // Only domain identities escape the component's index-based presentation.
        let menu_target: Rc<RefCell<Option<String>>> = Rc::default();
        let mut tabs = TabBar::new("workspace-tabs")
            .pill()
            .menu(true)
            .small()
            .flex_1()
            .min_w_0()
            .h_full()
            .max_width(window.rem_size() * TAB_MAX_REM)
            .selected_index(
                order
                    .iter()
                    .position(|id| *id == view.workspace.active)
                    .unwrap_or(0),
            )
            .track_scroll(&view.workspace.scroll);
        for id in order {
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
            let activation = id.clone();
            let key_id = id.clone();
            let hover_id = id.clone();
            let drop_id = id.clone();
            let drag = TabDrag {
                session: self.shown_key.clone().unwrap_or_default(),
                id: id.clone(),
                title: title.clone(),
            };
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
            let group: SharedString = format!("workspace-tab-{stable}").into();
            let target = menu_target.clone();
            let context_id = stable.clone();
            let tab = Tab::new()
                .label(title.clone())
                .prefix(gpui_kit::component::Icon::new(icon).size_3p5())
                .aria_label(title.clone())
                .group(group.clone())
                .debug_selector(move || selector.clone())
                .track_focus(&focus)
                .tab_stop(selected)
                .on_mouse_down(MouseButton::Right, move |_, _, _| {
                    *target.borrow_mut() = Some(context_id.clone());
                })
                .when(
                    cx.has_active_drag() && view.workspace.dragging.as_ref() == Some(id),
                    |tab| tab.opacity(0.5),
                )
                .when(cx.has_active_drag(), |tab| {
                    tab.when_some(edge, |tab, after| {
                        if after {
                            tab.border_r_2().border_color(cx.theme().primary)
                        } else {
                            tab.border_l_2().border_color(cx.theme().primary)
                        }
                    })
                })
                .suffix(
                    // Retain the slot while hiding an inactive tab's close action.
                    div()
                        .flex_none()
                        .when(!selected, |close| {
                            close
                                .opacity(0.)
                                .group_hover(group, |style| style.opacity(1.))
                                .in_focus(|style| style.opacity(1.))
                        })
                        .child(
                            Button::new(format!("close-workspace-{stable}"))
                                .ghost()
                                .xsmall()
                                .icon(IconName::X)
                                .when(selected, |button| {
                                    button.custom(
                                        ButtonCustomVariant::new(cx)
                                            .foreground(cx.theme().primary_foreground)
                                            .hover(cx.theme().primary_hover)
                                            .active(cx.theme().primary_active),
                                    )
                                })
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
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx));
            tabs = tabs.child(tab);
        }
        let add = chrome::control(
            "workspace-add-control",
            chrome::button("workspace-add")
                .debug_selector(|| "workspace-add".into())
                .icon(IconName::Plus)
                .tooltip(t(cx, "workspace-new-tab"))
                .accessibility_label(t(cx, "workspace-new-tab"))
                .on_click(cx.listener(|this, _, window, cx| this.new_workspace_tab(window, cx))),
        );
        tabs = tabs
            .suffix(add)
            .last_empty_space(div().h_full().min_w(rems(DRAG_SPACE_REM)).flex_1());
        let reset_target = menu_target.clone();
        let guard_target = menu_target.clone();
        let owner = cx.weak_entity();
        let session = self.shown_key.clone();
        let tabs = div()
            .id("workspace-strip-context")
            .flex_1()
            .min_w_0()
            .h_full()
            .capture_any_mouse_down(move |event, _, _| {
                if event.button == MouseButton::Right {
                    reset_target.borrow_mut().take();
                }
            })
            .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                // A blank titlebar remains a native drag region, not a stale tab menu.
                if guard_target.borrow().is_none() {
                    cx.stop_propagation();
                }
            })
            .child(tabs)
            .context_menu(move |mut menu, _, cx| {
                let Some(id) = menu_target.borrow().clone() else {
                    return menu;
                };
                for (mode, label) in [
                    (0, "workspace-close-tab"),
                    (1, "workspace-close-others"),
                    (2, "workspace-close-right"),
                ] {
                    let owner = owner.clone();
                    let id = id.clone();
                    let session = session.clone();
                    menu = menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                        move |_, window, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                if this.shown_key == session {
                                    this.close_workspace_tabs(&id, mode, window, cx);
                                }
                            });
                        },
                    ));
                }
                menu
            });
        let mut row = h_flex()
            .h_full()
            .min_w_0()
            .items_center()
            .px_2()
            .gap_1()
            .child(tabs)
            .child(div().h_full().min_w(rems(DRAG_SPACE_REM)));
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

fn local_timestamp(milliseconds: i64) -> Option<String> {
    let date =
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
            .ok()?;
    let offset = time::UtcOffset::local_offset_at(date).unwrap_or(time::UtcOffset::UTC);
    let date = date.to_offset(offset);
    Some(format!(
        "{} {:02}:{:02}",
        date.date(),
        date.hour(),
        date.minute()
    ))
}
