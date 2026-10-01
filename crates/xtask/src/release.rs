mod distribution;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use semver::Version;
use sha2::{Digest, Sha256};
use tracing::info;

use crate::bundle::settings;
use crate::cli::{MacosSigning, ReleaseArgs, ReleaseCommand};
use crate::context::workspace_root;
use crate::error::{Result, XtaskError};

struct Release {
    version: String,
    tag: String,
    repository: String,
    publisher: String,
    cultures: Vec<&'static str>,
    signing: MacosSigning,
}

pub fn run(command: ReleaseCommand) -> Result<()> {
    let root = workspace_root()?;
    match command {
        ReleaseCommand::Prepare(args) => Release::load(&root, args)?.prepare(&root),
        ReleaseCommand::Collect { release, draft } => {
            Release::load(&root, release)?.collect(&root, draft)
        }
    }
}

impl Release {
    fn load(root: &Path, args: ReleaseArgs) -> Result<Self> {
        let path = root.join("Cargo.toml");
        let (package, bundle, locales) = settings::read_bundle_settings(&path)?;
        let parsed = Version::parse(&package.version)
            .map_err(|error| XtaskError::msg(format!("invalid release version: {error}")))?;
        if !parsed.pre.is_empty()
            || !parsed.build.is_empty()
            || parsed.major > 255
            || parsed.minor > 255
            || parsed.patch > 65535
        {
            return Err(XtaskError::msg(
                "release version must be MAJOR.MINOR.PATCH within MSI limits (255, 255, 65535)",
            ));
        }
        if args.tag != format!("v{}", package.version) {
            return Err(XtaskError::msg(format!(
                "tag {} must match package version v{}",
                args.tag, package.version
            )));
        }
        let manifest: toml::Value = toml::from_str(&fs::read_to_string(path)?)?;
        let repository = manifest["package"]["repository"]
            .as_str()
            .and_then(|url| url.strip_prefix("https://github.com/"))
            .ok_or_else(|| XtaskError::msg("package.repository must be a GitHub repository URL"))?;
        Ok(Self {
            version: package.version,
            tag: args.tag,
            repository: repository.trim_end_matches('/').to_owned(),
            publisher: bundle
                .publisher
                .ok_or_else(|| XtaskError::msg("missing bundle publisher"))?,
            cultures: locales.iter().map(|locale| locale.wix_language).collect(),
            signing: args.macos_signing,
        })
    }

    fn prepare(&self, root: &Path) -> Result<()> {
        let sha = git_output(root, &format!("refs/tags/{}^{{commit}}", self.tag))?;
        if sha != git_output(root, "HEAD")? {
            return Err(XtaskError::msg(
                "checkout HEAD must match the existing release tag",
            ));
        }
        if let Some(path) = std::env::var_os("GITHUB_OUTPUT") {
            let mut output = OpenOptions::new().create(true).append(true).open(path)?;
            writeln!(
                output,
                "tag={}\nsha={sha}\nmacos_signing={}",
                self.tag,
                self.signing.as_str()
            )?;
        }
        info!(
            tag = self.tag,
            sha,
            signing = self.signing.as_str(),
            "release tag verified"
        );
        Ok(())
    }

    fn expected_packages(&self) -> BTreeSet<String> {
        let mut packages = BTreeSet::new();
        let suffix = if self.signing == MacosSigning::Development {
            "_development"
        } else {
            ""
        };
        for arch in ["aarch64", "x86_64"] {
            for extension in ["zip", "dmg"] {
                packages.insert(format!(
                    "Gupi_{}_{arch}_macos{suffix}.{extension}",
                    self.version
                ));
            }
        }
        for culture in &self.cultures {
            packages.insert(format!("Gupi_{}_x64_{culture}.msi", self.version));
        }
        packages.insert(format!("Gupi_{}_amd64.deb", self.version));
        packages
    }

