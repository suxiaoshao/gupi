# 图形设置的字段筛选

状态：首批已采用。以 [研究快照](README.md#研究依据) 为依据，2026-10-10 补充核对 Gupi 调用方式和 Pi 的实际消费者。首批图形控件按 [首批实施方案](implementation.md) 实现；其余字段保持文件编辑。

表格记录协调者的字段取舍。“模型/会话/网络”表示语义归属，不要求各自独立成页；经过设计师讨论，当前共同推荐以“对话/网络”两个实质配置页组织首批字段，另保留连接和已有资源页。若 Shell 字段入选，网络页改名“环境”。详细页面及双方尚存分歧见 [逐页设计](page-design.md)。

## 筛选标准

优先提供能解决 Gupi 用户实际任务、作用清楚、能用有限控件准确表达的配置。Pi 支持一个 JSON 字段，不自动意味着 Gupi 应提供控件。

- **常用**：主要分组直接显示。
- **页内高级**：确有用途但低频，放在所属页的高级分组；搜索命中时应能直接定位，不建立杂项堆积页。
- **文件编辑**：语义复杂或低频，保留打开对应 `settings.json` 的入口；不删除、不改写已有值。
- **不提供控件**：Pi 终端专用、内部状态，或无法兑现 Gupi 行为的字段。
- **已有入口**：复用现有资源编辑器，不生成另一套原始字段编辑器。

## 推荐清单

| 字段 | 推荐 | 任务与理由 |
| --- | --- | --- |
| `defaultProvider` + `defaultModel` | 常用：模型 | 选择默认模型；一个模型选择器同时写 provider 和 id，避免不匹配组合 |
| `defaultThinkingLevel` | 常用：模型 | 调整默认思考强度；区别于会话工具栏当前值 |
| `modelThinkingLevels` | 页内高级候选：模型 | 为不同模型保存不同强度；需要可添加/删除的覆盖行，首批是否纳入待确认 |
| `enabledModels` | 文件编辑，后续可评估模型范围编辑器 | 是启动/轮换范围，不是模型授权白名单；Gupi 可用模型查询不按此字段过滤，不能直接叫“允许的模型” |
| `thinkingBudgets` | 文件编辑 | 模型/供应商适用范围复杂，通用 token 输入容易误导；等级选择已覆盖常见任务 |
| `cacheWarming` | 页内高级候选：模型，全局 | 有性能与用量权衡；三选一并给简短费用含义，不默认创建单独页面 |
| `compaction.enabled` | 常用：会话 | 控制上下文自动压缩 |
| `compaction.reserveTokens`、`keepRecentTokens` | 页内高级：会话 | 调整响应余量和保留的近期内容；以 token 为单位，保持两个值独立 |
| `compaction.modelOverrides` | 文件编辑 | 多模型嵌套覆盖增加编辑复杂度，首批先提供普通参数；按模型 UI 保留为备选 |
| `steeringMode`、`followUpMode` | 常用：会话 | 分别控制执行中插入消息和后续消息的投递方式；各自展示“逐条/全部”，不能合为同一开关 |
| `images.autoResize` | 常用：会话 | 控制发送前图片缩放，确有 RPC 输入处理路径 |
| `images.blockImages` | 常用：会话 | 控制是否向模型发送图片，不能描述成禁用 Gupi 附件或截图功能 |
| `branchSummary.reserveTokens` | 文件编辑 | Pi 核心有消费者，但尚未证明对应 Gupi 导航流程会触发分支摘要；不因类型存在就加控件 |
| `branchSummary.skipPrompt` | 不提供控件 | 当前查到的询问消费者在 Pi interactive 模式，不控制 Gupi 对话框 |
| `sessionDir` | 文件编辑 | Gupi 已读取全局和已知项目目录配置；暂不加存储迁移 UI，避免把改路径误认为搬迁历史文件 |
| `retry.enabled`、`maxRetries` | 常用：网络 | 网络暂时失败后是否重试、重试几次，作用明确 |
| `retry.baseDelayMs`、`maxAgentDelayMs` | 页内高级：网络 | Agent 层退避参数，单位显示明确；不要与 Provider 重试混合 |
| `httpProxy` | 常用：网络，全局 | 解决 Pi 请求连通性；只影响 Pi 管理的客户端，不能称为整个 Gupi 的代理 |
| `transport` | 页内高级：网络 | 排查供应商 SSE/WebSocket 兼容问题；自动为常态 |
| `httpIdleTimeoutMs`、`websocketConnectTimeoutMs` | 页内高级：网络 | 超时排障有实际价值；明确“关闭超时”，不把 0 当成未设置 |
| `retry.provider.timeoutMs/maxRetries/maxRetryDelayMs` | 文件编辑 | Provider 与 Agent 两层重试容易混淆；上游建议 Provider 重试次数维持 0，首批无需复制整套底层调优 |
| `defaultTools` | 文件编辑；独立工具编辑器为备选 | 替换、增减、空数组、扩展工具语义不同；简单复选列表可能破坏手工配置，且不能宣传为安全沙箱 |
| `codemode.mode/inlineBudget` | 文件编辑 | 依赖工具启用与扩展声明；不在首批创建只有两个底层参数的工具页 |
| `shellPath`、`shellCommandPrefix` | 文件编辑，页内高级为备选 | 有核心消费者，但低频且需准确表示命令环境；不建立空泛的工具分类 |
| `npmCommand` | 文件编辑 | Gupi 资源发现也读取此字段；argv 数组不能随意拼成 shell 命令字符串，已有默认行为覆盖常用需求 |
| `enableSkillCommands` | 不提供控件 | Pi interactive 模式读取；RPC `get_commands` 直接枚举技能，不可用于承诺隐藏 Gupi 技能命令 |
| `enableInstallTelemetry` | 页内高级候选：网络 | 也控制部分供应商归因头，确有核心消费者；不是“关闭所有网络遥测”总开关，环境变量 `PI_TELEMETRY` 可覆盖 |
| `enableAnalytics` | 不提供首批控件 | 当前文档限定实验性首次设置流程；未找到足以定义 Gupi 功能的运行消费者 |
| `defaultProjectTrust` | 文件编辑；信任功能另行设计 | 属于 Pi 全局信任策略；不在普通偏好页悄悄新增授权流程 |
| `warnings.anthropicExtraUsage` | 不提供控件 | 警告消费者在 Pi interactive 模式；不能让 Gupi 用户误认为控制了 Gupi 的费用提醒 |
| `packages/extensions/skills/prompts` | 已有入口 | 复用全局包、Skills、模板及现有资源管理；不再提供路径数组表单 |
| `themes` | 不提供新的 Pi 主题路径编辑器 | Pi 终端主题资源与 Gupi 外观不同；保留文件和包资源内容 |
| 终端显示、导航、外部编辑器、Markdown 呈现字段 | 不提供控件 | 包括 `hideThinkingBlock`、`showCacheMissNotices`、`theme`、`terminal.*`、`markdown.*` 等；Gupi 界面不能靠这些 Pi 终端配置控制 |
| 内部和旧版字段 | 不提供控件 | `$schema/lastChangelogVersion/trackingId/deviceId` 与迁移字段保留其原有用途，不作为偏好暴露 |

完整字段名、类型、默认值见 [设置清单](settings.md)。筛选结果不意味着在写入其他字段时删除上述保留项。

## 源码核对得到的关键边界

### 默认模型与当前会话

Pi `sdk.ts` 会优先恢复已有会话模型和思考记录。Gupi 的 [reconnect.rs](../../../crates/gupi-conversation/src/conversation/reconnect.rs) 显式传递当前 `--provider/--model/--thinking`，因此不能声称更改默认值并重连就能替换当前会话模型。

Pi 该快照的 RPC `set_model` 调用 `session.setModel(model)`，没有传 `persist: true`；`set_thinking_level` 也未要求持久化默认值。队列、自动压缩和自动重试 setter 则会写设置。需要逐命令区分，不能把所有会话 setter 当成默认配置保存接口。

`enabledModels` 在 `main.ts` 解析为启动/轮换范围；RPC `get_available_models` 返回模型运行时的可用快照，不等于此范围。它不是安全限制。

### 内存配置与磁盘配置

图片拦截、请求超时、部分重试和缓存预热在执行时查询 `SettingsManager`，但查的是 Pi 进程中的配置对象。Gupi 修改磁盘文件并不自动触发其重载。其他字段在创建 Agent 时拷贝到实例；不能统一归为即时生效。

本轮不为设置页增加自动重启或隐式 RPC 同步。真正的生效策略仍需按入选字段和实际 Pi 版本确定。保存成功只能先确认对应文件已经保存。

### 会话目录与资源命令

[session_catalog.rs](../../../crates/gupi-conversation/src/session_catalog.rs) 的 `scan_scope` 读取全局 `settings.json` 和已知项目 `.pi/settings.json` 中的 `sessionDir`，并按 cwd 解析相对目录。此前的“未支持设置文件目录”初步判断已撤回；保留文件编辑的理由是产品范围与迁移语义，而非不存在读取支持。

[资源发现](../../../crates/gupi-resources/src/pi_resources/discovery.rs) 已读取 `npmCommand`。文件编辑项依然可能对 Gupi 有作用，不能一概归为终端专用。

### 遥测与终端专用开关

`provider-attribution.ts` 通过 `isInstallTelemetryEnabled` 决定部分归因头，`PI_TELEMETRY` 优先于设置值。若提供控件，环境覆盖必须有准确状态，不能展示一个看似生效的无效开关。

`getEnableSkillCommands`、`getBranchSummarySkipPrompt`、`getWarnings().anthropicExtraUsage` 的当前消费者集中在 `interactive-mode.ts`；RPC `get_commands` 直接枚举已加载技能。这是排除对应 Gupi 控件的依据。

上游代码均以固定研究提交为准：[sdk.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/sdk.ts)、[rpc-mode.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/modes/rpc/rpc-mode.ts)、[agent-session.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/agent-session.ts)、[main.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/main.ts)、[provider-attribution.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/provider-attribution.ts)。

## 首批结论

- 按模型思考覆盖、缓存预热、安装遥测、Shell、传输方式与超时，首批均不提供图形控件，仍可通过打开 settings.json 编辑。
- 网络独立成页，承载代理与 Agent 重试。
- 工具选择保持文件编辑。
- 保存方式（按页保存）、未信任项目状态与已有会话的应用方式，见 [首批实施方案](implementation.md)。
