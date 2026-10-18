# Pi 供应商认证（Gupi #48）

状态：实施步骤 1–4 已实现，本地自动检查通过；原生交互验证尚未完成。

依据：Pi v1.2.0（本地 `/Users/sushao/Documents/code/pi`，用 `git show v1.2.0:<path>` 核对），路径相对 `packages/coding-agent/src/`，`ai/` 指 `packages/ai/src/`。`models.json` 与 `mcp.json` 的文件编辑已由 [#48 + #51 文件编辑](../issue-48-51/README.md) 完成，本文只覆盖 [Gupi #48](https://github.com/suxiaoshao/gupi/issues/48) 的剩余部分：全局 `auth.json`。

## 目标与范围

Pi 设置新增“认证”页，只有全局作用域：

- 列出 `auth.json` 中已存凭据的供应商及其类型、OAuth 过期时间；
- 设置、替换、移除供应商的 API Key；
- 注销 OAuth 凭据；
- 按需检查某个供应商的 Pi 认证配置。

**用户已确认的决定**：

- Gupi 直接写 `auth.json`，遵循 Pi 的文件锁与写入方式。
- 凭据只存在 Pi 的 `auth.json`，权限沿用 Pi 现有行为；不使用系统钥匙串，不迁移，不复制到 Gupi 配置。
- 供应商 OAuth 登录暂缓：Gupi 显示状态并提供注销，登录引导用户在 Pi 终端执行 `/login <provider>`；不模拟 TUI 按键，不引入 JS 桥接。
- 保存后不自动重连会话，页面说明更改在新会话中完整生效。

**不在本期**：Gupi 内的 OAuth 登录流程、MCP OAuth（`mcp-auth.json`，归 #51）、环境变量或 `models.json` `apiKey` 的生效判断、令牌有效性校验、服务端吊销。

## Pi 规则

| 规则 | 依据 |
| --- | --- |
| 路径为 `<agent-dir>/auth.json`；路径规范化，锁用 `realpath: false` | `core/auth-storage.ts:61-66,86` |
| 根是以供应商 ID 为键的对象，每个供应商一条带 `type` 标签的凭据：`api_key { key?, env? }` 或 `oauth { access, refresh, expires, ... }` | `ai/auth/types.ts:17-37` |
| `api_key.env` 保存供应商级环境/配置值（如 Cloudflare 账户、网关 ID），与 `key` 并列 | `ai/auth/types.ts:17-24` |
| 读取为严格 JSON，去 BOM | `core/auth-storage.ts:363-367` |
| 写入流程：父目录不存在时以 0700 创建，文件不存在时以 0600 写 `{}`；加 `auth.json.lock` 目录锁，锁内重读、修改、**原地** `writeFileSync`（不经临时文件替换），已有文件的权限与 ACL 不变 | `core/auth-storage.ts:36-37,68-107,175-200` |
| 锁阈值不统一：同步路径用 proper-lockfile 默认值（10 s 过期、5 s 心跳），异步路径 30 s 过期、15 s 心跳、最长重试 30 s，Windows `EPERM` 最多 5 次 | `core/auth-storage.ts:81-94,117-148`、[proper-lockfile 4.1.2 `lib/lockfile.js:190-202`](https://github.com/moxystudio/node-proper-lockfile/blob/v4.1.2/lib/lockfile.js#L190-L202) |
| 修改回调返回 `undefined` 表示不改；删除是单独操作，删整条凭据；写回统一为 `JSON.stringify(data, null, 2)` | `core/auth-storage.ts:450-489` |
| 运行中的 Pi 读凭据时比对文件版本并在锁内重读，外部写入会在下次读取时生效；运行时覆盖（`--api-key` 等）优先 | `core/auth-storage.ts:402-443`、`core/runtime-credentials.ts:24-27` |
| 外部写入本身不触发 `ModelRuntime.synchronizeCredentialState` 的供应商重组或模型可用性同步；Pi 之后查询可用模型时仍可能自行刷新 | `core/model-runtime.ts:489-504,599-615,826-845`、`modes/rpc/rpc-mode.ts:471,489` |
| 注销只删除本地条目，不吊销服务端令牌；环境变量、`models.json`、运行时凭据仍可能让请求通过认证 | `ai/models.ts:825-833` |
| `pi auth check --provider <id> --json --no-refresh` 用只读存储、不联网、不刷新，输出 `status`（ready / not_ready / invalid）、`reason`、`authType`，不加 `--credentials` 时不含凭据；退出码 ready 0、not_ready 1、invalid 2，参数错误也是 2 且只写 stderr | `cli/auth-check.ts:8-51,66-72`、`main.ts:176-206` |
| 该检查不验证令牌：已过期的 OAuth 在支持 OAuth 的供应商上仍报 ready；内部错误统一报 `invalid_state` | `ai/models.ts:657-664`、`cli/auth-check.ts:46-50` |
| `/login`、`/logout` 只在交互模式；RPC 没有认证命令，`setRuntimeApiKey` 不持久化 | `modes/interactive/interactive-mode.ts:3293-3302`、`modes/rpc/rpc-types.ts:20` 起、`core/runtime-credentials.ts:3-27` |

## 设计

### 存储：`gupi-resources::pi_auth`

新模块拥有 `auth.json` 的读取、元数据投影和凭据写入；它是独立的凭据操作，不复用 #68 的 JSON 文件编辑器。

**读取**：严格 JSON、去 BOM。解析后立即投影为元数据，只保留：

- 供应商 ID、凭据类型（API Key / OAuth / 无法识别）；
- OAuth 的 `expires`（缺失或不是有效数字时为“未知”，不当作 1970 年）；
- API Key 条目是否带 `env`（只记录有无，用于移除确认）。

另在模块内保留每个条目的不透明修订值，由**整个条目**（含 `key`、`env` 值、令牌、未知字段）计算，条目缺失有单独标记；它只用于冲突检测，不显示、不记录、不持久化。

凭据值不离开模块：文档与条目类型不实现 `Debug`，错误信息只区分“文件无法读取”“格式无效”“锁被占用”等类别，不带解析器原文、行内容或值。根不是对象或文件无效时页面只读，拒绝写入。

**锁**：把 `pi_settings/lock.rs` 中已与 proper-lockfile 兼容的目录锁提升为 crate 内共享模块，`settings.json` 与 `auth.json` 共用。

- 继续遵守现有规则：从不删除其他进程的锁，短暂重试后报“忙”；在第一次破坏性写入前续期 mtime。正常写入只持锁毫秒级，但文件系统阻塞时不保证低于 Pi 的 10 s 过期窗口。
- 凭据路径先自行以私有权限创建父目录，再用 `acquire_existing_parent` 加锁；不使用 `acquire` 内的普通 `create_dir_all`，也不使用 `Lock::write_atomic`。
- 锁的身份判定、续期和放弃逻辑沿用现有实现与测试。

**写入**：

1. 父目录不存在时以 0700 创建，文件不存在时以排他方式按 0600 新建（Unix），已被并发创建则直接使用，绝不截断；Windows 沿用目录继承的 ACL，与 Pi 一致。
2. 加锁，重读并解析当前文件；文件无效时放弃。
3. 只修改目标供应商这一个键，其他条目、未知字段和键顺序原样保留；替换已有键时保留原位置。
4. 按 Pi 的格式（2 空格缩进）序列化，续期后**原地写回**（截断写入并 `sync`），不经临时文件替换，以保留已有文件的权限、Windows ACL 与符号链接目标。部分写入或 `sync` 失败作为错误上报；崩溃安全性与 Pi 自身写入相同。
5. 释放锁。

**操作语义**：

- **设置/替换 API Key**：条目为 `api_key` 时只替换 `key`，保留 `env` 与其他字段；条目不存在时写 `{ "type": "api_key", "key": … }`；条目是 OAuth 或无法识别时整条替换，界面先确认。
- 值原样保存，不对 `!command`、`$NAME` 求值，不做格式校验；空字符串不接受（用“移除”表达删除）。
- **移除 API Key / 注销 OAuth**：删除整条凭据，与 Pi `delete` / `logout` 的磁盘结果相同。条目带 `env` 时确认文案说明会一并移除。
- 冲突以单个供应商为单位：操作或对话框开始时取目标条目的修订值，锁内重读后重新计算，目标条目有任何变化（包括只改 `key`、`env` 值或令牌，以及从缺失变为存在）就报冲突，不覆盖；其他供应商的外部修改不影响保存。

### 检查：按需调用 `pi auth check`

- 只在用户点“检查”时运行，不自动、不轮询；同一供应商检查进行中不重复启动。
- 用连接页配置的 Pi 命令与运行时环境快照启动，参数 `auth check --provider <id> --json --no-refresh`，设超时。供应商 ID 来自列表或已校验的输入，不经过 shell。
- 退出码为 0/1/2 且 stdout 是合法结构化结果时视为有效回复，与进程是否“成功”无关。`status`、`reason`、`authType` 映射到已知枚举再显示本地化文案，未知取值和其他原文一律丢弃；返回的 `provider` 与请求不一致视为无效。
- 启动失败、超时、输出不是合法协议（包括只写 stderr 的参数错误）显示“检查失败”；stderr 始终丢弃。
- 结果文案说明这是配置检查：“Pi 找到了可用的凭据配置”，不表述为令牌有效。

### 界面

“认证”页位于“模型与供应商”之后，只有全局作用域，结构与 #68 两页一致：带图标的侧栏入口、分组、组标题右侧“添加 API Key”、点击行打开操作、行尾 `⋯`、空列表一行空状态。

- **列表**来源：`auth.json` 中的条目，加上 `models.json` 中尚无凭据的供应商（标“未存储凭据”）。`auth.json` 中已无对应供应商的旧条目照常列出，保证可以移除。Pi 没有枚举全部内置及扩展供应商的非交互接口，因此不展示完整供应商目录。
- **行内容**：供应商 ID（等宽）、类型标签；OAuth 显示“记录的到期时间 <本地时间>”或“已超过记录的到期时间”，到期时间未知时不显示。说明：“仅显示本地记录；Pi 使用凭据时可能尝试刷新，必要时请重新登录。”
- **行操作**：
  - API Key：替换、移除、检查；
  - OAuth：注销、检查，以及复制 `/login <provider>` 命令并提示在 Pi 终端中重新登录；
  - 未存储凭据：设置 API Key、检查；
  - 无法识别：移除已存条目、替换为 API Key（确认整条替换）；不显示 OAuth 到期或登录状态。
- **添加 API Key 对话框**：供应商 ID 输入（候选来自 `models.json` 与已存条目，也可手填）、密码型 API Key 输入；确认后立即写入，不进入页面草稿。原值从不进入输入框，已有凭据只显示“已设置”。
- **说明文字**（组说明一行）：“凭据保存在 Pi 的 auth.json 中。更改在新会话中完整生效。Gupi 不会自动重连或同步已有会话的模型列表。”
- 注销确认说明：只删除本机凭据，不会吊销服务端令牌；环境变量或 `models.json` 中的 `apiKey` 仍可能生效。
- 复制命令、检查结果和错误提示均不包含凭据值；无障碍文本同样只出现“已设置”。

每项操作独立立即提交，不与 #68 页面的“保存/放弃”草稿和离开保护耦合。

## 实施顺序

1. `gupi-resources`：锁提升为共享模块；新增 `pi_auth`（读取、元数据投影、设置/替换/移除、按条目冲突、原地写入）。
2. `gupi-resources` 或 `gupi-pi-runtime`（按现有 Pi 命令启动代码的归属放置）：`auth check` 调用与白名单解析、超时。
3. `src/features/settings`：认证页、列表、添加/替换对话框、确认与检查状态；接入设置搜索关键词。
4. 九种语言文案。

## 验证

**单元测试**（`gupi-resources`，使用临时目录，不读真实 `~/.pi`）：

- 设置/替换/移除只改目标键：其他条目、未知字段、键顺序不变；替换 `api_key` 保留 `env`；
- 新建文件为 `{ provider: { type, key } }`，Unix 下文件 0600、新目录 0700；已有文件写入后权限不变、符号链接仍是链接；
- 锁被占用时报忙且文件不变；文件无效或根不是对象时拒绝写入；目标条目只改 `key`、只改 `env` 值、只改 OAuth 令牌、从缺失变为存在均报冲突，其他条目外部修改不阻止；并发创建文件时不截断已有内容；OAuth `expires` 缺失或无效显示为未知；
- 错误与 `Debug` 输出不含凭据值；
- `auth check` 结果解析：退出码 0/1/2 与对应 JSON 的组合、未知 `reason`/`authType` 取值丢弃、`provider` 不一致、退出码 2 仅有 stderr、非 JSON 输出均按规则处理。

**原生界面**：worker 用 `script/gupi-ui-gallery` 临时配置和伪造的 `auth.json` 检查列表、添加、替换、移除、注销确认、复制登录命令、检查结果显示，浅色/深色各一次；确认无障碍文本不含凭据。不读取、不修改真实 Pi 凭据，不发起 OAuth。

**检查命令**：`cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`，以及 `cargo test -p gupi -p gupi-resources --locked`。

## 验证与传输澄清

- 经架构师核对 Pi v1.2.0，供应商 ID 无 ASCII 语法限制。存储查找、替换与移除保留完整 ID；手填只要求非空、单行，不允许控制字符及 Unicode 行分隔符，不静默 trim。
- Pi 会在消费 `--provider` 的值之前扫描认证开关。检查入口拒绝 `--credentials`、`--json`、`--no-refresh`、`--min-expiry` 这四个精确 ID，以及控制字符、行分隔符或 trim 后改变的 ID，不启动子进程；这些凭据仍可设置和移除。
- `/login` 是 Pi TUI 指令，内部空格不做 shell 转义；无法安全完整表示的 ID 复制 `/login`，提示在 Pi 终端选择供应商。

## 实现与验证结果

- `gupi-resources::pi_auth` 提供凭据元数据投影、SHA-256 不透明条目修订、单供应商即时写入和删除；共享目录锁提升至 `pi_lock`。严格 JSON/BOM、私有新建权限、锁内重读、原地写入和同步保持 Pi 的约定。
- 设置新增仅全局的“认证”页，紧随“模型与供应商”：列表与候选、密码输入、替换和移除确认、OAuth 注销与复制登录指令、按需检查、重新读取及本地化错误。九种语言均新增 46 个同名文案键，插值变量核对一致。
- `cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`、`git diff --check` 通过。`cargo test -p gupi -p gupi-resources --locked` 通过：Gupi 115 项、resources 68 项，失败 0；资源测试覆盖存储、冲突、权限、符号链接、锁、脱敏、协议及危险供应商 ID 不启动进程；生产编辑器交互测试确认空值不能提交、输入的 API Key 不进入无障碍 value、替换保留 env。
- 最终测试显式使用临时 Pi 目录；认证页的测试构造器没有默认用户目录，布局测试注入临时路径。首次测试调用遗漏目录环境变量（本地时间 01:49:58 开始，01:50:16 以 130 退出），随后重跑。源码追踪显示当时新增认证页只可能在布局测试中触发 auth/models 读取，未点击认证操作；不能创建 auth 文件或锁、写入/删除凭据或启动认证检查。未取得系统访问轨迹，不能确认该读取是否实际执行。
- 原生尝试使用 `script/gupi-ui-gallery --prepare-only --keep` 创建的临时配置、伪造 auth/models 与假 Pi 检查进程，功能构建可启动，最终代码的 `cargo build -p gupi --locked` 通过。CUA 两次超时；macOS 原生后备只能识别进程、菜单和 CoreGraphics 窗口，AX 树递归返回 application，窗口截图失败。因此列表、添加、替换、移除、注销、复制、检查结果的浅色/深色原生交互和原生无障碍脱敏均未取得通过结论，不能由自动测试代替。未发起 OAuth。
- 与架构师确认的 ID 校验/传输澄清见上节。按用户新决定，删除上游需求草稿及其计划条目，不创建或提交上游请求。

### Review 修正

- 认证检查复用 Pi probe 的可执行文件解析，按运行时快照 PATH 查找（包含 Windows PATHEXT），Windows 采用 CREATE_NO_WINDOW；解析失败不启动进程。
- API Key 编辑器持有写入期间的输入与状态，成功后才清空并关闭；失败在对话框内显示并保留密钥，可直接重试。OAuth/未知条目的整条替换确认在同一对话框中完成，避免切换弹层丢失输入。
- OAuth 的本地记录与刷新说明在供应商组中只显示一次。
- 本轮复测：格式、Clippy（Gupi all-targets，deny warnings）及 Gupi/resources 测试通过（115 + 68，失败 0）。新增快照 PATH/缺失命令回归，编辑器回归验证 Busy 后原输入重试、取消确认保留输入与成功后关闭；测试均显式隔离 Pi 目录。Windows 原生运行未验证，原生界面覆盖限制沿用上文记录。
