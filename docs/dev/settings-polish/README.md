# 设置页布局、无障碍与文案修正

状态：步骤 1–8 已实现并完成要求的验证；组件自带的设置搜索占位翻译按用户决定暂缓。

来源：#75 合并前对全部设置页的原生走查（zh-CN 960×740、德语 800×600，浅色/深色，全局/项目作用域）和代码审查。认证页与模型页的同类问题已在 #75 修复，本文只覆盖其余现有页面。依据为 [GPUI Kit Design Guides](../../../.agents/skills/gpui-kit-design-guides/references/design-guides.md) 的 Forms and settings、Responsive desktop windows、Spatial grammar、Components and composition、Interface language 与 Accessibility checklist。Pi 规则依据 Pi v1.2.0（本地 `/Users/sushao/Documents/code/pi`，路径相对 `packages/coding-agent/src/`）。

## 目标与范围

在受支持的最小窗口 800×600 和较长语言（德语）下，所有设置页的主要操作都可见、可点击；控件有正确的无障碍名称；同一概念在各页、各语言使用同一词汇；资源命名提示与校验和 Pi 的实际规则一致。

**不在本期**：模型页与 MCP 页编辑对话框的重新设计（用户决定以后单独讨论）；新增设置项或改变保存方式；依赖升级。

## 设计

### 1. 最小窗口下的布局（P1/P2）

共同原因：标题栏、标签与操作区假定文字很短，弹性子项缺少可收缩约束，右侧宽度写死。修正原则：

- 文字列可收缩、可换行；操作区保持稳定宽度，不被挤出窗口。
- 主要操作（标签页、作用域、添加、确认）在最小宽度下始终可见；只有次要操作在空间不足时移入标准的溢出菜单。
- 可复用的“管理区标题”组合（标题 + 说明 + 操作区）由 #75 的认证/模型页标题抽出共享，不做通用设置框架。

涉及：

| 页面 | 现象 | 修正 |
| --- | --- | --- |
| 项目作用域的包、提示词、模型、工具页 | 标题、作用域、信任控件把“浏览”标签、说明、重新读取挤出或截断 | 标题/作用域/信任与标签或工具栏分为两行；主要标签和作用域始终可达 |
| 快捷键录制 | 录制状态下取消/确认超出右边缘，确认不可点 | 录制框宽度受限；窄宽度下操作放到录制框下方，保留默认确认与取消 |
| 网络页 | 重试次数、延迟、超时的标签压住数值 | 标签可换行并有固定控件区；窄宽度下改为上下排列 |
| 通用页、关于页 | 语言选择、配置文件操作、诊断操作超出右边缘 | 字段宽度随可用宽度收缩；操作组在最小宽度换行 |
| 外观页图标主题 | 德语名称溢出到相邻卡片，选中勾号挤压文字 | 名称最多两行并截断，完整名称通过 tooltip 显示且无障碍文本保持完整；卡片保留选中标记位置，卡片尺寸一致 |
| Skill 页 | 长描述单张卡片超过一屏，“查看”“更多”被推出屏幕 | 描述预览最多 3 行，全文通过已有的“查看”；操作区位置固定 |
| 包页 | 展开后无内容只剩空白；长来源路径遮住包名 | 无内容时显示一行空状态；先显示可识别的包名，完整来源作为次要信息 |

共享的管理区标题只是设置页内的展示组合（标题、说明、操作区插槽与收缩/换行几何），各页保留自己的操作、状态、焦点标识和作用域/信任控件；不新增控制器，也不强制迁移无关的标题。

文字宽度、对话框与字段尺寸改用 rem 或已有的主题/尺寸 token，随基础字号缩放，不为替换零散字面量另建全局尺寸抽象；预览图像素和测得的视口值不改（`layout.rs`、`global_keys/editor.rs`、`resources/editor.rs`、`keys.rs` 中的像素字面量）。

### 2. 控件语义与无障碍（P2）

