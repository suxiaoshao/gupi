# Pi 设置清单

本页列出 `settings.json` 字段，不是 Pi 全部配置文件的全集。Gupi 已做/未做的逐项对照见[配置覆盖统计](coverage.md)；独立 `mcp.json` 见[MCP 配置盘点](mcp-configuration.md)。

研究版本与来源见 [README](README.md#研究依据)。表格中的默认值来自设置文档、Schema、默认值定义和访问器；实际模型能力、环境变量、CLI 参数或会话恢复可进一步影响运行行为。字段路径以点号表达嵌套 JSON 对象。

## 模型与思考

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `defaultProvider`、`defaultModel` | string / 自动选择 | 启动默认供应商与模型 |
| `defaultThinkingLevel` | `off/minimal/low/medium/high/xhigh/max` / `medium` | 默认思考等级；实际可用等级取决于模型 |
| `modelThinkingLevels` | object / 无 | 按准确的 `provider/modelId` 指定等级 |
| `thinkingBudgets` | object / 内置预算 | `minimal/low/medium/high` 的 token 预算 |
| `enabledModels` | string[] / 所有可用模型 | 启动选择与轮换使用的模型模式 |
| `cacheWarming` | `off/streaming/idle` / `streaming` | 缓存预热，仅全局；预热请求产生用量 |

## 交互、工具与上下文

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `steeringMode`、`followUpMode` | `all/one-at-a-time` / `one-at-a-time` | 两种消息队列的投递方式 |
| `defaultTools` | string[] / `read/bash/edit/write` | 启动工具选择，支持增减修饰项 |
| `codemode.mode` | `on/only` / `on` | 直接暴露工具或通过 codemode 调用 |
| `codemode.inlineBudget` | number / `3000` | codemode 描述内工具声明的估算 token 预算 |
| `sessionDir` | string / Pi 会话目录 | 会话存储目录；相对路径基于工作目录，环境变量与 CLI 可覆盖 |
| `compaction.enabled` | boolean / `true` | 自动压缩 |
| `compaction.reserveTokens` | 非负安全整数 / `16384` | 为模型响应保留的 token |
| `compaction.keepRecentTokens` | 非负安全整数 / `20000` | 不摘要的近期 token |
| `compaction.modelOverrides` | object / 无 | 按 `provider/modelId` 分别覆盖两个 token 参数 |
| `branchSummary.reserveTokens` | number / `16384` | 分支摘要预留 token |
| `branchSummary.skipPrompt` | boolean / `false` | 跳过分支摘要询问并默认不生成摘要 |

压缩的两个 token 参数独立按模型覆盖、普通配置、内置默认值解析。项目和全局对象先合并，再查找模型覆盖。

## 网络与重试

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `transport` | `auto/sse/websocket/websocket-cached` / `auto` | 支持多种传输的供应商使用的首选方式 |
| `httpProxy` | string / 无 | Pi 管理的 HTTP 客户端代理，仅全局 |
| `httpIdleTimeoutMs` | number 或 `disabled` / `300000` | HTTP header/body 空闲超时，毫秒 |
| `websocketConnectTimeoutMs` | number 或 `disabled` / `15000` | WebSocket 连接超时，毫秒 |
| `retry.enabled` | boolean / `true` | Agent 层自动重试 |
| `retry.maxRetries` | number / `3` | Agent 层最大重试次数 |
| `retry.baseDelayMs` | number / `2000` | 指数退避初始延迟 |
| `retry.maxAgentDelayMs` | number / `60000` | Agent 层最大重试延迟 |
| `retry.provider.timeoutMs` | number / 回退到 HTTP 空闲超时 | Provider 请求超时 |
| `retry.provider.maxRetries` | number / `0` | Provider 层重试次数 |
| `retry.provider.maxRetryDelayMs` | number / `60000` | 服务端要求等待的上限；`0` 禁用上限 |

两个顶层网络超时字段支持 `0` 或字符串 `"disabled"` 禁用，Schema 比设置文档的类型表更完整。Provider 重试与 Agent 重试不同；上游建议除非确有需要，否则保留 Provider 重试次数为 `0`。

## 图片、Shell 与资源

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `images.autoResize` | boolean / `true` | 发送模型前缩放至最大 2000×2000 |
| `images.blockImages` | boolean / `false` | 阻止图片发送给模型 |
| `shellPath` | string / 平台默认 | Shell 可执行文件，支持前导 `~` |
| `shellCommandPrefix` | string / 无 | Shell 命令前缀 |
| `npmCommand` | string[] / `npm` | npm 命令及参数，例如 `["mise", "exec", "node@20", "--", "npm"]` |
| `packages` | array / `[]` | 包来源字符串或资源过滤对象 |
| `extensions`、`skills`、`prompts`、`themes` | string[] / `[]` | 各类资源文件或目录 |
| `enableSkillCommands` | boolean / `true` | 将技能注册为 `/skill:name` 命令 |

资源格式和组合规则见 [配置格式](configuration.md#资源列表的例外)。

## 信任、遥测与提示

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `defaultProjectTrust` | `ask/always/never` / `ask` | 项目信任回退策略，仅全局 |
| `enableInstallTelemetry` | boolean / `true` | 匿名安装/更新报告与部分供应商归因头，不控制更新检查 |
| `enableAnalytics` | boolean / `false` | 分析数据共享，目前用于实验性首次设置流程 |
| `warnings.anthropicExtraUsage` | boolean / `true` | Anthropic 订阅认证可能使用付费额外用量时提示 |

## 终端呈现与交互

这些字段属于 Pi 的显示与终端交互契约。不能因为写入 Pi 设置就认定它们会改变 Gupi 界面。

| 字段 | 类型 / 默认值 | 用途 |
| --- | --- | --- |
| `theme` | string / `system` | 终端主题 |
| `quietStartup` | boolean 或 `header` / `false` | 隐藏启动信息，或仅保留头部 |
| `collapseChangelog` | boolean / `false` | 折叠更新日志 |
| `hideThinkingBlock` | boolean / `false` | 隐藏思考块 |
| `showCacheMissNotices` | boolean / `false` | 缓存成本、预热、压缩用量与供应商恢复提示 |
| `externalEditor` | string / `$VISUAL`、`$EDITOR`，再回退平台默认 | 外部编辑器命令 |
| `doubleEscapeAction` | `tree/fork/none` / `tree` | 空编辑器双 Escape 行为 |
| `treeFilterMode` | `default/no-tools/user-only/labeled-only/all` / `default` | 会话树初始筛选 |
| `tuiMode` | `regular/fullscreen` / `fullscreen` | 终端模式 |
| `fullscreenExitOutput` | `transcript/resume-hint` / `transcript` | 全屏退出后输出 |
| `fullscreenScrollbar` | `auto/always/hidden` / `auto` | 全屏滚动条 |
| `fullscreenCopyOnSelect` | boolean / `true` | 全屏选中文本自动复制 |
| `fullscreenWheelScrollLines` | `auto` 或 number / `auto` | 滚轮行数，数值限制为 1–100 |
| `editorPaddingX` | number / `0` | 编辑器水平内边距，0–3 单元格 |
| `outputPad` | `0/1` / `1` | 正文水平内边距 |
| `autocompleteMaxVisible` | number / `5` | 补全可见条数，3–20 |
| `showHardwareCursor` | boolean / `false` | 使用终端光标；环境变量也可影响默认值 |
| `terminal.showImages` | boolean / `true` | 显示终端内联图片 |
| `terminal.imageWidthCells` | number / `60` | 内联图片首选宽度 |
| `terminal.clearOnShrink` | boolean / `false` | 内容缩小时清空余行；环境变量也可影响默认值 |
| `terminal.showTerminalProgress` | boolean / `false` | OSC 9;4 终端进度 |
| `terminal.hyperlinks` | boolean 或 `auto` / `auto` | OSC 8 链接能力覆盖 |
| `terminal.images` | `kitty/iterm2/auto` 或 `false` / `auto` | 图片协议能力覆盖 |
| `terminal.trueColor` | boolean 或 `auto` / `auto` | 真彩色能力覆盖 |
| `markdown.codeBlockIndent` | string / 两个空格 | 代码块缩进前缀 |
| `markdown.mermaid` | `off/final/streaming` / `streaming` | Mermaid 渲染方式 |

## 内部字段与兼容字段

- `$schema`：编辑器使用的 JSON Schema 引用。
- `lastChangelogVersion`：更新日志版本记录。
- `trackingId`：启用分析时生成的标识。
- `deviceId`：全局安装标识。

以上不作为普通偏好控件候选。未知字段应保留。

旧格式迁移包括 `queueMode → steeringMode`、`websockets → transport`、`retry.maxDelayMs → retry.provider.maxRetryDelayMs`，以及旧版 `skills` 对象向路径数组和 `enableSkillCommands` 的迁移。新界面使用当前格式，不生成旧字段。

