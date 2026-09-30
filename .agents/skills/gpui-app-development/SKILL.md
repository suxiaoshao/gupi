---
name: gpui-app-development
description: Choose app structure, shared ownership, and relevant skills for Gupi development.
---

# GPUI App Development

Gupi is the root package. Follow its existing `app`, `foundation`, `features`, and `state` boundaries. Product policy belongs in `src/`. Pi RPC transport, window helpers, Tokio integration and Lucide icons live in the matching internal `crates/`. Theme, Form, Operation, Store and platform helpers are Git dependencies pinned in the root manifest; inspect their locked source before changing integration.

Keep one authority per business fact. Derive cheap values rather than caching them; a necessary cache needs clear invalidation. Let a retained task or runtime variant express activity without parallel loading flags.

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
