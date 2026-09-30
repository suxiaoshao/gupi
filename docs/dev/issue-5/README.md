# 发行能力与对外产品文档

状态：In progress

范围来源：[Gupi #5](https://github.com/suxiaoshao/gupi/issues/5)。

## 目标与职责

让用户能够理解 Gupi 的用途，完成安装及首次会话，并通过原生安装包和包管理器获得后续版本。Pi 仍负责模型、认证、工具和会话内容；本项不改变 Pi/Gupi 职责边界。

- `README.md` / `README.zh-CN.md`：英文与简体中文完整产品文档，各自包含截图、Quick Start、安装和日常使用。
- `docs/screenshots/`：当前真实应用、隔离演示数据的截图及更新说明。
- `crates/xtask`：原生打包、签名、tag 校验、产物汇总、校验和、分发清单与草稿 Release。
- `.github/workflows/release.yml`：矩阵调度、凭据注入与上传；`.github/scripts/` 仅保留 macOS 临时证书导入。
- Homebrew tap / WinGet manifests：引用已经公开的固定版本产物。

## 当前设计

每种语言仅维护一份完整 README，按「一句话介绍与主截图 → Quick Start → 安装 → 使用方法 → 快捷键与设置 → 常见问题 → 开发与反馈、许可」组织。平台依赖与本地打包命令使用折叠区，历史和设置截图紧邻对应功能；同一截图不在同一 README 重复。安装及日常操作都能在本文完成，内部算法、组件实现、签名发行和历史记录由既有开发文档承载。两种语言使用对应的章节、命令与功能范围。应用帮助菜单直接打开 README 的使用章节。

tag workflow 提供 macOS 两种架构 DMG/ZIP、Windows x64 九种语言 MSI、Linux x64 deb 及 SHA256SUMS。Linux 保留现有发布能力，本轮新增与本机验证聚焦 macOS 和 Windows。维护者创建 tag，构建完成后审核草稿再发布。沿用该流程，不增加自动版本递增或自动创建 tag。

macOS DMG 在图标和资源变更、应用最终签名完成后生成，包含 Gupi.app 和 Applications 链接。正式模式覆盖外层 DMG 的签名、公证与 stapling。汇总阶段一次检查完整产物集并计算校验和，Rust MSI 库直接读取实际产品信息。发行相关 Python、PowerShell 和打包包装脚本已移除；不在每个 tag 重复 CI 测试、挂载检查或安装/卸载检查。原生安装和旧版升级在首次发布或修改安装行为时定向验证。

工作流生成待审阅的分发清单 artifact。Homebrew 通过维护者 tap 的 Cask 分发，使用架构对应的正式签名 DMG URL 和 SHA256；development 模式不生成 Cask。WinGet 复用九种语言 MSI，清单包含真实 ProductCode、UpgradeCode、安装语言、下载地址和 SHA256。仅在稳定 Release 正式发布后更新外部配方/清单；跳过草稿和预发布。外部仓库写入使用最小所需权限，首次提交在真实产物验证后进行。

## 实施顺序

1. 完成中英文各一份完整 README，合并安装与使用说明，并保留六张实际界面截图。文档明确当前可用安装路径，不把规划中的分发方式写成可用命令。
2. 扩展 DMG、产物校验及签名流程；进行受影响 xtask 验证和 macOS 安装/启动检查。
3. 根据工作流的实际产物生成 Cask 与 WinGet manifests；首次外部上架时验证安装、升级、卸载以及用户数据保留。
4. 完成首个 tag 构建和发布后更新流程验证。正式公开分发前完成签名凭据配置与真实公证；图标来源、署名及 MIT 许可已收录于 `THIRD_PARTY_NOTICES.md`，由打包器随应用分发。

## 文档验证

核对安装命令与当前 toolchain、setup action、xtask 输出；核对功能与快捷键定义；检查双语章节对应和本地链接。截图从同一构建取得，不使用真实模型调用、凭据或个人会话；它们展示界面，不作为模型能力或跨平台发布成功的证据。

2026-09-30 文档阶段已完成：

- 英文、简体中文 README 已按对应结构合并安装和使用说明；独立安装与使用指南已删除，帮助菜单、仓库约定及开发文档入口已更新。六张截图覆盖主工作区、历史分支预览和外观设置。
- 当前源代码 `3efcdca` 使用原生 Rustup 完成 macOS arm64 release 构建，并以该二进制启动隔离演示。构建中的本机 `rust-objcopy` 调试信息剥离警告未阻止构建或启动。
- 安装命令与当前源码、setup action 和 Pi 官方说明对应；快捷键与界面标签沿用已核对的内容。合并后检查 8 份 Markdown 中的 89 处本地链接与锚点、双语章节结构、每种语言的 5 个折叠区和 3 张截图引用；旧安装与使用指南路径无残留。
- 浏览器预览确认中英文截图正常加载、Quick Start 页内导航和安装说明折叠可用。`cargo fmt --all -- --check` 与 `git diff --check` 通过。
- 截图使用演示数据，未进行真实模型调用、Windows/Linux 本机安装、正式签名或发布。

发行代码与检查流程已扩展；真实 Developer ID 凭据验证、首次 tag 远程运行和包管理器外部上架尚未执行。


## 发行实现验证

2026-09-30：

- xtask 的既有 8 项测试通过；Rust 发行汇总新增两项回归通过，覆盖真实 MSI 数据库读取、旧版 MSI 拒绝、缺失 DMG 拒绝和 development 重新生成时清理旧 Cask。MSI fixture 是数据库样本，不作为安装或升级验证。
- `cargo check -p xtask --locked`、`cargo clippy -p xtask --all-targets --locked -- -D warnings` 和 `actionlint 1.7.12` 已用于本次变更；发行入口使用 Rustup，无 Python bootstrap 或专用 Python CI job。
- 前一轮原生 macOS arm64 development 打包、DMG 挂载、签名/架构/版本核对以及复制后启动已通过。本次收敛没有修改打包内容，不重复应用构建和 UI 检查。
- Windows 安装、Intel/Linux 运行、真实 Developer ID、公证和首次远程 tag 构建尚未验证。Homebrew tap 与 WinGet 社区提交仍需公开版本及对应渠道验证。

应用内检查、下载、安装和重启见[应用更新设计](../app-updates/README.md)，更新密钥与 appcast 发布配置见[发行指南](../../releasing.md#application-update-signing)。
