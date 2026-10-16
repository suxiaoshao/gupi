//! Compact ordered-list and map boxes; all new input is explicit and temporary.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Collection {
    Strings,
    Secrets,
    Exposure,
    Models,
    Overrides,
}
pub(super) struct Addition {
    pub(super) focus: FocusHandle,
    input: Entity<InputState>,
    open: bool,
    epoch: u64,
    _subscription: Subscription,
}
pub(super) fn path_id(path: &[String]) -> String {
    serde_json::to_string(path).unwrap_or_default()
}
fn text(id: impl Into<ElementId>, value: String, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .min_w_0()
        .text_sm()
        .font_family(cx.theme().mono_font_family.clone())
        .truncate()
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(value.clone()).build(window, cx)
        })
}
fn literal(value: Option<Value>) -> String {
    match value {
        Some(Value::String(s)) if !s.is_empty() && s.trim() == s => s,
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
impl EntryEditor {
    fn add_input(
        &mut self,
        path: &[String],
        kind: Collection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(add) = self.additions.get(path) {
            return add.input.clone();
        }
        let placeholder = t(
            cx,
            if kind == Collection::Strings {
                "pi-files-add-value"
            } else if kind == Collection::Models {
                "pi-files-label-id"
            } else {
                "pi-files-key"
            },
        );
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let p = path.to_vec();
        let sub = cx.subscribe_in(&input, window, move |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                let p = p.clone();
                let epoch = this.additions.get(&p).map(|a| a.epoch);
                cx.defer_in(window, move |this, window, cx| {
                    if this
                        .additions
                        .get(&p)
                        .is_some_and(|a| a.open && Some(a.epoch) == epoch)
                    {
                        this.finish_add(&p, kind, true, window, cx);
                    }
                });
            }
        });
        self.additions.insert(
            path.to_vec(),
            Addition {
                focus: cx.focus_handle(),
                input: input.clone(),
                open: false,
                epoch: 0,
                _subscription: sub,
            },
        );
        input
    }
    fn open_add(
        &mut self,
        path: &[String],
        kind: Collection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.add_input(path, kind, window, cx);
        let add = self.additions.get_mut(path).unwrap();
        add.open = true;
        add.epoch += 1;
        input.update(cx, |input, cx| input.set_value("", window, cx));
        window.focus(&input.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn finish_add(
        &mut self,
        path: &[String],
        kind: Collection,
        append: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(add) = self.additions.get(path).filter(|a| a.open) else {
            return;
        };
        let input = add.input.clone();
        let value = input.read(cx).value().to_string();
        let mut model = None;
        if append {
            if kind != Collection::Strings && value.is_empty() {
                self.show_error(Error::Invalid, cx);
                return;
            }
            let mut p = path.to_vec();
            p.push(value.clone());
            if !matches!(kind, Collection::Strings | Collection::Models)
                && (self.entry.present(&p)
                    || (kind == Collection::Secrets && self.original.present(&p)))
            {
                self.show_error(Error::Duplicate, cx);
                return;
            }
            let result = match kind {
                Collection::Strings => self.entry.append(path, json!(value)),
                Collection::Secrets => {
                    self.secrets.insert(p.clone(), 1);
                    self.entry
                        .credential(&p, Credential::Replace(String::new()))
                }
                Collection::Exposure => self.entry.set(&p, Some(json!("codemode"))),
                Collection::Models => {
                    let ix = self.entry.len(path).unwrap_or_default();
                    model = Some((vec!["models".into(), ix.to_string()], false));
                    let _ = self.original.append(path, json!({}));
                    self.entry.append(path, json!({"id":value}))
                }
                Collection::Overrides => {
                    model = Some((p.clone(), true));
                    let _ = self.original.set(&p, Some(json!({})));
                    self.entry.set(&p, Some(json!({})))
                }
            };
            if let Err(e) = result {
                self.show_error(e, cx);
                return;
            }
        }
        let add = self.additions.get_mut(path).unwrap();
        window.focus(&add.focus, cx);
        add.open = false;
        add.epoch += 1;
        input.update(cx, |input, cx| input.set_value("", window, cx));
        if let Some((path, overrides)) = model {
            self.open_model(path, overrides, window, cx);
        }
        cx.notify();
    }
    fn add_line(
        &mut self,
        path: &[String],
        kind: Collection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input = self.add_input(path, kind, window, cx);
        let open = self.additions[path].open;
        let add_focus = self.additions[path].focus.clone();
        let p = path.to_vec();
        let escape = p.clone();
        let dismiss = p.clone();
        let focus = input.clone();
        let cancel = p.clone();
        h_flex()
            .id(SharedString::from(format!("add-line-{}", path_id(path))))
            .w_full()
            .min_w_0()
            .gap_1()
            .items_center()
            .on_action(cx.listener(
                move |this, _: &gpui_kit::component::dialog::Cancel, w, cx| {
                    if this.additions.get(&dismiss).is_some_and(|a| a.open) {
                        this.finish_add(&dismiss, kind, false, w, cx);
                    } else {
                        cx.propagate();
                    }
                },
            ))
            .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, w, cx| {
                if event.keystroke.key == "escape" && focus.read(cx).focus_handle(cx).is_focused(w)
                {
                    this.finish_add(&escape, kind, false, w, cx);
                    cx.stop_propagation();
                }
            }))
            .when(open, |line| {
                line.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&input).small().aria_label(t(
                            cx,
                            if kind == Collection::Strings {
                                "pi-files-add-value"
                            } else {
                                "pi-files-key"
                            },
                        ))),
                )
            })
            .child(
                gpui_kit::base::Button::new(SharedString::from(format!("add-{}", path_id(path))))
                    .track_focus(&add_focus)
                    .accessibility_label(t(
                        cx,
                        if open {
                            "pi-files-confirm-add"
                        } else {
                            "pi-resource-add"
                        },
                    ))
                    .h_6()
                    .px_2()
                    .rounded_md()
                    .text_sm()
                    .border_1()
                    .border_color(if add_focus.is_focused(window) {
                        cx.theme().ring
                    } else {
                        gpui_kit::transparent_black()
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().accent))
                    .active(|style| style.bg(cx.theme().accent))
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                gpui_kit::component::Icon::new(if open {
                                    IconName::Check
                                } else {
                                    IconName::Plus
                                })
                                .small(),
                            )
                            .child(t(
                                cx,
                                if open {
                                    "pi-files-confirm-add"
                                } else {
                                    "pi-resource-add"
                                },
                            )),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| {
                        if this.additions.get(&p).is_some_and(|a| a.open) {
                            this.finish_add(&p, kind, true, w, cx)
                        } else {
                            this.open_add(&p, kind, w, cx)
                        }
                    })),
            )
            .when(open, |line| {
                line.child(
                    Button::new("cancel-add")
                        .ghost()
                        .xsmall()
                        .icon(IconName::X)
                        .accessibility_label(t(cx, "action-cancel"))
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.finish_add(&cancel, kind, false, w, cx)
                        })),
                )
            })
            .into_any_element()
    }
    fn collection_menu(&self, path: &[String], array: bool, cx: &Context<Self>) -> AnyElement {
        let e = cx.entity();
        let p = path.to_vec();
        Button::new(SharedString::from(format!("collection-{}", path_id(path))))
            .w_6()
            .flex_shrink_0()
            .ghost()
            .xsmall()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-field-actions"))
            .dropdown_menu(move |menu, _, cx| {
                [None, Some(if array { json!([]) } else { json!({}) })]
                    .into_iter()
                    .fold(menu, |menu, value| {
                        let (e, p) = (e.clone(), p.clone());
                        let label = if value.is_none() {
                            t(cx, "pi-value-unset")
                        } else if array {
                            "[]".into()
                        } else {
                            "{}".into()
                        };
                        menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                            e.update(cx, |this, cx| {
                                this.clear_controls(&p);
                                let _ = this.original.set(&p, value.clone());
                                this.write(&p, value.clone(), cx)
                            })
                        }))
                    })
            })
            .into_any_element()
    }
    fn item_menu(
        &self,
        path: &[String],
        ix: usize,
        count: usize,
        array: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = path.to_vec();
        let e = cx.entity();
        Button::new(SharedString::from(format!(
            "item-menu-{}-{ix}",
            path_id(path)
        )))
        .ghost()
        .xsmall()
        .icon(IconName::Ellipsis)
        .accessibility_label(t(cx, "pi-resource-entry-actions"))
        .dropdown_menu(move |menu, _, cx| {
            let mut menu = menu;
            for (valid, to, label) in [
                (ix > 0, ix.saturating_sub(1), "pi-resource-move-up"),
                (ix + 1 < count, ix + 1, "pi-resource-move-down"),
            ] {
                if valid {
                    let (e, p) = (e.clone(), p.clone());
                    menu = menu.item(PopupMenuItem::new(t(cx, label)).on_click(move |_, _, cx| {
                        e.update(cx, |this, cx| {
                            if array {
                                this.change_array(&p, ix, Some(to));
                            } else {
                                let _ = this.entry.move_item(&p, ix, to);
                                let _ = this.original.move_item(&p, ix, to);
                            }
                            cx.notify()
                        })
                    }));
                }
            }
            let (e, p) = (e.clone(), p.clone());
            menu.item(PopupMenuItem::new(t(cx, "pi-files-delete")).on_click(
                move |_, window, cx| {
                    e.update(cx, |this, cx| {
                        if array {
                            this.change_array(&p, ix, None);
                        } else if let Ok(keys) = this.entry.keys(&p)
                            && let Some(key) = keys.get(ix)
                        {
                            let mut item = p.clone();
                            item.push(key.clone());
                            this.clear_controls(&item);
                            let _ = this.entry.set(&item, None);
                            let _ = this.original.set(&item, None);
                        }
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })
                },
            ))
        })
        .into_any_element()
    }
    pub(super) fn model_title(&self, path: &[String], overrides: bool) -> String {
        if overrides {
            path.last().cloned().unwrap_or_default()
        } else {
            let mut id = path.to_vec();
            id.push("id".into());
            self.entry
                .visible(&id)
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        }
    }
    fn model_row(
        &self,
        path: Vec<String>,
        overrides: bool,
        ix: usize,
        count: usize,
        cx: &Context<Self>,
    ) -> AnyElement {
        let title = self.model_title(&path, overrides);
        let mut name = path.clone();
        name.push("name".into());
        let name = self
            .entry
            .visible(&name)
            .and_then(|v| v.as_str().map(str::to_owned));
        let p = path.clone();
        let parent = path[..path.len() - 1].to_vec();
        h_flex()
            .gap_1()
            .items_center()
            .w_full()
            .min_w_0()
            .child(
                Button::new(SharedString::from(format!("model-row-{}", path_id(&path))))
                    .ghost()
                    .small()
                    .flex_1()
                    .min_w_0()
                    .accessibility_label(title.clone())
                    .child(
                        h_flex()
                            .w_full()
                            .gap_1()
                            .child(text("id", title.clone(), cx).flex_1().child(title))
                            .when_some(name, |line, name| {
                                line.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .truncate()
                                        .child(name),
                                )
                            }),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| {
                        this.open_model(p.clone(), overrides, w, cx)
                    })),
            )
            .child(self.item_menu(&parent, ix, count, !overrides, cx))
            .into_any_element()
    }
    pub(super) fn collection(
        &mut self,
        path: Vec<String>,
        kind: Collection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let array = matches!(kind, Collection::Strings | Collection::Models);
        let mut content = v_flex()
            .id(SharedString::from(format!("box-{}", path_id(&path))))
            .w_full()
            .min_w_0()
            .p_1()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().group_box);
        let mut keys = if array {
            self.entry
                .len(&path)
                .map(|len| (0..len).map(|ix| ix.to_string()).collect::<Vec<_>>())
        } else {
            self.entry.keys(&path)
        };
        if kind == Collection::Secrets
            && let Ok(keys) = &mut keys
        {
            let mut all = self.original.keys(&path).unwrap_or_default();
            for key in keys.iter() {
                if !all.contains(key) {
                    all.push(key.clone());
                }
            }
            *keys = all;
        }
        match keys {
            Err(_) => {
                content = content.child(
                    div()
                        .px_1()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-resource-unsupported")),
                );
            }
            Ok(keys) => {
                if keys.is_empty() {
                    content = content.child(
                        div()
                            .px_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(if self.entry.present(&path) {
                                if array { "[]".into() } else { "{}".into() }
                            } else {
                                t(cx, "pi-resource-absent")
                            }),
                    );
                }
                let count = keys.len();
                for (ix, key) in keys.into_iter().enumerate() {
                    let mut p = path.clone();
                    p.push(key.clone());
                    let child = match kind {
                        Collection::Models | Collection::Overrides => {
                            self.model_row(p, kind == Collection::Overrides, ix, count, cx)
                        }
                        Collection::Strings => {
                            let label = literal(self.entry.visible(&p));
                            h_flex()
                                .gap_1()
                                .items_center()
                                .min_w_0()
                                .child(
                                    text(
                                        SharedString::from(format!("item-{ix}")),
                                        label.clone(),
                                        cx,
                                    )
                                    .flex_1()
                                    .child(label),
                                )
                                .child(self.item_menu(&path, ix, count, true, cx))
                                .into_any_element()
                        }
                        Collection::Secrets => {
                            let mode = self.secrets.get(&p).copied().unwrap_or_default();
                            let mut line = h_flex().gap_1().items_center().min_w_0().child(
                                text(SharedString::from(format!("key-{key}")), key.clone(), cx)
                                    .flex_1()
                                    .child(key),
                            );
                            if mode == 1 {
                                line = line.child(div().flex_1().min_w_0().child(self.input(
                                    &p,
                                    Field::Secret,
                                    window,
                                    cx,
                                )));
                            } else {
                                line = line.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(self.secret_status(&p, cx)),
                                );
                            }
                            line.child(self.field_menu(&p, true, cx)).into_any_element()
                        }
                        Collection::Exposure => h_flex()
                            .gap_1()
                            .items_center()
                            .min_w_0()
                            .child(
                                text(SharedString::from(format!("key-{key}")), key.clone(), cx)
                                    .flex_1()
                                    .child(key),
                            )
                            .child(div().w_24().flex_shrink_0().child(self.choice(
                                &p,
                                Field::Choice(EXPOSURE),
                                cx,
                            )))
                            .child(self.item_menu(&path, ix, count, false, cx))
                            .into_any_element(),
                    };
                    content = content.child(child);
                }
                content = content.child(self.add_line(&path, kind, window, cx));
            }
        }
        super::layout::collection_row(
            super::layout::field_label(path.last().unwrap(), cx),
            content.into_any_element(),
            self.collection_menu(&path, array, cx),
        )
    }
}
