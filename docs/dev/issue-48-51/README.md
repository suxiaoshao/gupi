# #48 + #51 自定义模型与 MCP 配置文件编辑

状态：Ready。架构师已审查；A/B/C 由用户确认。

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

上述内容另开 PR。

## Pi 规则

| 规则 | 依据 |
| --- | --- |
| `models.json` 只有全局一份（`<agent-dir>/models.json`），没有项目层 | `core/agent-session-services.ts:144`、`core/model-runtime.ts:218-221` |
| `models.json`：根为 `providers` 映射。供应商字段 `name/baseUrl/api/apiKey/headers/compat/authHeader/oauth`，以及 `models` 数组与 `modelOverrides` 映射。模型定义必须有 `id`；覆盖只含部分元数据，不支持 `api/baseUrl` | `core/model-config.ts:32-87` |
| `models` 与 `modelOverrides` 语义不同：前者新增或替换同 ID 模型，后者只修改已存在的模型（包括扩展注册的模型，在扩展组合之后应用），不创建新 ID | `provider-composer.ts:180-248,300-339,553-574` |
| Pi 读取 `models.json` 时允许 BOM、`//` 行注释和尾逗号，**不接受** `/* */` 块注释；Pi 自己不写这个文件 | `core/model-config.ts:138-140`、`utils/json.ts:1-6` |
| `providers` 必填：删除最后一个供应商后必须保留 `{"providers": {}}` | `core/model-config.ts:80-87,148-154` |
| 供应商的 `oauth` 只是标量字面量 `"radius"`，与 MCP 的 OAuth 对象无关 | `core/model-config.ts:67-77` |
| `mcp.json` 用普通 `JSON.parse` 读取，不接受注释和 BOM | `extensions/mcp/config.ts:91-103,232-234` |
| `exposure` 的旧别名 `codemode-deferred` 仍被接受 | `core/mcp-servers.ts:43-57,215-229` |
| `oauth.callbackPort` 必须是 1–65535 的整数 | `core/mcp-servers.ts:174-176` |
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
  - 这个限制写在本文档里，不另做加锁；也不使用 settings 的 `.lock` 目录，因为它不能与 Pi 同步，不能当作保护。
- **C（用户确认）**：`models.json` 保存后不保留注释、尾逗号和原有排版，字段内容不丢。只适用于 `models.json`；`mcp.json` 按 Pi 的严格 JSON 读取。
  - 文件含 `//` 注释时，页面显示一行提示：“保存会移除文件中的注释”。检测须跳过字符串内容，不把 `https://` 当作注释。
- **敏感字段**不明文显示，也不复制到 Gupi 配置、日志、剪贴板和错误信息中。
  - 只保护以下具名字段，不做通用的密钥扫描：
    - 供应商 `apiKey`；
    - 供应商、每个模型、每个 `modelOverrides` 项的 `headers` 值；
    - MCP 的 `env` 值、`headers` 值和 `oauth.clientSecret`。
  - 编辑状态为三种：**保持**、**替换为新值**（可以是空字符串）、**移除**。
    - 原值不进入输入框；“已设置”只是显示，绝不写入文件。
    - 移除与替换为空字符串是两件事。
  - 不对 `${NAME}`、`!command` 求值。
  - 原始文档和补丁不通过 `Debug` 输出到日志或错误信息。

## 界面

Pi 设置新增两页：“模型与供应商”（仅全局）和“MCP”（全局/项目作用域，与现有作用域选择器一致）。

### 列表

- 两页都是条目列表：
  - 模型页每行一个供应商，只显示键名（`baseUrl` 可能含凭据，不在列表预览）；
  - MCP 页每行一个服务，显示名称、stdio/HTTP 类型、`enabled: false` 时的“已关闭”标记。
- 标题行右侧“添加”。行尾 `⋯` 菜单：编辑、删除。
- **项目作用域只显示项目文件中的条目**，与 #47 相同，不显示继承的全局服务。
  - 不提供“从全局服务中选择”的选择器。新建条目时类型选 stdio / http / “覆盖全局同名服务”。选后者时只填名称和三个允许字段，`enabled` 由用户明确选择，不自动写 `true`。
  - 局部覆盖行标“覆盖”，不显示推断的 stdio/HTTP 类型。
  - 添加前检查同文件名称冲突（含 `-`/`_` 冲突），并检查与全局文件中原始服务名的 `-`/`_` 冲突。两个文件之间不是事务。
- `autoEnableCodemode` 在 MCP 页是一行设置，三态：未写入 / 开 / 关。

### 编辑

点击编辑打开表单（sheet 或 dialog，按 GPUI Kit 设计指南选型）。保存时把改动的字段合并回原对象，未编辑和未知字段原样保留。