- 资源页标题栏的“信息”“重新读取”等图标按钮提供本地化无障碍名称，不只依赖 tooltip。
- 语言选择器及其选项有名称。
- 快捷键录制控件暴露命令名、当前快捷键与录制状态，不是匿名的可聚焦容器。
- 模型页、MCP 页的布尔/枚举菜单和 Codemode 菜单用勾号标出当前值（含“未写入”和保留的未知值），与 Pi 设置页一致；只共享选中状态的呈现，不改各文件的保存语义。

### 3. 模板任务对话框（P2）

- 模型与思考等级只保留紧凑的选择器，移除不可编辑的消息输入框，使其不再获得焦点或被读作“可写消息”。
- Pi 启动或模型列表加载失败时，显示本地化说明和就近的“重试”，保留已填内容；与会话中模型选择器的失败状态一致。

### 4. 资源命名（P2）

| 类型 | Pi 规则 | Gupi 新建时 |
| --- | --- | --- |
| Skill | 非空的 frontmatter `name` 优先，否则取目录名；推荐规则为小写字母、数字、单个连字符，不以连字符开头或结尾，最长 64 字符；不合规时 Pi 仍加载并给出警告（`core/skills.ts:11,92-111,320-331`） | 保留文件名安全校验（与[职责边界](../../gui-boundary.md)一致，允许中文等安全文件名）；名称不符合 Pi 推荐规则时显示非阻断的本地化警告，不阻止创建或管理。该检查针对新建时填写的目录名，不保证之后编辑的 `SKILL.md` frontmatter 名称 |
| 提示词模板 | 名称为去掉 `.md` 的文件名；`/名称` 匹配 `^/([^\s]+)`，含空白的名称无法调用（`core/prompt-templates.ts:129,304-313`） | 合法文件名且不含空白（新建时拒绝，因为含空白的模板无法调用）；提示写明“用于 /名称 调用”。已有模板的读取、编辑不受此限制 |

校验与提示按类型分开，取代现在共用的、与实际校验不符的“英文字母、数字、连字符或下划线”。存储层的通用文件名检查保留。

### 5. 用词与确认（P3）

- 同一动作同一词：重新读取资源、配置或选项数据的“从磁盘重读 / 重新读取 / 刷新”统一为“重新读取”；OAuth 令牌刷新、刷新已有会话等不同操作保留原词。“思考程度 / 思考等级”统一为“思考等级”，包括会话输入框与模板任务对话框共用的 `composer-model-thinking`，不限于 `settings-` 前缀的文案；Skill 在侧栏和工具页同名；其他语言按各自的统一术语改，不逐字对应中文。
- 会打开对话框的命令带省略号（如“添加模板任务…”“编辑…”），对话框标题不带；新建模板任务的对话框标题不再写“编辑”。
- 确认对话框标题写明动作与对象：如“移除 review-package？”“清理已释放的工作目录？”“删除模板任务“名称”？”；正文只补充后果，不重复提问。
- 应用自有文案中的英文占位补齐翻译。组件自带的设置搜索占位按下述限制暂缓。

### 6. 测试工具隔离

`cargo run -p xtask --locked -- gallery ui` 把 `HOME` 指向私有临时根目录，使 `~/.agents/skills` 等基于主目录的发现也落在临时数据上（`crates/gupi-resources/src/resources.rs:173`）；同时设置配置、日志、数据、Pi 与会话目录。不改应用的发现规则。

macOS 临时 bundle 恢复原先成功连接的固定标识 `top.sushao.gupi.ui-gallery`，保留原生 bootstrap。标识与真实应用不同；每次运行的临时路径与隔离环境仍各自独立。工具通过绝对路径 `/usr/bin/cc` 编译 bootstrap；它检查必需的 fixture 目录，切换到 project-a，以绝对路径和静态环境白名单 `execve` 内部 gupi-bin，并转发参数；缺目录、chdir 或 execve 失败即退出，不合并宿主环境。Info.plist 的 LSEnvironment 仅保留为冗余元数据，不作为隔离保证。通过 bundle 入口启动和 Launch Services 重启均须观察最终 GUI 进程的可执行文件、cwd 和覆盖变量，确认后才绑定 CUA。用户明确将手动执行内部 gupi-bin 排除在隔离范围外；不修改应用源码，不声称内部二进制具有独立隔离守卫。

