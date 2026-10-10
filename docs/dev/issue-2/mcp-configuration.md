# MCP 配置盘点

依据：Pi 1.1.0，提交 `6fb2e7815167e6b19006fc526d1a5d0f5f998787`。这是待接入能力研究，不是已批准的实施方案。总览见[配置覆盖统计](coverage.md)。

## 当前覆盖

Pi 已有内置 MCP 扩展 `builtin:mcp`。Gupi 尚无 MCP 服务配置、认证或连接状态专页。包页能安装 MCP 相关扩展，不等于能编辑内置 MCP 配置，也不能据此认定 Gupi 中 MCP 不可运行。

配置位于 `<agent-dir>/mcp.json` 和 `<cwd>/.pi/mcp.json`；OAuth 凭据由 Pi 保存在全局 `mcp-auth.json`。本次没有读取用户这些文件，也没有启动服务器验证连接。

## 配置字段清单

以下全部尚无 Gupi 专用控件。按结构列出，不把任意服务名、工具名和环境变量名当作固定字段计数。

| 层级 | 字段 | 用途与约束 |
| --- | --- | --- |
| 根对象 | `mcpServers` | 服务名到配置对象的映射 |
| 根对象 | `autoEnableCodemode` | 是否自动启用 codemode；项目值优先 |
| 服务通用 | `type` | 可省略；支持 stdio、http、streamable-http；不支持旧 SSE |
| stdio | `command`、`args`、`env`、`cwd` | 可执行文件与分立参数；command 不是 Shell 命令串；相对 cwd 基于会话目录 |
| HTTP | `url`、`headers` | Streamable HTTP 地址和请求头 |
| 服务通用 | `enabled` | 默认启用；关闭保留配置但不连接 |
| 服务通用 | `timeout` | 请求超时，单位秒，默认 60；进度通知会重置超时 |
| 服务通用 | `description` | 服务描述，参与工具发现及系统提示 |
| 服务通用 | `exposure` | codemode（默认）／deferred／direct／hidden；兼容别名 codemode-deferred |
| 服务通用 | `toolExposure` | 按具体工具名或 `*` 模式覆盖；精确名称优先，模式按对象中的首次匹配 |
| HTTP OAuth | `oauth.clientId`、`oauth.clientSecret` | 预注册客户端；secret 支持变量和命令引用 |
| HTTP OAuth | `oauth.callbackPort`、`oauth.callbackUrl` | 本机回环回调；具体 URI、端口与路径限制由 Pi 校验 |
| HTTP OAuth | `oauth.scope`、`oauth.clientName` | 请求权限及注册名称 |
| HTTP OAuth | `oauth.clientRegistration` | dcr／cimd；cimd 与部分自定义客户端字段互斥 |
| HTTP OAuth | `oauth.authServerMetadataUrl` | 显式认证服务器元数据地址；HTTPS 或回环地址 |
| HTTP Provider 认证 | `auth.provider` | 使用 Pi Provider 的令牌；只允许全局定义，目标必须 HTTPS（回环例外） |

`env`、`headers` 可引用 `${NAME}` 或完整的 `!command`。配置查看和表单预览不应自行求值，否则读配置可能执行命令。OAuth 与 Provider 认证由 Pi 处理；Gupi 只保存引用，不应把解析后的令牌写回项目文件。

## 与普通 settings.json 不同的规则

- 项目 MCP 文件仅在项目受信任后加载。保存文件不等于信任项目。
- 项目完整服务定义替换同名全局定义，不能套用 settings 的递归对象合并。
- 特例：项目条目不含 `command`、`url`、`type` 时，是局部覆盖；只允许 `enabled`、`exposure`、`toolExposure`，且必须已有同名全局服务。其他内容沿用全局配置；`toolExposure` 本身按对象替换，不能自行改成逐工具深合并。
- 服务名只允许字母、数字、`_`、`-`，`a-b` 与 `a_b` 的工具命名空间冲突。
- OAuth 凭据按服务名与 URL 关联；不同名称同 URL 分别登录，相同名称和 URL 可共享登录。
- 扩展可通过 `registerMcpServer` 注册会话级服务；不落盘，同名文件配置优先。必须区分配置来源与运行时来源。
- 安装的扩展若注册 `/mcp`（如 pi-mcp-adapter），会替代会话内置 MCP；这时会话不读取内置 `mcp.json`。`pi mcp` CLI 仍使用内置实现，故 CLI 成功不能证明当前会话实际加载了同一批服务。

## 可用上游入口与待调研边界

| 上游入口 | 已知行为 | Gupi 接入仍需确认 |
| --- | --- | --- |
| `pi mcp add/remove` | 修改全局或 `--local` 项目文件；add 不连接，remove 保留 OAuth 凭据 | CLI 与直接文件编辑的选择、冲突保护与错误映射 |
| `pi mcp list` | 连接所有启用服务并报告状态；失败退出码为 1 | 结构化输出、超时/取消及副作用；不能当作纯文件读取 |
| `pi mcp login/logout` | 登录或注销 MCP OAuth | 浏览器回调、取消、凭据状态与 Gupi 生命周期 |
| `/mcp` | 终端有列表与交互；非交互模式打印状态 | RPC 实际返回形态与可展示状态，不假定终端 UI 能直接移植 |
| `/mcp login/logout/reconnect <server>` | 会话内直接动作 | RPC 调用契约、错误和取消、替代扩展兼容性 |
| `/reload` 或重连 | 上游支持重载外部编辑 | Gupi 首批仍按已定手动重连契约；不得自行引入自动重启 |

建议第一步提供独立 MCP 页面，明确全局／项目作用域、配置来源、继承覆盖、文件错误及启停；连接状态独立于配置保存状态。是否将 OAuth、工具级暴露和第三方替代扩展纳入第一批，仍待选择。不得执行 `pi mcp list` 来“无副作用读取配置”。

## 源码依据

路径相对于本地 Pi 的 `packages/coding-agent/`：

- `docs/mcp.md`、`docs/cli.md`：用户配置、操作入口及替代扩展行为。
- `src/core/mcp-servers.ts`：stdio/HTTP/OAuth/Provider 认证结构和校验。
- `src/extensions/mcp/config.ts`：全局／项目加载、局部覆盖、写入与认证限制。
- `src/core/trust-manager.ts`：项目 MCP 文件需要信任。

以上属于源码与文档核对，尚未验证 Gupi RPC 中的 MCP 管理交互。
