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

`--keychain /path/to/keychain-db` selects a keychain containing both the identity and profile. The keychain must also be in the user search list so `codesign` can resolve intermediate certificates; an explicit `--keychain` path alone does not configure that search list. `--entitlements /path/to/entitlements.plist` optionally adds the entitlements required by a particular distribution configuration; relative paths resolve from the project root.

Formal signing happens after all bundle mutations. xtask signs nested Mach-O files/code bundles from the inside out using hardened runtime and a secure timestamp, verifies the result, submits a ZIP to Apple, requires an `Accepted` status, staples and validates the ticket, and runs Gatekeeper assessment. It then exports a ZIP and creates a compressed, read-only HFS+ DMG from that stapled app. The DMG receives its own Developer ID signature, notarization submission, stapled ticket and Gatekeeper assessment. Neither final filename carries the `_development` suffix. A failed packaging or notarization step fails the job; no packages are uploaded. Notarization waits up to 30 minutes; a timeout or rejection fails the build. Use the reported submission ID with `notarytool log` to investigate; a first submission may need more time.

The v0.1.0 macOS arm64 and Intel packages passed Developer ID signing, Apple notarization, stapling, and Gatekeeper assessment. The arm64 DMG was also used for an isolated installation and startup check. Windows installers and Linux packages are currently unsigned.

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

Each signed macOS job imports credentials into a temporary keychain with a generated password, adds it to the job's user search list, and stores a validated notarization profile there. It removes temporary key files and deletes the keychain in its cleanup step. Signing credentials are available only to the signed macOS jobs.

Release logic lives in Rust `xtask`: `release prepare` verifies the tag and version; `bundle` owns package construction and required signatures; `release collect` checks the combined artifact set once, hashes the files, reads MSI identity directly and generates distribution manifests. GitHub Actions coordinates the runners and uploads. The only release shell script imports temporary macOS signing credentials.

Workspace tests run in normal CI. Tag builds do not repeat the complete test suite, mount DMGs again, or install/uninstall MSI packages on every release. First-public-release validation and changes to installer behavior should include a targeted native install, launch and upgrade check; an English MSI install/uninstall without launch or an old-version upgrade would provide only limited evidence.

After all builds pass, the workflow requires the complete set of **14 packages** (two macOS architectures × ZIP/DMG, nine Windows MSI languages, one Linux deb), generates `SHA256SUMS` and `Gupi_<version>_distribution.tar.gz`, and creates a **draft** release with `gh release create --draft --verify-tag`. The archive contains the same reviewable Cask and WinGet manifests as `distribution/`, with normalized file metadata. Only the collection job has `contents: write`. It requires an existing tag, leaves an existing release for manual review, and never changes a release to published.

Before publishing a draft, verify installation/launch on the intended platforms. [Third-party notices](../THIRD_PARTY_NOTICES.md) records the Pi icon sources, attribution, adaptations, and upstream MIT license and is included as a resource in every platform package. See [the icon provenance](../build-assets/icon/README.md) for asset details. Draft notes identify development DMGs/ZIPs and unsigned installers. Creating tags and publishing a reviewed draft are separate maintainer actions.

For v0.1.0, native checks covered the macOS arm64 package and the Simplified Chinese MSI on Windows 11 x64: initial setup, main window and settings, preference persistence, quit/relaunch, and Pi detection. Windows validation also covered first-install UAC and manual update checking while no public release existed. Both macOS architectures passed artifact verification; all 11 update feeds passed Ed25519 verification against the actual packages. Windows old-to-new native updating, physical Intel Mac runtime behavior, Linux desktop runtime behavior, and package-manager installation/upgrade/uninstall remain outside this validation. First installation and update discovery do not establish that the native upgrade handoff works.

References: [GitHub runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub certificate import](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications), [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Apple notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow), and [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).

## Create a versioned release

1. Update the root `gupi` package version and its `Cargo.lock` entry, run CI, and merge the release changes. This version determines the application's About version, installer version and update comparison. Internal crate versions and the Rust compiler in `rust-toolchain.toml` do not need to change with each application release. The workflow file and scripts must be present at the tagged commit.
2. Create and push the matching tag from that commit; for example, when `Cargo.toml` declares `0.1.0`:

   ```sh
   git tag -a v0.1.0 -m "Gupi 0.1.0"
   git push origin v0.1.0
   ```

3. Follow **Release packages** in Actions. Tag creation triggers all builds, package checks, distribution-manifest generation and a draft Release automatically. No automatic version increment or tag creation is performed.
4. Download the packages and verify intended-platform launch. Review the release notes and publish the draft when ready, selecting **Set as the latest release** for the stable version users should receive. The draft is created with `--latest=false`; the application reads GitHub's `/releases/latest` endpoint, so merely pushing a tag or leaving a draft does not announce an update. Tag automation intentionally stops at the draft.
5. Update download links and installation instructions in both READMEs, verify the public package and appcast URLs, and record any outstanding validation or distribution work in the issue. Keep release-status tracking out of the product documentation.

