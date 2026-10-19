//! Faithful models.json and mcp.json editing. No authentication or server I/O.
//! Raw documents and edits deliberately implement no Debug or serialization API.
use crate::{persistence, pi_settings};
use pi_settings::{Change, Document, Scope, ordered_eq};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Models,
    Mcp,
}
impl Kind {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Models => "models.json",
            Self::Mcp => "mcp.json",
        }
    }
    pub fn collection(self) -> &'static str {
        match self {
            Self::Models => "providers",
            Self::Mcp => "mcpServers",
        }
    }
    pub fn file(self, agent: &Path, scope: &Scope) -> PathBuf {
        match (self, scope) {
            (Self::Mcp, Scope::Project(cwd)) => cwd.join(".pi").join(self.filename()),
            _ => agent.join(self.filename()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("could not read configuration file")]
    Read,
    #[error("invalid configuration JSON object")]
    Parse,
    #[error("configuration entry changed on disk")]
    Conflict,
    #[error("could not write configuration file")]
    Write,
    #[error("unsupported value shape")]
    Shape,
    #[error("invalid configuration field")]
    Invalid,
    #[error("duplicate configuration name")]
    Duplicate,
    #[error("project directory is unavailable")]
    Project,
}

#[derive(Clone)]
pub struct File {
    kind: Kind,
    document: Document,
    comments: bool,
}
impl File {
    pub fn load(kind: Kind, path: &Path) -> Result<Self, Error> {
        let Some(bytes) = persistence::read(path).map_err(|_| Error::Read)? else {
            return Ok(Self {
                kind,
                document: Document::default(),
                comments: false,
            });
        };
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::Parse)?;
        let (source, comments) = match kind {
            Kind::Models => clean_models(text),
            Kind::Mcp => (text.to_owned(), false),
        };
        let Value::Object(root) = serde_json::from_str(&source).map_err(|_| Error::Parse)? else {
            return Err(Error::Parse);
        };
        Ok(Self {
            kind,
            document: Document::from_object(root),
            comments,
        })
    }
    pub fn comments(&self) -> bool {
        self.comments
    }
    pub fn names(&self) -> Result<Vec<String>, Error> {
        match self.document.get(&[self.kind.collection()]) {
            None => Ok(vec![]),
            Some(Value::Object(map)) => Ok(map.keys().cloned().collect()),
            _ => Err(Error::Shape),
        }
    }
    pub fn entry(&self, name: &str) -> Option<Entry> {
        self.document
            .get(&[self.kind.collection(), name])
            .map(|value| Entry {
                kind: self.kind,
                value: value.clone(),
            })
    }
    /// Saved-definition OAuth availability only. Never evaluates values or reads
    /// Pi's MCP credential store; Pi remains the final configuration validator.
    pub fn mcp_oauth_eligibility(
        &self,
        name: &str,
        global: Option<&Self>,
        scope: &Scope,
        trusted: bool,
    ) -> crate::pi_mcp::Eligibility {
        use crate::pi_mcp::Eligibility;
        if !crate::pi_mcp::addressable_name(name) {
            return Eligibility::UnsupportedName;
        }
        if matches!(scope, Scope::Project(_)) && !trusted {
            return Eligibility::UntrustedProject;
        }
        if self.kind != Kind::Mcp {
            return Eligibility::Ineligible;
        }
        let Some(entry) = self.entry(name) else {
            return Eligibility::Ineligible;
        };
        // Ambiguous namespace definitions are never guessed to be CLI targets.
        let namespace = name.replace('-', "_");
        let Ok(names) = self.names() else {
            return Eligibility::Ineligible;
        };
        if names
            .iter()
            .filter(|other| other.replace('-', "_") == namespace)
            .count()
            != 1
        {
            return Eligibility::Ineligible;
        }
        let project = matches!(scope, Scope::Project(_));
        if project
            && global.is_some_and(|file| {
                file.names().is_ok_and(|names| {
                    names
                        .iter()
                        .any(|other| other != name && other.replace('-', "_") == namespace)
                })
            })
        {
            return Eligibility::Ineligible;
        }
        let eligible = if project && entry.partial() {
            let allowed = entry.value.as_object().is_some_and(|map| {
                map.keys()
                    .all(|key| matches!(key.as_str(), "enabled" | "exposure" | "toolExposure"))
            });
            allowed
                && global
                    .and_then(|file| file.entry(name))
                    .is_some_and(|base| base.mcp_uses_oauth())
        } else {
            entry.mcp_uses_oauth()
        };
        if eligible {
            Eligibility::Eligible
        } else {
            Eligibility::Ineligible
        }
    }
    pub fn auto_enable(&self) -> Option<Value> {
        self.document.get(&["autoEnableCodemode"]).cloned()
    }
    pub fn changed(&self, other: &Self, name: &str) -> bool {
        !ordered_eq(
            self.document.get(&[self.kind.collection(), name]),
            other.document.get(&[self.kind.collection(), name]),
        )
    }
    /// Re-read using the same file-specific parser, then apply independent entry leaves.
    /// Pi does not cooperate with locks for these files; the final read/write gap remains.
    pub fn commit(
        &self,
        agent: &Path,
        scope: &Scope,
        edits: &[(String, Option<Entry>)],
        auto: Option<Option<Value>>,
    ) -> Result<Self, Error> {
        let validated_scope = scope.validated().map_err(|_| Error::Project)?;
        let scope = &validated_scope;
        if matches!(scope, Scope::Project(_)) {
            if self.kind == Kind::Models {
                return Err(Error::Invalid);
            }
            scope.validated().map_err(|_| Error::Project)?;
        }
        let path = self.kind.file(agent, scope);
        let mut next = Self::load(self.kind, &path)?;
        let mut changes: Vec<Change> = edits
            .iter()
            .map(|(name, entry)| {
                Change::new(
                    &[self.kind.collection(), name],
                    self.document.get(&[self.kind.collection(), name]).cloned(),
                    entry.as_ref().map(|e| e.value.clone()),
                )
            })
            .collect();
        if let Some(auto) = auto {
            changes.push(Change::new(
                &["autoEnableCodemode"],
                self.auto_enable(),
                auto,
            ));
        }
        pi_settings::apply_changes(&mut next.document, &changes, true).map_err(
            |error| match error {
                pi_settings::Error::Conflict(_) => Error::Conflict,
                _ => Error::Shape,
            },
        )?;
        if !changes.is_empty() {
            let mut bytes =
                serde_json::to_vec_pretty(next.document.root()).map_err(|_| Error::Write)?;
            bytes.push(b'\n');
            if let Scope::Project(cwd) = scope {
                match std::fs::create_dir(cwd.join(".pi")) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(_) => return Err(Error::Project),
                }
                persistence::write_atomic_existing_parent(&path, &bytes)
                    .map_err(|_| Error::Write)?;
            } else {
                persistence::write_atomic(&path, &bytes).map_err(|_| Error::Write)?;
            }
            next.comments = false;
        }
        Ok(next)
    }
    pub fn check_name(&self, name: &str, global: Option<&Self>) -> Result<(), Error> {
        if name.is_empty() {
            return Err(Error::Invalid);
        }
        let names = self.names()?;
        if names.iter().any(|n| n == name) {
            return Err(Error::Duplicate);
        }
        if self.kind == Kind::Mcp {
            if !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            {
                return Err(Error::Invalid);
            }
            let namespace = name.replace('-', "_");
            if names.iter().any(|n| n.replace('-', "_") == namespace) {
                return Err(Error::Duplicate);
            }
            if let Some(global) = global
                && global
                    .names()?
                    .iter()
                    .any(|n| n != name && n.replace('-', "_") == namespace)
            {
                return Err(Error::Duplicate);
            }
        }
        Ok(())
    }
}

