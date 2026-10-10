# Pi 配置覆盖与缺口统计

核对日期：2026-10-10。依据本地 Pi 1.1.0，提交 `6fb2e7815167e6b19006fc526d1a5d0f5f998787`，以及当前 Gupi 工作区的实现。没有拉取新版本、读取个人凭据或连接 MCP 服务。本文件是配置覆盖盘点，不扩大[首批实施方案](implementation.md)的开发范围。

## 统计口径

此前的“14 项”是 Gupi 新增的表单控件数，实际写入 **15 个 JSON 叶字段**（默认模型同时写 `defaultProvider` 和 `defaultModel`）。它不表示 Pi 总共只有 14 项，也不包含原有资源编辑器。

Pi `settings.schema.json` 有 **58 个顶层键**，其中嵌套对象算一个键，动态模型映射不按模型数量计数。按当前设置 UI 覆盖情况划分：

| 分类 | 顶层键数 | 含义 |
| --- | ---: | --- |
| 已覆盖 | 7 | 本次表单覆盖这些键的当前配置 |
| 部分覆盖 | 7 | 部分叶字段或全局资源操作可编辑，并非完整结构编辑器 |
| 尚无专用配置 UI | 18 | Pi 配置能力存在，Gupi 尚未提供对应控件 |
| Pi 终端或交互专用 | 20 | 不等同于 Gupi 界面偏好；接入前要核对 RPC 中的意义 |
| 元数据或旧兼容键 | 6 | 保留兼容，不作为普通偏好编辑项 |

**没有专用 UI 不代表 Pi 运行时不支持**：Pi 子进程仍可能按自己的规则加载这些文件；本次没有逐项运行验证。独立文件、环境变量和任意第三方扩展配置不计入 58 个键，不能据此给整个 Pi 配置计算完成百分比。

## settings.json 完整顶层对照

叶字段含义、类型与默认值见[设置清单](settings.md)。下表逐一覆盖 Schema 的 58 个顶层键。

| 顶层键 | 当前状态 | 说明 |
| --- | --- | --- |
| `$schema` | 元数据/兼容 | — |
| `lastChangelogVersion` | 元数据/兼容 | — |
| `defaultProvider` | 已覆盖 | — |
| `defaultModel` | 已覆盖 | — |
| `defaultThinkingLevel` | 已覆盖 | — |
| `modelThinkingLevels` | 尚无专用 UI | — |
| `transport` | 尚无专用 UI | — |
| `steeringMode` | 已覆盖 | — |
| `followUpMode` | 已覆盖 | — |
| `theme` | 终端/交互专用 | — |
| `compaction` | 部分覆盖 | 已做 enabled/reserveTokens/keepRecentTokens；未做 modelOverrides |
| `branchSummary` | 尚无专用 UI | — |
| `retry` | 部分覆盖 | 已做 Agent 四项；未做 provider.* |
| `hideThinkingBlock` | 终端/交互专用 | — |
| `showCacheMissNotices` | 终端/交互专用 | — |
| `externalEditor` | 终端/交互专用 | — |
| `shellPath` | 尚无专用 UI | — |
| `quietStartup` | 终端/交互专用 | — |
| `defaultProjectTrust` | 尚无专用 UI | 未做默认信任策略控件；单个项目的信任操作写 trust.json |
| `shellCommandPrefix` | 尚无专用 UI | — |
| `npmCommand` | 尚无专用 UI | — |
| `collapseChangelog` | 终端/交互专用 | — |
| `enableInstallTelemetry` | 尚无专用 UI | — |
| `enableAnalytics` | 尚无专用 UI | — |
| `trackingId` | 元数据/兼容 | — |
| `packages` | 部分覆盖 | 已有全局包管理；未做项目包编辑及任意资源过滤结构编辑 |
| `extensions` | 部分覆盖 | 已有全局资源启停；未做完整路径列表、内置扩展开关及项目配置编辑 |
| `deviceId` | 元数据/兼容 | — |
| `skills` | 部分覆盖 | 已有全局资源管理；未做项目资源与完整路径表达式编辑 |
| `prompts` | 部分覆盖 | 已有全局模板管理；未做项目资源与完整路径表达式编辑 |
| `themes` | 部分覆盖 | 包内主题资源可管理；没有 Pi 主题配置专页，Gupi 外观设置独立 |
| `enableSkillCommands` | 终端/交互专用 | — |
| `terminal` | 终端/交互专用 | — |
| `images` | 已覆盖 | autoResize、blockImages 均已做 |
| `enabledModels` | 尚无专用 UI | — |
| `defaultTools` | 尚无专用 UI | — |
| `doubleEscapeAction` | 终端/交互专用 | — |
| `treeFilterMode` | 终端/交互专用 | — |
| `thinkingBudgets` | 尚无专用 UI | — |
| `editorPaddingX` | 终端/交互专用 | — |
| `outputPad` | 终端/交互专用 | — |
| `autocompleteMaxVisible` | 终端/交互专用 | — |
| `showHardwareCursor` | 终端/交互专用 | — |
| `markdown` | 终端/交互专用 | — |
| `warnings` | 尚无专用 UI | anthropicExtraUsage；需核对 Gupi 中的提示行为 |
| `codemode` | 尚无专用 UI | — |
| `sessionDir` | 尚无专用 UI | 未做目录配置/迁移界面；不表示会话发现不支持该目录 |
| `httpProxy` | 已覆盖 | 仅全局 |
| `httpIdleTimeoutMs` | 尚无专用 UI | — |
| `cacheWarming` | 尚无专用 UI | — |
| `websocketConnectTimeoutMs` | 尚无专用 UI | — |
| `tuiMode` | 终端/交互专用 | — |
| `fullscreenExitOutput` | 终端/交互专用 | — |
| `fullscreenScrollbar` | 终端/交互专用 | — |
| `fullscreenCopyOnSelect` | 终端/交互专用 | — |
| `fullscreenWheelScrollLines` | 终端/交互专用 | — |
| `queueMode` | 元数据/兼容 | — |
| `websockets` | 元数据/兼容 | — |

