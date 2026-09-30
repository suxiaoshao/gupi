# Gupi

基于 [GPUI Kit](https://github.com/longbridge/gpui-kit) 的 Pi 原生桌面客户端，通过本机 Pi 的 RPC 提供会话工作区。

- 按项目浏览与恢复 Pi 会话，保留未发送草稿，查看历史分支并另开会话。
- 阅读流式回答、思考过程与工具详情，使用正文查找、文件引用和图片附件。
- 用临时窗口完成短任务，配置全局快捷键、个人模板任务和回答回填。
- 管理个人 Pi 插件包、Skill、提示词，接收标准扩展问答和会话提醒。
- 支持主题、七种应用图标和九种界面语言。

[使用指南](docs/user-guide.md) · [开发文档](docs/dev/README.md) · [发行与签名](docs/releasing.md)

## 先准备 Pi

安装并配置 [Pi](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#readme)，在终端确认：

```sh
pi --version
```

登录、模型账户和提供方凭据在 Pi 中配置。Gupi 使用已有 Pi 命令，设置页可以指定可执行文件的绝对路径并检查版本；首次引导也可保留默认值，之后再配置。

协议盘点基线为 Pi **0.99.1**；新 RPC 字段的应用接入情况与 TUI 能力边界见 [接入盘点](docs/dev/pi-rpc-gaps.md)。

## 从源码运行

```sh
git clone https://github.com/suxiaoshao/gupi.git
cd gupi
nix develop
cargo run -p gupi --locked
```

macOS / Linux 的本地开发环境由 [Nix](flake.nix)提供；CI 使用 Rustup 与系统依赖，见 [CI 工作流](.github/workflows/ci.yml)。macOS 构建还需要 Xcode 的 Metal 工具链；使用完整 Xcode 并选择相应开发者目录。未使用 Nix 时，按 [rust-toolchain.toml](rust-toolchain.toml)准备 Rust，Linux 开发库清单见 flake。Windows 使用该 Rust 版本的 MSVC 工具链以及 Visual Studio C++ 构建工具 / Windows SDK。

根 `gupi` 包是唯一产品入口，内部 crate 位于 `crates/`。共享主题、平台、Store、Operation 和 Form 等服务从原 [GPUI 仓库固定提交](https://github.com/suxiaoshao/gpui/tree/ca2c45f9bd96d24e06acfb7f445dcd2f6227273d)使用；版本和 features 以 [Cargo.toml](Cargo.toml) / [Cargo.lock](Cargo.lock) 为准。

## 打包

macOS / Linux 发行构建先退出 `nix develop`，在原生终端使用 [rust-toolchain.toml](rust-toolchain.toml)指定的 Rustup 工具链。macOS 使用宿主 Xcode，Linux 使用系统原生依赖；具体准备步骤见 [发行指南](docs/releasing.md)。独立构建目录可避免复用 Nix 编译缓存：

```sh
CARGO_TARGET_DIR=target/native cargo run -p xtask --locked -- bundle
```

打包器只处理 Gupi，并拒绝含 Nix store 依赖的产物。可分发产物写入 `dist/`：macOS ZIP、Windows 各语言 MSI、Linux deb。macOS 本机开发打包默认 ad-hoc 签名；正式 Developer ID 签名、公证、目标架构及 Windows 命令见发行指南。

## 配置与诊断

默认配置为系统 config 目录下 `gupi/config.toml`；布局和未发送草稿保存到同目录的 `state.toml`、`conversations.toml`。macOS 日志为 `~/Library/Logs/gupi/gupi.log`。帮助菜单可以打开日志位置、复制基础诊断和提交问题。

`GUPI_CONFIG_DIR`、`GUPI_LOG_DIR`、`GUPI_DATA_DIR` 可分别覆盖配置、日志和应用数据目录。相对路径在启动时按工作目录转成绝对路径。详细设置与平台行为见 [使用指南](docs/user-guide.md)。

## 开发

目录边界与验证约定见 [AGENTS.md](AGENTS.md)。现行设计及能力边界由 [开发索引](docs/dev/README.md)导航，本次仓库分离决策见 [独立项目记录](docs/dev/standalone-project/README.md)。

已有隔离体验脚本：

```sh
script/gupi-ui-gallery
script/gupi-runtime-gallery
```

这些脚本使用本机 Pi、临时配置和 `tests/fixtures`，用于桌面试用；脚本选项可查看各自源码。它们不会安装 Pi 或向日常配置注册扩展。

## 许可

Gupi 源码使用 [MIT License](LICENSE)。内部 crate 和第三方资产保留各自许可与来源，图标来源见 [图标记录](build-assets/icon/README.md)。