### 7. 认证页行点击（用户已确认）

点击供应商行直接打开该行最常用的操作，不再弹出与行尾菜单重复的操作对话框：API Key 行打开替换，未存储的行打开设置，OAuth 行打开注销确认，无法识别的行打开替换编辑器，填写掩码新密钥后在保存时沿用既有的整条替换确认。检查、复制登录命令、跳转等其余操作留在行尾菜单；可用操作与文案由同一处定义，行与菜单共用。行的无障碍名称说明该操作。

### 8. Gallery 迁入 xtask（用户已确认，本 PR）

起因：#76 的 Codex 审查指出 UI gallery 的退出与 `--cleanup` 依赖未声明的 `lsof`。用户决定不在 `flake.nix` 补依赖，而把两个 gallery 迁入 xtask，在本 PR 完成。

目标：`script/gupi-ui-gallery`（Python）和 `script/gupi-runtime-gallery`（Node）改为 xtask 子命令并删除原脚本。Python、Node 不再作为 gallery 启动器的解释器；Pi 仍需其自身运行时，Pi wrapper 仍由 `/bin/sh` 执行，macOS UI bootstrap 仍需 `/usr/bin/cc`。

gallery 是开发测试工具，清理保持简单（用户决定）：不做全系统进程扫描或权限相关的完整性判定。

- 隔离：临时根目录 0700；`HOME`、`GUPI_CONFIG_DIR`、`GUPI_LOG_DIR`、`GUPI_DATA_DIR`、`PI_CODING_AGENT_DIR`、`PI_CODING_AGENT_SESSION_DIR` 指向临时目录；UI gallery 使用固定环境白名单；不读写真实 `~/.pi`、`~/.agents` 或 Gupi 配置；`environment.json` 不写入继承的敏感变量。Runtime gallery 同样施加这些覆盖（原脚本未隔离 `HOME`、`GUPI_DATA_DIR`），参数、案例、faux provider 和退出后保留目录不变。
- macOS UI gallery：固定标识 `top.sushao.gupi.ui-gallery`；bundle 可执行入口仍是只用白名单环境 `execve` 内部 gupi-bin 的原生 bootstrap（Launch Services 会忽略 LSEnvironment）。手动运行内部二进制不在隔离范围内。
- 退出清理：UI gallery 由 xtask 等待自己启动的进程结束后删除本次临时目录，`--keep` 时保留；runtime gallery 退出后始终保留目录。
- 显式清理：`--cleanup ROOT_NAME...` 只处理临时目录下当前用户所有、具有 fixture 结构的直属目录，`--exclude` 优先；若有正在运行的进程，其可执行文件位于该目录内（被重新启动的 gallery 本体），则保留并提示，否则删除。删除不跟随 symlink，不终止进程。仍有其他 Gallery bundle 时拒绝启动的规则保留。
- Windows 不在范围内：明确报错退出。

实现复用 xtask 已有依赖；新依赖写完整版本号。同步更新开发索引中的入口和前置说明、本文第 6 节与“当前 Gallery 生命周期修正”中的 Python 实现细节，以及当前使用说明中的旧脚本路径；历史验证记录不改。

## 实施顺序

1. 共享的管理区标题组合与对话框/字段尺寸约定；按表逐页修正布局。
2. 无障碍名称与菜单勾号。
3. 模板任务对话框。
4. 资源命名：提示词模板的新建校验（`gupi-resources`），Skill 的非阻断警告，以及按类型的提示文案。
5. 用词与确认文案，九种语言。
6. gallery 隔离 `HOME`。
7. 认证页行点击。
8. Gallery 迁入 xtask，删除原脚本。

## 验证

