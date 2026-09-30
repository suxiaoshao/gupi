# Release packaging

Gupi uses `tauri-bundler` to create a macOS application ZIP, nine localized Windows x64 MSI installers, and a Linux x64 Debian package. The root `Cargo.toml` owns the product version and bundle metadata; the bundle identifier remains `top.sushao.gupi`. Pi is an external runtime and is not included in these packages.

## Build locally

Use the Rust version in `rust-toolchain.toml`. macOS packaging needs full Xcode, including Metal tools. Windows packaging needs the MSVC toolchain; tauri-bundler downloads its WiX tools. Linux packaging needs the native build packages listed in [the setup action](../.github/actions/setup/action.yml), including `dpkg-dev` and `binutils`.

For distributable macOS and Linux packages, use a native rustup toolchain outside the Nix development shell. xtask rejects binaries whose Mach-O dependencies/load commands or ELF interpreter/RPATH contain `/nix/store/`. Keep the native build output separate when you also use Nix for development:

```sh
# macOS, after exiting the Nix shell:
CARGO_TARGET_DIR=target/native MACOSX_DEPLOYMENT_TARGET=11.0 \
  cargo run -p xtask --locked -- bundle

# Linux, with native Ubuntu libraries and rustup:
CARGO_TARGET_DIR=target/native cargo run -p xtask --locked -- bundle

# Windows, in an MSVC development environment:
cargo run -p xtask --locked -- bundle
```

Ensure `cargo` in the active PATH comes from rustup. A native Cargo target directory alone does not remove Nix compiler/linker environment variables.

`bundle` always selects Gupi; it takes no application-name argument. It builds with `--release --locked` and an explicit Rust target. `--target` supports these targets on a matching operating system:

| Host | Target | Downloadable output |
| --- | --- | --- |
| macOS | `aarch64-apple-darwin` | `Gupi_<version>_aarch64_macos_development.zip` |
| macOS | `x86_64-apple-darwin` | `Gupi_<version>_x86_64_macos_development.zip` |
| Windows | `x86_64-pc-windows-msvc` | `Gupi_<version>_x64_<culture>.msi` |
| Linux | `x86_64-unknown-linux-gnu` | `Gupi_<version>_amd64.deb` |

`rustup target add <triple>` installs the Rust standard library when needed. Cross-OS packaging is unsupported; the release workflow uses native runners for every target.

Packages are exported to `dist/<triple>/`. Build outputs are under `<target-root>/<triple>/release/`; bundle staging is `<target-root>/xtask-bundle/<triple>/release/`. The macOS app remains at `<target-root>/xtask-bundle/<triple>/release/bundle/macos/Gupi.app` for local launch. `<target-root>` is `target/` by default; `CARGO_TARGET_DIR` overrides it, and relative values resolve from the project root. Rebuilding replaces only that target's staging and exported packages.

Windows `--install` opens the English MSI, falling back to another MSI if English is unavailable. Other hosts reject this option. Linux dependencies come from `dpkg-shlibdeps`, plus the Vulkan/EGL/Wayland libraries loaded dynamically by GPUI. Build Linux release packages on Ubuntu 22.04 to preserve that native library baseline.

## Bundle resources

xtask derives iconsets and ICO files from `build-assets/icon/app-icon.png` in temporary directories. It compiles the seven macOS icon themes when the installed Xcode supports them, with the existing ordinary-icon fallback. Localization resources map the nine declared application locales to macOS `.lproj` directories and WiX cultures; Windows exports one MSI per culture.

The minimum macOS bundle version defaults to 11.0 and follows `MACOSX_DEPLOYMENT_TARGET` when configured. The workflow sets it to 11.0 for both architectures. The Liquid Glass icon compilation target is separate from the application's minimum OS version.

## macOS signature modes

The default `development` mode applies an ad-hoc signature after icon injection, then verifies the bundle and exports a ZIP ending in `_development`. It has no Developer ID or notarization ticket and remains subject to Gatekeeper restrictions.

The `developer-id` mode requires an installed **Developer ID Application** identity and an existing `notarytool` keychain profile. Set up the profile in Keychain using Apple's documented `xcrun notarytool store-credentials` workflow. Then build:

```sh
CARGO_TARGET_DIR=target/native MACOSX_DEPLOYMENT_TARGET=11.0 \
  cargo run -p xtask --locked -- bundle \
  --macos-signing developer-id \
  --signing-identity 'Developer ID Application: Your Name (TEAMID)' \
  --notary-profile gupi-notary
```

`--keychain /path/to/keychain-db` selects a keychain containing both the identity and profile. `--entitlements /path/to/entitlements.plist` optionally adds the entitlements required by a particular distribution configuration; relative paths resolve from the project root.

Formal signing happens after all bundle mutations. xtask signs nested Mach-O files/code bundles from the inside out using hardened runtime and a secure timestamp, verifies the result, submits a ZIP to Apple, requires an `Accepted` status, staples and validates the ticket, and runs Gatekeeper assessment. It exports the final ZIP without the `_development` suffix only after these steps pass. Notarization waits up to 30 minutes; a timeout or rejection fails the build. Use the reported submission ID with `notarytool log` to investigate; a first submission may need more time.

The project's current macOS formal-signing path has been implemented but has not been exercised with real credentials. Windows installers and Linux packages are currently unsigned.

## GitHub Actions

[CI](../.github/workflows/ci.yml) builds, tests, formats, and runs Clippy on macOS arm64, Windows x64, and Linux x64 using native dependencies. Actions are pinned to full commit SHAs.

[Release packages](../.github/workflows/release.yml) runs on an existing `v<version>` tag push or a manual dispatch with an existing tag. It verifies that the tag matches the root package version and builds all four target variants from the same commit. It does not run on ordinary main-branch pushes.

Manual dispatch defaults to `macos_signing=development` and `create_draft=true`. Set `create_draft=false` to retain packages only as workflow artifacts. Tag builds use the repository variable `MACOS_SIGNING_MODE`, which defaults to `development`.

To enable `developer-id` builds, configure these repository values:

| Kind | Name | Content |
| --- | --- | --- |
| Variable | `MACOS_SIGNING_MODE` | `developer-id` for signed tag builds |
| Variable | `MACOS_SIGNING_IDENTITY` | Full Developer ID Application identity |
| Secret | `MACOS_CERTIFICATE_BASE64` | Base64-encoded exported signing certificate/private key (`.p12`) |
| Secret | `MACOS_CERTIFICATE_PASSWORD` | Password for that `.p12` |
| Secret | `MACOS_NOTARY_API_KEY_BASE64` | Base64-encoded App Store Connect **team** API key (`.p8`) |
| Secret | `MACOS_NOTARY_KEY_ID` | Team API key ID |
| Secret | `MACOS_NOTARY_ISSUER_ID` | Team API key issuer UUID |

Each signed macOS job imports credentials into a temporary keychain with a generated password and stores a validated notarization profile there. It removes temporary key files and deletes the keychain in its cleanup step. Signing credentials are available only to the signed macOS jobs.

After all builds pass, the workflow collects packages, generates `SHA256SUMS`, and creates a **draft** release with `gh release create --draft --verify-tag`. Only this final job has `contents: write`. It requires an existing tag, leaves an existing release for manual review, and never changes a release to published.

Before publishing a draft, verify installation/launch on the intended platforms and resolve the permission noted in [the icon provenance](../build-assets/icon/README.md). Downloaded development ZIPs, unsigned installers, and the retained brand-asset permission state are described in the draft notes. Creating tags and publishing a reviewed draft are separate maintainer actions.

References: [GitHub runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub certificate import](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications), [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Apple notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow), and [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).
