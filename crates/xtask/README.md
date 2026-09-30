# xtask

Run packaging outside the Nix shell with native rustup and Xcode or native Linux libraries. Build Gupi from the project root:

```sh
cargo run -p xtask -- bundle
```

The command builds the root `gupi` package for the native target and exports packages to `dist/<target-triple>/`. Use `--target` for an explicit supported target. Windows `--install` opens the English MSI.

xtask derives platform icons from `build-assets/icon/app-icon.png` in temporary staging. macOS icon themes, bundle identity, and nine declared macOS/WiX localizations are retained. macOS bundles receive their final signature after icon injection. Development packages use an ad-hoc signature and a `_development.zip` filename.

See [Release packaging](../../docs/releasing.md) for native build requirements, exact output paths, signing, and draft release automation.
