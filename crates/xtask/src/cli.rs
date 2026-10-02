use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::error::{Result, XtaskError};

#[derive(Parser)]
#[command(name = "xtask", about = "Build and release Gupi application packages")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Prepare archived WinGet manifests and check whether an update can be submitted.
    Winget(WingetArgs),
    /// Prepare native updater SDKs or generate an update signing key.
    Updater {
        #[command(subcommand)]
        command: UpdaterCommand,
    },
    /// Build and package Gupi for the current operating system.
    Bundle(BundleArgs),
    /// Prepare a release tag or collect its packages and distribution manifests.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
}

#[derive(Args)]
pub struct WingetArgs {
    /// Existing public stable Gupi release tag.
    #[arg(long)]
    pub tag: String,
    /// New directory for the three archived manifests; existing output is not replaced.
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Subcommand)]
pub enum UpdaterCommand {
    /// Fetch the pinned, checksum-verified SDK for macos or windows.
    Prepare { platform: String },
    /// Create a new private seed file; print only its public key. Never overwrites a key.
    Keygen { output: PathBuf },
}

#[derive(Subcommand)]
pub enum ReleaseCommand {
    /// Check the existing tag against HEAD and Cargo.toml; emit GitHub job outputs.
    Prepare(ReleaseArgs),
    /// Collect packages in dist/, write checksums and distribution/, optionally create a draft.
    Collect {
        #[command(flatten)]
        release: ReleaseArgs,
        #[arg(long)]
        draft: bool,
    },
}

#[derive(Args)]
pub struct ReleaseArgs {
    #[arg(long)]
    pub tag: String,
    #[arg(long, value_enum, default_value_t = MacosSigning::Development)]
    pub macos_signing: MacosSigning,
}

#[derive(Args)]
pub struct BundleArgs {
    /// Rust target triple; defaults to the native host target.
    #[arg(long, env = "GUPI_TARGET")]
    pub target: Option<String>,
    /// Open the English MSI after packaging (Windows only).
    #[arg(short = 'i', long)]
    pub install: bool,
    /// Development uses an ad-hoc signature; developer-id requires notarization.
    #[arg(long, env = "GUPI_MACOS_SIGNING", value_enum, default_value_t = MacosSigning::Development)]
    pub macos_signing: MacosSigning,
    /// Full Developer ID Application identity already installed in a keychain.
    #[arg(long, env = "GUPI_MACOS_SIGNING_IDENTITY")]
    pub signing_identity: Option<String>,
    /// Existing notarytool keychain profile used for notarization.
    #[arg(long, env = "GUPI_MACOS_NOTARY_PROFILE")]
    pub notary_profile: Option<String>,
    /// Keychain containing both the signing identity and notarization profile.
    #[arg(long, env = "GUPI_MACOS_KEYCHAIN")]
    pub keychain: Option<PathBuf>,
    /// Optional hardened-runtime entitlements plist, relative to the project root.
    #[arg(long)]
    pub entitlements: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum MacosSigning {
    Development,
    DeveloperId,
}

impl MacosSigning {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::DeveloperId => "developer-id",
        }
    }
}

impl BundleArgs {
    pub fn validate_signing(&self) -> Result<()> {
        match self.macos_signing {
            MacosSigning::Development => {
                if self.signing_identity.is_some()
                    || self.notary_profile.is_some()
                    || self.keychain.is_some()
                    || self.entitlements.is_some()
                {
                    return Err(XtaskError::msg(
                        "signing options require --macos-signing developer-id",
                    ));
                }
            }
            MacosSigning::DeveloperId => {
                if !self.signing_identity.as_deref().is_some_and(|identity| {
                    identity.starts_with("Developer ID Application: ")
                        && identity.trim() != "Developer ID Application:"
                }) {
                    return Err(XtaskError::msg(
                        "developer-id requires --signing-identity 'Developer ID Application: …'",
                    ));
                }
                if !self
                    .notary_profile
                    .as_deref()
                    .is_some_and(|profile| !profile.trim().is_empty())
                {
                    return Err(XtaskError::msg(
                        "developer-id requires --notary-profile; a signed but unnotarized app is not exported as a release package",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, MacosSigning};
    use clap::Parser;

    #[test]
    fn single_product_bundle_keeps_install_and_target_options() {
        let cli = Cli::try_parse_from([
            "xtask",
            "bundle",
            "--target",
            "x86_64-pc-windows-msvc",
            "--install",
        ])
        .unwrap();
        let Commands::Bundle(args) = cli.command else {
            panic!("expected bundle")
        };
        assert!(args.install);
        assert_eq!(args.target.as_deref(), Some("x86_64-pc-windows-msvc"));
        assert_eq!(args.macos_signing, MacosSigning::Development);
        assert!(args.validate_signing().is_ok());
        assert!(Cli::try_parse_from(["xtask", "bundle", "gupi"]).is_err());
    }

    #[test]
    fn official_mode_cannot_export_ad_hoc_or_unnotarized_packages() {
        for options in [
            vec!["--macos-signing", "developer-id"],
            vec![
                "--macos-signing",
                "developer-id",
                "--signing-identity=-",
                "--notary-profile",
                "gupi",
            ],
            vec![
                "--macos-signing",
                "developer-id",
                "--signing-identity",
                "Developer ID Application: Example (TEAM)",
            ],
            vec!["--notary-profile", "gupi"],
        ] {
            let mut argv = vec!["xtask", "bundle"];
            argv.extend(options);
            let Commands::Bundle(args) = Cli::try_parse_from(argv).unwrap().command else {
                panic!("expected bundle")
            };
            assert!(args.validate_signing().is_err());
        }

        let Commands::Bundle(args) = Cli::try_parse_from([
            "xtask",
            "bundle",
            "--macos-signing",
            "developer-id",
            "--signing-identity",
            "Developer ID Application: Example (TEAM)",
            "--notary-profile",
            "gupi",
        ])
        .unwrap()
        .command
        else {
            panic!("expected bundle")
        };
        assert!(args.validate_signing().is_ok());
    }
}
