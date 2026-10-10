//! Personal and project Pi resources. Never imports extension code.
mod discovery;
mod packages;
pub use packages::package_identity;
pub use packages::project_package_action;
pub use packages::project_package_allowed;
#[cfg(test)]
mod tests;

use crate::persistence;
use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

pub use discovery::scan;
pub use discovery::scan_project;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Extension,
    Skill,
    Prompt,
    Theme,
}
impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Extension => "extensions",
            Self::Skill => "skills",
            Self::Prompt => "prompts",
            Self::Theme => "themes",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Resource {
    pub kind: Kind,
    pub path: PathBuf,
    pub base: PathBuf,
    pub package: Option<String>,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub editable: bool,
    project_owned: bool,
    /// The name Pi resolves same-name collisions by, when it can be determined:
    /// a prompt's file name, or a loadable skill's frontmatter or folder name.
    pub key: Option<String>,
}
impl Resource {
    /// Resolved file ownership captured by the project scan, independent of editability.
    pub fn is_project_owned(&self) -> bool {
        self.project_owned
    }

    pub fn new(kind: Kind, path: PathBuf, base: PathBuf, name: String) -> Self {
        Self {
            kind,
            path,
            base,
            name,
            package: None,
            description: String::new(),
            enabled: true,
            editable: false,
            project_owned: false,
            key: None,
        }
    }
}

/// The file contents an editor was opened with, checked again before saving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Baseline {
    Missing,
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Package {
    pub source: String,
    pub path: PathBuf,
    pub version: Option<String>,
    delta: bool,
}
impl Package {
    pub fn is_delta(&self) -> bool {
        self.delta
    }

    pub fn new(source: String, path: PathBuf) -> Self {
        Self {
            source,
            path,
            version: None,
            delta: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Catalog {
    pub root: PathBuf,
    pub packages: Vec<Package>,
    pub resources: Vec<Resource>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, thiserror::Error)]
#[error("{0}")]
#[non_exhaustive]
pub struct Error(pub String);
impl Error {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    /// Whether a save stopped because the file changed after it was opened.
    pub fn is_changed(&self) -> bool {
        self.0 == CHANGED
    }
}
const CHANGED: &str = "The file changed after it was opened";

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
pub fn agent_dir() -> Result<PathBuf, Error> {
    // Share the session catalog's environment and relative-path convention.
    Ok(crate::paths::Discovery::environment()?.agent)
}
pub(super) fn resolve(base: &Path, text: &str) -> PathBuf {
    let path = if text == "~" {
        dirs_next::home_dir().unwrap_or_else(|| base.to_owned())
    } else if let Some(path) = text.strip_prefix("~/") {
        dirs_next::home_dir()
            .unwrap_or_else(|| base.to_owned())
            .join(path)
    } else {
        base.join(text)
    };
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            part => normalized.push(part.as_os_str()),
        }
    }
    normalized
}
pub(super) fn read_json(path: &Path) -> Result<Value, Error> {
    match persistence::read(path)? {
        None => Ok(json!({})),
        Some(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| Error(format!("{}: {e}", path.display())))
        }
    }
}
pub(super) fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn write_settings(
    root: &Path,
    change: impl FnOnce(&mut Value) -> Result<(), Error>,
) -> Result<(), Error> {
    let path = crate::pi_settings::Scope::Global.file(root);
    let mut failure = None;
    // Shares Pi's lock and re-reads the latest file, like the settings pages.
    let result = crate::pi_settings::update(&path, |object| {
        let mut value = Value::Object(std::mem::take(object));
        let outcome = change(&mut value);
        if let Value::Object(changed) = value {
            *object = changed;
        }
        outcome.map_err(|error| {
            let message = error.0.clone();
            failure = Some(error);
            message
        })
    });
    match (result, failure) {
        (_, Some(error)) => Err(error),
        (Ok(()), None) => Ok(()),
        (Err(error), None) => Err(Error(error.to_string())),
    }
}
pub fn set_enabled(root: &Path, resource: &Resource, enabled: bool) -> Result<(), Error> {
    write_settings(root, |settings| {
        let target = if let Some(source) = &resource.package {
            let packages = settings["packages"]
                .as_array_mut()
                .ok_or_else(|| Error("Package no longer configured".into()))?;
            let entry = packages
                .iter_mut()
                .find(|v| v.as_str().or_else(|| v["source"].as_str()) == Some(source))
                .ok_or_else(|| Error("Package no longer configured".into()))?;
            if entry.is_string() {
                *entry = json!({"source": source});
            }
            entry
        } else {
            settings
        };
        let key = resource.kind.key();
        let mut patterns = strings(&target[key]);
        // An explicitly empty package filter means all disabled, unlike an absent filter.
        if resource.package.is_some()
            && target[key].as_array().is_some_and(Vec::is_empty)
            && target["autoload"] != false
        {
            patterns.push("!**".into());
        }
        patterns.retain(|p| {
            !p.strip_prefix(['+', '-'])
                .is_some_and(|p| discovery::exact(&resource.path, p, &resource.base))
        });
        let path = resource
            .path
            .strip_prefix(&resource.base)
            .unwrap_or(&resource.path)
            .to_string_lossy()
            .replace('\\', "/");
        patterns.push(format!("{}{path}", if enabled { '+' } else { '-' }));
        target[key] = json!(patterns);
        Ok(())
    })
}
pub fn register_skill(root: &Path, path: &Path) -> Result<(), Error> {
    if !path.is_absolute() || !path.exists() {
        return Err(Error(
            "Choose an existing absolute file or directory".into(),
        ));
    }
    write_settings(root, |settings| {
        let mut paths = strings(&settings["skills"]);
        let path = path.to_string_lossy().into_owned();
        if !paths.contains(&path) {
            paths.push(path);
        }
        settings["skills"] = json!(paths);
        Ok(())
    })
}
pub fn create_path(root: &Path, kind: Kind, name: &str) -> Result<PathBuf, Error> {
    let name = name.trim();
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        || name.ends_with('.')
    {
        return Err(Error(
            "Use a valid file name without path separators".into(),
        ));
    }
    Ok(match kind {
        Kind::Skill => root.join("skills").join(name).join("SKILL.md"),
        Kind::Prompt => root.join("prompts").join(format!("{name}.md")),
        _ => return Err(Error("Resource type is not editable".into())),
    })
}
/// Saves an editor's text unless the file no longer matches `baseline`.
/// The check detects changes made since opening; it cannot exclude a writer
/// racing between the check and the replace.
pub fn save_text(path: &Path, text: &str, create: bool, baseline: &Baseline) -> Result<(), Error> {
    let current = persistence::read(path)?;
    let unchanged = match baseline {
        Baseline::Missing => current.is_none(),
        Baseline::Text(expected) => current.as_deref() == Some(expected.as_bytes()),
    };
    if !unchanged {
        return Err(Error(CHANGED.into()));
    }
    if create {
        use std::io::Write;
        let parent = path
            .parent()
            .ok_or_else(|| Error("Missing parent".into()))?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(text.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist_noclobber(path)
            .map_err(|e| Error(e.to_string()))?;
    } else {
        persistence::write_atomic(path, text.as_bytes())?;
    }
    Ok(())
}
pub async fn package_action(
    command: PathBuf,
    root: PathBuf,
    action: &'static str,
    source: String,
    env: Vec<(OsString, OsString)>,
) -> Result<(), Error> {
    let source = source.trim();
    if source.is_empty() || source.starts_with('-') {
        return Err(Error("Enter a package source".into()));
    }
    tokio::fs::create_dir_all(&root).await?;
    let mut cmd = tokio::process::Command::new(command);
    cmd.envs(env)
        .env("PI_CODING_AGENT_DIR", &root)
        .current_dir(&root)
        .arg(action);
    if action == "update" {
        cmd.arg("--extension");
    }
    let output = cmd
        .arg(source)
        .arg("--no-approve")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        let message = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        Err(Error(format!(
            "Pi {action} ({}): {}",
            output.status,
            message.chars().take(4000).collect::<String>()
        )))
    }
}

