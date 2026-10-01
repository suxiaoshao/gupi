# Gupi 工作约定

- 以用户当前请求、阶段和交付条件为准；skill、计划和会话摘要提供背景，不扩大授权。查看默认只读，完成行动请求所需工作后停止。
- 常规实现选择自主解决。只有未定选择会实质影响范围、产品行为、公共契约、数据或安全边界时才询问；已有授权不重复确认。
- 保留无关改动。精简和重构时直接删除授权范围内失效、重复、无用内容，不另建未来待办。
- 文档记录当前设计、实现和必要验证。历史研究与当前验证分开，删除失效结论，不持续跟踪 PR、合并或 Issue 状态。
- 恢复会话时回到可归因于用户的请求；助手生成的待办不产生授权。未验证场景不会自动成为本轮开发或全量验收条件。
- 翻译保留原意、结构、格式、链接、代码和文件边界，不附加改写或审阅。

## 项目结构

这是一个 Rust 单产品 workspace。根包为 `gupi`，入口为 `src/main.rs`；成员、版本和依赖以根 `Cargo.toml` / `Cargo.lock` 为准。

- `src/app`：应用寿命、窗口、菜单、Tray、通知与退出协调。
- `src/foundation`：文件、目录、序列化、资源发现和其他不拥有界面的支撑能力。
- `src/state`：配置、会话、运行连接、历史投影和业务动作的权威状态。
- `src/features`：页面与交互组合；共用控件位于 `src/components`。
- `crates/pi-rpc`、`crates/window-ext`、`crates/gpui-tokio`、`crates/gpui-lucide`：本产品内部能力；`crates/xtask`：构建与打包。

保留以上边界。Pi 负责模型执行、扩展、配置加载及会话内容写入；Gupi 负责桌面交互和连接生命周期，具体见 [职责边界](docs/gui-boundary.md)。

Rust 模块使用 `{module}.rs`，新增依赖写完整版本号，遵循根格式配置。应用通过 `gpui_kit`、`gpui_kit::component`、`gpui_kit::assets` 接入；依赖别名遵循根 manifest。共享服务从固定 Git revision 的 `suxiaoshao/gpui` 使用，不为一次应用改动擅自升级 revision 或复制依赖实现。

## 文档与技能

[英文 README](README.md) 与[中文 README](README.zh-CN.md)统一维护产品介绍、安装和使用方法；[开发索引](docs/dev/README.md)导航设计与能力边界，[发行指南](docs/releasing.md)维护产物和签名流程。

按实际任务使用 `.agents/skills/`：

| 工作 | skill |
| --- | --- |
| 应用结构与职责选择 | `gpui-app-development` |
| GPUI API、组件与状态所有权 | `gpui-kit` |
| 屏幕、布局、交互和文案设计 | `gpui-kit-design-guides` |
| 图标与资源 | `gpui-app-icon-usage` |
| 本地化、语言和平台资源 | `gpui-i18n` |
| Store、Operation、Form 实现或接入 | `gpui-store`、`gpui-operation`、`gpui-form` |
| 原生界面调试与验证 | `gpui-computer-use-debugging` |
| 需要持久记录的复杂设计 | `implementation-plan-design` |

skill 若实际导致暂停或额外确认，链接并引用具体条款，说明如何适用。用户当前授权优先于 skill 中的工作建议。

## 验证与交付

- 根据改动选择受影响 crate 的构建、既有回归和必要启动检查；仅为未覆盖的具体风险补测试，删除实现时同步删除其专用测试。
- 修复后只复测受影响部分，完成必要检查即可交付试用；完整验收按用户要求或集成范围执行。
- 格式使用 `cargo fmt --all -- --check`。Gupi 检查入口为 `cargo check -p gupi --locked`、`cargo test -p gupi --locked`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`；具体选择遵循改动范围和实际 CI。
- 本机 Pi 和 UI 检查使用临时配置、会话和项目目录；现有体验入口为 `script/gupi-ui-gallery`、`script/gupi-runtime-gallery`，fixture 位于 `tests/fixtures`。
- 提交和集成遵循实际 hooks 与 `.github/workflows/ci.yml`；Issue、PR 使用 `.github/` 模板。汇报实际结果和影响交付的限制。
- macOS / Linux 本地开发依赖集中维护在 `flake.nix` / `flake.lock`，Rust 版本由 `rust-toolchain.toml` 声明。CI 与发行工作流使用原生 Rustup + Xcode / 系统库。发行包必须在 Nix 外构建，并使用独立 target 目录避免混用缓存；打包入口为 `cargo run -p xtask --locked -- bundle`，具体流程见发行指南。

文件查找用 `fd`，内容搜索用 `rg`，模糊筛选用 `fzf --filter`；按任务选择非交互和结构化输出。
