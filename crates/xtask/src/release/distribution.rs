use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::Release;
use crate::cli::MacosSigning;
use crate::error::{Result, XtaskError};

const IDENTIFIER: &str = "suxiaoshao.Gupi";
const SCHEMA: &str = "1.10.0";

impl Release {
    pub(super) fn distribution(
        &self,
        dist: &Path,
        output: &Path,
        hashes: &BTreeMap<String, String>,
    ) -> Result<()> {
        let homepage = format!("https://github.com/{}", self.repository);
        if self.signing == MacosSigning::DeveloperId {
            let arm = &hashes[&format!("Gupi_{}_aarch64_macos.dmg", self.version)];
            let intel = &hashes[&format!("Gupi_{}_x86_64_macos.dmg", self.version)];
            let cask_dir = output.join("homebrew/Casks/g");
            let auto_updates = if crate::updater::public_key()?.is_some() {
                "  auto_updates true\n"
            } else {
                ""
            };
            fs::create_dir_all(&cask_dir)?;
            fs::write(
                cask_dir.join("gupi.rb"),
                format!(
                    r#"cask "gupi" do
  arch arm: "aarch64", intel: "x86_64"

  version "{}"
  sha256 arm:   "{arm}",
         intel: "{intel}"

  url "{homepage}/releases/download/v#{{version}}/Gupi_#{{version}}_#{{arch}}_macos.dmg"
  name "Gupi"
  desc "Native desktop workspace for Pi"
  homepage "{homepage}"

{auto_updates}  depends_on :macos

  app "Gupi.app"

  caveats "Install and configure the Pi coding agent separately before using Gupi."
end
"#,
                    self.version
                ),
            )?;
        }
        let mut installers = String::new();
        for culture in &self.cultures {
            let name = format!("Gupi_{}_x64_{culture}.msi", self.version);
            let properties = msi_properties(&dist.join(&name))?;
            let property = |key: &str| {
                properties
                    .get(key)
                    .map(String::as_str)
                    .ok_or_else(|| XtaskError::msg(format!("{name}: missing MSI property {key}")))
            };
            if property("ProductName")? != "Gupi"
                || property("ProductVersion")? != self.version
                || property("Manufacturer")? != self.publisher
                || property("ALLUSERS")? != "1"
                || property("ProductLanguage")? != language_code(culture).to_string()
            {
                return Err(XtaskError::msg(format!(
                    "{name}: MSI identity, version, scope or language differs from bundle settings"
                )));
            }
            let code = quoted(property("ProductCode")?);
            let upgrade = quoted(property("UpgradeCode")?);
            let publisher = quoted(property("Manufacturer")?);
            let version = quoted(property("ProductVersion")?);
            let url = quoted(&format!("{homepage}/releases/download/{}/{name}", self.tag));
            let hash = hashes[&name].to_uppercase();
            installers.push_str(&format!("- Architecture: x64\n  InstallerLocale: {culture}\n  InstallerUrl: {url}\n  InstallerSha256: {hash}\n  ProductCode: {code}\n  AppsAndFeaturesEntries:\n  - DisplayName: Gupi\n    Publisher: {publisher}\n    DisplayVersion: {version}\n    ProductCode: {code}\n    UpgradeCode: {upgrade}\n"));
        }
        let winget = output
            .join("winget/manifests/s/suxiaoshao/Gupi")
            .join(&self.version);
        fs::create_dir_all(&winget)?;
        self.write_yaml(&winget, "", "version", "DefaultLocale: en-US\n")?;
        self.write_yaml(&winget, ".installer", "installer", &format!("InstallerType: wix\nScope: machine\nUpgradeBehavior: install\nInstallers:\n{installers}"))?;
        self.write_yaml(&winget, ".locale.en-US", "defaultLocale", &format!(
            "PackageLocale: en-US\nPublisher: {}\nPackageName: Gupi\nPackageUrl: {homepage}\nLicense: MIT\nLicenseUrl: {homepage}/blob/{}/LICENSE\nShortDescription: Native desktop workspace for Pi\nReleaseNotesUrl: {homepage}/releases/tag/{}\n",
            quoted(&self.publisher), self.tag, self.tag))
    }

    fn write_yaml(&self, directory: &Path, suffix: &str, kind: &str, body: &str) -> Result<()> {
        fs::write(
            directory.join(format!("{IDENTIFIER}{suffix}.yaml")),
            format!(
                "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{SCHEMA}.schema.json\n\nPackageIdentifier: {IDENTIFIER}\nPackageVersion: {}\n{body}ManifestType: {kind}\nManifestVersion: {SCHEMA}\n",
                quoted(&self.version)
            ),
        )?;
        Ok(())
    }
}

// Read the distributed MSI directly on the collection runner. No Windows COM
// process, installation, or second metadata artifact is needed.
fn msi_properties(path: &Path) -> Result<BTreeMap<String, String>> {
    let mut package = msi::open(path)?;
    if package.summary_info().arch() != Some("x64") {
        return Err(XtaskError::msg(format!(
            "{}: expected an x64 MSI",
            path.display()
        )));
    }
    let rows = package.select_rows(msi::Select::table("Property"))?;
    Ok(rows
        .filter_map(|row| match (&row["Property"], &row["Value"]) {
            (msi::Value::Str(key), msi::Value::Str(value)) => Some((key.clone(), value.clone())),
            _ => None,
        })
        .collect())
}

fn quoted(value: &str) -> String {
    // JSON string literals are valid YAML quoted scalars.
    serde_json::to_string(value).expect("string serialization cannot fail")
}

fn language_code(culture: &str) -> u16 {
    // msi 0.10 omits Taiwan and maps es-ES to traditional sort. These values
    // match our WiX localizations (Taiwan and modern Spanish sort).
    match culture {
        "zh-TW" => 1028,
        "es-ES" => 3082,
        _ => msi::Language::from_tag(culture).code(),
    }
}
