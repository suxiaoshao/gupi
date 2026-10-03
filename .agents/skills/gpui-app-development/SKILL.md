---
name: gpui-app-development
description: Choose app structure, shared ownership, and relevant skills for Gupi development.
---

# GPUI App Development

Use the `gpui-kit` skill and the official [Coding Guides](https://gpui-kit.com/docs/coding-guides/)
for architecture, state ownership, file organization, naming, and public APIs.
Do not maintain a competing set of framework rules here. The official skill is
installed and maintained through `npx skills`; keep Gupi-specific guidance in
this skill and project documents.

Gupi is the root application package. [AGENTS.md](../../../AGENTS.md#项目结构)
holds project conventions. The [capability architecture](../../../docs/dev/capability-architecture/README.md)
describes the six internal capability crates and their host interfaces. Keep
native windows, notification delivery, global shortcut registration and ordered
shutdown in the application composition layer. Change an owning capability and
its callers together; do not restore the removed `state` / `foundation` facade.

Pi owns model execution, extensions, configuration loading, and session content
writes. Gupi owns desktop interaction and connection lifecycles; preserve the
[product boundary](../../../docs/gui-boundary.md). Internal crate membership and
dependency sources are defined in the root manifest. Shared services use a pinned
Git revision; inspect the locked source before changing integration.

Applications enter through `gpui_kit::application()` and `gpui_kit::init(cx)`. Dependency paths and versions follow the root manifest.

Use the relevant focused skill:

- `gpui-kit`: framework APIs, components, state ownership and coding guides.
- `gpui-kit-design-guides`: component composition and visible UI design.
- `gpui-app-icon-usage`, `gpui-i18n`: resources and localized text.
- `gpui-store`, `gpui-operation`, `gpui-form`: implementation or deliberate integration of those crates. Ordinary state, async code or inputs do not require adopting them.
- `gpui-computer-use-debugging`: runtime behavior that needs direct observation.

## Dependency integration

Applications import `gpui_kit` and `gpui_kit::component`; shared crate aliases
follow the root manifest. Official skills may describe newer APIs: verify the
locked dependency source before using them.

- Shared `app-theme` owns editor and Markdown syntax colors. Do not force reparsing
  or create another app-specific syntax palette just to update colors.
- For gradient-capable surfaces use `Theme.tokens.<role>.background`; use semantic
  `Hsla` fields for text, icons, borders and low-level painting.
- Form/domain owns business values, catalog owns available options, and native
  component entities own focus, query, scroll and popup state. Replacing options
  must not become user input or choose a business fallback. After replacing a
  Combobox delegate, project the authoritative selection explicitly if required.
- Use existing gpui-form bindings for source suppression and deferred writes.
  Do not duplicate their feedback-loop routing in app render callbacks.
