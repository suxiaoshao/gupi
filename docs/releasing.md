# Release packaging

Gupi uses `tauri-bundler` to create macOS application ZIP and DMG packages, nine localized Windows x64 MSI installers, and a Linux x64 Debian package. The root `Cargo.toml` owns the product version and bundle metadata; the bundle identifier remains `top.sushao.gupi`. Pi is an external runtime and is not included in these packages.

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
| macOS | `aarch64-apple-darwin` | `Gupi_<version>_aarch64_macos_development.{dmg,zip}` |
| macOS | `x86_64-apple-darwin` | `Gupi_<version>_x86_64_macos_development.{dmg,zip}` |
| Windows | `x86_64-pc-windows-msvc` | `Gupi_<version>_x64_<culture>.msi` |
| Linux | `x86_64-unknown-linux-gnu` | `Gupi_<version>_amd64.deb` |

`rustup target add <triple>` installs the Rust standard library when needed. Cross-OS packaging is unsupported; the release workflow uses native runners for every target.

Packages are exported to `dist/<triple>/`. Build outputs are under `<target-root>/<triple>/release/`; bundle staging is `<target-root>/xtask-bundle/<triple>/release/`. The macOS app remains at `<target-root>/xtask-bundle/<triple>/release/bundle/macos/Gupi.app` for local launch. `<target-root>` is `target/` by default; `CARGO_TARGET_DIR` overrides it, and relative values resolve from the project root. Rebuilding replaces only that target's staging and exported packages.

Windows `--install` opens the English MSI, falling back to another MSI if English is unavailable. Other hosts reject this option. Linux dependencies come from `dpkg-shlibdeps`, plus the Vulkan/EGL/Wayland libraries loaded dynamically by GPUI. Build Linux release packages on Ubuntu 22.04 to preserve that native library baseline.

## Bundle resources

xtask derives iconsets and ICO files from `build-assets/icon/app-icon.png` in temporary directories. It compiles the seven macOS icon themes when the installed Xcode supports them, with the existing ordinary-icon fallback. Localization resources map the nine declared application locales to macOS `.lproj` directories and WiX cultures; Windows exports one MSI per culture.

The minimum macOS bundle version defaults to 11.0 and follows `MACOSX_DEPLOYMENT_TARGET` when configured. The workflow sets it to 11.0 for both architectures. The Liquid Glass icon compilation target is separate from the application's minimum OS version.

## macOS signature modes

The default `development` mode applies an ad-hoc signature after icon injection, then verifies the bundle and exports ZIP and DMG files whose names end in `_development`. The DMG contains the finalized `Gupi.app` and an `Applications` symlink; open it and drag Gupi into Applications. The development DMG itself is unsigned. It has no Developer ID or notarization ticket and remains subject to Gatekeeper restrictions.

The `developer-id` mode requires an installed **Developer ID Application** identity and an existing `notarytool` keychain profile. Set up the profile in Keychain using Apple's documented `xcrun notarytool store-credentials` workflow. Then build:

```sh
CARGO_TARGET_DIR=target/native MACOSX_DEPLOYMENT_TARGET=11.0 \
  cargo run -p xtask --locked -- bundle \
  --macos-signing developer-id \
  --signing-identity 'Developer ID Application: Your Name (TEAMID)' \
  --notary-profile gupi-notary
```

`--keychain /path/to/keychain-db` selects a keychain containing both the identity and profile. `--entitlements /path/to/entitlements.plist` optionally adds the entitlements required by a particular distribution configuration; relative paths resolve from the project root.

Formal signing happens after all bundle mutations. xtask signs nested Mach-O files/code bundles from the inside out using hardened runtime and a secure timestamp, verifies the result, submits a ZIP to Apple, requires an `Accepted` status, staples and validates the ticket, and runs Gatekeeper assessment. It then exports a ZIP and creates a compressed, read-only HFS+ DMG from that stapled app. The DMG receives its own Developer ID signature, notarization submission, stapled ticket and Gatekeeper assessment. Neither final filename carries the `_development` suffix. A failed packaging or notarization step fails the job; no packages are uploaded. Notarization waits up to 30 minutes; a timeout or rejection fails the build. Use the reported submission ID with `notarytool log` to investigate; a first submission may need more time.

