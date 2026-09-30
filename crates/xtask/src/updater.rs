use crate::{
    cli::UpdaterCommand,
    context::workspace_root,
    error::{Result, XtaskError},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn run(command: UpdaterCommand) -> Result<()> {
    match command {
        UpdaterCommand::Prepare { platform } => {
            println!("{}", sdk(&workspace_root()?, &platform)?.display());
        }
        UpdaterCommand::Keygen { output } => {
            let mut seed = [0; 32];
            getrandom::fill(&mut seed)
                .map_err(|_| XtaskError::msg("system random source failed"))?;
            let key = SigningKey::from_bytes(&seed);
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(output)?;
            writeln!(file, "{}", BASE64_STANDARD.encode(key.as_bytes()))?;
            println!("{}", BASE64_STANDARD.encode(key.verifying_key().as_bytes()));
        }
    }
    Ok(())
}

pub fn public_key() -> Result<Option<String>> {
    std::env::var("GUPI_UPDATE_PUBLIC_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .map(|key| {
            let bytes = BASE64_STANDARD
                .decode(key.trim())
                .map_err(|_| XtaskError::msg("invalid GUPI_UPDATE_PUBLIC_KEY"))?;
            let bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| XtaskError::msg("update public key must contain 32 bytes"))?;
            ed25519_dalek::VerifyingKey::from_bytes(&bytes)
                .map_err(|_| XtaskError::msg("invalid Ed25519 public key"))?;
            Ok(BASE64_STANDARD.encode(bytes))
        })
        .transpose()
}

pub fn sdk(root: &Path, platform: &str) -> Result<PathBuf> {
    let (name, url, hash) = match platform {
        "macos" => (
            "Sparkle-2.9.6",
            "https://github.com/sparkle-project/Sparkle/releases/download/2.9.6/Sparkle-2.9.6.tar.xz",
            "52bf9e88cdd972fc0c81501377a880e90d47031bd8ca5462488f843e2609e192",
        ),
        "windows" => (
            "WinSparkle-0.9.4",
            "https://github.com/vslavik/winsparkle/releases/download/v0.9.4/WinSparkle-0.9.4.zip",
            "6037df37fc263bd1650a1c4949681a9d40ffe991d01f35892a406cb5d103c976",
        ),
        _ => return Err(XtaskError::msg("updater platform must be macos or windows")),
    };
    let cache = root.join("target/updater");
    fs::create_dir_all(&cache)?;
    let archive = cache.join(url.rsplit('/').next().unwrap());
    if !archive.is_file() {
        let download = tempfile::NamedTempFile::new_in(&cache)?;
        crate::cmd::run_cmd_os(
            "curl",
            &[
                "--fail".as_ref(),
                "--location".as_ref(),
                "--silent".as_ref(),
                "--show-error".as_ref(),
                "--proto".as_ref(),
                "=https".as_ref(),
                "--max-time".as_ref(),
                "180".as_ref(),
                "--output".as_ref(),
                download.path().as_os_str(),
                url.as_ref(),
            ],
            None,
        )?;
        if crate::release::sha256(download.path())? != hash {
            return Err(XtaskError::msg("updater SDK checksum mismatch"));
        }
        download
            .persist(&archive)
            .map_err(|error| XtaskError::msg(error.to_string()))?;
    }
    if crate::release::sha256(&archive)? != hash {
        return Err(XtaskError::msg("cached updater SDK checksum mismatch"));
    }
    let unpacked = cache.join(name);
    if !unpacked.join(".verified").is_file() {
        if unpacked.exists() {
            fs::remove_dir_all(&unpacked)?;
        }
        fs::create_dir_all(&unpacked)?;
        crate::cmd::run_cmd_os(
            "tar",
            &[
                "-xf".as_ref(),
                archive.as_os_str(),
                "-C".as_ref(),
                unpacked.as_os_str(),
            ],
            None,
        )?;
        fs::write(unpacked.join(".verified"), hash)?;
    }
    Ok(unpacked)
}

/// Standard Sparkle/WinSparkle appcasts; signatures cover the exact published package bytes.
pub fn appcasts(
    root: &Path,
    version: &str,
    repository: &str,
    packages: &[String],
) -> Result<Vec<String>> {
    let Some(public) = public_key()? else {
        return Ok(Vec::new());
    };
    let secret = match std::env::var("GUPI_UPDATE_KEY_FILE") {
        Ok(path) => fs::read_to_string(path)?,
        Err(_) => std::env::var("GUPI_UPDATE_PRIVATE_KEY").map_err(|_| {
            XtaskError::msg(
                "signed updates require GUPI_UPDATE_PRIVATE_KEY or GUPI_UPDATE_KEY_FILE",
            )
        })?,
    };
    let key = signing_key(&secret, &public)?;
    write_appcasts(&root.join("dist"), version, repository, packages, &key)
}

