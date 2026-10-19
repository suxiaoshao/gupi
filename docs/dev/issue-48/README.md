# Pi 供应商认证（Gupi #48）

状态：实施步骤 1–5 与用户批准的首批界面改善 A1–A6 已实现；本地自动检查和隔离 Gallery 原生回归通过；退出阶段的工具重启异常见验证结果。

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
| 运行时凭据覆盖已存凭据。存在已存凭据时，Pi 按凭据类型调用供应商的认证实现，不再使用 `models.json` 的 `apiKey`；没有已存凭据时，API Key 分支先取扩展的 `apiKey`，其次 `models.json` 的 `apiKey`，否则交给供应商自身解析（可能结合环境变量或账户配置）。认证失败不会回退到下一来源 | `core/runtime-credentials.ts:24-27`、`core/provider-composer.ts:388-392,437-472`、`ai/auth/helpers.ts:20` 起、`ai/auth/resolve.ts:70-92` |
| 供应商 `headers`/`authHeader` 参与请求认证组合，不因已存凭据被整体忽略；扩展可覆盖这些配置，`authHeader: true` 要求解析出 API Key，最终请求还可能有模型或请求级覆盖 | `core/provider-composer.ts:374-400,429`、`ai/models.ts:758-764,870-874` |
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
- 值原样保存，不对 `!command`、`$NAME` 求值，不做格式校验；空 API Key 不接受（用“移除”表达删除）。
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
- **添加 API Key 对话框**：供应商 ID 使用 InputGroup，候选菜单为输入框尾部的箭头附加按钮（无障碍名称“选择供应商”）；候选来自 `models.json` 与已存条目，也可手填，无候选或替换固定供应商时隐藏箭头。配合密码型 API Key 输入；确认后立即写入，不进入页面草稿。原值从不进入输入框，已有凭据只显示“已设置”。
- **说明文字**（紧凑的标题/添加/重新读取行下方，独立全宽段落）：“凭据保存在 Pi 的 auth.json 中。更改在新会话中完整生效。Gupi 不会自动重连或同步已有会话的模型列表。”
- 注销确认说明：只删除本机凭据，不会吊销服务端令牌；环境变量或 `models.json` 中的 `apiKey` 仍可能生效。
- 复制命令、检查结果和错误提示均不包含凭据值；无障碍文本同样只出现“已设置”。

每项操作独立立即提交，不与 #68 页面的“保存/放弃”草稿和离开保护耦合。

### 与“模型与供应商”页的关联（用户选定方案 A）

两页保持分开：认证页立即写 `auth.json`，模型页按草稿保存 `models.json`，保存方式不混在同一页。两处都能设置 API Key，因此补充关联与优先级说明：

- **模型页供应商行**：该供应商在 `auth.json` 已存凭据时，摘要中显示“认证页已存凭据”（模型页只拿到元数据，不接触凭据值；这只表示已存在，不代表认证有效）；行尾 `⋯` 增加“在认证页管理”，跳转到认证页。`auth.json` 无法读取时不显示该标记，也不显示为“未存储”，不阻止模型页编辑。
- **模型页供应商对话框的 `apiKey` 字段**：说明一行“认证页存在已存凭据时，Pi 不回退使用此处的 apiKey。已存凭据不代表认证有效。”
- **认证页**：组说明补一句“已存凭据会影响 Pi 对 models.json 中 apiKey 的使用；实际认证由 Pi 按供应商规则处理。”；来自 `models.json` 的行在 `⋯` 中增加“在模型与供应商中编辑”，跳转到模型页。
- 跳转只切换设置页，不打开对话框、不改草稿；离开模型页时沿用现有离开保护。
- 认证页写入成功后，模型页的标记在下次进入或重新读取时更新，不要求实时同步。

## 实施顺序

1. `gupi-resources`：锁提升为共享模块；新增 `pi_auth`（读取、元数据投影、设置/替换/移除、按条目冲突、原地写入）。
2. `gupi-resources` 或 `gupi-pi-runtime`（按现有 Pi 命令启动代码的归属放置）：`auth check` 调用与白名单解析、超时。
3. `src/features/settings`：认证页、列表、添加/替换对话框、确认与检查状态；接入设置搜索关键词。
4. 九种语言文案。
5. 两页关联：模型页凭据标记、互相跳转与优先级说明（见“与‘模型与供应商’页的关联”）。

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