- 受影响的界面回归：德语最小宽度下各页主要操作的边界在窗口内、焦点/标签可达；菜单勾号；模板任务对话框无多余焦点；资源命名按类型处理（提示词模板含空白时拒绝新建；Skill 不合推荐规则时只警告、仍可创建；已有资源照常管理）。
- 原生：worker 用隔离的 gallery（临时 `HOME`、Pi、配置目录与假 Pi）复查走查表中每一项，德语 800×600 与 zh-CN 默认尺寸各一次、各取一种主题；检查无障碍名称。
- Gallery 迁移：xtask 的 clippy；macOS 上直接启动与 bundle `open -n` 重新打开均在绑定 CUA 前确认最终 GUI 进程的可执行文件、cwd 与覆盖变量；正常退出后目录被删除、`--keep` 后 `--cleanup` 删除、运行中的 gallery 目录被保留；`gallery runtime --prepare-only` 与一次启动；在本机 OrbStack NixOS 虚拟机的 `nix develop` 中用一次性 fixture 跑一次 `--cleanup`。
- 检查命令：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test --workspace --locked`（隔离 `HOME` 与 Pi 目录）。

## 已确认的实现澄清

- 语言选择器改用锁定版本已有的标准 Select：单一保留状态和原有偏好所有者不变，设置与引导共用；保留可搜索、本地语言选项名称和语言切换后重新投影当前值。该组件提供公开的 accessibility_label，避免用无名 Combobox 的子标签冒充控件名称。
- 用户决定仅暂缓组件自带的 Settings 搜索占位翻译。gpui-kit / gpui-component 0.7.3 的 Settings 从内置目录读取搜索占位（仅 en/zh/it），没有公开覆盖入口，其他语言继续显示英文回退；保留搜索行为，不改依赖、目录或私有状态，不伪造 locale。待组件提供公开覆盖入口并升级依赖时再处理；该输入初始化时写入占位，届时也应验证运行中的语言切换。
- Skill 的“查看”原本剥离完整 frontmatter，不能恢复被摘要截断的 description；现将完整的非空元数据说明作为独立换行文本放在正文之前，两者与加载/错误状态共用有界纵向滚动区。操作区在上方，不随预览滚动；读正文失败时仍保留完整说明。使用同一公开 ScrollHandle 和保留焦点，区域内 PageUp/PageDown/Home/End 翻页，Tab 正常离开，不改变 Markdown 或文件内容。

## 实现与验证结果

已实现步骤 1–7：设置内共享管理标题、窄布局与尺寸修正；语言 Select、录制控件及图标的无障碍名称、当前菜单值勾号；模板任务紧凑选择器和保留输入的本地化重试；按资源类型的新建规则；九语言文案一致性；认证行与菜单共用主要动作；Gallery 启动隔离。

自动检查使用临时 HOME、PI_CODING_AGENT_DIR 与 PI_CODING_AGENT_SESSION_DIR；CARGO_HOME / RUSTUP_HOME 仅指向现有编译工具链缓存。

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --workspace --locked`：440 通过、0 失败、2 个既有忽略项；其中 Gupi 125、gupi-resources 72、gupi-settings 21 通过，包含九语言一致性检查。
- `cargo build -p gupi --locked`：通过。
- 原生 bootstrap 使用绝对路径 C 编译器的生成/编译检查通过；实际 `open -n` 启动及重启验证见下。

Cargo 仍报告既有第三方 `block 0.1.6` future-incompatibility 提示，不影响上述检查退出状态。日志在 `/tmp/gupi-polish-final-{clippy,workspace}.log`。

新增回归覆盖 Skill 完整说明、正文读取失败时说明保留、实际键盘翻页与 Tab 离开、操作区位于滚动区上方；语言原始值与语言切换后的控件名称；录制控件包含命令、绑定和状态；资源命名对新建/已有文件的不同规则。认证现有回归已按直接行操作更新。补充生产视图回归：德语 800 宽度的 Codemode 操作在界内；未知值项不可操作，明确选择未写入/开/关只改暂存状态，不写文件。

原生验证：使用新建的私有 fixture，zh-CN 960×740 浅色和德语 800×600 深色各复查一次。证据为会话内联截图；未另造截图导出路径。启动验证、清理日志、fixture/hash 记录保存在 `/tmp/gupi-settings-polish-20261019`。

