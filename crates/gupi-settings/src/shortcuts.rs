use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct Shortcuts {
    pub launcher: String,
    pub tasks: Vec<ShortcutTask>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct ShortcutTask {
    pub id: String,
    pub name: String,
    pub binding: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub template: PathBuf,
    pub source: InputSource,
    pub model: Option<ModelChoice>,
    pub thinking: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelChoice {
    pub provider: String,
    pub id: String,
}
impl ModelChoice {
    pub fn new(provider: String, id: String) -> Self {
        Self { provider, id }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    Selection,
    Clipboard,
    #[default]
    SelectionOrClipboard,
}
fn enabled() -> bool {
    true
}
impl Shortcuts {
    pub fn clear_bindings(&mut self) {
        self.launcher.clear();
        for task in &mut self.tasks {
            task.binding.clear();
        }
    }
    pub fn has_bindings(&self) -> bool {
        !self.launcher.is_empty()
            || self
                .tasks
                .iter()
                .any(|t| t.enabled && !t.binding.is_empty())
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        let mut bindings = BTreeSet::new();
        if !self.launcher.is_empty() {
            bindings.insert(system_binding(&self.launcher)?);
        }
        for task in &self.tasks {
            if task.id.is_empty()
                || task.name.trim().is_empty()
                || task.template.as_os_str().is_empty()
                || !ids.insert(&task.id)
            {
                return Err("error-config-validation".into());
            }
            if !task.binding.is_empty() {
                // Retain valid saved bindings on disabled tasks without reserving them.
                let normalized = system_binding(&task.binding)?;
                if task.enabled && !bindings.insert(normalized) {
                    return Err("settings-key-conflict".into());
                }
            }
        }
        Ok(())
    }
}
/// Keep the same GPUI syntax as the existing inline shortcut editor.
pub fn system_binding(binding: &str) -> Result<String, String> {
    crate::keybindings::syntax(binding).map_err(str::to_owned)?;
    let key = gpui_kit::Keystroke::parse(binding).map_err(|_| "settings-key-invalid".to_owned())?;
    if key.modifiers.function
        || !(key.modifiers.control || key.modifiers.platform || key.modifiers.alt)
    {
        return Err("settings-key-invalid".into());
    }
    let mut parts = Vec::new();
    if key.modifiers.control {
        parts.push("Control".to_owned());
    }
    if key.modifiers.platform {
        parts.push("Super".to_owned());
    }
    if key.modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if key.modifiers.shift {
        parts.push("Shift".to_owned());
    }
    parts.sort();
    parts.dedup();
    parts.push(
        match key.key.as_str() {
            "-" => "Minus",
            "=" => "Equal",
            "/" => "Slash",
            "," => "Comma",
            "." => "Period",
            ";" => "Semicolon",
            "'" => "Quote",
            "[" => "BracketLeft",
            "]" => "BracketRight",
            "\\" => "Backslash",
            "`" => "Backquote",
            "enter" => "Enter",
            "escape" => "Escape",
            "space" => "Space",
            "backspace" => "Backspace",
            "up" => "ArrowUp",
            "down" => "ArrowDown",
            "left" => "ArrowLeft",
            "right" => "ArrowRight",
            other => other,
        }
        .to_owned(),
    );
    let value = parts.join("+");
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        value
            .parse::<global_hotkey::hotkey::HotKey>()
            .map(|key| key.to_string())
            .map_err(|_| "settings-key-invalid".to_owned())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_only_removes_bindings_and_duplicate_ids_are_invalid() {
        let task = ShortcutTask {
            id: "a".into(),
            name: "Translate".into(),
            binding: "ctrl-alt-t".into(),
            enabled: true,
            template: "/tmp/translate.md".into(),
            ..Default::default()
        };
        let mut config = Shortcuts {
            launcher: "ctrl-alt-space".into(),
            tasks: vec![task.clone()],
        };
        assert!(config.validate().is_ok());
        config.clear_bindings();
        assert!(!config.has_bindings());
        assert_eq!(config.tasks[0].name, task.name);
        assert!(config.tasks[0].enabled);
        config.tasks.push(task);
        assert!(config.validate().is_err());
    }
    #[test]
    fn system_shortcut_requires_a_modifier_and_has_one_canonical_form() {
        assert!(system_binding("x").is_err());
        assert!(system_binding("ctrl-x ctrl-y").is_err());
        assert_eq!(
            system_binding("ctrl-alt-x").unwrap(),
            system_binding("alt-ctrl-x").unwrap()
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            system_binding("secondary-x").unwrap(),
            system_binding("cmd-x").unwrap()
        );
    }
    #[test]
    fn disabled_tasks_release_bindings_and_reenable_checks_conflicts() {
        let task = ShortcutTask {
            id: "disabled".into(),
            name: "Disabled".into(),
            binding: "ctrl-alt-t".into(),
            enabled: false,
            template: "/test.md".into(),
            ..Default::default()
        };
        let mut config = Shortcuts {
            launcher: String::new(),
            tasks: vec![task.clone()],
        };
        assert!(config.validate().is_ok());
        assert!(!config.has_bindings());
        config.launcher = "alt-ctrl-t".into();
        assert!(config.validate().is_ok());
        config.tasks[0].enabled = true;
        assert_eq!(config.validate().unwrap_err(), "settings-key-conflict");

        config.tasks[0].enabled = false;
        config.launcher.clear();
        config.tasks.push(ShortcutTask {
            id: "enabled".into(),
            enabled: true,
            ..task
        });
        assert!(config.validate().is_ok());
        assert!(config.has_bindings());
        config.tasks[0].enabled = true;
        assert_eq!(config.validate().unwrap_err(), "settings-key-conflict");
        config.tasks[1].enabled = false;
        assert!(config.validate().is_ok());
        assert_eq!(config.tasks[1].binding, "ctrl-alt-t");
        config.tasks[1].binding = "invalid-unmodified-key".into();
        assert!(
            config.validate().is_err(),
            "disabled bindings must still have valid syntax"
        );
    }
}
