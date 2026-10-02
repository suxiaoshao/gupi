#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use semver::Version;
use serde::Deserialize;
use tracing::info;

use crate::cli::WingetArgs;
use crate::error::{Result, XtaskError};

const REPOSITORY: &str = "github.com/suxiaoshao/gupi";
const COMMUNITY: &str = "github.com/microsoft/winget-pkgs";
const PACKAGE: &str = "suxiaoshao.Gupi";
const DIRECTORY: &str = "manifests/s/suxiaoshao/Gupi";

#[derive(Deserialize)]
struct PublishedRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
}

#[derive(Deserialize)]
struct CommunityEntry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Proposal {
    url: String,
    files: Vec<ChangedFile>,
}

#[derive(Deserialize)]
struct ChangedFile {
    path: String,
}

pub fn run(args: WingetArgs) -> Result<()> {
    let version = stable_version(&args.tag)?;
    let release: PublishedRelease = decode(&gh(&[
        "api",
        &format!("repos/suxiaoshao/gupi/releases/tags/{}", args.tag),
    ])?)?;
    release.verify(&args.tag)?;

    // Read-only GitHub access: a preparation run never creates a fork, branch or PR.
    let entries = community_entries()?;
    let proposals: Vec<Proposal> = decode(&gh(&[
        "pr",
        "list",
        "--repo",
        COMMUNITY,
        "--state",
        "all",
        "--search",
        "\"suxiaoshao.Gupi\" in:title",
        "--limit",
        "1000",
        "--json",
        "url,files",
    ])?)?;
    let reason = submission_blocker(&version, &entries, &proposals)?;

    let download = tempfile::tempdir()?;
    let name = format!("Gupi_{version}_distribution.tar.gz");
    gh(&[
        "release",
        "download",
        &args.tag,
        "--repo",
        REPOSITORY,
        "--pattern",
        &name,
        "--dir",
        &download.path().to_string_lossy(),
    ])?;
    let manifests = archived_manifests(&download.path().join(name), &version)?;
    // Refuse stale output rather than mixing files from different versions.
    fs::create_dir(&args.output)?;
    for (name, contents) in manifests {
        fs::write(args.output.join(name), contents)?;
    }

    let message = reason
        .as_deref()
        .unwrap_or("New stable version ready for submission");
    if let Some(path) = std::env::var_os("GITHUB_OUTPUT") {
        let mut output = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(output, "version={version}\neligible={}", reason.is_none())?;
    }
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let mut summary = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(
            summary,
            "WinGet {PACKAGE} {version}: {message}.\n\nPrepared three archived manifests; no PR was submitted by this step."
        )?;
    }
    info!(%version, eligible = reason.is_none(), message);
    Ok(())
}

fn stable_version(tag: &str) -> Result<Version> {
    let value = tag.strip_prefix('v').unwrap_or_default();
    let version = Version::parse(value)
        .map_err(|_| XtaskError::msg("expected a stable release tag vMAJOR.MINOR.PATCH"))?;
    if !version.pre.is_empty() || !version.build.is_empty() || version.to_string() != value {
        return Err(XtaskError::msg(
            "expected a stable release tag vMAJOR.MINOR.PATCH",
        ));
    }
    Ok(version)
}

impl PublishedRelease {
    fn verify(&self, tag: &str) -> Result<()> {
        if self.tag_name != tag
            || self.draft
            || self.prerelease
            || self.published_at.as_deref().is_none_or(str::is_empty)
        {
            return Err(XtaskError::msg("WinGet requires a public stable release"));
        }
        Ok(())
    }
}

fn submission_blocker(
    version: &Version,
    entries: &[CommunityEntry],
    proposals: &[Proposal],
) -> Result<Option<String>> {
    let mut versions = Vec::new();
    for entry in entries.iter().filter(|entry| entry.kind == "dir") {
        versions.push(Version::parse(&entry.name).map_err(|_| {
            XtaskError::msg(format!("cannot compare community version {}", entry.name))
        })?);
    }
    if let Some(latest) = versions.iter().max()
        && version <= latest
    {
        return Ok(Some(format!(
            "Community already contains version {latest} or newer"
        )));
    }
    let prefix = format!("{DIRECTORY}/{version}/");
    if let Some(proposal) = proposals.iter().find(|proposal| {
        proposal
            .files
            .iter()
            .any(|file| file.path.starts_with(&prefix))
    }) {
        return Ok(Some(format!("Existing version proposal: {}", proposal.url)));
    }
    if versions.is_empty() {
        return Ok(Some(
            "First package inclusion still requires the existing manual submission".into(),
        ));
    }
    Ok(None)
}

fn archived_manifests(path: &Path, version: &Version) -> Result<BTreeMap<String, String>> {
    let prefix = format!("winget/{DIRECTORY}/{version}/");
    let names = [
        format!("{PACKAGE}.yaml"),
        format!("{PACKAGE}.installer.yaml"),
        format!("{PACKAGE}.locale.en-US.yaml"),
    ];
    let mut manifests = BTreeMap::new();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(File::open(path)?));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_string_lossy().replace('\\', "/");
        let Some(name) = entry_path.strip_prefix(&prefix) else {
            continue;
        };
        if !names.iter().any(|expected| expected == name)
            || !entry.header().entry_type().is_file()
            || entry.size() > 1024 * 1024
            || manifests.contains_key(name)
        {
            return Err(XtaskError::msg(
                "unexpected entry in archived WinGet manifests",
            ));
        }
        let mut contents = String::new();
        entry.read_to_string(&mut contents)?;
        // These two fields belong to our generator; WinGetCreate performs schema validation.
        if scalar(&contents, "PackageIdentifier") != Some(PACKAGE)
            || scalar(&contents, "PackageVersion") != Some(version.to_string().as_str())
        {
            return Err(XtaskError::msg(
                "archived manifest identity does not match the release",
            ));
        }
        manifests.insert(name.to_owned(), contents);
    }
    if manifests.len() != names.len() {
        return Err(XtaskError::msg(
            "release archive is missing WinGet manifests",
        ));
    }
    Ok(manifests)
}

fn scalar<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let (field, value) = line.split_once(':')?;
        (field == key).then(|| value.trim().trim_matches('"'))
    })
}

fn community_entries() -> Result<Vec<CommunityEntry>> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/microsoft/winget-pkgs/contents/{DIRECTORY}"),
        ])
        .output()?;
    if output.status.success() {
        return decode(&output.stdout);
    }
    // Only the explicit missing-package response is an empty catalog. Other API failures stop.
    let body: serde_json::Value = decode(&output.stdout)?;
    if body["status"] == "404" {
        return Ok(Vec::new());
    }
    Err(XtaskError::msg(
        "cannot read the WinGet community package directory",
    ))
}

fn gh(args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("gh").args(args).output()?;
    if !output.status.success() {
        return Err(XtaskError::msg(format!(
            "GitHub command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|error| XtaskError::msg(format!("invalid GitHub response: {error}")))
}