    // Check once at collection: this is where files from independent jobs meet.
    fn packages(&self, dist: &Path) -> Result<BTreeMap<String, String>> {
        let expected = self.expected_packages();
        let actual = fs::read_dir(dist)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| matches!(ext, "zip" | "dmg" | "msi" | "deb"))
            })
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(XtaskError::msg(format!(
                "release packages differ: missing {:?}, unexpected {:?}",
                expected.difference(&actual).collect::<Vec<_>>(),
                actual.difference(&expected).collect::<Vec<_>>()
            )));
        }
        expected
            .into_iter()
            .map(|name| {
                let path = dist.join(&name);
                let metadata = path.symlink_metadata()?;
                if !metadata.is_file() || metadata.len() == 0 {
                    return Err(XtaskError::msg(format!(
                        "invalid or empty package: {}",
                        path.display()
                    )));
                }
                Ok((name, sha256(&path)?))
            })
            .collect()
    }

    fn collect(&self, root: &Path, draft: bool) -> Result<()> {
        let dist = root.join("dist");
        let packages = self.packages(&dist)?;
        let output = root.join("distribution");
        // Regenerate owned outputs so an unsigned build cannot retain an old Cask.
        if output.exists() {
            fs::remove_dir_all(&output)?;
        }
        fs::create_dir_all(&output)?;
        self.distribution(&dist, &output, &packages)?;
        let feeds = crate::updater::appcasts(
            root,
            &self.version,
            &self.repository,
            &packages.keys().cloned().collect::<Vec<_>>(),
        )?;
        fs::write(
            dist.join("SHA256SUMS"),
            packages
                .iter()
                .map(|(name, hash)| format!("{hash}  {name}\n"))
                .collect::<String>(),
        )?;
        fs::write(
            output.join("README.txt"),
            format!(
                "Gupi {}: manifests for review, not submitted.\nPublish the matching stable GitHub release before distributing these files.\nHomebrew is generated only for developer-id packages.\nSee docs/releasing.md for channel installation and submission.\n",
                self.tag
            ),
        )?;
        let manifests = self.archive_distribution(root)?;
        info!(
            packages = packages.len(),
            "release checksums and distribution manifests generated"
        );
        if draft {
            self.create_draft(root, &packages, &feeds, &manifests)?;
        }
        Ok(())
    }

    fn archive_distribution(&self, root: &Path) -> Result<PathBuf> {
        let directory = root.join("distribution");
        let path = root
            .join("dist")
            .join(format!("Gupi_{}_distribution.tar.gz", self.version));
        let compressed = flate2::GzBuilder::new()
            .mtime(0)
            .write(File::create(&path)?, flate2::Compression::default());
        let mut archive = tar::Builder::new(compressed);
        let mut entries = walkdir::WalkDir::new(&directory)
            .min_depth(1)
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| XtaskError::msg(format!("cannot archive manifests: {error}")))?;
        entries.sort_by_key(|entry| entry.path().to_owned());
        for entry in entries {
            if entry.file_type().is_dir() {
                continue;
            }
            if !entry.file_type().is_file() {
                return Err(XtaskError::msg(
                    "distribution manifests must be regular files",
                ));
            }
            let mut source = File::open(entry.path())?;
            let mut header = tar::Header::new_gnu();
            header.set_size(source.metadata()?.len());
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            archive.append_data(
                &mut header,
                entry
                    .path()
                    .strip_prefix(&directory)
                    .expect("manifest path"),
                &mut source,
            )?;
        }
        archive.into_inner()?.finish()?;
        Ok(path)
    }

    fn create_draft(
        &self,
        root: &Path,
        packages: &BTreeMap<String, String>,
        feeds: &[String],
        manifests: &Path,
    ) -> Result<()> {
        let existing = Command::new("gh")
            .args(["release", "view", &self.tag, "--repo", &self.repository])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if existing.success() {
            return Err(XtaskError::msg(
                "a release already exists for this tag; refusing to replace it",
            ));
        }
        let mut notes = tempfile::NamedTempFile::new()?;
        let status = match self.signing {
            MacosSigning::Development => {
                "development ad-hoc app signature; not notarized and subject to Gatekeeper restrictions"
            }
            MacosSigning::DeveloperId => "Developer ID signed, notarized and stapled",
        };
        write!(
            notes,
            "Gupi {} packages for review.\n\n- macOS: {status}. Open the DMG and drag Gupi to Applications, or extract the ZIP.\n- Windows x64: open the MSI matching your installer language; packages are unsigned.\n- Linux x64: install the unsigned deb package.\n- Pi is an external runtime and is not bundled.\n- Pi icon attribution and the upstream MIT license are included in THIRD_PARTY_NOTICES.md in each package. Gupi is an independent project.\n- Review platform installation before publishing.\n",
            self.tag
        )?;
        let status = Command::new("gh")
            .args([
                "release",
                "create",
                &self.tag,
                "--repo",
                &self.repository,
                "--draft",
                "--verify-tag",
                "--latest=false",
                "--title",
            ])
            .arg(format!("Gupi {}", self.tag))
            .arg("--notes-file")
            .arg(notes.path())
            .args(packages.keys().map(|name| root.join("dist").join(name)))
            .arg(root.join("dist/SHA256SUMS"))
            .args(feeds.iter().map(|name| root.join("dist").join(name)))
            .arg(manifests)
            .status()?;
        if !status.success() {
            return Err(XtaskError::msg("GitHub draft release creation failed"));
        }
        Ok(())
    }
}

fn git_output(root: &Path, revision: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", revision])
        .output()?;
    if !output.status.success() {
        return Err(XtaskError::msg(format!(
            "cannot resolve {revision}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(crate) fn sha256(path: &Path) -> Result<String> {
    let mut source = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
