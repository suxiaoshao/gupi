# #48 + #51 自定义模型与 MCP 配置文件编辑

状态：Draft，待架构师审查。

依据：Pi v1.2.0（本地 `/Users/sushao/Documents/code/pi`，标签 `v1.2.0`），路径相对 `packages/coding-agent/src/`。研究记录见 [MCP 配置盘点](../issue-2/mcp-configuration.md)。

## 目标与范围

本 PR 只做两个文件的**原样编辑**：

- `models.json`：自定义供应商与模型，#48 的文件部分；
- `mcp.json`：MCP 服务，全局与项目，#51 的文件部分。

**原则**：与 #47、#49/#50 相同，Gupi 按 JSON 文件原样编辑。

- 不解析、不显示 Pi 的生效结果或默认值；
- 不探测连接，不预览合并后的模型目录；
- 未编辑的字段和未知字段原样保留，对象键顺序保留。

**不在本 PR**：

- `auth.json`：API Key 设置/移除、OAuth 登录登出；
- MCP OAuth 登录登出（`mcp-auth.json`）；
- 连接和认证状态显示；
- 保存后让运行中的会话重新加载。

上述内容另开 PR。#48、#51 在本 PR 合并后保持打开，在评论中说明剩余部分。

## Pi 规则

| 规则 | 依据 |
| --- | --- |
| `models.json` 只有全局一份（`<agent-dir>/models.json`），没有项目层 | `core/agent-session-services.ts:144`、`core/model-runtime.ts:218-221` |
| `models.json`：根为 `providers` 映射。供应商字段 `name/baseUrl/api/apiKey/headers/compat/authHeader/oauth`，以及 `models` 数组与 `modelOverrides` 映射。模型定义必须有 `id`；覆盖只含部分元数据，不支持 `api/baseUrl` | `core/model-config.ts:32-87` |
| `models` 与 `modelOverrides` 语义不同：前者新增或替换同 ID 模型，后者只修改已存在的模型，不创建新 ID | `provider-composer.ts:180-248,300-339` |
| Pi 读取时允许 BOM 和注释，Pi 自己不写 `models.json` | `core/model-config.ts:124-161` |
| `mcp.json`：根为 `mcpServers` 映射与可选 `autoEnableCodemode` | `extensions/mcp/config.ts:74-149` |
| 项目 `.pi/mcp.json` 只在项目受信任后加载；项目里同名完整定义整条替换全局 | 同上 |
| 项目条目不含 `command`、`url`、`type` 时是**局部覆盖**：只允许 `enabled`、`exposure`、`toolExposure`，且必须有同名全局服务；`toolExposure` 整体替换 | 同上 |
| 项目完整定义不能使用 `auth.provider` | 同上 |
| 服务名只能用字母、数字、`_`、`-`；`a-b` 与 `a_b` 冲突 | `core/mcp-servers.ts` |
| `toolExposure` 中精确名称优先，模式按对象中的先后取第一个匹配，**键顺序有意义** | `core/mcp-servers.ts` |
| Pi 的 `pi mcp add/remove` 与 `/mcp` 写 `mcp.json` 时不加锁，原地覆盖写入 | `extensions/mcp/config.ts:169-241` |
| 两个文件都在会话或扩展启动时读取；标准 RPC 没有重新加载命令 | `model-runtime.ts:848-856`、`extensions/mcp/index.ts:1114-1128` |

## 已确认的决定

- **B（用户确认）**：`mcp.json` 采用“保存前重新读取，发现条目变化就报冲突”。
  - Pi 写这个文件不加锁，Gupi 读完到写入之间仍有极短的覆盖窗口。
  - 这个限制写在本文档和界面说明里，不另做加锁。
- **C（用户确认）**：`models.json` 保存后不保留注释和原有排版，字段内容不丢。
  - 文件含注释时，页面显示一行提示：“保存会移除文件中的注释”。
- **敏感字段**不明文显示，也不复制到 Gupi 配置、日志、剪贴板和错误信息中。
  - 涉及的字段：`apiKey`、所有 `headers` 的值、`env` 的值、`oauth.clientSecret`。
  - 已有值显示为“已设置”，只能替换或清除。
  - 不对 `${NAME}`、`!command` 求值。

## 界面

Pi 设置新增两页：“模型与供应商”（仅全局）和“MCP”（全局/项目作用域，与现有作用域选择器一致）。

### 列表

- 两页都是条目列表：
  - 模型页每行一个供应商，显示名称和 `baseUrl`；
  - MCP 页每行一个服务，显示名称、stdio/HTTP 类型、`enabled: false` 时的“已关闭”标记。
