# 接入限制与待确定问题

本文记录接入研究。用户已确认首批方案，当前开发以 [实施方案](implementation.md) 为准；下面的待确定表保留研究背景，不作为本期阻塞清单。

用户已认可同一设置页面内分 Gupi / Pi 的总体方向。当前字段范围建议见 [字段筛选](field-selection.md)，具体分类与界面见 [逐页 UI/UX](page-design.md)；下面的表格保留接入边界，不作为另一份并行字段清单。

## 已核对的接口边界

Pi RPC 提供 `set_model`、`set_thinking_level`、`set_steering_mode`、`set_follow_up_mode`、`set_auto_compaction`、`set_auto_retry` 等专用命令，但没有通用的设置读取、按作用域写入或设置重载命令。

`get_state` 只包含当前模型、思考等级、队列模式、自动压缩等部分会话状态，不等于完整设置，也不提供每个字段的全局/项目来源。队列、自动压缩和自动重试 setter 会写设置；此研究快照的 RPC 模型与思考 setter 未传持久化默认值选项。必须逐命令判断，不能直接作为项目级设置保存接口。

SDK 的 `SettingsManager` 可读取全局、项目及合并配置，提供重载与若干 setter；大部分偏好 setter 写入全局，项目 setter 主要面向资源列表。Gupi 通过进程 RPC 接入，不能把 SDK 对象方法当成已有 RPC 能力。

依据：[RPC 定义](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/modes/rpc/rpc-types.ts)、[AgentSession](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/agent-session.ts)、[SettingsManager](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/settings-manager.ts)。

## 保存与运行时生效

Pi 终端文档要求手工修改后执行 `/reload`。这不能直接转换为“Gupi 保存后所有设置立即生效”的承诺，也不能假设向 RPC 发送同名文本就有终端命令的行为。

`defaultTools` 的上游重载行为也不是全量替换当前工具：会启用新加入的工具，不自动禁用被移除的工具，也不会重启用用户已关闭且配置未变化的工具；CLI 工具覆盖仍参与解析。

后续需要对选定字段分别核对：启动时读取、运行中读取、是否必须重载、是否存在专用 RPC，以及 RPC 是否产生全局持久化副作用。当前仅完成源码契约研究，没有进行这些字段在 Gupi 中的运行验证。

## 首批结论

此前列出的范围、资源组织、作用域入口、继承表示、保存接口、生效方式、版本与信任问题，已在 [首批实施方案](implementation.md) 中确定：

- 首批字段见实施方案的交付范围。
- 资源保持全局，并复用现有编辑器。
- 按 Pi 文件格式在兼容锁内定点修改。
- 保存与对已有会话的影响分开说明，不自动重启。
- 新能力要求 Pi 1.1.0 及以上。
- 信任通过明确的“信任项目…”操作写入。

本文其余部分保留接口边界的研究依据。
