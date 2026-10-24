# 独立 Shell 输入框

状态：双输入设计已确认，尚未实现。采用 Kit Editor + 底栏；Pi 的执行语义与适合沿用的处理方案见下表，GUI 呈现不受 TUI 单缓冲形式限制。基于 `feat/82-shell-input` 的 `b7e113f`、锁定的 GPUI Kit 0.8.0。本文负责输入侧；执行、实时输出和历史继续遵循 [Shell 命令交互](../bang-commands/README.md)。

## 结果

当前会话有两个独立输入：普通 Composer 保留原草稿，Shell 使用 Kit Editor 编辑 Bash 命令。界面在同一输入位置切换，Shell 标识、高亮与底栏明确表示模式；`!` / `!!` 只是入口及上下文选择，不显示在命令正文中。普通正文、文件/技能 token、图片、引用附件及选区不被入口吞掉，也不随命令发送或清空。命令接受执行后清空 Shell 正文并留在 Shell，返回消息时恢复原 Composer。

Shell 正文使用 Kit `Editor`，底栏由公开的 `Toolbar`、`ToggleGroup` 和 `Button` 直接组合。Pi 负责执行、扩展处理、上下文和历史；Gupi 只把正文送入现有 `bash` RPC。Shell 不是终端模拟器，不新增交互式终端、Shell 历史搜索、路径补全、LSP 或命令解释器。