- 经架构师核对 Pi v1.2.0，供应商 ID 无 ASCII 语法限制。存储键不限制为空与否，查找、设置、替换与移除保留完整原始 ID，包括 `""`；手填只要求非空、单行，不允许控制字符及 Unicode 行分隔符，不静默 trim。
- Pi 会在消费 `--provider` 的值之前扫描认证开关。检查入口拒绝 `--credentials`、`--json`、`--no-refresh`、`--min-expiry` 这四个精确 ID，以及空 ID、控制字符、行分隔符或 trim 后改变的 ID，不启动子进程；界面禁用检查并提供本地化原因；这些凭据仍可设置和移除。
- 编辑器区分手填输入与已选候选：固定条目或候选携带原始 ID（包括空 ID）和其准确 revision，空 models 候选持有 missing revision；真实编辑清除候选身份，程序填入不触发 Change。空手填仍不可提交。空 ID 在认证与模型页用斜体本地化占位名称显示、提供相同无障碍名称，不写入输入值或存储键；行、菜单、标记和跳转继续使用原始 ID。
- `/login` 是 Pi TUI 指令，内部空格不做 shell 转义；无法安全完整表示的 ID 复制 `/login`，提示在 Pi 终端选择供应商。

## 实现与验证结果

- `gupi-resources::pi_auth` 提供凭据元数据投影、SHA-256 不透明条目修订、单供应商即时写入和删除；共享目录锁提升至 `pi_lock`。严格 JSON/BOM、私有新建权限、锁内重读、原地写入和同步保持 Pi 的约定。
- 设置新增仅全局的“认证”页，紧随“模型与供应商”：列表与候选、密码输入、替换和移除确认、OAuth 注销与复制登录指令、按需检查、重新读取及本地化错误。九种语言均补齐认证与两页关联文案，插值变量核对一致。
- `cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`、`git diff --check` 通过。`cargo test -p gupi -p gupi-resources --locked` 通过：Gupi 117 项、resources 68 项，失败 0；资源测试覆盖存储、冲突、权限、符号链接、锁、脱敏、协议及危险供应商 ID 不启动进程；生产编辑器交互测试确认空手填供应商及空 API Key 不能提交、输入的 API Key 不进入无障碍 value、替换保留 env。
- 最终测试显式使用临时 Pi 目录；认证页的测试构造器没有默认用户目录，布局测试注入临时路径。首次测试调用遗漏目录环境变量（本地时间 01:49:58 开始，01:50:16 以 130 退出），随后重跑。源码追踪显示当时新增认证页只可能在布局测试中触发 auth/models 读取，未点击认证操作；不能创建 auth 文件或锁、写入/删除凭据或启动认证检查。未取得系统访问轨迹，不能确认该读取是否实际执行。
- 原生验证使用 `script/gupi-ui-gallery --prepare-only --keep` 的临时配置、伪造 auth/models 和假 Pi；`cargo build -p gupi --locked` 通过。早期 CUA/后备捕获失败后，本轮 CUA 已恢复，可正常读取 AX、操作窗口和截图。以下原生结果取代早期未验证结论；未发起 OAuth，也未读取或修改真实 Pi 文件。
- 与架构师确认的 ID 校验/传输澄清见上节。按用户新决定，删除上游需求草稿及其计划条目，不创建或提交上游请求。

### Review 修正

- 认证检查复用 Pi probe 的可执行文件解析，按运行时快照 PATH 查找（包含 Windows PATHEXT），Windows 采用 CREATE_NO_WINDOW；解析失败不启动进程。
- API Key 编辑器持有写入期间的输入与状态，成功后才清空并关闭；失败在对话框内显示并保留密钥，可直接重试。OAuth/未知条目的整条替换确认在同一对话框中完成，避免切换弹层丢失输入。
- OAuth 的本地记录与刷新说明在供应商组中只显示一次。
- 本轮复测：格式、Clippy（Gupi all-targets，deny warnings）及 Gupi/resources 测试通过（117 + 68，失败 0）。新增快照 PATH/缺失命令回归，编辑器回归验证 Busy 后原输入重试、取消确认保留输入与成功后关闭；测试均显式隔离 Pi 目录。Windows 原生运行未验证；macOS 原生结果见下节。

### 两页关联