/// The project configuration folder, `<cwd>/.pi`, under the canonical cwd.
pub fn project_root(cwd: &Path) -> PathBuf {
    crate::pi_settings::trust::canonical(cwd).join(".pi")
}

/// Whether `path` resolves inside the project's own `.pi` folder, following
/// symlinks. A path still to be created is judged by its nearest existing
/// ancestor. A `.pi` that is itself a link elsewhere owns nothing.
pub fn project_owns(cwd: &Path, path: &Path) -> bool {
    let root = project_root(cwd);
    let mut existing = path;
    let mut missing = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(real) => {
                let mut real = crate::pi_settings::trust::canonical(&real);
                for part in missing.iter().rev() {
                    real.push(part);
                }
                return real.starts_with(&root) && real != root;
            }
            Err(_) => {
                let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
                    return false;
                };
                if name == ".." {
                    return false;
                }
                missing.push(name.to_owned());
                existing = parent;
            }
        }
    }
}

/// Reuse the discovery parser for one edited file, preserving catalog order.
pub fn reload_resource(catalog: &mut Catalog, resource: Resource) {
    let index = catalog
        .resources
        .iter()
        .position(|r| r.path == resource.path)
        .unwrap_or(catalog.resources.len());
    let prefix = format!("{}:", resource.path.display());
    catalog
        .warnings
        .retain(|warning| !warning.starts_with(&prefix));
    catalog.resources.retain(|r| r.path != resource.path);
    let previous_len = catalog.resources.len();
    let project_owned = resource.project_owned;
    discovery::add(
        catalog,
        resource.kind,
        resource.path,
        resource.base,
        resource.package,
        resource.enabled,
        resource.editable,
    );
    if catalog.resources.len() > previous_len {
        let mut updated = catalog.resources.pop().unwrap();
        updated.project_owned = project_owned;
        catalog
            .resources
            .insert(index.min(catalog.resources.len()), updated);
    }
}