[Issue #82](https://github.com/suxiaoshao/gupi/issues/82) 正文中的 InputGroup 外框和 2–8 行自动增高已不符合当前 Kit 能力，不能照旧承诺。[Kit 维护者对 #3657 的结论](https://github.com/longbridge/gpui-kit/pull/3657#issuecomment-6379788798) 是由调用方直接组合 Editor 与工具栏，不增加 InputGroup 变体。本文不复制该 PR，不改依赖，也不自绘仿造 InputGroup。

## 所有权

| 对象 | 所有者与边界 |
| --- | --- |
| 普通消息与附件 | 仍由 `ConversationState::Session` 保存 `InputContent`、图片、引用；`SessionView.input` 保留 Textarea、选区与撤销历史。Shell 入口不改写这份草稿，不把原正文或 token 复制、序列化成命令。 |
| Shell 输入 | 当前窗口、当前会话的 `SessionView` 持有独立 `EditorState`、模式和 `excludeFromContext`，后者是上下文选择的权威值。提交后清空正文但留在 Shell；显式退出时清除未运行命令。Kit 管理各输入的选择、撤销、IME 与滚动，不合并两套撤销栈，不增加 Shell `DraftFile` 字段。 |
| Shell 输入历史 | 同一 SessionView 的内存命令列表保存正文与上下文选项，最多 100 条、连续相同项去重；只用于显式召回，不是执行事实或持久会话历史。 |
| 运行中的命令 | 继续归 `Session.shell` 和连接 binding 所有；复用 `can_run_shell`、`run_shell`、`stop_shell`，不在编辑器再存一份运行事实。 |
| 焦点与路由 | Home 负责显示哪个输入面板、切换模式和动作路由；Kit 负责正文编辑、IME、选择、复制、撤销、搜索和滚动。 |

现有接点是 [home.rs](../../../crates/gupi-conversation-ui/src/home.rs) 的 `SessionView` 与输入订阅、[home/composer.rs](../../../crates/gupi-conversation-ui/src/home/composer.rs) 的输入区域、[composer.rs](../../../crates/gupi-conversation-ui/src/composer.rs) 的普通 InputGroup。Shell 面板在同一能力内独立组合，不把普通 `Composer` 泛化成任意编辑器框架。

切换会话不停止原会话命令，也不把输入带到新会话。临时窗口复用相同面板及既有隐藏/释放生命周期，新临时会话从空输入开始。扩展提出输入请求时，既有扩展交互优先；不能用 Shell 输入替代扩展回答。

## 关键决定

### Kit Editor + 底栏

上方使用 Kit `Editor` 默认外观，下方直接组合 `Toolbar`、`ToggleGroup` 和 `Button`。外层只安排布局，不仿造 InputGroup 外框，不要求与普通 Composer 的轮廓、焦点环或高度一致。普通输入框未来如何实现不影响本项，也不需要先做自写编辑组件的选型。

Editor 使用 Bash 高亮、等宽字体、软换行，隐藏行号、折叠与缩进线。采用 `8rem` 固定编辑区域和 Kit 内部滚动；这是本项的初始布局值，不是“2–8 行自动增高”的模拟。Kit 0.8.0 没有 Editor 自动高度，按公开能力使用，不读取私有 display map 或按行高补算。

底栏显示 `SquareTerminal`、“Shell”、真实会话 cwd、`! 加入上下文 / !! 不加入` 单选切换和 primary “运行”。目录过长时截断并提供完整 tooltip；未就绪时显示连接状态并禁用运行。运行中显示“命令运行中，可在对话中停止”，继续复用现有输出行的停止入口。提供键盘可达的 ghost “返回消息”；其空闲行为与 Esc 相同，清除未运行的 Shell 输入后返回，不另存一个可恢复的命令草稿。显式“返回消息”只退出输入面板，不停止正在运行的任务；任务停止由 Esc 的优先级或既有停止按钮处理。

底栏用组件自身的主题、尺寸与焦点行为；窄窗先压缩目录，必要时按控件组换行，保持运行与返回可达。不手量控件宽度，不为统一外观补画边框。Shell 模式隐藏普通附件、模型选择、token 统计和发送下拉，隐藏不代表发送或删除附件。

### Kit 0.8.0 能力边界

源码链接固定到 v0.8.0；不是对未合入 main 的承诺。

| 需求 | 公开能力与处理 |
| --- | --- |
| Bash、等宽、软换行 | `EditorState::language("bash")`、`soft_wrap(true)`；沿用现有应用高亮初始化。Editor 自带主题代码字体、字号和相对行高，不复制其私有行高常量。[Editor render](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/component/src/input/editor.rs#L166)、[EditorState](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/base/state.rs#L11962)。 |
| 隐藏行号、折叠、缩进线 | 公开 `set_line_number(false)`、`set_folding(false)`、`set_indent_guides(false)`；使用运行时 setter，不依赖标为 doc-hidden 的构造器。缩进线实现见 [indent.rs](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/editor/indent.rs#L181)。其余编辑行为保留 Kit 默认，不照搬 Zed 整套编辑器配置。 |
| 占位符与提交 | `placeholder` 显示“输入要运行的命令”，`submit_on_enter(true)`、`InputEvent::PressEnter` 处理提交；Shift+Enter 仍由 Kit 插入换行。[提交实现](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/base/state.rs#L2167)。 |
| 高度 | Editor 只有公开 `.h(...)` 与 `Styled` 布局；`auto_grow` / `set_auto_grow` 属于 TextareaMode，不属于 EditorMode。[Textarea 与 Editor 分界](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/base/state.rs#L11900)。采用固定 `8rem` 编辑区域、内部滚动；这不是精确 2–8 行，也不按 display map 算高度。 |
| 选择、撤销与 IME | Kit 原生 Editor 行为；公开 `selected_range` / `set_selected_range`、`EntityInputHandler::marked_text_range` 可用于模式入口判断。组合中的 marked text 不切面板。入口只消费当前用户的前缀输入；程序化恢复、撤销或普通草稿本来含有 `!`，不作为新的切换请求。[输入处理](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/base/state.rs#L4303)。 |
| 粘贴 | Textarea 和 Editor 都有公开 `on_paste`；普通输入符合入口条件时消费本次前缀粘贴，避免先写入普通草稿再整份重建。其余粘贴交给所在输入组件。Shell 不附加图片，不在本项移植 Pi 的剪贴板图片临时文件和路径转义。[Editor on_paste](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/component/src/input/editor.rs#L124)。 |
| 搜索与 Esc | Kit 提供 `search_session()`、`close_search()`；Kit 的 Escape 处理菜单、额外光标、补全及 IME，但 `unmark_text` 后仍可 propagate。应用不能把同一次取消 IME 的 Esc 又解释为停任务/退出。[Escape](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/base/state.rs#L2288)、[搜索](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/input/editor/search.rs#L136)。 |
| 底栏 | `Toolbar` / `ToolbarGroup` 提供语义和键盘导航，`ToggleGroup` / `Button` 提供控件行为；不使用 InputGroupAddon 包裹一个假的 Input。[Toolbar](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/component/src/toolbar.rs#L136)。 |

Zed 是编辑体验上限，不是另一套实现依赖。本机 Zed `79fe6a645f` 的 `crates/editor/src/editor.rs:485–500` 有 `AutoHeight` 与 `Full` 模式；Kit 没有对应 Editor 自动高度，就不复制 Zed 的测量和滚动实现。本项采用 Kit Editor 固定高度，不再把是否对齐 Composer 或改用 Textarea 列为待选项。

### 参照 Pi：语义、处理方案与 TUI 形式

依据本地 Pi **`36be9c612`** 的源码，不是原生运行测量。下表区分 Pi 的执行语义、值得沿用的处理方案、TUI 当前呈现方式及实现边界；“TUI 形式”不表示终端技术上只能如此，也不把偶然分支当成应复制的产品规则。

| 行为 | Pi 怎么做 | 属于哪类 | Gupi 怎么做 | 理由 |
| --- | --- | --- | --- | --- |
| `!` / `!!` 的执行与上下文 | TUI 解析一位/两位前缀，命令分别加入/排除模型上下文；RPC 用 `bash` 的 `excludeFromContext`，不走 prompt。[P1]、[P2] | Pi 执行语义，直接采用 | 隐藏入口前缀，正文与底栏布尔选项调用同一 `bash` 入口 | 不能把 Shell 发成模型消息或自行展开模板、变量。 |
| 单缓冲、可编辑前缀 | `onChange` 按 `trimStart().startsWith("!")` 切模式；原消息与命令共用文字。[P3] | TUI 当前呈现方式，不是 agent 契约 | 独立 Textarea / Bash Editor，原消息草稿不动，模式由界面表示 | GUI 能清楚区分任务，无需为 TUI 的形式牺牲草稿和高亮。 |
| 已有草稿时进入 | 在现有文字前加 `!`，剩余旧文字直接成为命令。[P3]、[P1] | 单缓冲带来的行为 | 光标在普通输入开头时，仅消费本次键入/粘贴的入口；旧草稿不进入命令 | 用户明确要求保留普通草稿，不能把原消息误当 Shell 执行。 |
| 第二个 `!` 与正文中的 `!` | 提交按前两位解析，没有“紧接”的时限；`!!! ls` 得到排除上下文的命令 `! ls`。[P1] | 前缀文本表示法 | 键入入口后紧接的第二个 `!` 切“不加入”；结束入口后 `!` 全是正文，后续改选项用底栏 | 隐藏前缀后不能继续扫描命令开头，把合法 Bash 字符反复吞掉。 |
| 普通文本粘贴 | 在光标处插入，提交才运行。[P4] | 更合适的处理方案，采用 | 普通输入只有符合入口条件的前缀粘贴被消费；Shell 内的粘贴一律按光标插入，不执行、不重新剥前缀 | 保持编辑与执行分离，粘贴不会替换隐藏草稿或误运行。 |
| 大粘贴占位标记 | 超过 10 行或 1000 字符存为 paste marker，提交时展开。[P4]、[P5] | TUI 的空间压缩方案 | 使用 Kit 多行编辑、滚动，显示实际命令 | GUI 已有足够的编辑能力，不另造占位符与展开模型。 |
| IME | 终端输入路径不提供桌面 marked-text 契约 | 平台差异，不能推导桌面行为 | 组合过程交给 Kit，不切模式、不执行；取消 IME 的 Esc 不再穿透 | 复用桌面输入组件，不自写输入法逻辑。 |
| Backspace 返回 | 普通删除前缀，`!!`→`!`→空；删掉 `!ls` 的前缀后 `ls` 成为普通文本。[P6] | 可编辑前缀形式的结果 | Shell 正文为空、无选区/IME 组合时 Backspace 返回消息；有正文正常删除，不把剩余命令移入普通输入 | GUI 没有可删的模式前缀，空输入返回是直接对应操作。 |
| Esc 优先级 | 补全先处理，再先停止模型、否则停止 Shell、都没有才清空退出。[P7]、[P8] | Pi 的任务处理方案，采用 | 先 Kit 浮层/IME/查找，再模型→命令→空闲退出 | 同时运行时顺序明确，一次 Esc 只做一件事。 |
| 停模型时恢复队列文字 | 取回 steering/follow-up 并合并到当前编辑器。[P9] | TUI 队列编辑方案，不是停止协议 | 保留 Gupi 已有队列操作，不自动把模型队列写入 Shell | 双输入已有明确归属，本项不重设计队列。 |
| 空闲退出、未运行命令 | Esc 清空输入后退出；再输入 `!` 不恢复旧命令。[P7] | 简单明确的取消方案，采用 | 空闲 Esc /“返回消息”清空未运行的 Shell 正文并恢复普通草稿；不另存退出副本 | 避免下次进入意外恢复已取消命令。会话临时切走不等于主动取消。 |
| 接受执行后清空 | 提交组件先清空输入；结果失败或取消也不自动回填。[P5] | 更合适的处理方案，采用 | 本地接受后立即清空本次命令；结果只更新输出，不覆盖下一条输入 | 减少重复执行，不让异步结果改写用户正在编辑的内容。 |
| 清空后退出 Bash | 空文字不再有前缀，`onChange("")` 使 Bash 模式关闭。[P3]、[P5] | 模式等于前缀的呈现限制 | **留在 Shell**，保留上下文选项，可继续编辑下一条 | 模式有独立 UI，不必重复进入；这是用户已确认的 GUI 行为，不声称 Pi TUI 也留在 Bash。 |
| 另有 Shell 运行时提交 | 警告并恢复刚被清空的文本，不排队。[P1] | 单活动命令与保留输入的方案，采用 | 运行禁用，Editor 仍可编辑；按钮与 Enter 共用判断，本地拒绝不清空 | 不必复制“先清再恢复”的内部顺序，也不新建队列。 |
| 模型回复期间运行 Shell | 允许，结果按 Pi 的时机记录/进入上下文。[P10] | Pi 支持的并发语义，采用 | 沿用 `can_run_shell`，不另加模型/Shell 互斥 | 不把命令转成 steer/follow-up，也不承诺当前模型回合立刻看见结果。 |
| 空 `!` / `!!` | 空命令没有进入执行分支，继续落到普通消息处理。[P1] | 当前分支边界，不是更好的方案 | 空白 Shell 正文禁用运行，不向模型发前缀 | 独立运行操作的对象是命令，不应产生另一种动作。 |
| 输入历史容量与去重 | 内存最多 100 条，连续重复去重；浏览开始保存当前输入，退回末尾恢复。[P11] | 成熟的输入召回方案，采用 | Shell 命令及上下文选项组成历史项；最多 100，连续相同项去重，显式召回并恢复浏览前输入 | 保留有用的召回能力，不增加磁盘历史或搜索。 |
| 普通消息与命令混合历史 | 单编辑器共用一份文本历史。[P1]、[P11] | 单缓冲的组织方式 | Shell 历史只收 Shell，不在这里添加普通消息召回 | 避免把消息/token 当命令，不扩大普通 Composer 的功能。 |
| 上下箭头召回 | 首末视觉行满足条件时浏览历史，另有显式 history 动作。[P12] | TUI 的按键复用方案 | 显式“上一条命令 / 下一条命令”Action，方向键留给 Kit Editor | 多行代码编辑优先移动光标，不读取私有视觉行信息仿造 TUI。 |
| 执行历史与重启 | `bashExecution` 恢复为输出组件，不回填编辑器历史；用户消息分支才补充输入历史。[P13] | Pi 的持久化边界，采用 | 既有会话历史继续取自 Pi；Shell 内存召回和未运行输入不重启恢复 | 不建第二份执行账本，不承诺 Pi 没保存的命令草稿。 |

[P1]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L3304
[P2]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L561
[P3]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L3080
[P4]: https://github.com/earendil-works/pi/blob/36be9c612/packages/tui/src/components/editor.ts#L1260
[P5]: https://github.com/earendil-works/pi/blob/36be9c612/packages/tui/src/components/editor.ts#L1364
[P6]: https://github.com/earendil-works/pi/blob/36be9c612/packages/tui/src/components/editor.ts#L1380
[P7]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L3027
[P8]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/components/custom-editor.ts#L103
[P9]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L4734
[P10]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L6983
[P11]: https://github.com/earendil-works/pi/blob/36be9c612/packages/tui/src/components/editor.ts#L428
[P12]: https://github.com/earendil-works/pi/blob/36be9c612/packages/tui/src/components/editor.ts#L925
[P13]: https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L3842

### 入口：只消费本次输入，不吞原草稿

与旧 #82 的“普通输入开头键入 `!`、粘贴 `!! ls`、前缀隐藏、普通草稿原样保留”一致，采用以下具体规则，不增加“只有空草稿才允许”的限制：

- 普通 Composer 聚焦、光标为第 0 位且没有选区、没有 IME 组合时，键入 `!` 进入空 Shell，选“加入上下文”。该 `!` 是 UI 入口，不写入普通草稿或 Shell 正文；之后键入的命令只进入 Shell。原普通草稿即使非空也完整保留。
- 同一位置粘贴**本次文本**以 `!!` 或 `!` 开头时，分别选“不加入”或“加入”；仅移除本次文本的模式前缀及紧随的入口分隔空白，把余下文本放入 Shell。粘贴 `!! ls` 得到 `ls`，粘贴 `!!! ls` 得到不加入上下文的 `! ls`。中间换行和正文字符不重写，不自动运行。
- 这里的“后续文本”指前缀之后继续键入的文字或**本次粘贴**的余文，绝不指旧普通草稿中光标右边的内容。例如原草稿是“稍后解释结果”，光标回到开头粘贴 `!! pwd`，Shell 只得到 `pwd`，返回后仍是“稍后解释结果”。
- 键入入口 `!` 后，紧接着的第二个键入 `!` 切“不加入”，不成为正文；无需时间阈值。该机会只属于这次键入入口，正文输入、粘贴、光标/选区操作或焦点转到其他控件后结束。之后（包括提交后清空、历史召回后）所有 `!` 都是命令字符；上下文改选走底栏。前缀粘贴已一次解析完，不再等待第二个 `!`。
- 光标不在第 0 位、存在选区、粘贴前有空白再出现 `!`，均按普通编辑处理；不扫描整份草稿、不因撤销、加载或模板填入的 `!` 自动切换。IME 的临时组成文字不触发；只在公共输入接口能区分的已确认用户输入上处理入口，不能靠原始键码猜字符或重做输入法。

这样既允许已有草稿时进入，也不会把原草稿或被选中文字变成命令。另一种“去掉前缀后把整份普通草稿送入 Shell”的规则违反本次明确的原样保留要求，不列为待选。实现必须用真实键入和粘贴证明普通 `InputContent`、token、附件、选区和撤销历史不变；不能用一次整体 `set_value` 重建来冒充保留。

### 命令召回与输入生命周期

Shell 内存历史只保存被执行入口接受的命令，历史项包含命令正文与 `excludeFromContext`，相邻二者都相同才去重，超过 100 条移除最旧项。召回不会运行命令，也不把隐藏模式前缀塞入正文；它更新 Editor 和底栏选项。第一次浏览前保存当前命令、选区及选项，返回历史末尾恢复；不改普通 Composer。

“上一条命令 / 下一条命令”使用显式 Action 并在现有命令面板暴露，作用于当前 Shell 输入；没有可召回项时禁用。Kit Editor 的上下方向键继续移动光标，不新增历史搜索面板或全局快捷键。

提交后清空本次输入，留在 Shell，保持本次上下文选项；空闲 Esc、“返回消息”清除未运行命令并退出，空 Shell 的 Backspace 返回。返回恢复原 Textarea 焦点、选区与草稿，不把命令转成普通消息。临时切换会话保留该 view 的当前输入，不等于主动退出；view/session 释放后不再保留。Shell 输入与召回列表都不写入 DraftFile，普通草稿原有持久化不变。

### 执行、停止与上下文

运行读取当前 Shell 正文与上下文开关，调用 [conversation/shell.rs](../../../crates/gupi-conversation/src/conversation/shell.rs) 的正式入口。`!` 对应 `excludeFromContext: false`，`!!` 对应 `true`；不调用 `prompt`、`steer`、`follow_up`，不预先展开变量、skill 或模板。

现有 `can_run_shell` 复用连接/会话准备/扩展交互等提交条件并排除另一个活动 Shell，没有把模型 streaming 作为排斥条件。因此保留“模型回复期间也能运行命令”，不额外加互斥锁。执行器是否允许其它阶段由现有入口和 Pi 决定，不新增应用状态矩阵。

命令被现有执行入口接受后，清空本次 Shell 正文并留在 Shell，保留上下文选项；已有活动命令或本地拒绝时保留文本。实现可让 `run_shell` 返回是否接受，不必复制 TUI“先清空再恢复”的内部顺序。异步结果只更新命令输出，不再修改用户已经开始写的下一条输入。空闲 Esc 清除命令；临时切换会话不等同于退出或停止。

Enter 运行、Shift+Enter 换行；活动 Shell 命令期间禁用运行但仍可编辑，模型回复期间仍允许 Shell。前缀已由界面表示，空白命令按 #82 保持禁用。Pi 当前代码的空命令会越过 Bash 分支继续走普通消息流程，这是已核实的差异；独立“运行”按钮不因此发送 `!` 给模型。

上下文开关只影响本次执行结果是否加入模型上下文，不改变历史是否记录，不触发模型回答；过去的结果不能靠切换当前开关改写。tooltip 使用完整文案：“输出会加入模型上下文，命令不会触发模型回复” / “输出不加入模型上下文，但仍记录在会话历史中”。模型正在回复时不要承诺该回合立即看到 Shell 结果，采用 Pi 自己的记录与排队时机。

Esc 的任务优先级采用 Pi；本项不移植 TUI“停止时自动取回整个队列到编辑器”的操作，Gupi 已有的队列保留其现有操作与所有权。事件留在输入焦点上下文，先让 Kit 菜单、IME、补全、搜索与现有浮层处理；临时窗口的 Enter 粘贴答案和 Esc 隐藏不得穿透 Shell 操作。RPC 的 `bash` / `abort_bash` 仍复用 [Pi rpc-mode.ts:561](https://github.com/earendil-works/pi/blob/36be9c612/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L561)。

### 范围与简化

本项只组合已有 Editor 与底栏，不自写输入组件，不为将来输入组件建立接口框架，也不以富文本选型作为前置条件。Pi 负责执行与记录，Zed 只作为代码编辑体验的参考上限，不移植它的终端、自动高度或编辑器布局。

不加命令危险性扫描、每次执行确认、粘贴确认、跨会话确认、超时自动取消、断线重试或新命令队列。Shell 内普通文本粘贴沿用光标插入；普通 Composer 的前缀入口只消费本次输入，不存在“替换原消息草稿前确认”的需要。必要的会话归属、IME 与一次动作只提交一次，沿用现有所有者和组件入口，不增加通用保护框架。

## 实现顺序

1. 在 Home 组合独立 Kit Editor + 底栏，保留原 Composer entity 和草稿；不等待输入组件研究或 InputGroup 扩展。
2. 接入只消费本次输入的键入/粘贴入口、第二个 `!`、空 Backspace 与返回操作；先证明原草稿不变、IME 不被截断，再接入 Shell 内存历史和显式召回。运行与返回动作限定到焦点上下文，不能同时触发消息发送或临时窗口快捷动作。
3. 调用现有 Shell 能力，接受后清空并留在 Shell，本地拒绝保留；沿用执行输出、停止和历史刷新。会话切换与异步结果不得覆盖新会话/下一条输入。
4. 同步九种语言的可见文案、tooltip 和无障碍名称；命令、路径及输出不翻译。完成受影响验证并记录实际结果，交付试用，不并入输出侧重构。

## 必要验证

新增行为的主要风险是误把普通草稿送去执行、破坏 IME/选择、按键穿透到旧发送入口，以及 Shell 结果覆盖另一会话或下一条输入。按 AGENTS.md 优先扩展现有 Home 正式组件回归与 Shell 能力回归：通过真实输入/粘贴和按钮/键盘路径观察草稿保留、RPC 类型及上下文参数、执行期间可继续编辑、返回焦点的用户结果；不为每个字段、builder 或禁用谓词另建测试，也不重新验证 Kit 整套编辑器。

实施后运行 `cargo fmt --all -- --check`，对实际受影响的 root/UI/conversation/settings 包执行锁定的全目标 Clippy（`-- -D warnings`）和相应测试；改到根集成时包含 `cargo check -p gupi --locked`。文档阶段只核实来源与文档差异，不运行 Rust 门禁。

原生试用聚焦浅色/深色和一个窄窗的实际输入流程，尤其是 IME 不误触发、Shift+Enter 多行、Esc 先处理输入层，以及临时窗口按键不穿透。组件回归与原生结果分别记录；Computer Use 自身故障立即报告原始错误，不把未执行场景写成通过。测试/原生结果不是额外扩大产品范围的理由。

## 剩余待定点

无产品选择待定。双输入、隐藏前缀、原草稿保留、只消费本次入口文本、提交后留在 Shell 和显式历史召回均按上述规则实施。若锁定 Kit 的公开接口不能在不破坏普通草稿/IME 的前提下接入键入入口，应报告具体 API 缺口，由用户决定入口调整或上游需求；不借机改为单缓冲、访问私有状态或重写输入组件。