- 模型页通过 `pi_auth` 元数据生成“认证页已存凭据”标记，读取失败隐藏标记且不妨碍编辑；认证页保留 models.json 来源名单，仅对这些行提供反向入口。两处菜单只发出导航意图，由设置壳沿用离开保护后切页，不自动打开编辑器或改变草稿。
- 模型页标题使用稳定的 keyed entry sentinel，仅以弱句柄延迟请求独立的认证元数据刷新，普通同页重绘不轮询；初始载入与显式重新读取合并在途请求，代际标记防止旧结果覆盖新目标。页面过滤或重挂载可能触发额外元数据读取，不重载文件草稿。
- 已按批准文案补充模型 apiKey 字段说明、认证页组说明和两处导航，九种语言保持键与变量一致。
- 本轮格式检查、Clippy（Gupi all-targets，deny warnings）和隔离测试通过：Gupi 117 项、resources 68 项，失败 0；新增两个生产界面交互回归，覆盖首次进入/同页重绘/侧栏离开再进入/显式重新读取、认证写入后标记刷新、无效认证文件不阻止模型编辑、模型草稿保留，以及双向菜单跳转和取消离开保护。五个关联文案键在九种语言中均已核对，中文采用批准原文。原生关联验证见下节。


### macOS 原生验证

- 一次性 Gallery 使用中文，在浅色与深色下确认 API Key、OAuth、无法识别、未存储四类行，OAuth 记录到期/已过期文字和组说明。截图为本次会话内 CUA 捕获；未导出截图文件。
- 浅色完成新增、API Key 替换（保留 env）、未知/OAuth 整条替换的行内确认、取消确认保留输入、移除和 OAuth 注销。临时占用 `auth.json.lock` 时错误在编辑器内显示，密钥仍以掩码保留；移除锁后无需重输即可成功。深色复核掩码新增、整条替换确认、移除与注销确认。
- 将复制结果粘贴到一次性设置搜索框，分别核对 `/login fixture-oauth` 与不可完整表示 ID 的 `/login`。假 Pi 返回 ready、not_ready、invalid 和仅 stderr 的失败；浅色/深色均按预期显示配置结果，未显示 stdout 的附加凭据或 stderr 原文。AX 在列表、输入和检查结果中不含任何 `fixture-secret-` 值。
- 深色模型页确认凭据标记、apiKey 字段说明及两个方向的菜单跳转；模型草稿离开确认取消后仍保留原草稿，放弃后只切页。认证移除/新增后返回模型页，标记分别消失/恢复；显式重新读取正常。未自动打开编辑器，models.json 字节未变。普通重绘不循环刷新及元数据读取不重置草稿另由生产界面测试覆盖。
- 原生检查发现认证页长说明把添加/重新读取按钮挤到窗口外：为页面容器和说明列增加最小宽度约束，修复后两种主题按钮均在窗口内且可操作，说明正常换行。没有产品行为或数据边界变更。
- 每组操作后按完整 JSON 比较预期文件。浅色结束 auth SHA-256 为 `bad68aa0aecae9b97ed4a1762dbc25a2b4d1c0a5497c44e3b7488b3810865251`；重置假 auth 后独立执行深色验证，结束为 `be5db0abdd67ab3a506959c2864e46ad5411925384cb18b9cdac0bd2efb0f6b1`。models 始终为 `551717f0541bac19e76232c57e4481962b0e26cf985ba6d5216f89316931bdcb`；8 次假检查参数均无 `--credentials`，未残留锁。
- 临时文件快照、检查 argv 和验证清单：`/tmp/gupi-issue48-native-20261019`。关键会话内截图标题包括“验证深色认证列表及记录到期状态”“验证跨页跳转触发未保存草稿确认”“验证深色密钥输入掩码及 AX 不含密钥”“验证反向跳转刷新凭据标记且不打开编辑器”；浅色 Busy/行内确认截图亦保留于会话。
- 布局修复后再次通过 `cargo fmt --all -- --check`、Clippy（Gupi all-targets，deny warnings）、`git diff --check` 和显式临时 Pi 目录下的 Gupi/resources 测试：117 + 68 项，失败 0。


### 首批界面改善（A1–A6）实现与验证结果

