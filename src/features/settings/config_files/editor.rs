//! Structured entry editor. Original credentials never become control values.
use super::{Entry, Error, Kind, error_key};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
mod collections;
mod layout;
use collections::{Addition, Collection, path_id};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use gupi_resources::pi_config_files::Credential;
use gupi_settings::{assets::IconName, i18n::t};
pub(super) use layout::field_label;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

struct EditorKeys;
impl Global for EditorKeys {}

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
    additions: BTreeMap<Vec<String>, Addition>,
    subview: Option<(Vec<String>, bool)>,
    scroll: ScrollHandle,
    parent_offset: Point<Pixels>,
    focus: FocusHandle,
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
        // Keep Enter local to inputs/buttons instead of the enclosing dialog's
        // "stage the whole entry" shortcut. Input's deeper context still owns Enter.
        if !cx.has_global::<EditorKeys>() {
            cx.bind_keys([KeyBinding::new(
                "enter",
                NoAction,
                Some("PiConfigFileEditor"),
            )]);
            cx.set_global(EditorKeys);
        }
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
            additions: BTreeMap::new(),
            subview: None,
            scroll: ScrollHandle::new(),
            parent_offset: point(px(0.), px(0.)),
            focus: cx.focus_handle(),
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
        let key = path.last().map(String::as_str).unwrap_or_default();
        let label = if path
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|key| matches!(key.as_str(), "headers" | "env"))
        {
            path.join(".")
        } else {
            t(cx, &format!("pi-files-label-{key}"))
        };
        if matches!(kind, Field::Json) {
            if !self.json.contains_key(path) {
                let text = self.invalid.get(path).cloned().unwrap_or_else(|| {
                    self.entry
                        .visible(path)
                        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
                        .unwrap_or_default()
                });
                let placeholder = t(cx, "pi-value-unset");
                let state = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .default_value(text)
                        .placeholder(placeholder)
                        .auto_grow(1, 4)
                });
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
                .font_family(cx.theme().mono_font_family.clone())
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
            let placeholder = t(cx, "pi-value-unset");
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .placeholder(placeholder)
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
}

#[cfg(test)]
mod tests {
    use super::{EntryEditor, Field};
    use gpui_kit::component::input::InputEvent;
    use gpui_kit::{AppContext as _, ParentElement as _, TestAppContext};
    use gupi_resources::pi_config_files::{File, Kind};
    struct Host;
    impl gpui_kit::Render for Host {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            gpui_kit::div()
        }
    }
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
    #[gpui_kit::test]
    fn model_subview_back_and_collapsed_advanced_preserve_entry(cx: &mut TestAppContext) {
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");
        std::fs::write(
            &path,
            r#"{"providers":{"p":{"models":[{"id":"first","compat":{"future":1}}]}}}"#,
        )
        .unwrap();
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
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(editor.read(cx).json.is_empty());
            editor.update(cx, |this, cx| {
                this.open_model(vec!["models".into(), "0".into()], false, window, cx)
            });
            window.render_frame(cx);
            assert!(editor.read(cx).json.is_empty());
            window.click("model-back", cx);
            window.render_frame(cx);
            assert!(editor.read(cx).subview.is_none());
            assert!(editor.read(cx).entry.same(&editor.read(cx).original));
        });
    }
    #[gpui_kit::test]
    fn stdio_inline_list_confirm_and_cancel_do_not_expose_http_fields(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt as _;
        use gpui_kit::test::TestWindowExt as _;
        cx.update(|cx| {
            gpui_kit::init(cx);
            gupi_settings::i18n::apply(gupi_settings::config::AppLanguage::English, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(
            &path,
            r#"{"mcpServers":{"s":{"type":"stdio","command":"never-run","args":["one"]}}}"#,
        )
        .unwrap();
        let file = File::load(Kind::Mcp, &path).unwrap();
        let mut editor = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let e = cx.new(|cx| {
                EntryEditor::new(
                    Kind::Mcp,
                    "s".into(),
                    file.entry("s").unwrap(),
                    true,
                    false,
                    window,
                    cx,
                )
            });
            editor = Some(e.clone());
            let host = cx.new(|_| Host);
            gpui_kit::component::Root::new(host, window, cx)
        });
        let editor = editor.unwrap();
        cx.update(|window, cx| {
            let content = editor.clone();
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .entrance(gpui_kit::component::dialog::DialogEntrance::None)
                    .content({
                        let content = content.clone();
                        move |body, _, _| body.child(content.clone())
                    })
            });
            window.render_frame(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(!editor.read(cx).text.contains_key(&vec!["url".into()]));
            window.click(gpui_kit::SharedString::from("add-[\"args\"]"), cx);
            window.render_frame(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx));
            assert!(
                editor.read(cx).additions[&vec!["args".into()]]
                    .focus
                    .is_focused(window)
            );
        });
        assert!(editor.read_with(cx, |this, _| this.entry.same(&this.original)));
        let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        cx.simulate_input("two");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx));
            assert!(
                editor.read(cx).additions[&vec!["args".into()]]
                    .focus
                    .is_focused(window)
            );
        });
        assert_eq!(
            editor.read_with(cx, |this, _| this.entry.visible(&["args".into()])),
            Some(serde_json::json!(["one", "two"]))
        );
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }
}
