# Product screenshots / 产品截图

These are captures of Gupi's actual macOS UI, using an isolated sample project and authored demonstration sessions. The sample answers are illustrative content, not a model benchmark or a record of real user work. The UI itself is not generated or retouched.

这些图片来自真实的 Gupi macOS 界面，使用隔离示例项目及编写的演示会话。示例回答只用于说明使用方式，不代表模型评测或用户真实工作记录；界面未经生成或修饰。

| Files | Purpose / 用途 |
| --- | --- |
| `workspace-en.jpg`, `workspace-zh-CN.jpg` | Project sessions, Markdown and composer / 项目会话、Markdown 与输入区 |
| `history-en.jpg`, `history-zh-CN.jpg` | History tree and branch preview / 历史树与分支预览 |
| `settings-en.jpg`, `settings-zh-CN.jpg` | Appearance and theme choices / 外观与主题选择 |

Capture baseline: Gupi 0.1.0, source commit `3efcdca`, Pi 0.99.1, macOS arm64, 2026-09-30. English and Simplified Chinese use the same build and matching demonstration content.

When refreshing the screenshots:

1. Build the current source and confirm the captured process belongs to it.
2. Use disposable `GUPI_CONFIG_DIR`, `GUPI_LOG_DIR`, `GUPI_DATA_DIR`, `PI_CODING_AGENT_DIR`, and `PI_CODING_AGENT_SESSION_DIR` directories. Use sample project names and session text, with no credentials or real model calls.
3. Prepare equivalent English and Chinese conversations with a tool result and two history branches. Capture the workspace, another branch in the history tree, and Appearance settings at a consistent window size.
4. Inspect the actual captures for truncation, transient menus, personal paths and private data. Keep each language's captions and filenames aligned with its README; place the history and settings captures beside their respective usage sections.
5. Close the disposable app and remove its temporary data. Update the baseline above when the UI changes.