The project's current macOS formal-signing path has been implemented but has not been exercised with real credentials. Windows installers and Linux packages are currently unsigned.

## GitHub Actions

[CI](../.github/workflows/ci.yml) builds, tests, formats, and runs Clippy on macOS arm64, Windows x64, and Linux x64 using native dependencies. Actions are pinned to full commit SHAs.

[Release packages](../.github/workflows/release.yml) runs on an existing `v<version>` tag push or a manual dispatch with an existing tag. It verifies that the tag matches the root package version and builds all four target variants from the same commit in `target/native`. Linux remains in the release matrix; this change adds no Linux functionality. Release versions use three numeric components (`MAJOR.MINOR.PATCH`), within MSI limits (255, 255, 65535); prerelease/build suffixes are not supported by this release contract. It does not run on ordinary main-branch pushes.

Manual dispatch defaults to `macos_signing=development` and `create_draft=true`. Set `create_draft=false` to retain packages and generated distribution manifests only as workflow artifacts. Tag builds use the repository variable `MACOS_SIGNING_MODE`, which defaults to `development`.

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

Release logic lives in Rust `xtask`: `release prepare` verifies the tag and version; `bundle` owns package construction and required signatures; `release collect` checks the combined artifact set once, hashes the files, reads MSI identity directly and generates distribution manifests. GitHub Actions coordinates the runners and uploads. The only release shell script imports temporary macOS signing credentials.

Workspace tests run in normal CI. Tag builds do not repeat the complete test suite, mount DMGs again, or install/uninstall MSI packages on every release. First-public-release validation and changes to installer behavior should include a targeted native install, launch and upgrade check; an English MSI install/uninstall without launch or an old-version upgrade would provide only limited evidence.

After all builds pass, the workflow requires the complete set of **14 packages** (two macOS architectures × ZIP/DMG, nine Windows MSI languages, one Linux deb), generates `SHA256SUMS`, and creates a **draft** release with `gh release create --draft --verify-tag`. Only the collection job has `contents: write`. It requires an existing tag, leaves an existing release for manual review, and never changes a release to published.

Before publishing a draft, verify installation/launch on the intended platforms. [Third-party notices](../THIRD_PARTY_NOTICES.md) records the Pi icon sources, attribution, adaptations, and upstream MIT license and is included as a resource in every platform package. See [the icon provenance](../build-assets/icon/README.md) for asset details. Draft notes identify development DMGs/ZIPs and unsigned installers. Creating tags and publishing a reviewed draft are separate maintainer actions.

References: [GitHub runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub certificate import](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications), [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Apple notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow), and [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).

## Create a versioned release

1. Update the root package version and its `Cargo.lock` entry, run CI, and merge the release changes. The workflow file and scripts must be present at the tagged commit.
2. Create and push the matching tag from that commit; for example, when `Cargo.toml` declares `0.1.0`:

   ```sh
   git tag -a v0.1.0 -m "Gupi 0.1.0"
   git push origin v0.1.0
   ```

3. Follow **Release packages** in Actions. Tag creation triggers all builds, package checks, distribution-manifest generation and a draft Release automatically. No automatic version increment or tag creation is performed.
4. Download the packages and verify intended-platform launch. Review the release notes and publish the draft when ready, selecting **Set as the latest release** for the stable version users should receive. The draft is created with `--latest=false`; the application reads GitHub's `/releases/latest` endpoint, so merely pushing a tag or leaving a draft does not announce an update. Tag automation intentionally stops at the draft.

To retry a failed tag build, rerun failed jobs in Actions or dispatch the existing tag:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0 \
  -f macos_signing=development -f create_draft=false