fn signing_key(secret: &str, public: &str) -> Result<SigningKey> {
    let bytes: [u8; 32] = BASE64_STANDARD
        .decode(secret.trim())
        .map_err(|_| XtaskError::msg("invalid update signing seed"))?
        .try_into()
        .map_err(|_| {
            XtaskError::msg(
                "update signing seed must contain 32 bytes (Sparkle's current key format)",
            )
        })?;
    let key = SigningKey::from_bytes(&bytes);
    if BASE64_STANDARD.encode(key.verifying_key().as_bytes()) != public {
        return Err(XtaskError::msg(
            "update signing key does not match the bundled public key",
        ));
    }
    Ok(key)
}

fn write_appcasts(
    dist: &Path,
    version: &str,
    repository: &str,
    packages: &[String],
    key: &SigningKey,
) -> Result<Vec<String>> {
    let mut feeds = Vec::new();
    for package in packages {
        let feed = if package.ends_with("_macos.zip") {
            let arch = if package.contains("_aarch64_") {
                "aarch64"
            } else {
                "x86_64"
            };
            format!("appcast-macos-{arch}.xml")
        } else if package.ends_with(".msi") {
            let culture = package.rsplit('_').next().unwrap().trim_end_matches(".msi");
            format!("appcast-windows-{culture}.xml")
        } else {
            continue;
        };
        let bytes = fs::read(dist.join(package))?;
        let signature = BASE64_STANDARD.encode(key.sign(&bytes).to_bytes());
        let url = format!("https://github.com/{repository}/releases/download/v{version}/{package}");
        let notes = format!("https://github.com/{repository}/releases/tag/v{version}");
        let minimum = if feed.contains("macos") {
            "<sparkle:minimumSystemVersion>11.0</sparkle:minimumSystemVersion>"
        } else {
            ""
        };
        fs::write(
            dist.join(&feed),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\" xmlns:sparkle=\"http://www.andymatuschak.org/xml-namespaces/sparkle\"><channel><title>Gupi</title><item><title>Gupi {version}</title><link>{notes}</link><sparkle:version>{version}</sparkle:version><sparkle:shortVersionString>{version}</sparkle:shortVersionString>{minimum}<enclosure url=\"{url}\" length=\"{}\" type=\"application/octet-stream\" sparkle:edSignature=\"{signature}\" /></item></channel></rss>\n",
                bytes.len()
            ),
        )?;
        feeds.push(feed);
    }
    Ok(feeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier};

    #[test]
    fn appcasts_sign_exact_package_bytes_and_reject_a_different_public_key() {
        let directory = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let public = BASE64_STANDARD.encode(key.verifying_key().as_bytes());
        assert!(signing_key(&BASE64_STANDARD.encode([7; 32]), &public).is_ok());
        assert!(signing_key(&BASE64_STANDARD.encode([8; 32]), &public).is_err());
        let packages = [
            "Gupi_1.2.3_aarch64_macos.zip",
            "Gupi_1.2.3_x64_zh-CN.msi",
            "Gupi_1.2.3_amd64.deb",
        ];
        for name in packages {
            fs::write(directory.path().join(name), b"signed package bytes").unwrap();
        }
        let feeds = write_appcasts(
            directory.path(),
            "1.2.3",
            "suxiaoshao/gupi",
            &packages.map(str::to_owned),
            &key,
        )
        .unwrap();
        assert_eq!(
            feeds,
            ["appcast-macos-aarch64.xml", "appcast-windows-zh-CN.xml"]
        );
        for feed in feeds {
            let xml = fs::read_to_string(directory.path().join(feed)).unwrap();
            let signature = xml
                .split("sparkle:edSignature=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            let signature =
                Signature::from_slice(&BASE64_STANDARD.decode(signature).unwrap()).unwrap();
            assert!(
                key.verifying_key()
                    .verify(b"signed package bytes", &signature)
                    .is_ok()
            );
            assert!(
                key.verifying_key()
                    .verify(b"tampered package bytes", &signature)
                    .is_err()
            );
            assert!(xml.contains("releases/download/v1.2.3/"));
        }
    }
}
