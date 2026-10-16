//! Form sections and the one-model-at-a-time sub-view.
use super::*;

pub(in super::super) fn field_label(key: &str, cx: &App) -> AnyElement {
    h_flex()
        .gap_1()
        .flex_wrap()
        .min_w_0()
        .child(t(cx, &format!("pi-files-label-{key}")))
        .child(
            div()
                .text_xs()
                .font_family(cx.theme().mono_font_family.clone())
                .text_color(cx.theme().muted_foreground)
                .child(format!("· {key}")),
        )
        .into_any_element()
}
pub(super) fn collection_row(
    label: AnyElement,
    control: AnyElement,
    menu: AnyElement,
) -> AnyElement {
    h_flex()
        .w_full()
        .items_start()
        .gap_2()
        .child(div().flex_1().min_w_0().pt_1().text_sm().child(label))
        .child(div().w_80().flex_shrink_0().min_w_0().child(control))
        .child(menu)
        .into_any_element()
}
fn section(key: &str, items: Vec<AnyElement>, cx: &App) -> AnyElement {
    v_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(t(cx, key)),
        )
        .children(items)
        .into_any_element()
}
impl EntryEditor {
    pub(super) fn secret_status(&self, path: &[String], cx: &App) -> String {
        t(
            cx,
            match self.secrets.get(path).copied().unwrap_or(0) {
                1 => "pi-files-secret-replace",
                2 => "pi-files-secret-remove",
                _ if self.original.present(path) => "pi-files-secret-present",
                _ => "pi-value-unset",
            },
        )
    }
    pub(super) fn secret_action(&mut self, path: &[String], mode: u8, cx: &mut Context<Self>) {
        self.clear_controls(path);
        self.secrets.insert(path.to_vec(), mode);
        let result = match mode {
            0 => self.entry.restore_credential(&self.original, path),
            1 => self
                .entry
                .credential(path, Credential::Replace(String::new())),
            _ => self.entry.credential(path, Credential::Remove),
        };
        if let Err(e) = result {
            self.error = Some(e)
        }
        cx.notify();
    }
    pub(super) fn field_menu(
        &self,
        path: &[String],
        secret: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = path.to_vec();
        let e = cx.entity();
        Button::new(SharedString::from(format!("field-{}", path_id(path))))
            .w_6()
            .flex_shrink_0()
            .ghost()
            .xsmall()
            .icon(IconName::Ellipsis)
            .accessibility_label(t(cx, "pi-field-actions"))
            .dropdown_menu(move |menu, _, cx| {
                if secret {
                    return [
                        (0, "pi-files-secret-keep"),
                        (1, "pi-files-secret-replace"),
                        (2, "pi-files-secret-remove"),
                    ]
                    .into_iter()
                    .fold(menu, |menu, (mode, label)| {
                        let (e, p) = (e.clone(), p.clone());
                        menu.item(PopupMenuItem::new(t(cx, label)).on_click(move |_, _, cx| {
                            e.update(cx, |this, cx| this.secret_action(&p, mode, cx))
                        }))
                    });
                }
                let (e, p) = (e.clone(), p.clone());
                menu.item(
                    PopupMenuItem::new(t(cx, "pi-value-unset")).on_click(move |_, _, cx| {
                        e.update(cx, |this, cx| {
                            this.clear_controls(&p);
                            this.write(&p, None, cx)
                        })
                    }),
                )
            })
            .into_any_element()
    }
    pub(super) fn choice(&self, path: &[String], kind: Field, cx: &Context<Self>) -> AnyElement {
        let current = self.entry.visible(path);
        let present = self.entry.present(path);
        let p = path.to_vec();
        let e = cx.entity();
        let mut values = vec![(None, t(cx, "pi-value-unset"))];
        match kind {
            Field::Bool => values.extend([
                (Some(json!(true)), t(cx, "pi-files-on")),
                (Some(json!(false)), t(cx, "pi-files-off")),
            ]),
            Field::Choice(options) => {
                values.extend(options.iter().map(|s| (Some(json!(s)), (*s).into())))
            }
            _ => {}
        }
        let label = match current {
            Some(Value::Bool(b)) => t(cx, if b { "pi-files-on" } else { "pi-files-off" }),
            Some(v) => v
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string()),
            None => t(
                cx,
                if present {
                    "pi-resource-unsupported"
                } else {
                    "pi-value-unset"
                },
            ),
        };
        Button::new(SharedString::from(format!("choice-{}", path_id(path))))
            .w_full()
            .small()
            .outline()
            .label(label)
            .dropdown_caret(true)
            .accessibility_label(if path.first().is_some_and(|p| p == "toolExposure") {
                path.last().unwrap().clone()
            } else {
                t(cx, &format!("pi-files-label-{}", path.last().unwrap()))
            })
            .dropdown_menu(move |menu, _, _| {
                values.iter().fold(menu, |menu, (value, label)| {
                    let (e, p, value) = (e.clone(), p.clone(), value.clone());
                    menu.item(PopupMenuItem::new(label.clone()).on_click(move |_, _, cx| {
                        e.update(cx, |this, cx| this.write(&p, value.clone(), cx))
                    }))
                })
            })
            .into_any_element()
    }
    fn field(
        &mut self,
        path: Vec<String>,
        kind: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = path.last().unwrap().clone();
        let control = match kind {
            Field::Secret if self.secrets.get(&path) == Some(&1) => {
                self.input(&path, kind, window, cx)
            }
            Field::Secret => div()
                .py_1()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(self.secret_status(&path, cx))
                .into_any_element(),
            Field::Bool | Field::Choice(_) => self.choice(&path, kind, cx),
            _ => self.input(&path, kind, window, cx),
        };
        let control = v_flex()
            .gap_1()
            .child(control)
            .when(self.invalid.contains_key(&path), |col| {
                col.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(t(cx, "pi-files-invalid")),
                )
            })
            .into_any_element();
        collection_row(
            field_label(&key, cx),
            control,
            self.field_menu(&path, matches!(kind, Field::Secret), cx),
        )
    }
    fn advanced(
        &mut self,
        prefix: &[String],
        keys: &[&str],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.expanded.contains(prefix);
        let p = prefix.to_vec();
        let mut body = v_flex().gap_2().child(
            Button::new(SharedString::from(format!("advanced-{}", path_id(prefix))))
                .ghost()
                .small()
                .icon(if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .label(t(cx, "pi-files-advanced"))
                .self_start()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.expanded.remove(&p) {
                        this.expanded.insert(p.clone());
                    }
                    cx.notify()
                })),
        );
        if open {
            for key in keys {
                let mut p = prefix.to_vec();
                p.push((*key).into());
                body = body.child(self.field(p, Field::Json, window, cx));
            }
        }
        body.into_any_element()
    }
    fn fields(
        &mut self,
        prefix: &[String],
        fields: &[(&str, Field)],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        fields
            .iter()
            .map(|(key, kind)| {
                let mut p = prefix.to_vec();
                p.push((*key).into());
                self.field(p, *kind, window, cx)
            })
            .collect()
    }
    pub(super) fn open_model(
        &mut self,
        path: Vec<String>,
        overrides: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.parent_offset = self.scroll.offset();
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.subview = Some((path.clone(), overrides));
        let mut first = path;
        first.push(if overrides { "name" } else { "id" }.into());
        let _ = self.input(&first, Field::Text, window, cx);
        window.focus(&self.text[&first].state.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.subview = None;
        self.scroll.set_offset(self.parent_offset);
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn render_model(
        &mut self,
        path: Vec<String>,
        overrides: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut body = v_flex().w_full().gap_4().child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new("model-back")
                        .ghost()
                        .small()
                        .icon(IconName::ChevronLeft)
                        .label(t(cx, "pi-files-back"))
                        .on_click(cx.listener(|this, _, w, cx| this.back(w, cx))),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(self.model_title(&path, overrides)),
                ),
        );
        let mut basic = vec![
            ("name", Field::Text),
            ("reasoning", Field::Bool),
            ("contextWindow", Field::Number),
            ("maxTokens", Field::Number),
        ];
        if !overrides {
            basic.insert(0, ("id", Field::Text));
        }
        let mut items = self.fields(&path, &basic, window, cx);
        let mut input = path.clone();
        input.push("input".into());
        items.push(self.collection(input, Collection::Strings, window, cx));
        body = body.child(section("pi-files-basic", items, cx));
        if !overrides {
            let items = self.fields(
                &path,
                &[("api", Field::Text), ("baseUrl", Field::Text)],
                window,
                cx,
            );
            body = body.child(section("pi-files-connection", items, cx));
        }
        let mut headers = path.clone();
        headers.push("headers".into());
        let headers = self.collection(headers, Collection::Secrets, window, cx);
        body = body.child(section("pi-files-authentication", vec![headers], cx));
        body.child(self.advanced(&path, COMPLEX, window, cx))
            .into_any_element()
    }
    fn definition_row(&self, cx: &Context<Self>) -> AnyElement {
        let e = cx.entity();
        let label = t(
            cx,
            if self.partial {
                "pi-files-override"
            } else {
                "pi-files-definition"
            },
        );
        collection_row(
            div()
                .text_sm()
                .child(t(cx, "pi-files-definition-kind"))
                .into_any_element(),
            Button::new("mcp-definition")
                .small()
                .outline()
                .w_full()
                .label(label)
                .dropdown_caret(true)
                .dropdown_menu(move |menu, _, cx| {
                    [(true, "pi-files-override"), (false, "pi-files-definition")]
                        .into_iter()
                        .fold(menu, |menu, (partial, label)| {
                            let e = e.clone();
                            menu.item(PopupMenuItem::new(t(cx, label)).on_click(move |_, _, cx| {
                                e.update(cx, |this, cx| {
                                    this.partial = partial;
                                    this.entry = Entry::new(Kind::Mcp);
                                    this.clear_controls(&[]);
                                    cx.notify()
                                })
                            }))
                        })
                })
                .into_any_element(),
            div().w_6().flex_shrink_0().into_any_element(),
        )
    }
    fn render_root(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let name = collection_row(
            div()
                .text_sm()
                .child(t(cx, "pi-files-name"))
                .into_any_element(),
            Input::new(&self.name)
                .small()
                .w_full()
                .disabled(self.editing)
                .aria_label(t(cx, "pi-files-name"))
                .into_any_element(),
            div().w_6().flex_shrink_0().into_any_element(),
        );
        let mut basic = vec![name];
        if self.kind == Kind::Models {
            basic.extend(self.fields(&[], &[("name", Field::Text)], window, cx));
        } else {
            if self.project && !self.editing {
                basic.push(self.definition_row(cx));
            }
            basic.extend(self.fields(
                &[],
                &[("description", Field::Text), ("enabled", Field::Bool)],
                window,
                cx,
            ));
        }
        if self.kind == Kind::Mcp && self.partial {
            basic = vec![collection_row(
                div()
                    .text_sm()
                    .child(t(cx, "pi-files-name"))
                    .into_any_element(),
                Input::new(&self.name)
                    .small()
                    .w_full()
                    .disabled(self.editing)
                    .aria_label(t(cx, "pi-files-name"))
                    .into_any_element(),
                div().w_6().into_any_element(),
            )];
            if !self.editing {
                basic.push(self.definition_row(cx));
            }
            basic.extend(self.fields(&[], &[("enabled", Field::Bool)], window, cx));
        }
        let mut body = v_flex()
            .w_full()
            .gap_4()
            .child(section("pi-files-basic", basic, cx));
        if !self.entry.is_object() {
            return body
                .child(t(cx, "pi-resource-unsupported"))
                .into_any_element();
        }
        if self.kind == Kind::Models {
            let connection = self.fields(
                &[],
                &[("baseUrl", Field::Text), ("api", Field::Text)],
                window,
                cx,
            );
            body = body.child(section("pi-files-connection", connection, cx));
            let models = self.collection(vec!["models".into()], Collection::Models, window, cx);
            let overrides = self.collection(
                vec!["modelOverrides".into()],
                Collection::Overrides,
                window,
                cx,
            );
            body = body.child(section(
                "pi-files-label-models",
                vec![models, overrides],
                cx,
            ));
            let mut auth = self.fields(
                &[],
                &[
                    ("apiKey", Field::Secret),
                    ("authHeader", Field::Bool),
                    ("oauth", Field::Choice(&["radius"])),
                ],
                window,
                cx,
            );
            auth.push(self.collection(vec!["headers".into()], Collection::Secrets, window, cx));
            body = body
                .child(section("pi-files-authentication", auth, cx))
                .child(self.advanced(&[], &["compat"], window, cx));
        } else {
            if !self.partial {
                let transport = self.transport();
                let mut connection = self.fields(
                    &[],
                    &[
                        ("type", Field::Choice(&["stdio", "http"])),
                        ("timeout", Field::Number),
                    ],
                    window,
                    cx,
                );
                if transport != Transport::Http {
                    connection.extend(self.fields(
                        &[],
                        &[("command", Field::Text), ("cwd", Field::Text)],
                        window,
                        cx,
                    ));
                    connection.push(self.collection(
                        vec!["args".into()],
                        Collection::Strings,
                        window,
                        cx,
                    ));
                    connection.push(self.collection(
                        vec!["env".into()],
                        Collection::Secrets,
                        window,
                        cx,
                    ));
                }
                if transport != Transport::Stdio {
                    connection.extend(self.fields(&[], &[("url", Field::Text)], window, cx));
                    connection.push(self.collection(
                        vec!["headers".into()],
                        Collection::Secrets,
                        window,
                        cx,
                    ));
                }
                body = body.child(section("pi-files-connection", connection, cx));
            }
            let mut exposure =
                self.fields(&[], &[("exposure", Field::Choice(EXPOSURE))], window, cx);
            exposure.push(self.collection(
                vec!["toolExposure".into()],
                Collection::Exposure,
                window,
                cx,
            ));
            body = body.child(section("pi-files-exposure-section", exposure, cx));
            let mut auth = Vec::new();
            if !self.partial && self.transport() != Transport::Stdio {
                auth.extend(self.fields(
                    &["oauth".into()],
                    &[
                        ("clientId", Field::Text),
                        ("clientSecret", Field::Secret),
                        ("callbackPort", Field::Number),
                        ("callbackUrl", Field::Text),
                        ("scope", Field::Text),
                        ("clientName", Field::Text),
                        ("clientRegistration", Field::Choice(&["dcr", "cimd"])),
                        ("authServerMetadataUrl", Field::Text),
                    ],
                    window,
                    cx,
                ));
                if !self.project {
                    auth.extend(self.fields(
                        &["auth".into()],
                        &[("provider", Field::Text)],
                        window,
                        cx,
                    ));
                }
            }
            let provider = vec!["auth".into(), "provider".into()];
            if self.project && self.entry.present(&provider) {
                let value = self
                    .entry
                    .visible(&provider)
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string())
                    })
                    .unwrap_or_else(|| t(cx, "pi-resource-unsupported"));
                let tooltip = value.clone();
                auth.push(collection_row(
                    field_label("provider", cx),
                    div()
                        .id("project-auth-provider")
                        .py_1()
                        .text_sm()
                        .font_family(cx.theme().mono_font_family.clone())
                        .truncate()
                        .child(value)
                        .tooltip(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tooltip.clone())
                                .build(window, cx)
                        })
                        .into_any_element(),
                    self.field_menu(&provider, false, cx),
                ));
            }
            if !auth.is_empty() {
                body = body.child(section("pi-files-authentication", auth, cx));
            }
        }
        body.into_any_element()
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Transport {
    Stdio,
    Http,
    Both,
}
impl EntryEditor {
    fn transport(&self) -> Transport {
        match self
            .entry
            .visible(&["type".into()])
            .as_ref()
            .and_then(Value::as_str)
        {
            Some("stdio") => Transport::Stdio,
            Some("http" | "streamable-http") => Transport::Http,
            Some(_) => Transport::Both,
            None if !self.entry.present(&["type".into()]) => match (
                self.entry.present(&["command".into()]),
                self.entry.present(&["url".into()]),
            ) {
                (true, false) => Transport::Stdio,
                (false, true) => Transport::Http,
                _ => Transport::Both,
            },
            _ => Transport::Both,
        }
    }
}
impl Render for EntryEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if let Some((path, overrides)) = self.subview.clone() {
            self.render_model(path, overrides, window, cx)
        } else {
            self.render_root(window, cx)
        };
        v_flex()
            .id("file-editor-body")
            .track_focus(&self.focus)
            .key_context("PiConfigFileEditor")
            .w_full()
            .min_h_0()
            .max_h(rems(32.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .gap_2()
            .when_some(self.error, |col, error| {
                col.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(t(cx, error_key(error))),
                )
            })
            .child(body)
    }
}