```

For a formal run, use `macos_signing=developer-id` after configuring the values above. A dispatch still checks out the tag's exact commit. Prefer rerunning the same workflow run when only one platform failed. The workflow refuses to replace an existing draft or published release; inspect the failure and existing assets before retrying release creation. Rebuilding an MSI can change its product code and hash, so always use manifests from the same successful run as the published assets.

## Release commands

Run from the repository root with the matching version. `prepare` requires the tag to exist and point to HEAD. It emits job outputs when `GITHUB_OUTPUT` is set. `collect` expects the four platform artifacts to have been downloaded into one flat `dist/` directory; it replaces generated `distribution/` output.

```sh
cargo run -p xtask --locked -- release prepare --tag v0.1.0
cargo run -p xtask --locked -- release collect --tag v0.1.0
# Also create a draft on GitHub (requires authenticated gh):
cargo run -p xtask --locked -- release collect --tag v0.1.0 --draft
```

Both commands accept `--macos-signing developer-id` for formal packages. `collect` always writes `dist/SHA256SUMS` and distribution manifests; only `--draft` writes to GitHub. The repository URL comes from the root package manifest. Bundle arguments may also be supplied through `GUPI_TARGET`, `GUPI_MACOS_SIGNING`, `GUPI_MACOS_SIGNING_IDENTITY`, `GUPI_MACOS_NOTARY_PROFILE` and `GUPI_MACOS_KEYCHAIN`, as used by the workflow.

## Application update signing

Gupi uses Sparkle 2.9.6 on macOS (compatible with the macOS 11 baseline) and WinSparkle 0.9.4 on Windows. The build downloads the official SDK archive into `target/updater/`, verifies its pinned SHA256, and bundles its framework or DLL and license. Windows also includes `gupi-update-helper.exe` for waiting until Gupi exits, running MSI, and relaunching the application. SDK preparation and appcast signing are Rust xtask commands.

Generate one long-lived Ed25519 seed **outside the repository**, with a private backup:

```sh
cargo run -p xtask --locked -- updater keygen /absolute/private/path/gupi-update.key
```

The file contains a base64-encoded 32-byte private seed compatible with Sparkle's current exported key format. The command prints only the public key and refuses to overwrite a key. On Unix the new private file has mode `0600`. Keep the private file and its backup out of Git, release assets, and build logs.

Configure these GitHub repository values before building the first update-capable release:

| Setting | Value |
| --- | --- |
| Variable `GUPI_UPDATE_PUBLIC_KEY` | The printed base64 public key |
| Secret `GUPI_UPDATE_PRIVATE_KEY` | The contents of the private seed file |

For local packaging, export `GUPI_UPDATE_PUBLIC_KEY` and use `GUPI_UPDATE_KEY_FILE=/absolute/private/path/gupi-update.key` during `release collect`. Collection derives the public key from the seed and rejects a mismatch. Build jobs receive only the public key; the private key is limited to the collection step.

With this configuration, collection signs the exact bytes of the two formal macOS ZIPs and nine localized Windows MSIs, and uploads `appcast-macos-{aarch64,x86_64}.xml` and `appcast-windows-<locale>.xml` with the draft. These are standard Sparkle appcasts with Ed25519 `sparkle:edSignature` attributes, served over HTTPS. Development macOS archives are not advertised as an update channel. All ordinary DMG/ZIP/MSI/deb artifacts remain available as before. The Homebrew Cask declares `auto_updates true` only when update signing is configured.

The app opens the **specific version's** appcast after the user chooses installation. Publishing that version must include its appcasts and matching packages. Do not replace either asset under an existing version. A GitHub draft or a tag alone is not an update. Do not rotate the public key casually: already-installed clients trust the old key, so key rotation requires the native engine's supported migration procedure.

Builds without a public key keep check-for-updates and manual downloads. They cannot install updates in-app; Windows also requires a recognized MSI installation. Ed25519 signing authenticates update packages and does not replace macOS Developer ID/notarization or Windows Authenticode signing. Validate an actual old-to-new update on each intended platform when changing the updater or installer integration, rather than adding repeated install/uninstall checks to every tag.

## Homebrew and WinGet

The collection job produces a **distribution-manifests** Actions artifact containing reviewable files generated from the actual packages, not placeholder URLs or hashes. Nothing is submitted to an external repository. Package artifacts expire after 14 days; manifest artifacts after 30 days. Download the manifests when preparing a release.

- Homebrew: `homebrew/Casks/g/gupi.rb`, with architecture-specific DMG URLs and SHA256. Generated only in `developer-id` mode; development runs omit the Cask so unsigned/unnotarized downloads are not presented as a supported Homebrew installation.
- WinGet: `winget/manifests/s/suxiaoshao/Gupi/<version>/`, containing version, default English locale and installer manifests. All nine MSI languages are listed. The cross-platform Rust MSI reader extracts ProductCode, UpgradeCode and publisher directly from each installer, checking its version, architecture, scope and language against bundle settings. Hashes are calculated from those same files; there is no separate metadata artifact or Windows inspection script. The proposed identifier is `suxiaoshao.Gupi`; it is not yet registered in the community repository.

These manifests reference the corresponding GitHub Release download URLs, which become available after publication. Only publish package-manager changes for a stable, public Release. Never replace assets under a published version; use a new version for changed binaries. Pi remains a separate prerequisite; neither channel silently installs it or deletes Pi data on uninstall.

### Homebrew: maintain a tap

Start with a maintainer repository such as `suxiaoshao/homebrew-tap`. It does not exist as part of this code change and needs to be created before the following command can work. Copy the generated `Casks/g/gupi.rb` into that repository after publishing the signed release. On macOS, audit the Cask, install and launch Gupi, then verify uninstall and upgrade from the previous available release:

```sh
brew tap suxiaoshao/tap
brew audit --cask --online suxiaoshao/tap/gupi
brew install --cask suxiaoshao/tap/gupi
brew upgrade --cask suxiaoshao/tap/gupi
brew uninstall --cask suxiaoshao/tap/gupi
```

The Cask uses the normal `app` install and has no `zap` stanza, preserving configuration and Pi sessions. Run installation on both Apple Silicon and Intel before advertising both. Once the tap is working, a stable-release publication job can open a PR updating only this Cask with a GitHub App or token scoped to the tap; the current workflow needs no cross-repository credential. Central `homebrew/cask` submission can be considered separately under its acceptance rules.

### WinGet: submit manifests

After the same version is publicly downloadable, use Windows with WinGet and the official [WinGet manifest creator](https://github.com/microsoft/winget-create). Validate the generated directory and test installation in Windows Sandbox or another disposable machine. Local-manifest installation requires the WinGet `LocalManifestFiles` setting:

```powershell
winget validate --manifest .\winget\manifests\s\suxiaoshao\Gupi\0.1.0
# In an elevated terminal on the disposable test machine:
winget settings --enable LocalManifestFiles
winget install --manifest .\winget\manifests\s\suxiaoshao\Gupi\0.1.0
winget uninstall --id suxiaoshao.Gupi --exact
```

Verify launch, each intended installer language, upgrade from the previous release when available, and retention of preferences/session data. Then submit the exact directory with `wingetcreate submit <manifest-directory>` or a PR to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs), and address its automated validation. GitHub authentication is required for submission. `winget install --id suxiaoshao.Gupi --exact` becomes a public installation path only after acceptance and source indexing.

For later stable versions, the same generator produces manifests with new MSI identity and hashes. A publication workflow can submit an update PR via `wingetcreate` after the first package is accepted. Keep submission failures independent of the already-published GitHub Release and retry with the same immutable assets. Windows packages are currently unsigned; Authenticode signing and actual community acceptance are not claimed by this implementation.

References: [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook), [custom taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap), [WinGet manifest format](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest), and [WinGet authoring and validation](https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md).
