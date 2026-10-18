//! Pi credentials: only metadata and opaque conflict revisions leave this module.
mod check;
use crate::pi_lock::Lock;
pub use check::{Check, CheckReason, CheckStatus, check};
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("could not read credentials")]
    Read,
    #[error("invalid credentials file")]
    Parse,
    #[error("credentials are busy")]
    Busy,
    #[error("credentials changed on disk")]
    Conflict,
    #[error("could not write credentials")]
    Write,
    #[error("invalid provider or empty API key")]
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    ApiKey,
    OAuth,
    Unknown,
    Missing,
}
/// Opaque, never displayed, logged or serialized.
#[derive(Clone, PartialEq, Eq)]
pub struct Revision(Option<[u8; 32]>);
#[derive(Clone)]
pub struct Entry {
    provider: String,
    kind: Kind,
    expires: Option<f64>,
    has_env: bool,
    revision: Revision,
}
impl Entry {
    pub fn provider(&self) -> &str {
        &self.provider
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn expires(&self) -> Option<f64> {
        self.expires
    }
    pub fn has_env(&self) -> bool {
        self.has_env
    }
    pub fn revision(&self) -> Revision {
        self.revision.clone()
    }
}
#[derive(Clone)]
pub struct File {
    path: PathBuf,
    entries: Vec<Entry>,
}
fn revision(value: Option<&Value>) -> Revision {
    Revision(value.map(|v| Sha256::digest(serde_json::to_vec(v).expect("JSON value")).into()))
}
fn read(path: &Path) -> Result<Map<String, Value>, Error> {
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(_) => return Err(Error::Read),
    };
    match serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|_| Error::Parse)? {
        Value::Object(v) => Ok(v),
        _ => Err(Error::Parse),
    }
}
fn project(provider: &str, value: Option<&Value>) -> Entry {
    let kind = match value.and_then(|v| v.get("type")).and_then(Value::as_str) {
        Some("api_key") => Kind::ApiKey,
        Some("oauth") => Kind::OAuth,
        _ if value.is_none() => Kind::Missing,
        _ => Kind::Unknown,
    };
    Entry {
        provider: provider.into(),
        kind,
        expires: if kind == Kind::OAuth {
            value
                .and_then(|v| v.get("expires"))
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v >= 0.)
        } else {
            None
        },
        has_env: kind == Kind::ApiKey && value.is_some_and(|v| v.get("env").is_some()),
        revision: revision(value),
    }
}
/// Manual input is a nonempty single line; existing IDs remain manageable verbatim.
pub fn valid_provider(provider: &str) -> bool {
    !provider.is_empty()
        && !provider
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}
/// Pi trims CLI/TUI references, so not every stored ID is addressable.
pub fn addressable_provider(provider: &str) -> bool {
    valid_provider(provider) && provider.trim() == provider
}

impl File {
    pub fn load(path: &Path) -> Result<Self, Error> {
        // Normalize without resolving symlinks, matching Pi's realpath:false lock.
        let absolute = std::path::absolute(path).map_err(|_| Error::Read)?;
        let mut path = PathBuf::new();
        for part in absolute.components() {
            match part {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    path.pop();
                }
                part => path.push(part.as_os_str()),
            }
        }
        let root = read(&path)?;
        Ok(Self {
            path,
            entries: root.iter().map(|(p, v)| project(p, Some(v))).collect(),
        })
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn entry(&self, provider: &str) -> Entry {
        self.entries
            .iter()
            .find(|e| e.provider == provider)
            .cloned()
            .unwrap_or_else(|| project(provider, None))
    }
    /// Immediate single-provider edit; `None` deletes the complete credential.
    pub fn commit(
        &self,
        provider: &str,
        expected: &Revision,
        key: Option<&str>,
    ) -> Result<Self, Error> {
        if provider.is_empty() || key == Some("") {
            return Err(Error::Invalid);
        }
        let parent = self.path.parent().ok_or(Error::Write)?;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            builder.mode(0o700);
        }
        builder.create(parent).map_err(|_| Error::Write)?;
        ensure_file(&self.path)?;
        let mut lock = Lock::acquire_existing_parent(&self.path).map_err(lock_error)?;
        let mut root = read(&self.path)?;
        if revision(root.get(provider)) != *expected {
            return Err(Error::Conflict);
        }
        if let Some(key) = key {
            let entry = root.entry(provider.to_owned()).or_insert(Value::Null);
            if entry.get("type").and_then(Value::as_str) != Some("api_key") {
                *entry = serde_json::json!({"type":"api_key"});
            }
            entry
                .as_object_mut()
                .ok_or(Error::Parse)?
                .insert("key".into(), Value::String(key.into()));
        } else {
            root.shift_remove(provider);
        }
        let bytes = serde_json::to_vec_pretty(&root).map_err(|_| Error::Write)?;
        // Open without truncation first; renew immediately before destructive I/O.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .map_err(|_| Error::Write)?;
        lock.renew().map_err(lock_error)?;
        file.set_len(0)
            .and_then(|_| file.write_all(&bytes))
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Write)?;
        Ok(Self {
            path: self.path.clone(),
            entries: root.iter().map(|(p, v)| project(p, Some(v))).collect(),
        })
    }
}
fn ensure_file(path: &Path) -> Result<(), Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => file
            .write_all(b"{}")
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Write),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(_) => Err(Error::Write),
    }
}
fn lock_error(e: io::Error) -> Error {
    if e.kind() == io::ErrorKind::WouldBlock {
        Error::Busy
    } else {
        Error::Write
    }
}
#[cfg(test)]
mod tests;