- 添加框使用 GPUI Kit InputGroup + InlineEnd 候选箭头；选择填入原输入状态，保留自由输入，固定供应商与无候选时隐藏附加按钮。无障碍名称“选择供应商”和新增保存进度文案在九种语言中对齐。
- 模型行将供应商 ID 放在首行，摘要与已存凭据标记在次级区域换行或截断，省略号保留独立操作列。Models/MCP 共用头部仅补充收缩约束，德语最小窗口的添加按钮不再被说明文字推出窗口；其他 MCP 表单布局未改。
- 认证页采用紧凑标题/添加/重新读取行；原有三段说明分别渲染为全宽段落，批准的中文原文不变。重新读取/保存保留已加载列表，忙碌期间禁止行和菜单操作，页头显示相应进度，只有首次读取没有旧行。
- 按架构师核对的公开 API 方案，为 Add、行及省略号触发器使用稳定标识的非 Tab 停靠焦点锚点；通过公开焦点遍历进入原 Button，再打开对话框。菜单入口等菜单关闭后再打开编辑器；保留原 Button 的角色、按键行为和焦点样式，原触发按钮的弱焦点句柄已过期时，在关闭回调后延迟回退 Add；成功的程序关闭也使用同一回退。正常恢复交给组件；仍有父弹层或新弹层时不抢焦点，目标未渲染/禁用时不跳到无关控件。没有组件依赖变更或行点击行为变更。
- 共享对话框页脚显式区分 Primary/Danger；凭据移除、OAuth 注销、包/资源移除、模板任务删除和临时目录清理使用 Danger，保存/暂存保持 Primary。
- 自动回归覆盖旧行在重新读取/保存期间保留、重新读取时点击禁用操作无效、添加框自由输入、固定供应商无候选箭头、Escape 恢复真实添加按钮焦点、Tab/Shift-Tab 无额外停靠、编辑时原行消失后取消回到 Add、新弹层不被回退抢焦点，以及德语 800×600 模型 ID/标记位置与添加按钮边界。最终 `cargo test -p gupi -p gupi-resources --locked` 通过：117 + 68 项，失败 0；显式隔离 HOME、Pi agent/session 目录。`cargo clippy -p gupi --all-targets --locked -- -D warnings` 和本地构建通过。一次整组导航断言未及时切页，随后单项与最终整组重跑通过；原生取消/放弃及双向跳转均通过，未修改导航实现。
- 原生使用现有 script/gupi-ui-gallery 一次性目录，显式隔离 HOME、Pi/config/data/session 目录，仅假 Pi、假凭据/模型/资源。中文 960×740 浅色与德语 800×600 深色确认 InputGroup、候选填入/隐藏、全宽说明、模型身份/标记/操作列、Models/MCP 添加可操作。中文验证 Escape/取消/关闭返回 Add，行和菜单替换返回原触发器；Tab 到重新读取；模型草稿取消离开保留、放弃后只切页及反向跳转正常。
- 临时锁下中文/德语保存期间旧行仍在、操作禁用、进度正确；Busy 后编辑框保留掩码输入。中文无编辑器的移除尝试也保留列表和页头“正在保存…”；锁占用使写入失败，未实际删除。原生确认凭据移除/注销、包移除、资源移到废纸篓、模板任务删除和清理均为危险按钮，均取消。AX 不含 `fixture-secret-`，假 auth/models 最终逐字节等于初始快照，无遗留锁。
- 本轮快照及校验清单在 `/tmp/gupi-part1-native-20261019`；截图为会话内 CUA 捕获，未导出文件。最后增补的过期触发器回退仅由生产界面自动回归验证；普通焦点路径已有上述原生覆盖，未在工具异常后重新调用 CUA。关键标题：“复查内嵌候选箭头及供应商菜单”“复查从省略号菜单替换后焦点返回原菜单触发器”“确认德语 800×600 模型与 MCP 的添加按钮可操作”“确认最终德语保存进度正确、旧行保留且操作禁用”“复查凭据移除危险确认及无弹层保存时保留列表和页头进度”。
- 工具退出异常：隔离 Gallery 退出后的 CUA 观测可能经 Launch Services 重启了同一临时 bundle（PID 28232），未继承隔离环境。立即停止 CUA；worker 的 SIGTERM 被自动审批拒绝，未执行，也未再发终止命令。协调者按用户批准准备终止时，该进程已退出，确认没有 Gallery/Gupi 进程；退出原因未知。协调者仅以 stat 确认真实 Pi 的 auth/models/mcp/settings 修改时间均早于事件、无锁；worker 未打开真实 Pi 文件。该进程不计入隔离验证证据，修改时间也不能证明应用从未读取真实目录。后续只通过显式隔离环境启动，退出后不再用 CUA 重新绑定或观测 bundle。


