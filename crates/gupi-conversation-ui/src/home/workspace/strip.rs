use super::*;
use crate::chrome;
use gpui_kit::component::menu::DropdownMenu;

mod interaction;
mod layer;
pub(super) use interaction::Interaction;

const TAB_MIN_REM: f32 = 10.;
const TAB_MAX_REM: f32 = 16.;
const DRAG_SPACE_REM: f32 = 2.;

#[derive(Clone)]
struct TabDrag {
    session: String,
}
/// The dragged tab moves inside the strip itself, so the pointer carries nothing.
impl Render for TabDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
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
            Some(GIT_TAB) => (t(cx, "diff-worktree"), t(cx, "diff-worktree")),
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
    fn drag_workspace_tab(&mut self, x: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let valid = self
            .shown_key
            .as_ref()
            .and_then(|key| self.views.get_mut(key))
            .is_some_and(|view| {
                let workspace = &mut view.workspace;
                workspace.strip.active()
                    && workspace
                        .strip
                        .advance(&mut workspace.order, &workspace.scroll, x)
            });
        if !valid {
            self.end_workspace_tab_drag(window, cx);
        }
        cx.notify();
    }
    pub(in crate::home) fn workspace_tab_drag_active(&self) -> bool {
        self.views
            .values()
            .any(|view| view.workspace.strip.active())
    }
    pub(in crate::home) fn end_workspace_tab_drag(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut ended = false;
        for view in self.views.values_mut() {
            ended |= view.workspace.strip.finish();
        }
        if ended {
            cx.stop_active_drag(window);
            cx.notify();
        }
    }
    pub(in crate::home) fn validate_workspace_tab_drag(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let invalid = self.views.iter().any(|(key, view)| {
            let workspace = &view.workspace;
            workspace.strip.active()
                && (self.shown_key.as_ref() != Some(key)
                    || !workspace.strip.valid(&workspace.order, &workspace.scroll))
        });
        if invalid {
            self.end_workspace_tab_drag(window, cx);
        }
        for view in self.views.values_mut() {
            let workspace = &mut view.workspace;
            if workspace.strip.motion.is_some()
                && !workspace.strip.valid(&workspace.order, &workspace.scroll)
            {
                workspace.strip.reset();
            }
        }
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
        let mut tabs = div()
            .id("workspace-tabs")
            .role(Role::TabList)
            .flex()
            .items_center()
            .min_w_0()
            .h_full()
            .gap_0p5()
            .overflow_x_scroll()
            .track_scroll(&view.workspace.scroll);
        let measured = view.workspace.order.clone();
        let scroll_geometry = view.workspace.scroll.clone();
        let session_geometry = self.shown_key.clone();
        let geometry_owner = cx.weak_entity();
        if let Some(motion) = &view.workspace.strip.motion {
            motion.moving.set(false);
        }
        for (ix, id) in order.iter().enumerate() {
            tabs = tabs.child(self.render_workspace_tab(ix, id, window, cx));
        }
        let tabs = div()
            .on_children_prepainted(move |_, window, cx| {
                let Some(geometry) = interaction::Geometry::read(&measured, &scroll_geometry)
                else {
                    return;
                };
                let _ = geometry_owner.update(cx, |home, cx| {
                    home.validate_workspace_tab_drag(window, cx);
                    if let Some(workspace) = session_geometry
                        .as_ref()
                        .and_then(|key| home.views.get_mut(key))
                        .map(|view| &mut view.workspace)
                        && workspace.order == measured
                    {
                        workspace.strip.measure(geometry);
                        if !workspace.strip.active()
                            && workspace
                                .strip
                                .motion
                                .as_ref()
                                .is_some_and(|motion| !motion.moving.get())
                        {
                            workspace.strip.motion = None;
                            cx.notify();
                        }
                    }
                });
            })
            .min_w_0()
            .h_full()
            .flex()
            .child(tabs);
        let scroll = view.workspace.scroll.clone();
        let overflowing = view.workspace.overflowing.clone();
        let owner = cx.weak_entity();
        let tabs = div()
            .flex_initial()
            .min_w_0()
            .h_full()
            // The all-tabs menu exists only while the strip clips tabs.
            .on_children_prepainted(move |_, _, cx| {
                let clipped = scroll.max_offset().x > px(0.);
                if overflowing.replace(clipped) != clipped {
                    let _ = owner.update(cx, |_, cx| cx.notify());
                }
            })
            .child(tabs);
        let add = chrome::control(
            "workspace-add-control",
            chrome::button("workspace-add")
                .debug_selector(|| "workspace-add".into())
                .icon(IconName::Plus)
                .tooltip(t(cx, "workspace-new-tab"))
                .accessibility_label(t(cx, "workspace-new-tab"))
                .on_click(cx.listener(|this, _, window, cx| this.new_workspace_tab(window, cx))),
        );
        let all = view.workspace.overflowing.get().then(|| {
            let entries: Vec<_> = order
                .iter()
                .map(|id| (id.clone(), self.tab_title(id, cx).0))
                .collect();
            let active = view.workspace.active.clone();
            let owner = cx.weak_entity();
            chrome::control(
                "workspace-all-control",
                chrome::button("workspace-all-tabs")
                    .icon(IconName::ChevronDown)
                    .accessibility_label(t(cx, "workspace-all-tabs"))
                    .tooltip(t(cx, "workspace-all-tabs"))
                    .dropdown_menu(move |mut menu, _, _| {
                        for (id, title) in &entries {
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
            )
        });
        let mut row = h_flex()
            .size_full()
            .min_w_0()
            .items_center()
            .px_2()
            .gap_1()
            .child(tabs)
            .child(add)
            .child(div().h_full().min_w(rems(DRAG_SPACE_REM)).flex_1())
            .children(all);
        if titlebar {
            let maximize = if self.pane_layout.maximized {
                "workspace-restore"
            } else {
                "workspace-maximize"
            };
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
                        .tooltip(t(cx, maximize))
                        .accessibility_label(t(cx, maximize))
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
    fn render_workspace_tab(
        &self,
        ix: usize,
        id: &Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = self
            .session_view()
            .expect("strip renders for a shown session");
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
        let drag_id = id.clone();
        let owner = cx.weak_entity();
        let drag_owner = cx.weak_entity();
        let session = self.shown_key.clone();
        let drag = TabDrag {
            session: session.clone().unwrap_or_default(),
        };
        let group: SharedString = format!("workspace-tab-{stable}").into();
        // Only the unshifted outer wrapper participates in strip layout/scroll.
        let workspace = &view.workspace;
        let motion = workspace
            .strip
            .motion
            .as_ref()
            .filter(|_| workspace.strip.valid(&workspace.order, &workspace.scroll));
        let frozen = motion.and_then(|motion| motion.geometry.slot(id));
        let held = workspace.strip.held_left(id);
        let offset = motion.zip(frozen).map(|(motion, slot)| {
            let spring = cx.theme().motion.spring_move;
            let key = (
                format!(
                    "workspace-tab-motion-{}-{}",
                    session.as_deref().unwrap_or_default(),
                    motion.epoch
                ),
                stable.clone(),
            );
            if motion.seeded.borrow_mut().insert(id.clone()) {
                gpui_kit::base::motion::spring(
                    key.clone(),
                    motion.starts.get(id).copied().unwrap_or(slot.left()),
                    spring.with_travel(false),
                    window,
                    cx,
                );
            }
            let target = held.unwrap_or(slot.left());
            let position = gpui_kit::base::motion::spring(
                key,
                target,
                if held.is_some() {
                    spring.with_travel(false)
                } else {
                    spring
                },
                window,
                cx,
            );
            // Springs can overshoot; a tab never leaves the visible strip while lifted.
            let position = if held.is_some() {
                position.clamp(
                    motion.geometry.viewport.left(),
                    motion.geometry.viewport.right() - slot.size.width,
                )
            } else {
                position
            };
            let offset = position - slot.left();
            if offset.abs() > px(0.1) {
                motion.moving.set(true);
            }
            offset
        });
        let lifted = held.is_some()
            || motion.is_some_and(|motion| {
                motion.lifted == *id && offset.is_some_and(|offset| offset.abs() > px(0.1))
            });
        let press_id = id.clone();
        let visual_id = id.clone();
        let visuals = workspace.strip.visuals.clone();
        let press_session = session.clone();
        let icon = match id.as_deref() {
            None => IconName::File,
            Some(id) if id.starts_with(NEW_TAB) => IconName::Plus,
            _ => IconName::FileDiff,
        };
        let selector = group.to_string();
        let theme = cx.theme();
        let (fg, muted, hover, ring) = (
            theme.foreground,
            theme.muted_foreground,
            theme.secondary,
            theme.ring,
        );
        let tab = div()
            .id(group.clone())
            .role(Role::Tab)
            .aria_label(title.clone())
            .aria_selected(selected)
            .aria_position_in_set(ix + 1)
            .aria_size_of_set(view.workspace.order.len())
            .debug_selector(move || selector.clone())
            .group(group.clone())
            .track_focus(&focus)
            .tab_stop(selected)
            // Keep a press on a tab from starting the titlebar's window drag.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |home, event: &MouseDownEvent, _, cx| {
                    if let Some(workspace) = press_session
                        .as_ref()
                        .and_then(|key| home.views.get_mut(key))
                        .map(|view| &mut view.workspace)
                    {
                        workspace.strip.arm(press_id.clone(), event.position.x);
                    }
                    cx.stop_propagation();
                }),
            )
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .min_w(rems(TAB_MIN_REM))
            .max_w(rems(TAB_MAX_REM))
            .when_some(frozen, |tab, bounds| {
                tab.w(bounds.size.width)
                    .min_w(bounds.size.width)
                    .max_w(bounds.size.width)
            })
            .h_7()
            .pl_2()
            .pr_0p5()
            .gap_1p5()
            .rounded(theme.radius)
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .text_sm()
            .map(|tab| {
                if selected {
                    tab.bg(hover).text_color(fg)
                } else {
                    tab.text_color(muted)
                        .hover(move |style| style.bg(hover.opacity(0.6)).text_color(fg))
                }
            })
            .focus_visible(move |style| style.border_color(ring))
            .when_some(offset, |tab, offset| tab.left(offset))
            .when(lifted, |tab| tab.bg(hover).text_color(fg).shadow_sm())
            .child(gpui_kit::component::Icon::new(icon).size_3p5().flex_none())
            .child(div().min_w_0().flex_1().truncate().child(title))
            .child(
                // Laid out even when hidden so the title never shifts on hover.
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
                let Some(view) = this.shown_key.as_ref().and_then(|key| this.views.get(key)) else {
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
            .on_drag(drag, move |drag, _, window, cx| {
                let x = window.mouse_position().x;
                let _ = drag_owner.update(cx, |this, cx| {
                    let valid = this.shown_key.as_ref() == Some(&drag.session)
                        && this.views.get_mut(&drag.session).is_some_and(|view| {
                            let workspace = &mut view.workspace;
                            workspace.strip.begin(
                                &drag_id,
                                &mut workspace.order,
                                &workspace.scroll,
                                x,
                            )
                        });
                    if !valid {
                        // on_drag installs the framework payload after this callback returns.
                        cx.defer_in(window, |home, window, cx| {
                            home.end_workspace_tab_drag(window, cx)
                        });
                    }
                    cx.notify();
                });
                cx.new(|_| drag.clone())
            })
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .context_menu(move |mut menu, _, cx| {
                for (mode, label) in [
                    (0, "workspace-close-tab"),
                    (1, "workspace-close-others"),
                    (2, "workspace-close-right"),
                ] {
                    let owner = owner.clone();
                    let id = menu_id.clone();
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
        let tab = if lifted {
            layer::lifted(tab, motion.unwrap().geometry.viewport).into_any_element()
        } else {
            tab.into_any_element()
        };
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    visuals.borrow_mut().insert(visual_id.clone(), *bounds);
                }
            })
            .id(format!("workspace-slot-{stable}"))
            .flex_none()
            .h_7()
            .when_some(frozen, |slot, bounds| slot.w(bounds.size.width))
            .child(tab)
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

/// Window capture owns this strip gesture; child tab/close handlers cannot swallow release.
pub(in crate::home) fn events(owner: WeakEntity<HomeView>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            let moves = owner.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.capture() {
                    return;
                }
                let _ = moves.update(cx, |home, cx| {
                    if home.workspace_tab_drag_active() {
                        if event.pressed_button == Some(MouseButton::Left) {
                            home.drag_workspace_tab(event.position.x, window, cx);
                        } else {
                            home.end_workspace_tab_drag(window, cx);
                        }
                    }
                });
            });
            let releases = owner.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if !phase.capture() || event.button != MouseButton::Left {
                    return;
                }
                let _ = releases.update(cx, |home, cx| {
                    let owned = home.workspace_tab_drag_active();
                    if owned {
                        home.drag_workspace_tab(event.position.x, window, cx);
                    }
                    home.end_workspace_tab_drag(window, cx);
                    if owned {
                        cx.stop_propagation();
                    }
                });
            });
        },
    )
    .absolute()
    .size_0()
}