| 原生覆盖 | 实际结果 |
| --- | --- |
| 通用、关于、外观 | 语言字段和文件/诊断操作可达；等宽图标卡片与完整 AX 名称；清理确认含对象。语言 Select 有名称和当前值，选项通过可达的非焦点静态文字暴露本地语言名称；切换语言后键盘搜索、选择和选中状态正常 |
| 快捷键、模板任务 | 录制取消/确认在窄窗口内；AX 含命令、绑定和录制状态；紧凑模型/思考等级选择；假 Pi 失败时德语/中文重试就近显示且保留名称/模板，重试后可保存；删除任务确认包含对象 |
| 网络 | 重试次数、初始/最大延迟与高级超时标签和值可读 |
| 项目包、提示词、工具、MCP、模型 | 仅选择 fixture project-a，保持未信任；作用域/信任带与操作栏分行，添加、浏览/已安装、重新读取可达；项目模型显示既有全局专属说明 |
| 模型/MCP 菜单 | 模型布尔值、MCP 未知枚举与工具 Codemode 当前项勾号；最终构建另验证自动 Codemode 的未知值、暂存未写入/开/关各自勾号和键盘选择，放弃后文件仍为未知值，不自动改写 |
| 包 | 长来源不遮住可识别包名；展开空包显示空状态；移除确认含对象 |
| Skill | 摘要三行，查看暴露完整说明与有界正文，操作在上方；聚焦后 End 改变实际滚动位置，Tab 可离开；不合推荐但安全的 Native_Skill 显示警告且成功创建 |
| 提示词 | 含空白新名称在两种语言下拒绝创建、输入保留；已有 old name 仍可编辑，未改写原文件 |
| 认证 | API Key、Missing、OAuth、Unknown 行分别直接打开替换、设置、注销确认、替换编辑器；Unknown 保存仍有整条替换确认；掩码输入及 AX 不含假 secret 值，取消未改 auth.json |

fixture 结束核对（native-final-hashes.json）：auth.json、models.json、已有提示词、项目配置及隔离 .agents 文件哈希与初始记录相同；mcp.json 的差异仅来自主动注入的未知枚举/工具 Codemode fixture。Native_Skill/SKILL.md 已创建；native prompt.md 与 german prompt.md 均不存在。配置中的语言和 Native task 为本次有意操作结果。

原生发现并修正：语言选项原先没有可达 AX 名称；各语言目录中的语言选项统一为 autonym。autoEnableCodemode 原先在窄宽度溢出且未显示选中项，改为标签/状态在上、选择和更多操作在下；选择依据有效暂存值，未知值以禁用的当前项保留。此修正经架构师确认，不改变保存语义。

### 隔离事件与修复

验证中曾出现临时 Gallery 进程已退出后，CUA 按 bundle 路径重新启动时未继承环境；发现真实配置路径后立即停止 UI 操作并终止该精确 fixture 进程，未用工具打开真实 Pi / .agents 文件。协调者仅用 stat 检查：真实 Pi auth 的 13:35:36 时间与用户 Pi 进程启动一致，models/mcp/settings 更早；真实 Gupi 配置仅 instance.json 在 15:16:32 被此次未隔离重启写入，其他条目未变。这些时间信息不证明运行时没有读取。

后续 `open -n` 证明仅写 LSEnvironment 仍会忽略隔离变量，已在 CUA 前发现并终止精确进程；shell bootstrap 被 Launch Services 以 -54 拒绝（仅记录该次观察，不推断普遍限制）。按架构师确认改为上述原生 bootstrap 后，实际启动与重启的最终 GUI 环境/cwd 均通过验证；后续仍按唯一 bundle 路径绑定并复核，不凭源码宣称覆盖所有 CUA 重启路径。

### 固定 Gallery 标识后的复核

