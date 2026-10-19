# 设置页布局、无障碍与文案修正

状态：实现与自动检查完成，原生复查待 Computer Use 授权恢复；组件自带的设置搜索占位翻译按用户决定暂缓。

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

`script/gupi-ui-gallery` 把 `HOME` 指向私有临时根目录，使 `~/.agents/skills` 等基于主目录的发现也落在临时数据上（`crates/gupi-resources/src/resources.rs:173`）；同时设置配置、日志、数据、Pi 与会话目录。不改应用的发现规则。

macOS 临时 bundle 使用与真实应用不同的固定标识 `top.sushao.gupi.ui-gallery.isolated` 及原生 bootstrap。该固定标识由用户批准，便于 Computer Use 的一次性授权；每次运行的临时路径与隔离环境仍各自独立。工具通过绝对路径 `/usr/bin/cc` 编译 bootstrap；它检查必需的 fixture 目录，切换到 project-a，以绝对路径和静态环境白名单 `execve` 内部 gupi-bin，并转发参数；缺目录、chdir 或 execve 失败即退出，不合并宿主环境。Info.plist 的 LSEnvironment 仅保留为冗余元数据，不作为隔离保证。直接启动和 Launch Services 重启都须观察最终 GUI 进程的可执行文件、cwd 和覆盖变量，确认后才绑定 CUA。

### 7. 认证页行点击（用户已确认）

点击供应商行直接打开该行最常用的操作，不再弹出与行尾菜单重复的操作对话框：API Key 行打开替换，未存储的行打开设置，OAuth 行打开注销确认，无法识别的行打开替换编辑器，填写掩码新密钥后在保存时沿用既有的整条替换确认。检查、复制登录命令、跳转等其余操作留在行尾菜单；可用操作与文案由同一处定义，行与菜单共用。行的无障碍名称说明该操作。

## 实施顺序

1. 共享的管理区标题组合与对话框/字段尺寸约定；按表逐页修正布局。
2. 无障碍名称与菜单勾号。
3. 模板任务对话框。
4. 资源命名：提示词模板的新建校验（`gupi-resources`），Skill 的非阻断警告，以及按类型的提示文案。
5. 用词与确认文案，九种语言。
6. gallery 隔离 `HOME`。
7. 认证页行点击。

## 验证

- 受影响的界面回归：德语最小宽度下各页主要操作的边界在窗口内、焦点/标签可达；菜单勾号；模板任务对话框无多余焦点；资源命名按类型处理（提示词模板含空白时拒绝新建；Skill 不合推荐规则时只警告、仍可创建；已有资源照常管理）。
- 原生：worker 用隔离的 gallery（临时 `HOME`、Pi、配置目录与假 Pi）复查走查表中每一项，德语 800×600 与 zh-CN 默认尺寸各一次、各取一种主题；检查无障碍名称。
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
- `cargo test --workspace --locked`：439 通过、0 失败、2 个既有忽略项；其中 Gupi 124、gupi-resources 72、gupi-settings 21 通过，包含九语言一致性检查。
- `cargo build -p gupi --locked`：通过。
- 原生 bootstrap 使用绝对路径 C 编译器的生成/编译检查通过；实际 `open -n` 启动及重启验证见下。

Cargo 仍报告既有第三方 `block 0.1.6` future-incompatibility 提示，不影响上述检查退出状态。日志在 `/tmp/gupi-polish-final-{clippy,workspace}.log`。

新增回归覆盖 Skill 完整说明、正文读取失败时说明保留、实际键盘翻页与 Tab 离开、操作区位于滚动区上方；语言原始值与语言切换后的控件名称；录制控件包含命令、绑定和状态；资源命名对新建/已有文件的不同规则。认证现有回归已按直接行操作更新。

原生验证限制：此前走查是问题来源，不能作为此次修正的验收。此次 protected Gallery 通过 `open -n` 启动和退出后的再次启动，均验证最终 gupi-bin 的可执行文件、project-a cwd、临时 HOME、GUPI_CONFIG_DIR / GUPI_LOG_DIR / GUPI_DATA_DIR、PI_CODING_AGENT_DIR / PI_CODING_AGENT_SESSION_DIR。随机标识阶段 CUA 返回 `Computer Use was not approved to use Gupi UI Gallery`，未尝试改名或换标识绕过拒绝；修正后的 zh-CN 默认尺寸、德语 800×600、主题及原生 AX 逐项复查尚未执行，暂无此次有效 UI 截图。启动证据与 fixture 位于 `/tmp/gupi-settings-polish-20261019`（verified-launches.jsonl）。

### 隔离事件与修复

验证中曾出现临时 Gallery 进程已退出后，CUA 按 bundle 路径重新启动时未继承环境；发现真实配置路径后立即停止 UI 操作并终止该精确 fixture 进程，未用工具打开真实 Pi / .agents 文件。协调者仅用 stat 检查：真实 Pi auth 的 13:35:36 时间与用户 Pi 进程启动一致，models/mcp/settings 更早；真实 Gupi 配置仅 instance.json 在 15:16:32 被此次未隔离重启写入，其他条目未变。这些时间信息不证明运行时没有读取。

后续 `open -n` 证明仅写 LSEnvironment 仍会忽略隔离变量，已在 CUA 前发现并终止精确进程；shell bootstrap 被 Launch Services 以 -54 拒绝（仅记录该次观察，不推断普遍限制）。按架构师确认改为上述原生 bootstrap 后，实际启动与重启的最终 GUI 环境/cwd 均通过验证；后续仍按唯一 bundle 路径绑定并复核，不凭源码宣称覆盖所有 CUA 重启路径。

### 固定 Gallery 标识后的复核

按用户批准，将每次随机的 CFBundleIdentifier 改为固定的 `top.sushao.gupi.ui-gallery.isolated`，应用名仍为 `Gupi UI Gallery`，保留原生 execve bootstrap 和冗余 LSEnvironment。停止旧 fixture PID 98433 后，以临时 HOME/Pi 环境重新执行 `cargo build -p gupi --locked`，通过；更新 bundle 并通过 `open -n` 启动最终 GUI PID 2757。绑定 CUA 前已用 ps 与 cwd 检查确认最终可执行文件为该临时 bundle 的 gupi-bin、工作目录为 project-a，且 HOME 与全部 GUPI/Pi/会话覆盖变量匹配 fixture。实际绑定仍返回 `Computer Use was not approved to use Gupi UI Gallery`；已停止原生尝试，未换别名重试，待用户在 Codex 中授权上述固定应用后继续。此轮没有新增 UI 验收覆盖或截图。