/// An opaque entry: UI readers never return an original credential value.
#[derive(Clone)]
pub struct Entry {
    kind: Kind,
    value: Value,
}
#[derive(Clone)]
pub enum Credential {
    Keep,
    Replace(String),
    Remove,
}
impl Entry {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            value: Value::Object(Map::new()),
        }
    }
    fn mcp_uses_oauth(&self) -> bool {
        let Some(map) = self.value.as_object() else {
            return false;
        };
        if self.kind != Kind::Mcp || map.contains_key("auth") || map.contains_key("command") {
            return false;
        }
        if map
            .get("type")
            .is_some_and(|value| !matches!(value.as_str(), Some("http" | "streamable-http")))
        {
            return false;
        }
        let Some(url) = map
            .get("url")
            .and_then(Value::as_str)
            .and_then(|value| url::Url::parse(value).ok())
        else {
            return false;
        };
        if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
            return false;
        }
        match map.get("headers") {
            None => true,
            Some(Value::Object(headers)) => headers.iter().all(|(key, value)| {
                !key.eq_ignore_ascii_case("authorization") && value.is_string()
            }),
            _ => false,
        }
    }
    pub fn is_object(&self) -> bool {
        self.value.is_object()
    }
    pub fn same(&self, other: &Self) -> bool {
        ordered_eq(Some(&self.value), Some(&other.value))
    }
    pub fn partial(&self) -> bool {
        ["command", "url", "type"]
            .iter()
            .all(|key| self.value.get(key).is_none())
    }
    pub fn disabled(&self) -> bool {
        self.value.get("enabled") == Some(&Value::Bool(false))
    }
    pub fn keys(&self, path: &[String]) -> Result<Vec<String>, Error> {
        match at(&self.value, path) {
            None => Ok(vec![]),
            Some(Value::Object(m)) => Ok(m.keys().cloned().collect()),
            _ => Err(Error::Shape),
        }
    }
    pub fn len(&self, path: &[String]) -> Result<usize, Error> {
        match at(&self.value, path) {
            None => Ok(0),
            Some(Value::Array(a)) => Ok(a.len()),
            _ => Err(Error::Shape),
        }
    }
    pub fn present(&self, path: &[String]) -> bool {
        at(&self.value, path).is_some()
    }
    pub fn visible(&self, path: &[String]) -> Option<Value> {
        // Containers containing credential fields are never exposed as JSON text.
        if sensitive(self.kind, path) || path.is_empty() {
            return None;
        }
        let value = at(&self.value, path)?;
        if contains_secret(self.kind, path, value) {
            return None;
        }
        Some(value.clone())
    }
    pub fn set(&mut self, path: &[String], value: Option<Value>) -> Result<(), Error> {
        if sensitive(self.kind, path) {
            return Err(Error::Invalid);
        }
        put(&mut self.value, path, value)
    }
    /// Restore a credential from the opaque original without exposing it to controls.
    pub fn restore_credential(&mut self, original: &Self, path: &[String]) -> Result<(), Error> {
        if !sensitive(self.kind, path) {
            return Err(Error::Invalid);
        }
        put(&mut self.value, path, at(&original.value, path).cloned())?;
        let parent = &path[..path.len() - 1];
        if let (Some(Value::Object(before)), Some(Value::Object(after))) =
            (at(&original.value, parent), at_mut(&mut self.value, parent))
        {
            let mut ordered = Map::new();
            for key in before.keys() {
                if let Some(value) = after.shift_remove(key) {
                    ordered.insert(key.clone(), value);
                }
            }
            ordered.extend(std::mem::take(after));
            *after = ordered;
        }
        Ok(())
    }
    pub fn credential(&mut self, path: &[String], change: Credential) -> Result<(), Error> {
        if !sensitive(self.kind, path) {
            return Err(Error::Invalid);
        }
        match change {
            Credential::Keep => Ok(()),
            Credential::Replace(value) => put(&mut self.value, path, Some(Value::String(value))),
            Credential::Remove => put(&mut self.value, path, None),
        }
    }
    pub fn append(&mut self, path: &[String], value: Value) -> Result<(), Error> {
        if !self.present(path) {
            self.set(path, Some(Value::Array(vec![])))?;
        }
        at_mut(&mut self.value, path)
            .and_then(Value::as_array_mut)
            .ok_or(Error::Shape)?
            .push(value);
        Ok(())
    }
    pub fn remove_item(&mut self, path: &[String], ix: usize) -> Result<(), Error> {
        let list = at_mut(&mut self.value, path)
            .and_then(Value::as_array_mut)
            .ok_or(Error::Shape)?;
        if ix >= list.len() {
            return Err(Error::Invalid);
        }
        list.remove(ix);
        Ok(())
    }
    pub fn move_item(&mut self, path: &[String], ix: usize, to: usize) -> Result<(), Error> {
        if let Some(Value::Array(list)) = at_mut(&mut self.value, path) {
            if ix >= list.len() || to >= list.len() {
                return Err(Error::Invalid);
            }
            let v = list.remove(ix);
            list.insert(to, v);
            return Ok(());
        }
        if let Some(Value::Object(map)) = at_mut(&mut self.value, path) {
            if ix >= map.len() || to >= map.len() {
                return Err(Error::Invalid);
            }
            let mut pairs: Vec<_> = std::mem::take(map).into_iter().collect();
            let pair = pairs.remove(ix);
            pairs.insert(to, pair);
            map.extend(pairs);
            return Ok(());
        }
        Err(Error::Shape)
    }
    pub fn validate(&self, project: bool) -> Result<(), Error> {
        self.validate_against(project, None)
    }
    /// Unsupported stored optional values survive unrelated edits.
    pub fn validate_edit(&self, original: &Self, project: bool) -> Result<(), Error> {
        self.validate_against(project, Some(original))
    }
    fn validate_against(&self, project: bool, original: Option<&Self>) -> Result<(), Error> {
        let object = self.value.as_object().ok_or(Error::Shape)?;
        if self.kind == Kind::Models {
            if let Some(Value::Array(models)) = object.get("models") {
                for model in models {
                    if model
                        .get("id")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                    {
                        return Err(Error::Invalid);
                    }
                }
            }
        } else if project && self.partial() {
            // Unknown existing fields are preserved. The editor only exposes the permitted fields.
            return Ok(());
        } else {
            if !["command", "url"].iter().any(|k| {
                object
                    .get(*k)
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
            }) {
                return Err(Error::Invalid);
            }
            if project && object.get("auth").and_then(|v| v.get("provider")).is_some() {
                return Err(Error::Invalid);
            }
        }
        validate_numbers(&self.value, original.map(|e| &e.value))?;
        Ok(())
    }
}
fn validate_numbers(value: &Value, original: Option<&Value>) -> Result<(), Error> {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                let before = original.and_then(|v| v.get(key));
                if before.is_some() && ordered_eq(Some(v), before) {
                    continue;
                }
                if ["contextWindow", "maxTokens", "timeout"].contains(&key.as_str())
                    && v.as_f64().is_none_or(|v| v <= 0.)
                {
                    return Err(Error::Invalid);
                }
                if key == "callbackPort" && v.as_u64().is_none_or(|v| !(1..=65535).contains(&v)) {
                    return Err(Error::Invalid);
                }
                // Raw complex objects are deliberately not schema validated.
                if key == "modelOverrides" {
                    if let Some(overrides) = v.as_object() {
                        for (name, value) in overrides {
                            validate_numbers(value, before.and_then(|v| v.get(name)))?;
                        }
                    }
                } else if ["models", "oauth"].contains(&key.as_str()) {
                    validate_numbers(v, before)?;
                }
            }
        }
        Value::Array(list) => {
            for (ix, v) in list.iter().enumerate() {
                validate_numbers(v, original.and_then(|v| v.get(ix)))?
            }
        }
        _ => {}
    }
    Ok(())
}
fn sensitive(kind: Kind, path: &[String]) -> bool {
    let p: Vec<_> = path.iter().map(String::as_str).collect();
    matches!(
        (kind, p.as_slice()),
        (Kind::Models, ["apiKey"])
            | (Kind::Models, ["headers", _])
            | (Kind::Models, ["models", _, "headers", _])
            | (Kind::Models, ["modelOverrides", _, "headers", _])
            | (Kind::Mcp, ["env", _])
            | (Kind::Mcp, ["headers", _])
            | (Kind::Mcp, ["oauth", "clientSecret"])
    )
}
fn contains_secret(kind: Kind, path: &[String], value: &Value) -> bool {
    if sensitive(kind, path) {
        return true;
    }
    match value {
        Value::Object(map) => map.iter().any(|(k, v)| {
            let mut p = path.to_vec();
            p.push(k.clone());
            contains_secret(kind, &p, v)
        }),
        Value::Array(list) => list.iter().enumerate().any(|(ix, v)| {
            let mut p = path.to_vec();
            p.push(ix.to_string());
            contains_secret(kind, &p, v)
        }),
        _ => false,
    }
}
fn at<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut value = root;
    for key in path {
        value = match value {
            Value::Array(a) => a.get(key.parse::<usize>().ok()?)?,
            _ => value.get(key)?,
        };
    }
    Some(value)
}
fn at_mut<'a>(root: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    let mut value = root;
    for key in path {
        value = match value {
            Value::Array(a) => a.get_mut(key.parse::<usize>().ok()?)?,
            _ => value.get_mut(key)?,
        };
    }
    Some(value)
}
fn put(root: &mut Value, path: &[String], value: Option<Value>) -> Result<(), Error> {
    if value.is_none() && at(root, path).is_none() {
        return Ok(());
    }
    let Some((key, parents)) = path.split_last() else {
        return Err(Error::Invalid);
    };
    let mut object = root;
    for part in parents {
        object = match object {
            Value::Object(map) => map
                .entry(part.clone())
                .or_insert_with(|| Value::Object(Map::new())),
            Value::Array(list) => list
                .get_mut(part.parse::<usize>().map_err(|_| Error::Invalid)?)
                .ok_or(Error::Invalid)?,
            _ => return Err(Error::Shape),
        };
    }
    match object {
        Value::Object(map) => {
            if let Some(value) = value {
                map.insert(key.clone(), value);
            } else {
                map.shift_remove(key);
            }
        }
        Value::Array(list) => {
            let ix = key.parse::<usize>().map_err(|_| Error::Invalid)?;
            if ix >= list.len() {
                return Err(Error::Invalid);
            }
            if let Some(value) = value {
                list[ix] = value
            } else {
                list.remove(ix);
            }
        }
        _ => return Err(Error::Shape),
    }
    Ok(())
}