固定 `.isolated` 标识尝试仍遇到 CUA 未授权；恢复原始固定 `top.sushao.gupi.ui-gallery` 并清理已批准的旧 bundle、创建新路径后，按精确 bundle 路径绑定成功。不能把先前失败仅归因于随机标识。最终 GUI 的可执行文件、project-a cwd 和六个 HOME/GUPI/Pi/会话覆盖变量均在 CUA 绑定前验证；证据为 verified-launches.jsonl。

### 旧启动器的目录清理

已实际删除用户逐项批准的 14 个闲置根目录，明确排除 0w4b4sua；备份假 fixture 后停止自己的 PID 2757，再单独删除该根目录。每次删除前复核无可见存活引用，未停止其他进程。日志为 cleanup-approved.log、cleanup-own.log。当时的新 fixture 通过 --keep 保留以供复查；没有把未列明根目录纳入删除授权。该历史清理记录由旧 Python 启动器完成，当前清理规则见第 8 节。

### Gallery 迁入 xtask 的实现与验证

步骤 8 实现为 `gallery ui` / `gallery runtime`；原有选项和离线案例保留，删除两份解释器启动脚本。共享临时目录、配置与 Pi wrapper；新增直接依赖仅为已锁定版本的 libc 0.2.190（Unix 所有者判断）与 sysinfo 0.31.4（只请求用户/可执行文件字段）。按用户更新后的开发工具规则，没有三态完整性判定、cwd/argv 扫描或原生身份适配层。UI 在自己启动的子进程结束后清理（Linux 也将 UI 可执行文件复制到 fixture，便于同一可执行路径检查识别运行实例），Runtime 始终保留；显式清理保留仍在该根目录内执行的 Gallery，不终止进程。

- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过；`cargo test --workspace --locked`：440 通过、0 失败、2 个既有忽略项，24 组；其中 xtask 12 通过。均使用临时 HOME/Pi/session 目录。
- macOS 最终版本直接启动成功，绑定 CUA 前核对 gupi-bin、project-a cwd 和六个 HOME/GUPI/Pi/会话覆盖变量；正常 Cmd+Q 后本次目录实际删除。
- `--keep` 退出后目录保留；同一 bundle 经 `open -n` 重启，重新核对最终进程环境/cwd 后成功绑定 CUA。运行中 `--cleanup` 保留根目录；关闭自己的重启实例后显式清理实际删除。另验证 `--exclude` 优先保留，随后不排除时删除。
- Runtime `--prepare-only --changes` 核对配置、原有 faux-provider wrapper 参数与八个显式环境覆盖，environment.json 无继承变量；默认 runtime 案例实际启动一次，核对其 xtask 父进程、项目 cwd 和全部覆盖后终止自己的 fixture 进程。目录按约定保留，未运行完整慢速模型回合。
- macOS 使用假 Pi 和一次性配置，不读取真实 Pi、.agents 或 Gupi 文件；生产清理不调用 ps/lsof。验证脚本仅为核对目标 fixture 的环境/cwd 使用这两个本机工具，不构成 Gallery 的运行依赖。证据与日志位于 `/tmp/gupi-xtask-gallery-verification`，包括 launches.jsonl、ui-auto-final.log、ui-keep-final.log、ui-running-final.log、ui-cleanup-final.log、runtime-prepared.log 与 runtime-verified.json；有效 CUA 连接证据在会话内。最终清理也删除了本 worker 旧的 _1x8zzv4 和本轮预备 YjhzDd 两个自有 fixture（old-own-fixtures-cleanup.log），未删除其他人的目录；本轮 UI bundle 均已退出并清理，Runtime 的目录按约定保留。
- Linux：OrbStack NixOS VM 的 `/tmp/gupi-xtask-migration-44cQhF/repo` 源码副本，在原有 flake 的 `nix develop` / Rust 1.99.0 环境中构建通过，xtask 10 项现有测试通过。正式 `gallery ui --cleanup` 删除 VM 本地一次性 fixture，--exclude 和未列出的根目录均保留；日志为该 VM 临时根下的 final-build.log、cleanup-probe.log。所有 HOME/Pi/session 与 probe 都在 VM 本地 `/tmp`，未读取挂载的 macOS 主目录数据，未改变 VM 系统配置。
