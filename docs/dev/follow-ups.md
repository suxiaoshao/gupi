# Gupi 剩余工作与能力边界

本页是 Gupi 跨阶段剩余工作的统一入口，放在应用开发文档根目录，不隶属于某个 Issue。按已确认范围区分应用收尾、上游依赖、独立后续、能力候选与验证边界；详细契约和证据链接到对应文档，不再维护第二份清单。

当前应用和依赖边界见 [Kit 0.9 升级审计](gpui-kit-0.9-upgrade/compromise-audit.md)。历史协议研究与运行验证保留原日期和范围；未验证场景不自动成为新的开发任务。

## 应用侧候选

| 工作 | 当前事实与处理边界 | 依据 |
| --- | --- | --- |
| GUI 启动时的环境与代理来源 | 登录 Shell 已捕获完整导出变量，失败回继承环境并保留诊断；macOS 系统代理不是 Shell export，不自动转换为 Pi 子进程代理变量。系统代理职责与优先级仍需独立确定 | [环境快照](../../crates/gupi-pi-runtime/src/environment.rs)、[Shell 捕获](../../crates/gupi-pi-runtime/src/shell_environment.rs) |

## 等待上游的工作

| 工作 | 当前限制 | 恢复条件与入口 |
| --- | --- | --- |
| 长回答深处定位的无障碍滚动 | gpui-pre 0.3.11 列表二次 prepaint 仍未回滚无障碍节点；无障碍激活时调试版可能重复节点断言，发行版会丢弃重复节点 | 保持正式依赖，等包含事务回滚修复的版本后升级并复测；不关闭无障碍、不改应用元素 ID 掩盖问题。[上游复现 #3295](https://github.com/longbridge/gpui-kit/issues/3295)、[应用验证](https://github.com/suxiaoshao/gpui/blob/ca2c45f9bd96d24e06acfb7f445dcd2f6227273d/docs/dev/dependency-refresh-0.7.0/README.md#原生验收发现长消息定位与无障碍树) |
| 插件子命令／参数补全 | 标准 RPC 仍不暴露插件 getArgumentCompletions；Gupi 已有命令名称补全 | 正式 RPC 提供参数查询后接入现有面板，不硬编码候选或自建桥接。[Gupi #19](https://github.com/suxiaoshao/gupi/issues/19)、[协议缺口](pi-rpc-gaps.md#原生-rpc-的主要缺口) |
| 队列逐条返回草稿、编辑、删除及完整附件恢复 | 0.99 已新增每份输入的 handled/queued/started，但仍仅给文字队列与整队清空结果，没有稳定条目 ID、完整快照和原子单条操作；实验 Harness 尚未接到 coding-agent RPC | 输入处理结果的限制已解除，逐条操作继续等权威身份、消费结果、完整消息与位置语义。不使用 clear/requeue 模拟，不维护第二套客户端队列。[队列协议与建议](issue-222/queue-composer.md#逐条操作的可行性与实现边界) |

正文搜索的匹配、上下项导航、原位高亮、离屏定位、用户消息搜索都已接入。原子 Token、Skill/模板选择填入输入框、文件/目录引用、图片附件和标准 Questionnaire 也已接入；它们不再属于等待组件发布的任务。

## 当前 Pi 协议对应用候选的影响

以下按当前正式消费者说明已接入能力和剩余边界。字段、固定源码和隔离运行结果统一见 [当前接入机会](pi-rpc-gaps.md#已有协议能力与-gupi-接入)。

| 内容 | 当前接入 | 仍有边界 |
| --- | --- | --- |
| 输入提交结果 | Client::prompt 已返回 typed InputDisposition，提交分支处理 handled / queued / started | 不提供队列 ID、不代表任务完成；handled 不会清空已有运行 |
| codemode/MCP 工具展示 | 已消费 parentToolCallId 与 nestedCalls，显示实时父子关系和有限历史记录 | 仍不持久化完整子输出；实时 structuredContent 的专用展示另行选择，不复刻 TUI renderer |
| 普通会话首轮失败后的历史 | Pi 已在第一条用户消息时创建会话文件；直接沿用上游修复，不需要 Gupi 另存正文 | 临时 `--no-session` 仍不持久化；系统代理来源仍需单独确定，不改变临时会话的存储边界 |
| Pi 配置管理 | 设置、模型范围、MCP、包、Skill/提示词、认证和信任均已有正式文件/CLI 接点与图形入口 | 没有通用管理 RPC 不等于缺少图形能力；保留文件冲突、继承与手动重连边界 |
| 其他 RPC/TUI 缺口 | 补全、插件快捷键、自定义 UI、组合问卷、树导航和进程内 reload 的直接契约仍缺失 | 标准 RPC 没有这些直接命令或 UI method；同文件节点续聊已通过内置扩展的公开 navigateTree 接入 |

## RPC 与 TUI 功能差距

协议研究基线、当前接入和宿主职责统一保存在 [pi-rpc-gaps.md](pi-rpc-gaps.md)。逐项 TUI 内置命令见 [24 项命令对照](issue-226/builtin-commands.md)。不能把“没有调用同名 RPC”直接等同于缺功能，也不能把传输层保留 JSON 算作界面已接入。

| 分类 | 仍需关注的内容 | 后续归属 |
| --- | --- | --- |
| 已接入的设置与 Shell | 设置通过 Pi 文件/CLI 管理；Bash Shell 已消费 bash / abort_bash / bash_execution_update | 不要求用同名 RPC 替代已成立的文件与领域所有权 |
| RPC 已有，可选参数或专用动作未接 | `compact.customInstructions`、`get_entries.since`、循环模型/思考、仅取消重试、普通新建的 `parentSession` | 可选能力或优化；不影响已有压缩、历史、显式选择、停止与新建，不自动增加任务 |
| RPC 可选结构化结果 | 实时 structuredContent 的专用展示尚未接入 | 该字段未被普通 ToolResultMessage 完整持久化，不承诺历史恢复；disposition 和嵌套调用已接入 |
| TUI 有，标准 RPC 缺契约 | 插件参数/子命令补全、插件键位、任意 TUI 组件及消息/工具 renderer、组合问卷、输入回读/插入式粘贴/主动撤销通知、插件显示控制、进程内 reload | 保留协议差距；标准问答、通用工具详情、历史预览/fork、同文件节点续聊、重连替代已经可用。节点续聊使用公开扩展 API 和标准 RPC；不自建私有协议或终端组件桥接 |
| TUI 交互，应用可以独立实现但尚未选定 | 输入正文的外部编辑器、全局展开/思考显示开关、额外树筛选、模型循环动作、`hotkeys` 搜索别名、Pi changelog 入口、JSONL 导入/导出 | 保留为能力候选，不冒充上游阻塞；`@` 文件/目录候选和会话信息弹窗已有 |
| 需要独立产品范围 | 分享、诊断上传、本地诊断包、压缩期间的客户端输入队列、模型上下文编辑标识 | 当前未纳入；Pi 安装、升级与 OAuth 登录仍由用户在 Pi 中管理；API Key 与项目信任已有图形入口 |

`turn_start` / `turn_end` 暂无独立界面需要；已有更细粒度消息、工具与 `agent_settled` 收尾。复制/回填继续取实际执行分支的可见原始回答，`get_last_assistant_text` 可能受 Pi `context_edit` 影响，不以替换 RPC 调用本身作为优化目标。

## 独立后续工作

这些是已确认的后续范围，不因当前收尾而自动提前实施，也不作为 Gupi 本轮新增交付条件。

| 工作 | 范围与前提 | 统一入口 |
| --- | --- | --- |
| 自研共享库契约修复与精简 | 窗口句柄生命周期、Windows 显示器/坐标契约、Form 校验依赖与错误归属、阻塞写入的退出收尾；再按消费者价值评估 Store/Operation 等精简。已有结论含静态风险，开始时重核当前源码，不能写成已复现崩溃或数据丢失 | [#248](https://github.com/suxiaoshao/gpui/issues/248) |

## 发行与验证边界

以下是尚未获得充分运行证据的场景。按目标平台和本轮改动选择必要验收；历史记录中的每个“未验证”不自动成为重新开发或全量测试的要求。

| 项目 | 已有证据 | 尚未覆盖部分与入口 |
| --- | --- | --- |
| macOS 全局快捷任务 | 窗口启动/隐藏/重建、会话隔离、停止、录制键位和落盘已验证 | 实体键盘系统级派发、重复热键、外部应用取词与自动回填、权限允许/拒绝及跨屏场景未完整验收；截图能力已接入冻结画面、选区和取消，用户确认权限请求失败不遮挡系统提示，完整运行边界见[截图文档](issue-18/README.md)。[临时窗口验证](issue-221/README.md#验证记录)、[本机验收](issue-223/README.md#2026-09-25-本机验收) |
| 通知、Dock 与 Tray | 用户已确认完成通知和 Dock/Tray 数字正常；未读、后台投递与回源有自动化覆盖 | 原生通知点击回源、注意力跳动及部分权限路径仍需实机证据；Dock Liquid Glass 材质/系统外观响应和 Tray 小尺寸动态菜单需要目视确认。[通知文档](issue-241/README.md)、[图标与桌面体验](issue-223/README.md) |
| 原生输入、插件与队列 | 资源标签、图片、标准 select/confirm/input/editor 已通过隔离原生或对应 UI 集成验证；问卷焦点环、提交和取消已验证 | 真实中文输入法组词、现有整队取回/清空的完整原生操作、真实模型压缩与 retain-none 压缩形态未完整覆盖；任意 TUI 插件不属于通用兼容承诺。[资源验证](issue-243/README.md#实现与验证)、[测试入口](issue-222/README.md)、[队列](issue-222/queue-composer.md) |
| 规模与资源占用 | 已验证 2000 条消息、普通/临时四任务重叠运行、隐藏恢复和受管退出；已有 CPU/RSS 采样 | 未测精确冷启动、帧率或长期高负载，不能声称排除所有泄漏；A11y 深处定位缺口仍独立保留。[测量范围与结果](issue-223/README.md#2026-09-25-本机验收) |
| Windows / Linux / 其他 macOS | 已有构建配置、平台代码和部分跨平台回归，macOS Apple Silicon 包已启动验证 | Windows 首次安装、启动、偏好持久化与渠道卸载已有验收记录；旧版到新版的原生更新、原生菜单/热键/通知和混合 DPI 仍按目标环境验证。Linux/Intel/旧 macOS 桌面运行不能由 CI 构建通过推定。Linux Tray/全局热键后端按当前明确范围不新增。[平台边界](issue-223/README.md) |
| 正式分发 | 正式 macOS 安装包已采用 Developer ID 签名、公证；Homebrew tap 已可安装，图标来源与许可已随包记录 | Windows 原生升级与 WinGet 公共源/渠道升级按各自环境验证，Windows MSI 仍未作 Authenticode 签名。本轮未重做发行验收；当前流程见[发行指南](../releasing.md)、[产品安装说明](../../README.zh-CN.md#安装)、[资源许可](../../THIRD_PARTY_NOTICES.md) |

## 已有能力的实现入口

用于判断剩余范围，不按 Issue 关闭与否重复立项：

- [应用骨架与恢复入口](issue-218/README.md)、[RPC 生命周期](issue-219/README.md)、[主会话与历史](issue-220/README.md)、[临时窗口](issue-221/README.md)。
- [动作与命令面板](issue-226/README.md)、[运行展示](issue-229/runtime-display-plan.md)、[目录读取](issue-229/README.md)、[个人资源与快捷键设置](issue-231/README.md)。
- [事件同步与局部刷新](issue-236/README.md)、[摘要/工具详情 Dialog 与复制](issue-238/README.md)、[用户通知](issue-241/README.md)。
- [插件持久消息、信息弹窗和当前分支查找](issue-242/README.md)、[原子资源标签与标准扩展问答](issue-243/README.md)、[图标/菜单/引导/本地化/打包](issue-223/README.md)。

## 维护规则

- 剩余工作只在本页保留摘要、归属和恢复条件；RPC 调用/事件表只在 `pi-rpc-gaps.md` 维护，专门交互设计由原文档负责。
- 解决后更新实现文档并从待办表移除；删除已经失效的等待条件，不靠累计 PR、提交或 Issue 状态维护完成记录。
- 升级先核对正式接口和实际消费者，再复测受影响场景。不得把实验 Harness、社区 fork 或 TUI 函数当作已发布 RPC。
- 新发现的可选能力、静态风险和未验证场景分别标明；文档条目本身不扩大用户授权或当前实施范围。