- **简单字段**用普通控件：字符串、数字、布尔、枚举。
  - `exposure` 选项：codemode / deferred / direct / hidden。
  - `type` 选项：stdio / http。别名 `streamable-http`、`codemode-deferred` 及不认识的枚举值在无关编辑中原样保留，不归一化。
  - 没写的字段保持“未写入”，不显示默认值。
- **字符串数组**（`args`、模型 `input`）：有序列表，复用 #47 的列表控件。
- **键值映射**（`headers`、`env`）：
  - 键可见，值按敏感字段处理；
  - 可添加、删除，可替换值。
- **`toolExposure`**：有序的“名称或模式 → exposure”列表，保留并可调整顺序。
- **MCP `oauth`**：各标量字段用普通控件，`clientSecret` 按敏感字段处理。
- **MCP `auth.provider`**：全局完整定义中的普通字符串字段，只是文件字段，不涉及登录。
- **供应商 `oauth`**：只有未写入或 `"radius"` 两种选择。
- **`models`**：供应商表单里的有序列表，每个模型可展开编辑，字段按 `model-config.ts:32-48`。
- **`modelOverrides`**：供应商表单里以模型 ID 为键的映射，每项可展开编辑，字段按 `model-config.ts:50-64`，没有 `api`、`baseUrl`。
- 缺键与 `null`、`[]`、`{}` 区分保存；已有的不支持形状在无关编辑中原样保留。
- **复杂嵌套对象**用 JSON 文本框编辑该字段的原始值，保存时只检查是合法 JSON。不为它们做结构化表单。涉及的对象：
  - `compat`、`cost`、`thinkingLevelMap`、`inputLimits`、`promptCache`；
  - `samplingParams`、`samplingParamsByThinkingLevel`。
- **局部覆盖条目**只编辑三个允许的字段；已有的其他键原样保留，不删除、不转成完整定义。
- **最低限度的检查**，只拦会让 Pi 直接拒绝的输入：
  - 必填：模型 `id`；完整 MCP 定义要有 `command` 或 `url`；
  - 服务名的字符集和 `-`/`_` 冲突；
  - 正数字段大于 0；`callbackPort` 为 1–65535 的整数；
  - 项目完整定义不能有 `auth.provider`。
  - 其余交给 Pi。保存成功不代表 Pi 一定接受该配置。

### 说明文字

两页的组说明各一行：“Pi 在新会话启动时读取此文件，已有会话不会重新加载。”

## 存储

- 复用 `pi_settings` 的按叶提交算法（重新读取、比对、应用、原子写入），抽出不带 `.lock` 目录的最小路径；不复用 settings 的继承与默认值解析。
  - 读取用各文件自己的解析器，**首次读取和保存前重读都用它**：`models.json` 去 BOM、`//` 行注释和尾逗号；`mcp.json` 严格 JSON。
  - 原子写入在同一目录暂存后替换；保留项目目录校验。
  - 删除条目只删该条目，不连带删除变空的父对象（删除最后一个供应商后保留 `{"providers": {}}`）。
  - **比较要区分键顺序**：`serde_json::Value` 的相等忽略键顺序（`preserve_order` 下也如此），所以脏检查和冲突检查都改用按顺序比较，例如递归按序比较或比较有序序列化结果。替换已有键时保留它原来的位置。
  - 冲突单位是单个条目：`["providers", <name>]`、`["mcpServers", <name>]`，以及 `["autoEnableCodemode"]`。
  - 一个条目被外部改过，保存这个条目就报冲突；其他条目不受影响。
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
- `models.json` 含 `//` 注释、尾逗号和 BOM 时，首次读取和保存前重读都能解析；保存后为普通 JSON；字符串里的 `https://` 不被当作注释；
- 删除最后一个供应商后保留 `{"providers": {}}`；
- 本地只调整顺序（如 `toolExposure`）也算修改并能保存；外部只调整顺序时报冲突；
- 别名与不认识的枚举值在无关编辑中保留；
- 敏感字段：保持、替换（含空字符串）、移除三种状态，覆盖供应商、模型、覆盖项的 headers 和 MCP 的 env、headers、clientSecret；值不进入日志或错误信息；
- 按条目冲突：外部修改其他条目不阻止保存；修改同一条目报冲突；
- 项目局部覆盖只写允许的三个字段，已有其他键保留；添加时的名称冲突被拦截；
- 校验拦截缺必填项、非法服务名、项目 `auth.provider`。

**原生界面**：worker 用 `script/gupi-ui-gallery` 临时配置，检查两页的列表、添加、编辑、删除、项目覆盖和敏感字段显示。

- 不读取真实 `~/.pi` 下的 `models.json`、`mcp.json`、`auth.json`；
- 不连接 MCP 服务，不发起 OAuth。

**检查命令**：`cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`，以及受影响包的测试。
