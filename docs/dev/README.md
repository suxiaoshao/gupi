# Gupi 开发文档

面向用户的入口：[English](../../README.md) · [简体中文](../../README.zh-CN.md)。每份 README 完整介绍产品、安装与日常使用。

**[发行能力与中英文产品文档](issue-5/README.md)**：DMG、Homebrew、WinGet 和对外文档的职责、实施顺序及验证。

**[应用更新](app-updates/README.md)**：检查、下载、签名验证、安装与重启的实现，原生引擎的退出协调、发行配置和验证范围。

本地构建环境见 README 的[安装章节](../../README.zh-CN.md#安装)，可分发产物使用[发行指南](../releasing.md)中的原生工具链。UI 与 RPC 隔离体验入口分别为 `script/gupi-ui-gallery`、`script/gupi-runtime-gallery`；它们使用已有 Pi、临时配置和 `tests/fixtures/`，不会向日常 Pi 配置注册扩展。脚本分别需要 Python 3 和 Node.js，均由 `nix develop` 提供；Python 不是 Gupi 应用运行或 Rust 构建的前置要求。

历史 `issue-*` 目录及未注明仓库的历史编号沿用原 `suxiaoshao/gpui`；已关闭议题和关联 PR 保留在[原仓库](https://github.com/suxiaoshao/gpui/issues?q=is%3Aissue+is%3Aclosed+gupi)。新项目议题使用明确的 `Gupi #N` 链接。

**[独立项目记录](standalone-project/README.md)**：仓库分离、依赖归属、单产品工具和本次迁移验证。

**[Gupi 与 Pi 的职责边界](../gui-boundary.md)**：输入、附件、会话内容、资源与进程生命周期的归属。

**[剩余工作与能力边界](follow-ups.md)**：应用收尾、上游依赖、独立后续、能力候选与发行验证的统一入口。

**[Pi RPC/TUI 接入盘点与能力差距](pi-rpc-gaps.md)**：33 个 RPC 命令、实时事件、9 类扩展 UI 的完整接入表及 TUI 功能差距。

- [原生体验与应用图标](issue-223/README.md)：图标、菜单/Tray、Startup、九种语言、平台资源、单实例和验证边界。
- [通知与用户提醒](issue-241/README.md)：应用内/系统投递、会话未读、Dock/托盘计数与验证边界。
- [会话阅读与查找 #242](issue-242/README.md)：插件消息、会话信息和正文查找的实现、范围与键位；应用侧已完成，无障碍定位和中文混排缺陷由后续正式依赖升级承接。
- [GPUI Kit 0.7.0 升级与搜索](https://github.com/suxiaoshao/gpui/blob/ca2c45f9bd96d24e06acfb7f445dcd2f6227273d/docs/dev/dependency-refresh-0.7.0/README.md)：原多应用 workspace 的固定升级与原生验证记录。
- [消息摘要与工具详情的查看和复制](issue-238/README.md)：摘要/工具详情 Dialog、分区复制、运行中局部更新与已确认的历史定位交互。
- [事件同步、重试进度与会话局部刷新](issue-236/README.md)：Pi 事件契约、会话变更批次、定向目录发现与回读、消息局部测量、设置按需加载及验证边界；用户提醒体系归 #241。
- [队列交互与输入框布局](issue-222/queue-composer.md)：逐条返回草稿、编辑、删除；新会话项目选择器、累计用量及 Pi 接口限制与待确定项。
- [临时窗口、全局快捷键与托盘开发计划](issue-221/README.md)：多临时会话、图片/文件输入、模板快捷任务、手动清理及平台接入。
- [临时窗口配置与生命周期](issue-221/temporary-window-comparison.md)：窗口配置、跨屏显隐、焦点与布局，以及窗口回收和会话保留的边界。
- [输入资源、Markdown 展示与扩展问答](issue-243/README.md)：正式 0.7.0 接口、Codex/Zed 资源交互对照、Pi 模板/问答边界与实施顺序；实现资源标签、文件候选、消息资源展示与标准扩展问答；记录受影响验证。
- [统一设置开发计划](issue-231/README.md)：分类导航、字段搜索、独立保存边界、快捷键与个人级插件、Skill、提示词管理；包含组件前置、数据契约和分步验证。
- [会话目录读取优化](issue-229/README.md)：顺序字节读取、sonic-rs 按字段解析；保留现有加载、排序与搜索，含性能依据和实施步骤。
- [运行状态与过程展示](issue-229/runtime-display-plan.md)：工作计时、过程折叠、工具组摘要、技能图标统一与验证边界。

- [第一阶段：应用骨架、启动引导与恢复入口](issue-218/README.md)
- [第二阶段：Pi RPC 与进程生命周期](issue-219/README.md)：内部 crate、应用多实例管理和现行退出契约。
- [第三阶段：会话页面功能与交互](issue-220/README.md)：功能要求与当前交互；早期范围、职责和实施验证见[原仓库记录](https://github.com/suxiaoshao/gpui/blob/ca2c45f9bd96d24e06acfb7f445dcd2f6227273d/docs/dev/issue-220/README.md)。
- [会话历史](issue-220/history.md)：树/列表、三级内容、列表分支范围、过程折叠、视口与必要验证。
- [数据获取与加载状态](issue-220/data-loading.md)：目录扫描进度、自定义状态机及会话数据加载边界的设计与实施安排。
- [统一标题栏](issue-220/titlebar.md)：单行窗口顶部、侧栏对齐、会话菜单、当前 session 刷新与原生窗口行为。
- [主窗口快捷键与命令入口](issue-226/README.md)：动作路由、会话快速打开、统一命令面板的双入口与刷新重连；含完整 Pi 内置命令能力对照。