## 独立文件与资源覆盖

| 配置载体 | 当前 Gupi 覆盖 | 尚缺内容 |
| --- | --- | --- |
| 全局／项目 `mcp.json` | 无专用编辑页 | 服务增删改、stdio/HTTP、启停、工具暴露、项目覆盖、认证与连接状态；见[MCP 盘点](mcp-configuration.md) |
| 全局 `models.json` | 可从 Pi 查询模型并选择默认模型；不是文件编辑器 | 自定义供应商、baseUrl/API/headers/apiKey 引用、models、modelOverrides、能力/上下文/价格、thinkingLevelMap、samplingParams/按等级采样、inputLimits、promptCache；模型协议的完整约束由上游 Schema 定义 |
| 全局 `auth.json` | 无专用凭据管理页 | Provider API Key、OAuth 登录/注销及状态；默认模型选择不代替认证管理 |
| 全局 `mcp-auth.json` | 无专用管理页 | MCP OAuth 登录/注销与失效状态；令牌由 Pi 管理，不建议做通用 JSON 编辑器 |
| 全局 `trust.json` | 已实现状态读取和明确的当前项目信任操作 | 未做全局信任记录列表、撤销/显式拒绝管理；`defaultProjectTrust` 是另一项 settings 配置 |
| 全局 `keybindings.json` | 无 Pi 终端快捷键编辑器 | Gupi 快捷键页编辑的是 Gupi 自己的绑定，不能计为 Pi 覆盖 |
| `AGENTS.override.md` / `AGENTS.md` / `CLAUDE.md` 等 | 无专用指令管理页 | 全局、当前目录及父目录指令来源和覆盖预览；普通文件查看不计作配置 UI |
| 全局 `SYSTEM.md`、`APPEND_SYSTEM.md` | 已有提示词页编辑 | 未做项目对应文件编辑与来源比较 |
| 全局 `extensions/skills/prompts/themes` | 已有包及资源管理，覆盖部分文件操作 | 不是所有资源类型的通用编辑器；内置扩展选择仍缺专用 UI |
| 项目 `.pi` 下资源与提示词 | Pi 的可信项目加载能力 | Gupi 当前资源页仍为全局，未做项目级资源管理 |
| 环境变量／启动参数 | Pi 可执行路径已有配置 | 没有通用环境变量/CLI 编辑器；需逐项区分路径、代理、遥测、认证和运行覆盖 |
| 扩展自有文件与 settings 未知键 | 保存时保留未知内容 | 无统一字段全集；需按扩展及版本独立盘点，不能把某扩展的设置当作 Pi 核心契约 |

## 建议顺序与待确定事项

下列只是建议，尚未选定为下一批开发范围：

1. **MCP 管理**：建议独立 Pi 页面。先解决服务列表、配置编辑、作用域、启停和错误定位；保存成功与连接成功分开展示。认证与运行状态接口需先验证。
2. **供应商与模型连接**：建议把 `models.json` 的端点管理和认证入口连贯设计，保留默认模型选择在对话页。优先常用端点，复杂模型元数据先保留文件编辑。
3. **工具与运行环境**：defaultTools、codemode、内置扩展开关、Shell/npm 配置；工具暴露与 MCP 应能相互跳转。
4. **现有页面高级项**：模型范围/按模型覆盖、缓存预热、网络超时/传输、Provider 重试、遥测，按语义放入现有页面。
5. **项目资源与指令**：项目包/Skill/提示词、AGENTS 来源、SYSTEM 覆盖及信任记录管理，需明确全局和项目资源的合并规则。

待确定：MCP 第一批是否包括 OAuth；连接验证是显式按钮还是保存后自动执行（验证会启动进程/联网）；是否首批管理第三方 MCP 替代扩展；models/auth 是否同批；高级字段与仅文件编辑的分界。不得把这些建议写成已批准实现。

## 依据与验证范围

Pi 源码路径相对于上述提交的 `packages/coding-agent/`：`schemas/settings.schema.json`、`docs/configuration.md`、`docs/settings.md`、`docs/models.md`、`docs/mcp.md`、`src/core/mcp-servers.ts`、`src/extensions/mcp/config.ts`。Gupi 对照：`src/features/settings/pi_config/fields.rs`、`src/features/settings/layout.rs`、`src/features/settings/resources/`、`crates/gupi-resources/src/pi_settings/`。

本次核对配置文档、Schema、关键读取/校验源码和 Gupi 设置实现；不代表每项配置已在 Gupi 窗口或 RPC 会话验收。扩展字段没有封闭总数，未扫描个人配置内容。