To retry a failed tag build, rerun failed jobs in Actions or dispatch the existing tag:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0 \
  -f macos_signing=development -f create_draft=false
```

For a formal run, use `macos_signing=developer-id` after configuring the values above. A dispatch still checks out the tag's exact commit. Prefer rerunning the same workflow run when only one platform failed. The workflow refuses to replace an existing draft or published release; inspect the failure and existing assets before retrying release creation. Rebuilding an MSI can change its product code and hash, so always use manifests from the same successful run as the published assets.

## Verify an old-to-new application update

Start with the previous public release installed through its signed macOS package or Windows MSI. Use that published binary as the update client; a freshly built development app or direct installation of the new package does not validate the upgrade path.

Before publication, check the new packages and their matching appcast signatures and perform targeted startup checks. The production client discovers updates through GitHub's public `/releases/latest` endpoint, so complete the real update check after publishing the new release as Latest. Drafts and tags alone are not discoverable. Keep the existing update signing key and never replace published assets to make a test pass.

- With automatic checks enabled, launch the old app and allow its initial ten-second delay plus network time. Confirm the new version is offered; also verify manual checking.
- Before installing, exercise cancel and skip. Cancel must leave the old app usable. A skipped version must remain suppressed for background notices after relaunch, while manual checking still finds it.
- Choose the in-app download/install action. On macOS, verify Sparkle replaces the app and relaunches it. On Windows, verify the helper/MSI handoff, any UAC prompt, and automatic relaunch; check that the old installation is upgraded and the handoff's temporary directory is cleaned up.
- In the relaunched app, verify the new About version, retained preferences, conversations and drafts, and that another update check reports the current version.

Record the actual old/new versions, platform, package identity and results. A successful build, verified signature, first installation, or update notification alone is not a completed automatic upgrade. Homebrew/WinGet upgrades are separate channel checks; Linux currently uses manual package installation rather than this native update path.

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

Builds without a public key keep check-for-updates and manual downloads. Development macOS bundles also use this path, even when a public key is configured: only `developer-id` bundles with a public key embed Sparkle. Windows also requires a recognized MSI installation for in-app updates. Ed25519 signing authenticates update packages and does not replace macOS Developer ID/notarization or Windows Authenticode signing. Validate an actual old-to-new update on each intended platform when changing the updater or installer integration, rather than adding repeated install/uninstall checks to every tag.

## Homebrew and WinGet

The collection job produces `Gupi_<version>_distribution.tar.gz` as a permanent release asset and a **distribution-manifests** Actions artifact. Both contain reviewable files generated from the actual packages, not placeholder URLs or hashes. Package artifacts expire after 14 days and manifest artifacts after 30 days; channel updates use the release asset so retries do not depend on Actions retention. Collection itself does not submit to external repositories. The v0.1.0 archive was backfilled from its successful build's manifests, with Homebrew formatting corrected; its installers and update feeds were not replaced.

- Homebrew: `homebrew/Casks/g/gupi.rb`, with architecture-specific DMG URLs and SHA256. Generated only in `developer-id` mode; development runs omit the Cask so unsigned/unnotarized downloads are not presented as a supported Homebrew installation.
- WinGet: `winget/manifests/s/suxiaoshao/Gupi/<version>/`, containing version, default English locale and installer manifests. All nine MSI languages are listed. The cross-platform Rust MSI reader extracts ProductCode, UpgradeCode and publisher directly from each installer, checking its version, architecture, scope and language against bundle settings. Hashes are calculated from those same files; there is no separate metadata artifact or Windows inspection script. Machine-wide MSI installation declares `ElevationRequirement: elevationRequired`. The identifier `suxiaoshao.Gupi` is pending community inclusion.

These manifests reference the corresponding GitHub Release download URLs, which become available after publication. Only publish package-manager changes for a stable, public Release. Never replace assets under a published version; use a new version for changed binaries. Pi remains a separate prerequisite; neither channel silently installs it or deletes Pi data on uninstall.

### Homebrew: maintainer tap

The public [suxiaoshao/homebrew-tap](https://github.com/suxiaoshao/homebrew-tap) repository owns the distributed Cask. Gupi's Rust xtask remains its generator; correct generation issues here. The Cask uses architecture-specific signed DMGs, declares `depends_on :macos`, and has no `zap` stanza, preserving preferences and Pi data on uninstall.

```sh
brew tap suxiaoshao/tap
brew audit --cask --online suxiaoshao/tap/gupi
brew install --cask suxiaoshao/tap/gupi
brew upgrade --cask --greedy suxiaoshao/tap/gupi
brew uninstall --cask suxiaoshao/tap/gupi
```

The tap's **Update Gupi** workflow checks the latest public stable release every six hours. Its manual trigger also accepts an existing stable tag. It rejects draft/prerelease versions and downgrades, downloads the matching permanent manifest archive, runs Homebrew style and online audit, and opens a versioned PR. Repeated runs reuse an existing proposal; a failed PR creation can be retried against the existing unchanged branch. Review and merge the PR to make the version available. Central `homebrew/cask` submission is separate.

The workflow uses only the tap repository's `GITHUB_TOKEN`, with `contents: write` and `pull-requests: write`; Gupi does not hold a cross-repository token. GitHub's **Allow GitHub Actions to create and approve pull requests** setting must be enabled in the tap. The workflow does not approve or merge PRs. GitHub may require maintainer approval for additional PR workflows created by `GITHUB_TOKEN`; style and audit already run in the update job before it creates the PR. Scheduled workflows can be delayed or disabled by GitHub's public-repository inactivity policy; use the manual trigger when needed.

Validate installation, launch, uninstall and data retention when introducing a channel or changing installer behavior. Record the actual architectures and upgrade paths tested. `auto_updates true` advertises the app's own updater; `--greedy` explicitly includes this Cask in Homebrew upgrades. Homebrew upgrade checks and the application's native update handoff are separate validations.

### WinGet: submit manifests

Start from the permanent manifest archive of a public, stable Release. Compare its hashes and MSI identity with the published installers. Check the community directory and open PRs for the same identifier/version before creating a version branch in a fork. For v0.1.0, the community submission adds `ElevationRequirement: elevationRequired` to the archived manifest; the published archive and installers remain unchanged. The generator includes this field for subsequent releases. Keep schema 1.10.0 unless validation or community requirements require a change.

Read the current [authoring](https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md) and [validation](https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md) requirements. Validate the directory and install it on the Windows machine used for verification. Local installation requires enabling `LocalManifestFiles` with administrator approval; restore its previous state after testing. Keep installer hash, certificate and malware checks enabled, including when adapting community test helpers:

```powershell
winget validate --manifest .\winget\manifests\s\suxiaoshao\Gupi\0.1.0
# In an elevated terminal on the Windows test machine:
winget settings --enable LocalManifestFiles
winget install --manifest .\winget\manifests\s\suxiaoshao\Gupi\0.1.0 --locale zh-CN
winget list --name Gupi --exact
# Before public indexing, use the installed MSI ProductCode from its ARP record:
winget uninstall --product-code '<installed ProductCode>' --exact
# If LocalManifestFiles was previously disabled:
winget settings --disable LocalManifestFiles
```

Check MSI metadata for all nine languages; prioritize Simplified Chinese and English for interactive installation, launch, About/version, preference persistence and uninstall checks. Verify that uninstall removes the program and Start menu entry while retaining configuration and Pi data. Test a channel upgrade only when a real previous release exists. These checks do not validate the application's native download, MSI handoff or restart after an update.

Submit only the three YAML files for that package version to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs), using a fork PR or the existing [WinGet manifest creator's submit command](https://github.com/microsoft/winget-create/blob/main/doc/submit.md). Address automated validation and moderator feedback. Local installation, community approval and source indexing are separate milestones. After indexing, run `winget source update --name winget`, then verify `winget show --id suxiaoshao.Gupi --exact --source winget` and installation by the same public ID on the test machine. Check installed-version recognition and uninstall again before publishing WinGet usage commands in both READMEs. Until then, keep their status as pending inclusion.

The v0.1.0 local manifests passed validation, Simplified Chinese and English installation, installed-version recognition, uninstall and test-data retention on Windows 11 Pro x64 (10.0.26200.9550), using WinGet v1.29.380. The Chinese installation also opened successfully. WinGet identified both installations through their local ARP records; public-source installation remains pending community inclusion. This channel check does not repeat the application feature validation above or establish an old-to-new upgrade path.

For later versions, reuse xtask's archived manifests and `wingetcreate submit` or the existing GitHub CLI submission flow; do not add another MSI parser or regenerate identities. Connect automated submissions after the first package has passed validation and the public source path has been tested. The workflow should select public stable Releases only, reject drafts/prereleases and versions at or below the latest community version, and check both existing versions and open PRs before proposing a deterministic version branch. Retries must reuse an unchanged branch/PR and immutable release assets, never replace an older community version. Run validation before opening the PR; maintainers review it and community moderators decide acceptance.

WinGet PR automation is not connected yet. Cross-repository submission cannot use Gupi's repository-scoped `GITHUB_TOKEN` alone. Before enabling it, review the workflow and its credential permissions: release/metadata reads, writes only to the submitting fork, and permission to open the upstream PR. Prefer a narrowly scoped GitHub App where supported; if using the manifest creator's token flow, review its documented token requirements and secure storage. Do not pass a token on the command line or include it in logs. Submission failures must remain independent of the already-published GitHub Release. Windows packages are currently unsigned; Authenticode signing and community acceptance are not implied by manifest validation.

References: [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook), [custom taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap), [WinGet manifest format](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest), and [WinGet authoring and validation](https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md).
