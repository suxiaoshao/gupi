# Gupi 独立项目

本次将 Gupi 从多应用 GPUI workspace 提取为独立公开产品仓库，保持现有运行行为、Pi 接入和应用数据格式。该文档记录迁移决策与本次验证，不承接既有功能后续清单。

## 仓库与历史

- 新仓库：[suxiaoshao/gupi](https://github.com/suxiaoshao/gupi)。最终本机路径为 `/Users/sushao/Documents/code/gupi`。
- 提取原 `app/gupi` 的子目录历史作为新仓库根历史；工作区包含迁移前尚未提交的 Gupi 修改，统一在独立项目提交。
- 原 GPUI 仓库维持其他应用和共享库的归属。共享库基线固定为 `ca2c45f9bd96d24e06acfb7f445dcd2f6227273d`，与迁移时 `5baab1c5` 的文件树一致。
- 分离不迁移、删除或重写用户的 Gupi / Pi 配置、凭据和会话。应用名称与 bundle identifier 保持 `Gupi` / `top.sushao.gupi`。

## 代码与依赖归属

| 内容 | 独立项目中的归属 |
| --- | --- |
| 产品入口、界面与业务 | 根 `gupi` 包；保留 `src/app`、`src/foundation`、`src/features`、`src/state` |
| Pi RPC、窗口、Tokio 桥接与图标封装 | 本地内部 crate：`pi-rpc`、`window-ext`、`gpui-tokio`、`gpui-lucide` |
| 构建和打包 | 精简的单产品 `xtask`，所有可分发产物归集 `dist/` |
| 共享主题、平台、Store、Operation、Form 等服务 | 从 `suxiaoshao/gpui` 的上述固定 Git revision 使用；具体依赖和 features 以根 manifest 与锁文件为准 |
| GPUI Kit | 保持当前正式依赖，分离本身不升级框架或引入 fork |
| 原生资源与体验 fixture | 根 `assets`、`build-assets`、`locales`、`tests`、`script` |

内部 `gpui-lucide` 保留原 Apache-2.0 metadata；Gupi 根包按现有 MIT metadata 补充 LICENSE。资产来源继续记录在对应目录。

## 文档与工具

- 根目录中英文 README 统一维护产品介绍、本机 Pi 前提、安装与[使用方法](../../../README.zh-CN.md#使用方法)。
- [职责边界](../../gui-boundary.md) 与 [RPC 生命周期](../issue-219/README.md) 移入本仓库；已有 Gupi 专属设计继续保留在 `docs/dev`。
- 旧跨应用升级、其他应用和早期跨 owner 实施记录按需要链接到原仓库固定 revision，不复制整套多应用历史。
- 根 AGENTS 自包含；只迁移 Gupi 实际使用的本地技能，并适配固定依赖源码和新的目录布局。
- 应用帮助和反馈链接指向新仓库；体验脚本使用根 `tests/fixtures`。
- Nix 提供本地开发与检查环境；本项目 CI 和发行工作流使用原生 Rustup + Xcode / 系统库。macOS / Linux 发行包在 Nix 外构建，并隔离 target 缓存；打包拒绝含 Nix store 依赖的二进制。macOS 迁移验证发现 Nix 构建链接到 `/nix/store` 中的 libiconv，因此不能直接分发开发产物。
- CI 使用本仓库工具链与锁文件；发布工作流使用原生环境，只由 tag 或手动触发，产物、签名与发布方式见 [发行指南](../../releasing.md)。

## 本次验证

2026-09-30 在 macOS Apple Silicon 完成以下验证。既有 Pi 0.99.1 文档增量保留，分离不实施其待适配功能，也不重审既有未验证清单。

| 范围 | 结果 |
| --- | --- |
| 独立 manifest、锁文件与依赖来源 | `cargo metadata --locked` 通过；根包和五个内部 crate，六个共享包固定到同一 Git revision，GPUI 依赖无重复来源，没有旧仓库本机路径。锁文件从 1364 个包收敛为 1235 个。Nix 四个平台 devShell 仅做求值检查并通过。 |
| Gupi / 内部 crate 的受影响构建与回归 | `cargo test --workspace --locked` 通过（Gupi 167 项应用测试及内部库/文档测试）；应用和内部库的全 target / feature Clippy 通过；Apple Silicon release 构建通过。 |
| 单产品 xtask 与打包产物 | `cargo test -p xtask --locked` 的 8 项测试及 Clippy 通过。原生 Rustup/Xcode、`CARGO_TARGET_DIR=target/native` 打包成功；ZIP 包含 9 个 bundle locale 和 `Assets.car`，ad-hoc 签名验证通过。`otool` 确认仅使用系统动态库，最低链接目标为 macOS 11.0。 |
| 文档本地引用与 fixture 路径 | 292 处本地文件引用、13 处本地标题锚点有效；63 份固定版本源码文件在对应本地 Git 对象中可解析。两个 gallery 的 `--prepare-only` 成功，生成 wrapper 的 fixture 路径与 Shell 语法有效；Python / Node 脚本语法和 `git diff --check` 通过。准备环境已清理，未启动界面或调用模型 |
| 发布工作流与脚本 | actionlint、8 份 YAML、Shell/Python 语法检查通过；20 项隔离脚本检查覆盖九语 MSI、tag/HEAD 匹配、产物命名、校验和及 draft 参数。未创建真实 tag/release，未配置签名凭据。 |
| 必要的本机启动检查 | 使用 LaunchServices 启动本次 `.app`，隔离 Gupi 配置/日志/数据和 Pi agent/session 目录；实际观察主窗口、设置页和临时配置路径。Cmd+Q 后日志显示 managed quit completed，测试进程已退出，临时目录已清理。 |

Windows/Linux 安装包、Intel Mac 运行、Developer ID 签名和 Apple 公证未在本机验证；本次生成的是明确标记的 development ZIP。原仓库的剩余应用 workspace check、xtask 5 项测试、Clippy 和格式检查通过。跨平台原生体验、签名凭据和正式分发条件继续按 [既有能力与验证边界](../follow-ups.md)及发行指南记录；本次迁移不将历史未验证场景新增为开发范围。
