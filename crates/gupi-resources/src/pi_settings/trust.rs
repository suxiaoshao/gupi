//! Pi's project trust records in `<agent-dir>/trust.json`.
//!
//! Keys are canonical absolute directories and values are `true`, `false` or
//! `null`. Pi uses the nearest ancestor with a boolean decision. Gupi only
//! ever records `true` for the exact directory the user confirmed.
use super::Error;
use super::lock::Lock;
use crate::persistence;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

/// The stored trust decision that applies to a directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Trusted by the record for this directory or an ancestor.
    Trusted(PathBuf),
    /// Distrusted by the record for this directory or an ancestor.
    Distrusted(PathBuf),
    /// No record applies; Pi falls back to `defaultProjectTrust`.
    Unset,
}

pub fn file(agent: &Path) -> PathBuf {
    agent.join("trust.json")
}

/// The directory key Pi uses: the real path, or the input when it cannot be
/// resolved.
pub fn canonical(cwd: &Path) -> PathBuf {
    let Ok(path) = std::fs::canonicalize(cwd) else {
        return cwd.to_owned();
    };
    #[cfg(windows)]
    if let Some(plain) = path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        return PathBuf::from(plain);
    }
    path
}

pub fn decision(agent: &Path, cwd: &Path) -> Result<Decision, Error> {
    let records = read(&file(agent))?;
    let mut current = Some(canonical(cwd));
    while let Some(dir) = current {
        match records.get(dir.to_string_lossy().as_ref()) {
            Some(Value::Bool(true)) => return Ok(Decision::Trusted(dir)),
            Some(Value::Bool(false)) => return Ok(Decision::Distrusted(dir)),
            _ => current = dir.parent().map(Path::to_path_buf),
        }
    }
    Ok(Decision::Unset)
}

/// Records `true` for exactly `cwd`. Ancestors, other records and the global
/// default are left untouched. Returns the recorded directory.
pub fn trust(agent: &Path, cwd: &Path) -> Result<PathBuf, Error> {
    // Pi keys records by real path; record only a folder that exists now.
    let dir = existing_directory(cwd)?;
    let path = file(agent);
    let mut lock = Lock::acquire(&path).map_err(|e| super::lock_error(&path, e))?;
    let mut records = read(&path)?;
    records.insert(dir.to_string_lossy().into_owned(), Value::Bool(true));
    // Pi writes the records sorted by directory.
    let mut entries: Vec<_> = records.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let sorted: Map<String, Value> = entries.into_iter().collect();
    super::write(&path, &mut lock, &sorted)?;
    Ok(dir)
}

fn existing_directory(cwd: &Path) -> Result<PathBuf, Error> {
    let invalid = |reason: &str| Error::Write(format!("{}: {reason}", cwd.display()));
    if !cwd.is_absolute() {
        return Err(invalid("not an absolute path"));
    }
    std::fs::canonicalize(cwd).map_err(|e| invalid(&e.to_string()))?;
    let dir = canonical(cwd);
    if !dir.is_dir() {
        return Err(invalid("not a folder"));
    }
    Ok(dir)
}

fn read(path: &Path) -> Result<Map<String, Value>, Error> {
    let invalid = |error: String| Error::Read(format!("{}: {error}", path.display()));
    let Some(bytes) = persistence::read(path).map_err(|e| invalid(e.to_string()))? else {
        return Ok(Map::new());
    };
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let Value::Object(records) =
        serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?
    else {
        return Err(invalid("expected an object".into()));
    };
    if records
        .values()
        .any(|value| !matches!(value, Value::Bool(_) | Value::Null))
    {
        return Err(invalid("values must be true, false or null".into()));
    }
    Ok(records)
}
