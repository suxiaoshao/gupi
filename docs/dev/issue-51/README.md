# MCP 登录与连接检查（Gupi #51）

状态：方案待评审。

依据：Pi v1.2.0（本地 `/Users/sushao/Documents/code/pi`），路径相对 `packages/coding-agent/src/`。`mcp.json` 的文件编辑（服务增删改、启停、`exposure` / `toolExposure`、项目覆盖）已由 [#48 + #51 文件编辑](../issue-48-51/README.md) 完成。研究记录见 [MCP 配置盘点](../issue-2/mcp-configuration.md)。本文只覆盖 [Gupi #51](https://github.com/suxiaoshao/gupi/issues/51) 的剩余部分。

## 目标与范围

在 **Pi → MCP** 页：

- 对使用 OAuth 的服务登录、登出；
- 用户点击时检查所有服务的连接状态；
- 说明如何让运行中的会话加载更改。

**用户已确认的决定**：

- 登录和登出放在设置页，调用 Pi 官方命令 `pi mcp login` / `pi mcp logout`，不在会话内通过 RPC 执行 `/mcp login`。
- 连接检查只在用户点击时运行，结果标为“上次检查”，不作为实时状态。检查会实际连接所有启用的服务，用户已接受这一点。
- 运行中的会话通过已有的会话菜单 **重新连接** 加载更改；保存时不自动重启会话。
- MCP 凭据只存在 Pi 的全局 `mcp-auth.json`，由 Pi 读写。Gupi 不读取、不解析、不复制该文件，也不新增存储。
- 不发起上游请求，不使用私有桥接或模拟终端按键。

**不在本期**：

- 实时连接状态面板；
- 对运行中的会话即时启停服务或更改暴露；
- 只读展示已存凭据的元数据；
- 清理已删除服务遗留的凭据；
- 第三方扩展替代内置 MCP 时的识别。

供应商 OAuth 登录仍不在 Gupi 中实现，因为 Pi 没有对应的非终端入口（`pi auth` 只有 `check`、`print-api-key`、`print-bearer-token`）。

## Pi 规则

| 规则 | 依据 |
| --- | --- |
| `pi mcp` 按进程 cwd 和 agent dir 加载配置：全局 `mcp.json`，加上已信任项目的 `<cwd>/.pi/mcp.json`；项目未信任时忽略项目文件，并在 list 的 JSON `note` 中说明 | `extensions/mcp/cli.ts:200-206` |
| `login` / `logout` 需要已解析配置中存在同名服务，并且该服务可用 OAuth（HTTP、无 `auth.provider`、无任何大小写的 `Authorization` 头），否则退出码 1 | `cli.ts:218-238`、`extensions/mcp/runtime.ts:82-86,215-216` |
| `logout` 只删除本地已存凭据，不连接、不吊销，也不断开运行中会话的连接 | `cli.ts:239-243` |
| `login` 默认限时 300 秒，可用 `--timeout` 指定；非 TTY 下通过浏览器回调完成，粘贴回调 URL 只在 TTY 可用；没有 JSON 输出；退出码 1 也可能是凭据已保存但随后重连失败 | `cli.ts:244-252,516-594` |
| `list --json` 连接所有启用的服务、枚举工具、可能刷新令牌、求值配置中的 `!command` 与环境引用，然后关闭连接；停用的服务不连接。没有单服务过滤或超时参数 | `cli.ts:209-216,434-485` |
| `list` 的 JSON 每项含 `name`、`scope`、`source`、`override`、`enabled`、`exposure`、`transport`、`state`、`tools`、`toolExposure`、`resources`、`error`；`state` 为 `connecting` / `connected` / `disconnected` / `needs-auth` / `failed` / `closed` 或停用时的 `disabled`。有服务未连接或配置有错时退出码为 1，但 JSON 仍有效 | `cli.ts:434-485`、`runtime.ts:57` |
| CLI 不加载会话扩展注册的令牌提供方；使用 `auth.provider` 的服务在 CLI 检查中可能显示为未认证，而会话中实际可用 | `cli.ts:118-126`、`runtime.ts:186-198` |
| 会话在 `session_start` 时读取 `mcp.json`；没有 RPC 重新加载命令；`/mcp reconnect` 只重开原连接，不重读配置 | `extensions/mcp/index.ts:720-726,1114-1128` |
| 处于 `needs-auth` 的会话连接会在下一轮 `turn_start` 时比较已存令牌并重连，因此在外部登录后，运行中的会话通常在下一轮对话时可用 | `index.ts:538-557,1211-1214` |
| 凭据按 `mcp__<名称>\|<URL>` 存在全局 `mcp-auth.json`，项目覆盖同名同 URL 的服务共用同一凭据 | `extensions/mcp/oauth.ts:114-132` |

## 设计

### 1. 调用方式

复用认证检查的子进程约定（`crates/gupi-resources/src/pi_auth/check.rs`）：通过 `pi_rpc::probe::resolve_command_with_env` 解析 Pi，使用当前运行时环境快照，不经过 shell，Windows 设置 `CREATE_NO_WINDOW`；stdout 与 stderr 有界、私有捕获，不写日志、不原样显示，超时或取消时终止子进程。

工作目录决定配置作用域：项目作用域使用项目文件夹；全局作用域使用不含 `.pi/mcp.json` 的空临时目录，以免意外合并某个项目的配置。

### 2. 登录与登出

- **入口**：MCP 服务行的菜单。只在该服务按已保存文件判断可用 OAuth 时提供（规则同上表）。项目覆盖行按合并后的有效定义判断。
- **前提**：页面有未保存修改时不可用，并提示先保存。原因是 Pi 读取的是磁盘上的文件，不是 Gupi 的草稿。
- **登录**：打开对话框说明“已在浏览器中打开登录页面，请在浏览器中完成登录”，提供 **取消**（终止子进程）。Gupi 设置有限的超时（推荐沿用 Pi 默认 300 秒，并通过 `--timeout` 传入）。结束后按退出码显示通用结果：成功、失败或超时；不显示 Pi 输出原文。退出码 1 的提示需说明“凭据可能已保存但连接失败”，并建议检查连接。
- **登出**：确认对话框“登出 MCP 服务 "<名称>"？”，使用 Danger 提交。说明只删除本机保存的凭据，运行中的会话在重新连接后才不再使用。
- 同一时间只运行一个登录、登出或检查操作；设置窗口关闭或离开页面时取消正在进行的操作。

### 3. 检查连接

- **入口**：MCP 页标题区的 **检查连接** 按钮，作用于当前作用域；运行时显示进度，可取消。Gupi 设置整体超时（推荐 60 秒）。
- **结果投影**：只从 JSON 取白名单字段：`name`、`scope`、`enabled`、`state`、工具数量、是否有资源，以及 `note` 是否存在。不显示或记录 `transport`（含完整 URL 或命令参数）、`error` 原文、`errors` 和 stderr；它们可能包含凭据。
- **展示**：每行显示状态标签：已连接（工具数）、需要登录、连接失败、已停用；标题区显示“上次检查 <时间>”。`needs-auth` 行提供登录入口。项目未信任时显示“项目未受信任，Pi 只使用全局配置”。
- 结果只代表当次检查；保存文件、登录或登出后清除结果，避免与磁盘内容不一致。说明文字提示：使用 `auth.provider` 的服务在检查中可能显示需要登录，但会话中可用。
- JSON 无法解析或缺少必需字段时，整体报告“检查失败”，不显示部分结果。

### 4. 让运行中的会话加载更改

保存 MCP 页、登录或登出成功后，页面提示：“已运行的会话需要在会话菜单中选择 **重新连接** 后加载更改；新会话自动加载。”复用 `crates/gupi-conversation/src/conversation/reconnect.rs` 的现有流程，不新增重连机制，也不从设置页批量重启会话。

### 5. 文案与本地化

新增文案覆盖九种语言，沿用 [设置文案](../settings-polish/README.md) 的用词：对话框标题写明对象和动作，确认按钮使用动词，检查结果使用“上次检查”。

## 实施顺序

1. `gupi-resources`：`pi mcp login/logout/list --json` 子进程封装、结果枚举与 list 白名单解析，含超时和取消。
2. MCP 页：检查连接按钮与行状态标签。
3. 行菜单登录、登出对话框与操作互斥。
4. 保存、登录、登出后的重新连接提示。
5. 九种语言文案。

## 验证

- 单元测试：list JSON 白名单解析（含退出码 1 时的有效 JSON、缺字段、未知状态、`note`）；OAuth 可用性判断（HTTP、`auth.provider`、`Authorization` 大小写、项目覆盖）；登录与登出退出码映射。不把 Pi 输出原文暴露给调用方。
- 子进程：用临时假 Pi 覆盖成功、失败、超时、取消，确认超时后子进程被终止、作用域对应的工作目录正确。
- 界面回归：未保存时入口不可用；操作互斥；检查结果在保存、登录、登出后清除；登出确认文案。
- 原生：worker 用 `cargo run -p xtask --locked -- gallery ui` 的隔离环境和假 Pi，检查 MCP 页的检查连接、状态标签、登录对话框（含取消）、登出确认与重连提示，浅色与深色各一次。不连接真实 MCP 服务，不发起真实 OAuth，不读取真实 `~/.pi`。
- 检查命令：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test --workspace --locked`（隔离 `HOME` 与 Pi 目录）。