/// Pi's string-aware line-comment and trailing-comma stripping (not JSON5).
fn clean_models(source: &str) -> (String, bool) {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut out = String::with_capacity(source.len());
    let mut quoted = false;
    let mut escape = false;
    let mut comments = false;
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            out.push(ch);
            if escape {
                escape = false
            } else if ch == '\\' {
                escape = true
            } else if ch == '"' {
                quoted = false
            }
            continue;
        }
        if ch == '"' {
            quoted = true;
            out.push(ch)
        } else if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            comments = true;
            for c in chars.by_ref() {
                if c == '\n' {
                    out.push(c);
                    break;
                }
            }
        } else {
            out.push(ch)
        }
    }
    let mut result = String::with_capacity(out.len());
    let chars: Vec<_> = out.chars().collect();
    quoted = false;
    escape = false;
    for (ix, ch) in chars.iter().copied().enumerate() {
        if quoted {
            result.push(ch);
            if escape {
                escape = false
            } else if ch == '\\' {
                escape = true
            } else if ch == '"' {
                quoted = false
            }
            continue;
        }
        if ch == '"' {
            quoted = true;
            result.push(ch)
        } else if ch == ','
            && chars[ix + 1..]
                .iter()
                .find(|c| !c.is_whitespace())
                .is_some_and(|c| *c == '}' || *c == ']')
        {
        } else {
            result.push(ch)
        }
    }
    (result, comments)
}
#[cfg(test)]
mod tests;
