//! Structured entry editor. Original credentials never become control values.
use super::{Entry, Error, Kind, error_key};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_config_files::Credential;
use gupi_settings::{assets::IconName, i18n::t};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
enum Field {
    Text,
    Number,
    Bool,
    Choice(&'static [&'static str]),
    Json,
    Secret,
}
struct TextControl {
    state: Entity<InputState>,
    _subscription: Subscription,
}
struct JsonControl {
    state: Entity<TextareaState>,
    _subscription: Subscription,
}
pub(super) struct EntryEditor {
    kind: Kind,
    entry: Entry,
    original: Entry,
    name: Entity<InputState>,
    editing: bool,
    project: bool,
    partial: bool,
    text: BTreeMap<Vec<String>, TextControl>,
    json: BTreeMap<Vec<String>, JsonControl>,
    invalid: BTreeMap<Vec<String>, String>,
    replacements: BTreeMap<Vec<String>, String>,
    secrets: BTreeMap<Vec<String>, u8>,
    expanded: BTreeSet<Vec<String>>,
    key_inputs: BTreeMap<Vec<String>, Entity<InputState>>,
    error: Option<Error>,
}
const EXPOSURE: &[&str] = &["codemode", "deferred", "direct", "hidden"];
const COMPLEX: &[&str] = &[
    "compat",
    "cost",
    "thinkingLevelMap",
    "inputLimits",
    "promptCache",
    "samplingParams",
    "samplingParamsByThinkingLevel",
];
impl EntryEditor {
    pub(super) fn new(
        kind: Kind,
        name: String,
        entry: Entry,
        editing: bool,
        project: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let partial = kind == Kind::Mcp && project && entry.partial();
        Self {
            kind,
            original: entry.clone(),
            entry,
            name: cx.new(|cx| InputState::new(window, cx).default_value(name)),
            editing,
            project,
            partial,
            text: BTreeMap::new(),
            json: BTreeMap::new(),
            invalid: BTreeMap::new(),
            replacements: BTreeMap::new(),
            secrets: BTreeMap::new(),
            expanded: BTreeSet::new(),
            key_inputs: BTreeMap::new(),
            error: None,
        }
    }
    pub(super) fn show_error(&mut self, error: Error, cx: &mut Context<Self>) {
        self.error = Some(error);
        cx.notify()
    }
    pub(super) fn prepare(&mut self, cx: &mut Context<Self>) -> Result<(String, Entry), Error> {
        let name = self.name.read(cx).value().to_string();
        let result = if name.is_empty()
            || !self.invalid.is_empty()
            || (self.kind == Kind::Mcp && !self.partial && self.entry.partial())
        {
            Err(Error::Invalid)
        } else {
            self.entry.validate_edit(&self.original, self.project)
        };
        if let Err(e) = result {
            self.show_error(e, cx);
            return Err(e);
        }
        Ok((name, self.entry.clone()))
    }
    fn write(&mut self, path: &[String], value: Option<Value>, cx: &mut Context<Self>) {
        if let Err(error) = self.entry.set(path, value) {
            self.error = Some(error)
        } else {
            self.invalid.remove(path);
        }
        cx.notify()
    }
    fn clear_controls(&mut self, path: &[String]) {
        self.text.retain(|p, _| !p.starts_with(path));
        self.json.retain(|p, _| !p.starts_with(path));
        self.invalid.retain(|p, _| !p.starts_with(path));
        self.replacements.retain(|p, _| !p.starts_with(path));
        self.secrets.retain(|p, _| !p.starts_with(path));
        self.expanded.retain(|p| !p.starts_with(path));
    }
    /// Array positions change; keep drafts attached to the same logical item.
    fn change_array(&mut self, path: &[String], ix: usize, to: Option<usize>) {
        let invalid = std::mem::take(&mut self.invalid);
        let secrets = std::mem::take(&mut self.secrets);
        let replacements = std::mem::take(&mut self.replacements);
        let expanded = std::mem::take(&mut self.expanded);
        self.clear_controls(path);
        let remap = |mut key: Vec<String>| -> Option<Vec<String>> {
            if key.starts_with(path) && key.len() > path.len() {
                let index = key[path.len()].parse::<usize>().ok()?;
                let next = match to {
                    None if index == ix => return None,
                    None if index > ix => index - 1,
                    Some(to) if index == ix => to,
                    Some(to) if ix < to && index > ix && index <= to => index - 1,
                    Some(to) if to < ix && index >= to && index < ix => index + 1,
                    _ => index,
                };
                key[path.len()] = next.to_string();
            }
            Some(key)
        };
        self.invalid = invalid
            .into_iter()
            .filter_map(|(k, v)| remap(k).map(|k| (k, v)))
            .collect();
        self.secrets = secrets
            .into_iter()
            .filter_map(|(k, v)| remap(k).map(|k| (k, v)))
            .collect();
        self.replacements = replacements
            .into_iter()
            .filter_map(|(k, v)| remap(k).map(|k| (k, v)))
            .collect();
        self.expanded = expanded.into_iter().filter_map(remap).collect();
        if let Some(to) = to {
            let _ = self.entry.move_item(path, ix, to);
            let _ = self.original.move_item(path, ix, to);
        } else {
            let _ = self.entry.remove_item(path, ix);
            let _ = self.original.remove_item(path, ix);
        }
    }
    fn input(
        &mut self,
        path: &[String],
        kind: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let secret = matches!(kind, Field::Secret);
        let label = path.join(".");
        if matches!(kind, Field::Json) {
            if !self.json.contains_key(path) {
                let text = self.invalid.get(path).cloned().unwrap_or_else(|| {
                    self.entry
                        .visible(path)
                        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
                        .unwrap_or_default()
                });
                let state = cx.new(|cx| TextareaState::new(window, cx).default_value(text).rows(4));
                let p = path.to_vec();
                let sub = cx.subscribe(&state, move |this, state, event, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = state.read(cx).value();
                        match serde_json::from_str::<Value>(&value) {
                            Ok(value) => this.write(&p, Some(value), cx),
                            Err(_) => {
                                this.invalid.insert(p.clone(), value.to_string());
                                cx.notify()
                            }
                        }
                    }
                });
                self.json.insert(
                    path.to_vec(),
                    JsonControl {
                        state,
                        _subscription: sub,
                    },
                );
            }
            return Textarea::new(&self.json[path].state)
                .small()
                .h(rems(6.))
                .aria_label(label)
                .into_any_element();
        }
        if !self.text.contains_key(path) {
            let value = if secret {
                self.replacements.get(path).cloned().unwrap_or_default()
            } else if let Some(text) = self.invalid.get(path) {
                text.clone()
            } else {
                self.entry
                    .visible(path)
                    .map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| v.to_string())
                    })
                    .unwrap_or_default()
            };
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .masked(secret)
            });
            let p = path.to_vec();
            let sub = cx.subscribe(&state, move |this, state, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = state.read(cx).value().to_string();
                    if secret {
                        this.replacements.insert(p.clone(), text.clone());
                        if let Err(e) = this.entry.credential(&p, Credential::Replace(text)) {
                            this.error = Some(e)
                        }
                        cx.notify()
                    } else {
                        let value = if matches!(kind, Field::Number) {
                            serde_json::from_str::<Value>(&text)
                                .ok()
                                .filter(Value::is_number)
                        } else {
                            Some(Value::String(text.clone()))
                        };
                        if let Some(value) = value {
                            this.write(&p, Some(value), cx)
                        } else {
                            this.invalid.insert(p.clone(), text);
                            cx.notify()
                        }
                    }
                }
            });
            self.text.insert(
                path.to_vec(),
                TextControl {
                    state,
                    _subscription: sub,
                },
            );
        }
        Input::new(&self.text[path].state)
            .small()
            .w_full()
            .aria_label(label)
            .into_any_element()
    }
    fn field(
        &mut self,
        path: Vec<String>,
        kind: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = path.last().cloned().unwrap_or_default();
        let id = serde_json::to_string(&path).unwrap_or_default();
        let entity = cx.entity();
        let current = self.entry.visible(&path);
        let present = self.entry.present(&path);
        let mut control = v_flex().gap_1().w_80().min_w_0().flex_shrink_0();
        if matches!(kind, Field::Secret) {
            let mode = self.secrets.get(&path).copied().unwrap_or(0);
            let p = path.clone();
            let e = entity.clone();
            let status = if mode == 0 {
                if self.original.present(&path) {
                    "pi-files-secret-set"
                } else {
                    "pi-value-unset"
                }
            } else if mode == 1 {
                "pi-files-secret-replace"
            } else {
                "pi-files-secret-remove"
            };
            control = control.child(
                Button::new(SharedString::from(format!("secret-{id}")))
                    .small()
                    .outline()
                    .label(t(cx, status))
                    .dropdown_caret(true)
                    .accessibility_label(label.clone())
                    .dropdown_menu(move |menu, _, cx| {
                        [
                            (0, "pi-files-secret-keep"),
                            (1, "pi-files-secret-replace"),
                            (2, "pi-files-secret-remove"),
                        ]
                        .into_iter()
                        .fold(menu, |menu, (mode, label)| {
                            let (e, p) = (e.clone(), p.clone());
                            menu.item(PopupMenuItem::new(t(cx, label)).on_click(move |_, _, cx| {
                                e.update(cx, |this, cx| {
                                    this.clear_controls(&p);
                                    this.secrets.insert(p.clone(), mode);
                                    let result = if mode == 0 {
                                        this.entry.restore_credential(&this.original, &p)
                                    } else {
                                        this.entry.credential(
                                            &p,
                                            if mode == 1 {
                                                Credential::Replace(String::new())
                                            } else {
                                                Credential::Remove
                                            },
                                        )
                                    };
                                    if let Err(e) = result {
                                        this.error = Some(e)
                                    }
                                    cx.notify()
                                })
                            }))
                        })
                    }),
            );
            if mode == 1 {
                control = control.child(self.input(&path, kind, window, cx));
            }
        } else if matches!(kind, Field::Bool | Field::Choice(_)) {
            let mut values = vec![(None, t(cx, "pi-value-unset"))];
            match kind {
                Field::Bool => {
                    values.push((Some(Value::Bool(true)), t(cx, "pi-files-on")));
                    values.push((Some(Value::Bool(false)), t(cx, "pi-files-off")))
                }
                Field::Choice(options) => {
                    for value in options {
                        values.push((Some(json!(value)), (*value).into()))
                    }
                }
                _ => {}
            }
            let label = current
                .as_ref()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string())
                })
                .unwrap_or_else(|| {
                    t(
                        cx,
                        if present {
                            "pi-resource-unsupported"
                        } else {
                            "pi-value-unset"
                        },
                    )
                });
            let p = path.clone();
            let choice_entity = entity.clone();
            control = control.child(
                Button::new(SharedString::from(format!("choice-{id}")))
                    .small()
                    .outline()
                    .label(label)
                    .dropdown_caret(true)
                    .accessibility_label(path.join("."))
                    .dropdown_menu(move |menu, _, _| {
                        values.iter().fold(menu, |menu, (value, label)| {
                            let (e, p, value) = (choice_entity.clone(), p.clone(), value.clone());
                            menu.item(PopupMenuItem::new(label.clone()).on_click(
                                move |_, _, cx| {
                                    e.update(cx, |this, cx| this.write(&p, value.clone(), cx))
                                },
                            ))
                        })
                    }),
            );
        } else {
            control = control.child(self.input(&path, kind, window, cx));
        }
        if self.invalid.contains_key(&path) {
            control = control.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().danger)
                    .child(t(cx, "pi-files-invalid")),
            );
        }
        let mut row = h_flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().text_sm().child(label))
            .child(control);
        if !matches!(kind, Field::Secret) {
            let p = path.clone();
            row = row.child(
                Button::new(SharedString::from(format!("field-{id}")))
                    .w_6()
                    .flex_shrink_0()
                    .ghost()
                    .xsmall()
                    .icon(IconName::Ellipsis)
                    .accessibility_label(t(cx, "pi-field-actions"))
                    .dropdown_menu(move |menu, _, cx| {
                        let p = p.clone();
                        let entity = entity.clone();
                        let mut menu = menu;
                        if p.len() == 2 && p[0] == "toolExposure" {
                            let parent = vec![p[0].clone()];
                            let keys = entity.read(cx).entry.keys(&parent).unwrap_or_default();
                            if let Some(ix) = keys.iter().position(|k| k == &p[1]) {
                                for (valid, to, key) in [
                                    (ix > 0, ix.saturating_sub(1), "pi-resource-move-up"),
                                    (ix + 1 < keys.len(), ix + 1, "pi-resource-move-down"),
                                ] {
                                    if valid {
                                        let (e, parent) = (entity.clone(), parent.clone());
                                        menu = menu.item(PopupMenuItem::new(t(cx, key)).on_click(
                                            move |_, _, cx| {
                                                e.update(cx, |this, cx| {
                                                    let _ = this.entry.move_item(&parent, ix, to);
                                                    cx.notify()
                                                })
                                            },
                                        ));
                                    }
                                }
                            }
                        }
                        menu.item(PopupMenuItem::new(t(cx, "pi-value-unset")).on_click(
                            move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.clear_controls(&p);
                                    this.write(&p, None, cx)
                                })
                            },
                        ))
                    }),
            );
        }
        if matches!(kind, Field::Secret) {
            row = row.child(div().w_6().flex_shrink_0());
        }
        row.into_any_element()
    }
    fn collection_header(&self, path: &[String], array: bool, cx: &Context<Self>) -> AnyElement {
        let label = path.last().cloned().unwrap_or_default();
        let entity = cx.entity();
        let p = path.to_vec();
        h_flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().text_sm().child(label))
            .when(!self.entry.present(path), |row| {
                row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(cx, "pi-value-unset")),
                )
            })
            .child(
                Button::new(SharedString::from(format!(
                    "collection-{}",
                    serde_json::to_string(path).unwrap_or_default()
                )))
                .ghost()
                .xsmall()
                .icon(IconName::Ellipsis)
                .accessibility_label(t(cx, "pi-field-actions"))
                .dropdown_menu(move |menu, _, cx| {
                    [None, Some(if array { json!([]) } else { json!({}) })]
                        .into_iter()
                        .fold(menu, |menu, value| {
                            let (e, p) = (entity.clone(), p.clone());
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
                }),
            )
            .into_any_element()
    }
    fn map(
        &mut self,
        path: Vec<String>,
        secret: bool,
        models: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut body = v_flex()
            .gap_2()
            .child(self.collection_header(&path, false, cx));
        let Ok(keys) = self.entry.keys(&path) else {
            return body
                .child(t(cx, "pi-resource-unsupported"))
                .into_any_element();
        };
        let keys = if secret {
            let mut all = self.original.keys(&path).unwrap_or_default();
            for key in keys {
                if !all.contains(&key) {
                    all.push(key)
                }
            }
            all
        } else {
            keys
        };
        for key in keys {
            let mut p = path.clone();
            p.push(key);
            body = body.child(if models {
                self.model(p, true, window, cx)
            } else {
                self.field(
                    p,
                    if secret {
                        Field::Secret
                    } else {
                        Field::Choice(EXPOSURE)
                    },
                    window,
                    cx,
                )
            });
        }
        if !self.key_inputs.contains_key(&path) {
            let input = cx.new(|cx| InputState::new(window, cx));
            self.key_inputs.insert(path.clone(), input);
        }
        let input = self.key_inputs[&path].clone();
        let p = path.clone();
        let e = cx.entity();
        body.child(
            h_flex()
                .gap_2()
                .child(Input::new(&input).small().aria_label(t(cx, "pi-files-key")))
                .child(
                    Button::new(SharedString::from(format!("map-add-{}", path.join("/"))))
                        .small()
                        .ghost()
                        .label(t(cx, "pi-resource-add"))
                        .on_click(move |_, window, cx| {
                            let key = input.read(cx).value().to_string();
                            if key.is_empty() {
                                return;
                            }
                            e.update(cx, |this, cx| {
                                let mut p = p.clone();
                                p.push(key);
                                if this.entry.present(&p) || (secret && this.original.present(&p)) {
                                    this.show_error(Error::Duplicate, cx);
                                    return;
                                }
                                if secret {
                                    let _ = this
                                        .entry
                                        .credential(&p, Credential::Replace(String::new()));
                                    this.secrets.insert(p, 1);
                                } else {
                                    if models {
                                        let _ = this.original.set(&p, Some(json!({})));
                                        this.expanded.insert(p.clone());
                                    }
                                    this.write(
                                        &p,
                                        Some(if models { json!({}) } else { json!("codemode") }),
                                        cx,
                                    )
                                }
                                cx.notify()
                            });
                            input.update(cx, |input, cx| input.set_value("", window, cx));
                        }),
                ),
        )
        .into_any_element()
    }
    fn strings(
        &mut self,
        path: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut body = v_flex()
            .gap_1()
            .child(self.collection_header(&path, true, cx));
        let Ok(len) = self.entry.len(&path) else {
            return body
                .child(t(cx, "pi-resource-unsupported"))
                .into_any_element();
        };
        for ix in 0..len {
            let mut p = path.clone();
            p.push(ix.to_string());
            let input = self.input(&p, Field::Text, window, cx);
            let e = cx.entity();
            let list = path.clone();
            body = body.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().min_w_0().child(input))
                    .child(
                        Button::new(SharedString::from(format!(
                            "list-menu-{}/{ix}",
                            path.join("/")
                        )))
                        .xsmall()
                        .ghost()
                        .icon(IconName::Ellipsis)
                        .accessibility_label(t(cx, "pi-field-actions"))
                        .dropdown_menu(move |menu, _, cx| {
                            let mut menu = menu;
                            for (valid, to, label) in [
                                (ix > 0, ix.saturating_sub(1), "pi-resource-move-up"),
                                (ix + 1 < len, ix + 1, "pi-resource-move-down"),
                            ] {
                                if valid {
                                    let (e, p) = (e.clone(), list.clone());
                                    menu = menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                                        move |_, _, cx| {
                                            e.update(cx, |this, cx| {
                                                this.clear_controls(&p);
                                                let _ = this.entry.move_item(&p, ix, to);
                                                cx.notify()
                                            })
                                        },
                                    ));
                                }
                            }
                            let (e, p) = (e.clone(), list.clone());
                            menu.item(PopupMenuItem::new(t(cx, "pi-files-delete")).on_click(
                                move |_, _, cx| {
                                    e.update(cx, |this, cx| {
                                        this.clear_controls(&p);
                                        let _ = this.entry.remove_item(&p, ix);
                                        cx.notify()
                                    })
                                },
                            ))
                        }),
                    ),
            );
        }
        let e = cx.entity();
        let p = path.clone();
        body.child(
            Button::new(SharedString::from(format!("list-add-{}", path.join("/"))))
                .ghost()
                .small()
                .label(t(cx, "pi-resource-add"))
                .on_click(move |_, _, cx| {
                    e.update(cx, |this, cx| {
                        let _ = this.entry.append(&p, json!(""));
                        cx.notify()
                    })
                }),
        )
        .into_any_element()
    }
    fn model(
        &mut self,
        path: Vec<String>,
        overrides: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut body = v_flex().gap_2();
        let key = path.join("/");
        let open = self.expanded.contains(&path);
        let p = path.clone();
        let title = if overrides {
            path.last().cloned().unwrap_or_default()
        } else {
            let mut id = path.clone();
            id.push("id".into());
            self.entry
                .visible(&id)
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| t(cx, "pi-files-model"))
        };
        let e = cx.entity();
        let item = path.clone();
        body = body.child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new(SharedString::from(format!("model-expand-{key}")))
                        .ghost()
                        .small()
                        .label(title)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.expanded.remove(&p) {
                                this.expanded.insert(p.clone());
                            }
                            cx.notify()
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("model-actions-{key}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Ellipsis)
                        .accessibility_label(t(cx, "pi-field-actions"))
                        .dropdown_menu(move |menu, _, cx| {
                            let mut menu = menu;
                            let parent = item[..item.len() - 1].to_vec();
                            let keys = if overrides {
                                e.read(cx).entry.keys(&parent).unwrap_or_default()
                            } else {
                                (0..e.read(cx).entry.len(&parent).unwrap_or_default())
                                    .map(|ix| ix.to_string())
                                    .collect()
                            };
                            if let Some(ix) = keys.iter().position(|k| Some(k) == item.last()) {
                                for (valid, to, label) in [
                                    (ix > 0, ix.saturating_sub(1), "pi-resource-move-up"),
                                    (ix + 1 < keys.len(), ix + 1, "pi-resource-move-down"),
                                ] {
                                    if valid {
                                        let (e, p) = (e.clone(), parent.clone());
                                        menu =
                                            menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                                                move |_, _, cx| {
                                                    e.update(cx, |this, cx| {
                                                        if overrides {
                                                            let _ =
                                                                this.entry.move_item(&p, ix, to);
                                                            let _ =
                                                                this.original.move_item(&p, ix, to);
                                                        } else {
                                                            this.change_array(&p, ix, Some(to));
                                                        }
                                                        cx.notify()
                                                    })
                                                },
                                            ));
                                    }
                                }
                                let (e, p, item) = (e.clone(), parent.clone(), item.clone());
                                menu = menu.item(
                                    PopupMenuItem::new(t(cx, "pi-files-delete")).on_click(
                                        move |_, _, cx| {
                                            e.update(cx, |this, cx| {
                                                if overrides {
                                                    this.clear_controls(&item);
                                                    let _ = this.entry.set(&item, None);
                                                    let _ = this.original.set(&item, None);
                                                } else {
                                                    this.change_array(&p, ix, None);
                                                }
                                                cx.notify()
                                            })
                                        },
                                    ),
                                );
                            }
                            menu
                        }),
                ),
        );
        if open {
            let mut fields = vec![
                ("name", Field::Text),
                ("reasoning", Field::Bool),
                ("contextWindow", Field::Number),
                ("maxTokens", Field::Number),
            ];
            if !overrides {
                fields.splice(
                    0..0,
                    [
                        ("id", Field::Text),
                        ("api", Field::Text),
                        ("baseUrl", Field::Text),
                    ],
                );
            }
            for (k, f) in fields {
                let mut p = path.clone();
                p.push(k.into());
                body = body.child(self.field(p, f, window, cx));
            }
            let mut input = path.clone();
            input.push("input".into());
            body = body.child(self.strings(input, window, cx));
            for k in COMPLEX {
                let mut p = path.clone();
                p.push((*k).into());
                body = body.child(self.field(p, Field::Json, window, cx));
            }
            let mut headers = path.clone();
            headers.push("headers".into());
            body = body.child(self.map(headers, true, false, window, cx));
        }
        body.into_any_element()
    }
}
impl Render for EntryEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex().gap_3().w_full();
        if let Some(error) = self.error {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(t(cx, error_key(error))),
            );
        }
        body = body.child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().flex_1().child(t(cx, "pi-files-name")))
                .child(
                    Input::new(&self.name)
                        .small()
                        .w_80()
                        .disabled(self.editing)
                        .aria_label(t(cx, "pi-files-name")),
                )
                .child(div().w_6().flex_shrink_0()),
        );
        if !self.entry.is_object() {
            return body.child(t(cx, "pi-resource-unsupported"));
        }
        let mut fields = vec![];
        if self.kind == Kind::Models {
            fields.extend([
                ("name", Field::Text),
                ("baseUrl", Field::Text),
                ("api", Field::Text),
                ("apiKey", Field::Secret),
                ("authHeader", Field::Bool),
                ("oauth", Field::Choice(&["radius"])),
                ("compat", Field::Json),
            ]);
        } else {
            if self.project && !self.editing {
                let current = if self.partial {
                    "pi-files-override"
                } else {
                    "pi-files-definition"
                };
                body = body.child(
                    Button::new("mcp-definition")
                        .small()
                        .outline()
                        .label(t(cx, current))
                        .dropdown_caret(true)
                        .dropdown_menu({
                            let e = cx.entity();
                            move |menu, _, cx| {
                                [(true, "pi-files-override"), (false, "pi-files-definition")]
                                    .into_iter()
                                    .fold(menu, |menu, (partial, label)| {
                                        let e = e.clone();
                                        menu.item(PopupMenuItem::new(t(cx, label)).on_click(
                                            move |_, _, cx| {
                                                e.update(cx, |this, cx| {
                                                    this.partial = partial;
                                                    this.entry = Entry::new(Kind::Mcp);
                                                    this.text.clear();
                                                    this.json.clear();
                                                    this.invalid.clear();
                                                    cx.notify()
                                                })
                                            },
                                        ))
                                    })
                            }
                        }),
                );
            }
            fields.extend([
                ("enabled", Field::Bool),
                ("exposure", Field::Choice(EXPOSURE)),
            ]);
            if !self.partial {
                fields.extend([
                    ("type", Field::Choice(&["stdio", "http"])),
                    ("description", Field::Text),
                    ("command", Field::Text),
                    ("url", Field::Text),
                    ("cwd", Field::Text),
                    ("timeout", Field::Number),
                ]);
            }
        }
        for (k, field) in fields {
            body = body.child(self.field(vec![k.into()], field, window, cx));
        }
        if self.kind == Kind::Models {
            body = body.child(self.map(vec!["headers".into()], true, false, window, cx));
            let path = vec!["models".into()];
            body = body.child(self.collection_header(&path, true, cx));
            if let Ok(len) = self.entry.len(&path) {
                for ix in 0..len {
                    body = body.child(self.model(
                        vec!["models".into(), ix.to_string()],
                        false,
                        window,
                        cx,
                    ));
                }
                body = body.child(
                    Button::new("model-add")
                        .small()
                        .ghost()
                        .label(t(cx, "pi-files-add-model"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let path = vec!["models".into()];
                            let ix = this.entry.len(&path).unwrap_or_default();
                            let _ = this.entry.append(&path, json!({}));
                            let _ = this.original.append(&path, json!({}));
                            this.expanded.insert(vec!["models".into(), ix.to_string()]);
                            cx.notify()
                        })),
                );
            } else {
                body = body.child(t(cx, "pi-resource-unsupported"));
            }
            body = body.child(self.map(vec!["modelOverrides".into()], false, true, window, cx));
        } else {
            body = body.child(self.map(vec!["toolExposure".into()], false, false, window, cx));
            if !self.partial {
                body = body
                    .child(self.strings(vec!["args".into()], window, cx))
                    .child(self.map(vec!["env".into()], true, false, window, cx))
                    .child(self.map(vec!["headers".into()], true, false, window, cx));
                for (k, f) in [
                    ("clientId", Field::Text),
                    ("clientSecret", Field::Secret),
                    ("callbackPort", Field::Number),
                    ("callbackUrl", Field::Text),
                    ("scope", Field::Text),
                    ("clientName", Field::Text),
                    ("clientRegistration", Field::Choice(&["dcr", "cimd"])),
                    ("authServerMetadataUrl", Field::Text),
                ] {
                    body = body.child(self.field(vec!["oauth".into(), k.into()], f, window, cx));
                }
                if !self.project {
                    body = body.child(self.field(
                        vec!["auth".into(), "provider".into()],
                        Field::Text,
                        window,
                        cx,
                    ));
                }
            }
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryEditor, Field};
    use gpui_kit::component::input::InputEvent;
    use gpui_kit::{AppContext as _, TestAppContext};
    use gupi_resources::pi_config_files::{File, Kind};
    #[gpui_kit::test]
    fn original_credential_never_enters_input_and_replacement_uses_real_subscription(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");
        std::fs::write(
            &path,
            r#"{"providers":{"p":{"apiKey":"fixture-original"}}}"#,
        )
        .unwrap();
        let file = File::load(Kind::Models, &path).unwrap();
        let entry = file.entry("p").unwrap();
        let mut editor = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let entity = cx.new(|cx| {
                EntryEditor::new(Kind::Models, "p".into(), entry, true, false, window, cx)
            });
            editor = Some(entity.clone());
            gpui_kit::component::Root::new(entity, window, cx)
        });
        let editor = editor.unwrap();
        let path = vec!["apiKey".to_owned()];
        let input = cx.update(|window, cx| {
            editor.update(cx, |this, cx| {
                let _ = this.input(&path, Field::Secret, window, cx);
                this.text[&path].state.clone()
            })
        });
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            ""
        );
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.set_value("fixture-replacement", window, cx);
                cx.emit(InputEvent::Change)
            })
        });
        cx.run_until_parked();
        assert!(editor.read_with(cx, |this, _| !this.entry.same(&this.original)));
        editor.update(cx, |this, cx| {
            this.entry
                .restore_credential(&this.original, &path)
                .unwrap();
            this.clear_controls(&path);
            cx.notify()
        });
        assert!(editor.read_with(cx, |this, _| this.entry.same(&this.original)));
    }
    #[gpui_kit::test]
    fn model_deletion_migrates_invalid_text_and_new_credentials(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");
        std::fs::write(&path,r#"{"providers":{"p":{"models":[{"id":"first"},{"id":"second","headers":{"Key":"original"}}]}}}"#).unwrap();
        let file = File::load(Kind::Models, &path).unwrap();
        let mut editor = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let e = cx.new(|cx| {
                EntryEditor::new(
                    Kind::Models,
                    "p".into(),
                    file.entry("p").unwrap(),
                    true,
                    false,
                    window,
                    cx,
                )
            });
            editor = Some(e.clone());
            gpui_kit::component::Root::new(e, window, cx)
        });
        let editor = editor.unwrap();
        let credential = vec!["models".into(), "1".into(), "headers".into(), "Key".into()];
        let number = vec!["models".into(), "1".into(), "maxTokens".into()];
        let (secret, input) = cx.update(|window, cx| {
            editor.update(cx, |this, cx| {
                this.secrets.insert(credential.clone(), 1);
                let _ = this.input(&credential, Field::Secret, window, cx);
                let _ = this.input(&number, Field::Number, window, cx);
                (
                    this.text[&credential].state.clone(),
                    this.text[&number].state.clone(),
                )
            })
        });
        cx.update(|window, cx| {
            secret.update(cx, |input, cx| {
                input.set_value("new-only", window, cx);
                cx.emit(InputEvent::Change)
            });
            input.update(cx, |input, cx| {
                input.set_value("-", window, cx);
                cx.emit(InputEvent::Change)
            });
        });
        cx.run_until_parked();
        editor.update(cx, |this, _| this.change_array(&["models".into()], 0, None));
        let credential = vec!["models".into(), "0".into(), "headers".into(), "Key".into()];
        let number = vec!["models".into(), "0".into(), "maxTokens".into()];
        cx.update(|window, cx| {
            editor.update(cx, |this, cx| {
                let _ = this.input(&credential, Field::Secret, window, cx);
                let _ = this.input(&number, Field::Number, window, cx);
                assert_eq!(
                    this.text[&credential].state.read(cx).value().as_str(),
                    "new-only"
                );
                assert_eq!(this.text[&number].state.read(cx).value().as_str(), "-");
                assert!(this.prepare(cx).is_err());
            })
        });
    }
}
