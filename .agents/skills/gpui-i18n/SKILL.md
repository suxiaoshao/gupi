---
name: gpui-i18n
description: Implement or review user-visible text, Fluent locales, language settings, or macOS bundle localization in Gupi.
---

# GPUI I18n

## Runtime text

- Gupi owns `gupi_settings::i18n`, Fluent bundles and an `I18n` global. UI uses `gupi_settings::i18n::t(cx, key)` or `t_with_args(cx, key, args)`; applying a changed language rebuilds that global.
- Runtime locale files are `locales/{de,en-US,es,fr,ja,ko,pt-BR,zh-CN,zh-TW}/main.ftl`. Keep changed keys and interpolation variables aligned across all nine languages.
- Use semantic keys and the app's naming convention. Use `FluentArgs` for interpolation; avoid composing sentences from translated fragments with `format!`.
- User-facing Rust literals are reserved for intentionally unlocalized text; debug/test strings are separate. Translation falls back from the selected language to English, then to the key itself. A raw key is not acceptable shipped copy.

## macOS bundle text

- Bundle strings live under `locales/macos/<locale>.lproj/InfoPlist.strings`; Simplified and Traditional Chinese use `zh-Hans` and `zh-Hant`.
- `crates/xtask/src/bundle/settings.rs` maps these resources; `bundle/macos.rs` sets `CFBundleAllowMixedLocalizations` and `CFBundleLocalizations`. Keep runtime and bundle locale declarations aligned.
- Text-only edits use key/variable parity checks. Bundle localization logic changes use affected xtask coverage; text-only edits do not require bundling.
