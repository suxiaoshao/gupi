//! Read-only Pi package catalog: npm registry packages tagged `pi-package`.
//!
//! Search results carry only registry metadata. Package details read the
//! published manifest's `pi` declarations, which are paths or globs rather than
//! resource counts. Installation stays with the Pi package operations.
use reqwest::Client;
use reqwest::StatusCode;
use reqwest::header;
use serde_json::Value;
use std::time::Duration;
use url::Url;

/// The keyword Pi documents for package discovery.
pub const KEYWORD: &str = "pi-package";
pub const PAGE_SIZE: usize = 20;
/// Pi's own browsable package directory, linked as an external resource.
pub const DIRECTORY_URL: &str = "https://pi.dev/packages";
const REGISTRY: &str = "https://registry.npmjs.org";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Problem {
    #[error("network")]
    Network,
    #[error("rate limited")]
    RateLimited,
    #[error("not found")]
    NotFound,
    #[error("invalid response")]
    InvalidResponse,
}
impl Problem {
    /// Localization key for the problem.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Network => "packages-problem-network",
            Self::RateLimited => "packages-problem-rate-limited",
            Self::NotFound => "packages-problem-not-found",
            Self::InvalidResponse => "packages-problem-invalid",
        }
    }
}

/// One search result. Only fields npm returned are present.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub npm: Option<String>,
    pub repository: Option<String>,
}
impl Entry {
    /// The Pi install source for this registry package, without a version.
    pub fn source(&self) -> String {
        source(&self.name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchPage {
    pub entries: Vec<Entry>,
    /// Raw registry hit count before package validation; not a browsable package count.
    pub total: usize,
    pub from: usize,
}

impl SearchPage {
    pub fn number(&self) -> usize {
        self.from / PAGE_SIZE + 1
    }

    pub fn has_next(&self) -> bool {
        self.from.saturating_add(PAGE_SIZE) < self.total
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declared {
    Extensions,
    Skills,
    Prompts,
    Themes,
}
impl Declared {
    pub const ALL: [Self; 4] = [Self::Extensions, Self::Skills, Self::Prompts, Self::Themes];
    pub fn key(self) -> &'static str {
        match self {
            Self::Extensions => "extensions",
            Self::Skills => "skills",
            Self::Prompts => "prompts",
            Self::Themes => "themes",
        }
    }
}

/// Manifest details of the latest published version.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Details {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    /// `None` when the manifest has no `pi` declarations; entries keep the
    /// declared paths and globs as written.
    pub declared: Option<Vec<(Declared, Vec<String>)>>,
}

/// npm package names: optional `@scope/`, lowercase URL-safe characters.
pub fn valid_name(name: &str) -> bool {
    let valid_part = |part: &str| {
        !part.is_empty()
            && !part.starts_with(['.', '_', '-'])
            && part.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_' | '~')
            })
    };
    if name.len() > 214 {
        return false;
    }
    match name.strip_prefix('@') {
        Some(scoped) => scoped
            .split_once('/')
            .is_some_and(|(scope, package)| valid_part(scope) && valid_part(package)),
        None => valid_part(name),
    }
}

pub fn source(name: &str) -> String {
    format!("npm:{name}")
}

/// The registry package name of an installed `npm:` source, without a version.
pub fn installed_name(source: &str) -> Option<&str> {
    let spec = source.strip_prefix("npm:")?.trim();
    // A scoped name keeps its leading `@`; a version follows the next `@`.
    let split = if let Some(scoped) = spec.strip_prefix('@') {
        scoped.find('@').map(|index| index + 1)
    } else {
        spec.find('@')
    };
    let name = split.map_or(spec, |index| &spec[..index]);
    valid_name(name).then_some(name)
}

/// The version an installed `npm:` source is fixed to, if it names one.
pub fn pinned_version(source: &str) -> Option<&str> {
    let spec = source.strip_prefix("npm:")?.trim();
    let name = installed_name(source)?;
    spec.get(name.len() + 1..)
        .filter(|version| !version.is_empty())
}

fn client() -> Result<Client, Problem> {
    // Proxy variables come from Gupi's process environment, the same
    // environment Pi package subprocesses inherit.
    Client::builder()
        .user_agent(concat!("Gupi/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .https_only(true)
        .build()
        .map_err(|_| Problem::Network)
}

pub async fn search(query: String, from: usize) -> Result<SearchPage, Problem> {
    let text = match query.trim() {
        "" => format!("keywords:{KEYWORD}"),
        query => format!("keywords:{KEYWORD} {query}"),
    };
    let url = Url::parse_with_params(
        &format!("{REGISTRY}/-/v1/search"),
        [
            ("text", text),
            ("size", PAGE_SIZE.to_string()),
            ("from", from.to_string()),
        ],
    )
    .map_err(|_| Problem::InvalidResponse)?;
    let body = get(&client()?, url).await?;
    parse_search(&body, from)
}

pub async fn details(name: String) -> Result<Details, Problem> {
    if !valid_name(&name) {
        return Err(Problem::NotFound);
    }
    let mut url = Url::parse(REGISTRY).map_err(|_| Problem::InvalidResponse)?;
    url.path_segments_mut()
        .map_err(|_| Problem::InvalidResponse)?
        .push(&name)
        .push("latest");
    let body = get(&client()?, url).await?;
    parse_details(&body, &name)
}

async fn get(client: &Client, url: Url) -> Result<Vec<u8>, Problem> {
    let mut response = client
        .get(url)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| Problem::Network)?;
    match response.status() {
        StatusCode::OK => {}
        StatusCode::NOT_FOUND => return Err(Problem::NotFound),
        StatusCode::TOO_MANY_REQUESTS => return Err(Problem::RateLimited),
        _ => return Err(Problem::Network),
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(Problem::InvalidResponse);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Problem::Network)? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(Problem::InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Only http(s) links are shown; `git+https://…/x.git` becomes `https://…/x`.
fn web_link(raw: &str) -> Option<String> {
    let raw = raw.trim().trim_start_matches("git+");
    let raw = raw.strip_suffix(".git").unwrap_or(raw);
    let url = Url::parse(raw).ok()?;
    matches!(url.scheme(), "https" | "http").then(|| url.to_string())
}

fn has_keyword(package: &Value) -> bool {
    package
        .get("keywords")
        .and_then(Value::as_array)
        .is_some_and(|keywords| keywords.iter().any(|k| k.as_str() == Some(KEYWORD)))
}

pub fn parse_search(body: &[u8], from: usize) -> Result<SearchPage, Problem> {
    let value: Value = serde_json::from_slice(body).map_err(|_| Problem::InvalidResponse)?;
    let objects = value
        .get("objects")
        .and_then(Value::as_array)
        .ok_or(Problem::InvalidResponse)?;
    let entries = objects
        .iter()
        .filter_map(|object| object.get("package"))
        // The search text is a ranking hint; keep only packages that carry the keyword.
        .filter(|package| has_keyword(package))
        .filter_map(|package| {
            let name = text(package, "name").filter(|name| valid_name(name))?;
            let links = package.get("links");
            Some(Entry {
                name,
                version: text(package, "version")?,
                description: text(package, "description"),
                publisher: package.get("publisher").and_then(|p| text(p, "username")),
                npm: links
                    .and_then(|l| l.get("npm"))
                    .and_then(Value::as_str)
                    .and_then(web_link),
                repository: links
                    .and_then(|l| l.get("repository"))
                    .and_then(Value::as_str)
                    .and_then(web_link),
            })
        })
        .collect();
    Ok(SearchPage {
        entries,
        total: value
            .get("total")
            .and_then(Value::as_u64)
            .map_or(0, |total| total as usize),
        from,
    })
}

pub fn parse_details(body: &[u8], name: &str) -> Result<Details, Problem> {
    let value: Value = serde_json::from_slice(body).map_err(|_| Problem::InvalidResponse)?;
    if text(&value, "name").as_deref() != Some(name) {
        return Err(Problem::InvalidResponse);
    }
    let repository = match value.get("repository") {
        Some(Value::String(url)) => web_link(url),
        Some(repository) => repository
            .get("url")
            .and_then(Value::as_str)
            .and_then(web_link),
        None => None,
    };
    let declared = value.get("pi").and_then(Value::as_object).map(|pi| {
        Declared::ALL
            .into_iter()
            .filter_map(|kind| {
                let paths: Vec<String> = match pi.get(kind.key())? {
                    Value::String(path) => vec![path.clone()],
                    Value::Array(paths) => paths
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                    _ => return None,
                };
                (!paths.is_empty()).then_some((kind, paths))
            })
            .collect::<Vec<_>>()
    });
    Ok(Details {
        name: name.to_owned(),
        version: text(&value, "version").ok_or(Problem::InvalidResponse)?,
        description: text(&value, "description"),
        license: text(&value, "license"),
        homepage: value
            .get("homepage")
            .and_then(Value::as_str)
            .and_then(web_link),
        repository,
        declared: declared.filter(|declared| !declared.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_keeps_raw_offsets_even_when_validation_empties_a_page() {
        let body = br#"{"total": 41, "objects": [
            {"package": {"name": "other", "version": "1.0.0", "keywords": ["cli"]}}
        ]}"#;
        for (from, number, has_next) in [(0, 1, true), (20, 2, true), (40, 3, false)] {
            let page = parse_search(body, from).unwrap();
            assert!(page.entries.is_empty());
            assert_eq!(page.number(), number);
            assert_eq!(page.has_next(), has_next);
        }
        let empty = parse_search(br#"{"total": 0, "objects": []}"#, 0).unwrap();
        assert_eq!(empty.number(), 1);
        assert!(!empty.has_next());
    }

    #[test]
    fn search_keeps_only_tagged_valid_packages() {
        let body = br#"{"total": 3, "objects": [
            {"package": {"name": "pi-web", "version": "1.0.0", "keywords": ["pi-package"],
                "description": " Web tools ", "publisher": {"username": "dev"},
                "links": {"npm": "https://www.npmjs.com/package/pi-web",
                          "repository": "git+https://github.com/dev/pi-web.git"}}},
            {"package": {"name": "other", "version": "1.0.0", "keywords": ["cli"]}},
            {"package": {"name": "Bad Name", "version": "1.0.0", "keywords": ["pi-package"]}}
        ]}"#;
        let page = parse_search(body, 20).unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.from, 20);
        assert_eq!(page.entries.len(), 1);
        let entry = &page.entries[0];
        assert_eq!(entry.source(), "npm:pi-web");
        assert_eq!(entry.description.as_deref(), Some("Web tools"));
        assert_eq!(
            entry.repository.as_deref(),
            Some("https://github.com/dev/pi-web")
        );
    }

    #[test]
    fn details_keep_declared_paths_and_unknown_contents() {
        let body = br#"{"name": "@a/pi", "version": "2.0.0", "license": "MIT",
            "repository": {"type": "git", "url": "git+ssh://git@github.com/a/pi.git"},
            "pi": {"extensions": ["./src/*.ts", "!./src/test.ts"], "skills": "./skills", "themes": []}}"#;
        let details = parse_details(body, "@a/pi").unwrap();
        assert_eq!(details.repository, None, "non-web links are not shown");
        assert_eq!(
            details.declared,
            Some(vec![
                (
                    Declared::Extensions,
                    vec!["./src/*.ts".into(), "!./src/test.ts".into()]
                ),
                (Declared::Skills, vec!["./skills".into()]),
            ])
        );
        let plain = parse_details(br#"{"name": "x", "version": "1.0.0"}"#, "x").unwrap();
        assert_eq!(plain.declared, None);
        assert!(parse_details(br#"{"name": "y", "version": "1"}"#, "x").is_err());
    }

    #[test]
    fn installed_sources_match_registry_names() {
        assert_eq!(installed_name("npm:pi-web"), Some("pi-web"));
        assert_eq!(installed_name("npm:pi-web@1.2.0"), Some("pi-web"));
        assert_eq!(installed_name("npm:@a/pi@^2"), Some("@a/pi"));
        assert_eq!(installed_name("https://github.com/a/pi"), None);
        assert_eq!(pinned_version("npm:@a/pi@2.0.0"), Some("2.0.0"));
        assert_eq!(pinned_version("npm:pi-web"), None);
        assert!(!valid_name("-flag"));
        assert!(!valid_name("UPPER"));
    }
}
