# xtask

Run packaging outside the Nix shell with native rustup and Xcode or native Linux libraries. Build Gupi from the project root:

```sh
cargo run -p xtask --locked -- bundle
```

The command builds the root `gupi` package for the native target with the `bundled` feature and exports packages to `dist/<target-triple>/`. This feature makes automatic update checks default to on; ordinary source builds default to off. An explicit saved preference overrides either default. Use `--target` for an explicit supported target. Windows `--install` opens the English MSI.

xtask derives platform icons from `build-assets/icon/app-icon.png` in temporary staging. macOS icon themes, bundle identity, and nine declared macOS/WiX localizations are retained. macOS bundles receive their final signature after icon injection. Development packages use an ad-hoc signature and `_development.zip` and `_development.dmg` filenames. DMGs contain the finalized app and an Applications link; formal builds also sign, notarize and staple the disk image.

The same Rust tool owns release preparation and collection:

```sh
cargo run -p xtask --locked -- release prepare --tag v0.1.0
cargo run -p xtask --locked -- release collect --tag v0.1.0
```

`prepare` verifies the existing tag against HEAD and the root package version. After all platform packages are downloaded into flat `dist/`, `collect` checks the combined set, writes SHA256SUMS and regenerates `distribution/` with Homebrew/WinGet manifests. It reads MSI databases directly on any host. Add `--draft` to create the GitHub draft through `gh`; without it the command only writes local artifacts. Both commands accept `--macos-signing developer-id`.

See [Release packaging](../../docs/releasing.md) for native build requirements, exact output paths, signing, and draft release automation.

Native updates use pinned Sparkle/WinSparkle SDKs and standard signed appcasts. Set `GUPI_UPDATE_PUBLIC_KEY` for builds and the matching `GUPI_UPDATE_PRIVATE_KEY` (or `GUPI_UPDATE_KEY_FILE`) during collection. Generate a private seed once, outside the repository, with `cargo run -p xtask --locked -- updater keygen /absolute/private/path/gupi-update.key`; only the public key is printed. The command never overwrites an existing key. `updater prepare macos` and `updater prepare windows` populate the checksum-verified SDK cache under `target/updater/`.