- 标题行右侧“添加”。行尾 `⋯` 菜单：编辑、删除。
- **项目作用域只显示项目文件中的条目**，与 #47 相同，不显示继承的全局服务。
  - 添加菜单分两项：“新服务”；“覆盖全局服务…”，从全局服务名中选择，新建只含 `enabled` 的局部覆盖。
  - 局部覆盖行标“覆盖”。
- `autoEnableCodemode` 在 MCP 页是一行设置，三态：未写入 / 开 / 关。

### 编辑

点击编辑打开表单（sheet 或 dialog，按 GPUI Kit 设计指南选型）。保存时把改动的字段合并回原对象，未编辑和未知字段原样保留。

- **简单字段**用普通控件：字符串、数字、布尔、枚举。
  - `exposure` 选项：codemode / deferred / direct / hidden。
  - `type` 选项：stdio / http。别名 `streamable-http` 原样保留。
  - 没写的字段保持“未写入”，不显示默认值。
- **字符串数组**（`args`、模型 `input`）：有序列表，复用 #47 的列表控件。
- **键值映射**（`headers`、`env`）：
  - 键可见，值按敏感字段处理；
  - 可添加、删除，可替换值。
- **`toolExposure`**：有序的“名称或模式 → exposure”列表，保留并可调整顺序。
- **`oauth`**：各标量字段用普通控件，`clientSecret` 按敏感字段处理。
- **`models`**：供应商表单里的有序列表，每个模型可展开编辑。
- **`modelOverrides`**：供应商表单里以模型 ID 为键的映射，每项可展开编辑。
- **复杂嵌套对象**用 JSON 文本框编辑该字段的原始值，保存时只检查是合法 JSON。不为它们做结构化表单。涉及的对象：
  - `compat`、`cost`、`thinkingLevelMap`、`inputLimits`、`promptCache`；
  - `samplingParams`、`samplingParamsByThinkingLevel`。
- **局部覆盖条目**只显示三个允许的字段。
- **最低限度的检查**，只拦会让 Pi 直接拒绝的输入：
  - 必填：模型 `id`；完整 MCP 定义要有 `command` 或 `url`；
  - 服务名的字符集和 `-`/`_` 冲突；
  - 正数字段大于 0；
  - 项目完整定义不能有 `auth.provider`。
  - 其余交给 Pi。

### 说明文字

两页的组说明各一行：“保存后在新会话中生效。” MCP 页另加一句并发写入的限制说明。

## 存储

- 复用 `gupi_resources::pi_settings` 的文档读取、按叶提交、重新读取比对和原子写入，但不复用 settings 的继承与默认值解析。
  - 冲突单位是单个条目：`["providers", <name>]`、`["mcpServers", <name>]`，以及 `["autoEnableCodemode"]`。
  - 一个条目被外部改过，保存这个条目就报冲突；其他条目不受影响。
- `models.json` 读取：去掉 BOM 和注释后解析。写回为普通 JSON。
- 项目 `mcp.json` 写入前检查项目目录存在，不自动信任项目。缺 `.pi/` 时创建，与 #47 项目写入一致。
- 全局 `mcp.json` 和项目 `mcp.json` 是两个文件，分别读取和提交。

## 所有权与实施顺序

1. `gupi-resources`：
   - `models.json`、`mcp.json` 的读取，包括容忍注释和 BOM；
   - 按条目提交；
   - 敏感字段的“替换/清除/保持”补丁。
2. `src/features/settings`：两个新页面、列表与编辑表单，接入作用域选择与离开保护。
3. 九种语言文案。

## 验证

**单元测试**：

- 两个文件的往返：未知字段、键顺序、未编辑条目保持不变；
- `models.json` 含注释和 BOM 时可读，保存后为普通 JSON；
- 敏感字段：未修改时保存保持原值；替换和清除都正确；值不进入日志或错误信息；
- 按条目冲突：外部修改其他条目不阻止保存；修改同一条目报冲突；
- 项目局部覆盖只写允许的三个字段；
- 校验拦截缺必填项、非法服务名、项目 `auth.provider`。

**原生界面**：worker 用 `script/gupi-ui-gallery` 临时配置，检查两页的列表、添加、编辑、删除、项目覆盖和敏感字段显示。

- 不读取真实 `~/.pi` 下的 `models.json`、`mcp.json`、`auth.json`；
- 不连接 MCP 服务，不发起 OAuth。

**检查命令**：`cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`，以及受影响包的测试。