### 空供应商 ID 的当前契约与验证

- 存储接受 Pi 的原始供应商键，包括空 ID；只拒绝空 API Key。固定条目与显式选择的候选持有准确身份及 revision，手填空输入仍无身份。新增空 models 候选可设置，既有空条目可替换、整条确认替换和移除；后续真实编辑清除候选选择。共享本地化占位名称用于认证/模型行、标题、确认和候选，空 ID 斜体显示，AX 名称与显示一致，原始键始终保留。
- 检查可用性与存储独立：空、trim 改变、控制字符/行分隔符、四个保留开关 ID 在界面禁用并解释原因，启动入口独立拦截；空 ID 的登录复制结果为 `/login`。模型页空 ID 的已存凭据标记、双向跳转和既有模型编辑均保留原始身份。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`git diff --check` 通过；`cargo test -p gupi -p gupi-resources --locked` 通过：Gupi 120 项、resources 71 项，失败 0。HOME 与 Pi/session 目录均指向 `/tmp/gupi-part1-verification` 内隔离目录。九种语言的新键与修改键各出现一次，键名与插值字段对齐。Cargo 仍提示既有第三方 `block 0.1.6` 的未来兼容警告，不影响上述检查成功。
- 新增存储回归覆盖空 ID 设置/替换/移除、仅空 key 拒绝、OAuth/未知条目整条替换、missing/changed/removed revision 冲突及相邻条目保留。生产 GPUI 交互回归覆盖占位与真实名称的不同 AX/元素身份、不可检查原因、掩码输入、未知条目确认、移除、空手填拒绝、models-only 空候选程序填入保留身份、真实编辑后失效和重新选择，以及精确 `/login` 回退；模型编辑、凭据标记和带未保存草稿保护的双向跳转也使用空 ID 验证。检查进程回归确认空 ID 与四个保留开关等均不启动假 Pi。
- 本轮按架构师澄清不新增原生 CUA 会话；以上新增界面结论来自生产 GPUI 自动交互测试。未打开或修改真实 Pi 文件，没有启动真实 Pi/OAuth。

### 关联导航回归的观察边界

- 一次 Linux 自动测试在取消离开保护后，第二次菜单关联操作没有显示确认；最终快照没有菜单或对话框，模型行仍在。本机原测试经过 200 个 GPUI 调度种子未复现，不能据此认定是产品焦点缺陷，也不能认定只是测试调度问题。
- 临时有序追踪覆盖菜单焦点/高亮、关联项 handler、事件接收、延迟导航、离开保护 busy/dirty 和取消。两次菜单使用不同的新实体与焦点句柄，初始无选中，Down 后目标项选中；两次均依次到达 handler → 接收 → 延迟导航 → `busy=false / dirty=true` → 打开确认，取消仅一次。临时追踪已删除，证据日志在 `/tmp/gupi-menu-ordered-trace.log`；原测试种子日志在 `/tmp/gupi-menu-seeds-before.log`。源码确认该取消路径使用同步 close_dialog，不走 Auth 的焦点锚点或 Root 延迟动画恢复；点击助手本身会绘制，不能将缺少额外绘制视作已证实原因。
- 导航语义测试改为打开真实菜单，核对作用域内项的名称后按 ID 点击，验证取消保留草稿、重新操作后放弃及双向跳转。独立键盘回归保留点击打开 → 菜单就绪/焦点断言 → 单独 Down → 目标高亮断言 → 单独 Enter → 确认断言；取消后也确认弹层已消失、草稿仍脏且非忙碌。使用完成帧和确定的观察，不加 sleep、重试或生产焦点改动。这是测试规格收紧，不是对原始瞬态失败根因的证明。
- 两条路径各 200 个调度种子通过；最终隔离 `cargo test -p gupi -p gupi-resources --locked` 通过（121 + 71，失败 0）。格式、workspace/all-targets/all-features Clippy（locked、deny warnings）、diff 检查通过。HOME/Pi/session 继续指向 `/tmp/gupi-part1-verification`，未运行原生 CUA、未访问真实 Pi。原 Linux 失败根因仍未决，需要后续 Linux 运行的分阶段断言证据。
