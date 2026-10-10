//! Scoped Pi `settings.json` files: reading, leaf edits and trust records.
//!
//! Edits are applied to the latest file inside Pi's lock and touch only the
//! requested leaves, so unknown fields and concurrent edits elsewhere survive.
mod lock;
pub mod trust;

use crate::persistence;
use lock::Lock;
use serde_json::Map;
use serde_json::Value;
use std::io;
use std::path::Path;
use std::path::PathBuf;

/// A dotted path to one JSON leaf, such as `["compaction", "enabled"]`.
pub type Leaf = &'static [&'static str];

/// Which `settings.json` a page edits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    /// A Pi working directory, not necessarily a Git root.
    Project(PathBuf),
}

impl Scope {
    /// Validates project targets without creating anything.
    pub fn validated(&self) -> Result<Self, Error> {
        match self {
            Self::Global => Ok(Self::Global),
            Self::Project(cwd) => trust::existing_directory(cwd).map(Self::Project),
        }
    }

    /// Revalidates the project at write time. Only `.pi` may be created;
    /// a missing project directory must never be recreated by saving settings.
    pub fn commit(&self, agent: &Path, changes: &[Change]) -> Result<Document, Error> {
        let scope = self.validated()?;
        let path = scope.file(agent);
        if let Self::Project(cwd) = scope {
            let parent = cwd.join(".pi");
            match std::fs::create_dir(&parent) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(Error::Write(format!("{}: {e}", parent.display()))),
            }
            let lock = Lock::acquire_existing_parent(&path).map_err(|e| lock_error(&path, e))?;
            commit_locked(&path, changes, lock)
        } else {
            commit(&path, changes)
        }
    }

    pub fn file(&self, agent: &Path) -> PathBuf {
        match self {
            Self::Global => agent.join("settings.json"),
            Self::Project(cwd) => cwd.join(".pi").join("settings.json"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The file exists but cannot be read or is not a JSON object.
    #[error("{0}")]
    Read(String),
    /// Another process holds the file's lock.
    #[error("{0}")]
    Locked(String),
    /// These leaves changed on disk since they were read.
    #[error("settings changed on disk")]
    Conflict(Vec<Leaf>),
    #[error("{0}")]
    Write(String),
}

/// The raw contents of one settings file; a missing file reads as empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    exists: bool,
    root: Map<String, Value>,
}

impl Document {
    /// An existing file with these contents.
    pub fn from_object(root: Map<String, Value>) -> Self {
        Self { exists: true, root }
    }
    pub fn exists(&self) -> bool {
        self.exists
    }
    pub fn get(&self, leaf: Leaf) -> Option<&Value> {
        let (last, parents) = leaf.split_last()?;
        let mut object = &self.root;
        for key in parents {
            object = object.get(*key)?.as_object()?;
        }
        object.get(*last)
    }
    pub fn root(&self) -> &Map<String, Value> {
        &self.root
    }
}

/// One requested leaf edit. `base` is the value the edit was made against;
/// `next` of `None` deletes the leaf so the value is inherited again.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    leaf: Leaf,
    base: Option<Value>,
    next: Option<Value>,
}

impl Change {
    pub fn new(leaf: Leaf, base: Option<Value>, next: Option<Value>) -> Self {
        Self { leaf, base, next }
    }
    pub fn leaf(&self) -> Leaf {
        self.leaf
    }
}

pub fn load(path: &Path) -> Result<Document, Error> {
    let read = |error: String| Error::Read(format!("{}: {error}", path.display()));
    let Some(bytes) = persistence::read(path).map_err(|e| read(e.to_string()))? else {
        return Ok(Document::default());
    };
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    match serde_json::from_slice(bytes).map_err(|e| read(e.to_string()))? {
        Value::Object(root) => Ok(Document { exists: true, root }),
        _ => Err(read("expected a JSON object".into())),
    }
}

/// Applies `changes` to the latest file. Nothing is written when any leaf
/// differs from its `base`; the conflicting leaves are returned instead.
pub fn commit(path: &Path, changes: &[Change]) -> Result<Document, Error> {
    let lock = Lock::acquire(path).map_err(|e| lock_error(path, e))?;
    commit_locked(path, changes, lock)
}

fn commit_locked(path: &Path, changes: &[Change], mut lock: Lock) -> Result<Document, Error> {
    let mut document = load(path)?;
    let conflicts: Vec<Leaf> = changes
        .iter()
        .filter(|change| document.get(change.leaf) != change.base.as_ref())
        .map(|change| change.leaf)
        .collect();
    if !conflicts.is_empty() {
        return Err(Error::Conflict(conflicts));
    }
    if changes.is_empty() {
        return Ok(document);
    }
    for change in changes {
        match &change.next {
            Some(value) => set(&mut document.root, change.leaf, value.clone())?,
            None => remove(&mut document.root, change.leaf),
        }
    }
    write(path, &mut lock, &document.root)?;
    document.exists = true;
    Ok(document)
}

/// Rewrites the file under Pi's lock; `change` sees the latest contents.
pub fn update(
    path: &Path,
    change: impl FnOnce(&mut Map<String, Value>) -> Result<(), String>,
) -> Result<(), Error> {
    let mut lock = Lock::acquire(path).map_err(|e| lock_error(path, e))?;
    let mut document = load(path)?;
    change(&mut document.root).map_err(Error::Write)?;
    write(path, &mut lock, &document.root)
}

fn write(path: &Path, lock: &mut Lock, root: &Map<String, Value>) -> Result<(), Error> {
    let mut bytes = serde_json::to_vec_pretty(root).map_err(|e| Error::Write(e.to_string()))?;
    bytes.push(b'\n');
    lock.write_atomic(path, &bytes)
        .map_err(|e| lock_error(path, e))
}

fn lock_error(path: &Path, error: io::Error) -> Error {
    if error.kind() == io::ErrorKind::WouldBlock {
        Error::Locked(path.display().to_string())
    } else {
        Error::Write(format!("{}: {error}", path.display()))
    }
}

fn set(root: &mut Map<String, Value>, leaf: Leaf, value: Value) -> Result<(), Error> {
    let Some((last, parents)) = leaf.split_last() else {
        return Ok(());
    };
    let mut object = root;
    for key in parents {
        object = object
            .entry(*key)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| Error::Write(format!("{} is not an object", leaf.join("."))))?;
    }
    object.insert((*last).to_owned(), value);
    Ok(())
}

/// Deletes a leaf and any parent objects that the deletion left empty.
fn remove(object: &mut Map<String, Value>, leaf: Leaf) {
    match leaf {
        [] => {}
        [last] => {
            object.shift_remove(*last);
        }
        [first, rest @ ..] => {
            if let Some(child) = object.get_mut(*first).and_then(Value::as_object_mut) {
                let had = !child.is_empty();
                remove(child, rest);
                if had && child.is_empty() {
                    object.shift_remove(*first);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "pi_settings/tests.rs"]
mod tests;
