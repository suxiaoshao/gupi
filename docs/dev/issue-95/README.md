# 选中文本引用到输入框

状态：已确认，待实现。首版 Diff 只支持单侧行号选区。本文是设计，不代表已实现或已验证交互。

## 目标与边界

从对话正文、现有只读源码或差异中选一段，把**当时显示的内容快照**加入当前会话的普通消息草稿，再写问题。引用是可编辑文本；多次引用就是多段文本，编辑、删除、撤销沿用输入框。引用动作不发送消息、不读取磁盘补全、不执行工具。

以 Codex 桌面版 Add to chat 为能力上限，采用已选定的简化呈现。没有引用 chip、独立批注编辑器、`:codex-annotation`、出处回跳、批注线程、自动修改文件或 PR 评论同步。Pi 仍只收到现有 `prompt.message`；不增加 RPC、会话元数据或引用持久化。历史使用 Pi 保存的普通用户消息和现有 Markdown 渲染。

## 入口与公开 API

依据锁定的 `gpui-base` / `gpui-component` **0.8.0**；下列路径相对各 crate 的 `src/`，不是对后续版本的假设。

| 来源 | 可用公开接口 | 取值与限制 |
| --- | --- | --- |
| 对话正文 | `TextViewState::selected_text()`、`selected_source_range()`、`rendered_text()`；`text/state.rs:407–421`。Gupi `home/messages/markdown.rs` 已保留 `Entity<TextViewState>`。 | 取当前这一个 Markdown 正文的选中文本，沿用当前默认 `SelectionFormat::Plain`，不重新解析消息、不用字符串搜索定位重复片段。角色从拥有这段正文的消息取得。`selected_source_range` 只在精确映射存在时可用，本功能不依赖其推算文件行号。 |
| 只读源码 | `EditorState` 是 `InputBaseState<EditorMode>`；公开 `selected_range()`、`selected_text()`、`text()`。见 Base `input/base/state.rs:3344,3996`；组件 `input/editor.rs:109` 的 `context_menu`。 | 对 `home/source.rs::Preview.editor` 的已加载文本取 UTF-8 字节范围和内容；行号按该文本中的逻辑换行计算，一基、末端不包含下一行。保留部分行，不扩成整行。折行显示不改变行号。只读不妨碍选择。 |
| Diff 行号选区 | `DiffState::selected_lines()`、`selected_text(cx)`、`DiffEvent::{SelectionChanged,SelectionEnded}`；`diff/state.rs:79,508,723`。`DiffLineRange` 公开 path、side/start、end_side/end（`diff/document.rs:66–108`）。 | 支持同一文件、同一侧的行号选区；行号为一基闭区间，正文由 Kit 返回，不包含 gutter、`+/-` 或批注。以 Gupi 已加载对象的真实路径和来源作标题，不使用 Git 文本适配器内部的占位文件名。 |

**Diff 的明确限制：**字符拖选可以通过 `selected_text(cx)` 得到文本，但精确源范围/侧别提取位于私有 `diff/selection.rs::selected_source_range`，`DiffState::selection()` 也为 `pub(crate)`；字符选区不能假定同时有可靠 `selected_lines()`。跨旧新侧的行号选区虽然有两个端点，返回正文可能混合两侧且没有符号，不能标成“新侧”或“旧侧”。首版建议只启用**单侧行号选区**；其他 Diff 选区提示“请从行号选择同一侧的内容”。不按鼠标 X 坐标猜侧别，不匹配文本反推位置，不复制依赖私有实现。若要求所有字符选区都携带精确侧别/行号，需要先有上游公开接口，不能以当前 API 宣称完整支持。

对话首版限一个正文 TextView 内的选区，不将跨消息选择拼为一个虚构角色。源码多光标不组合为连续选区。Diff 的纯文本 fallback、二进制、超限或无法读取状态没有可靠的文件侧别/行号，不提供该入口；仍保留已有复制能力。Pi Changes 和 Git 的正常 DiffBlock 共用入口，出处语义分别来自各自拥有者。

## 交互

统一操作名为“引用到输入框” / “Quote in composer”。选区对象的**右键菜单**提供操作，不加跟随鼠标的浮动工具条，也不加专用快捷键。菜单保留已有 Copy 等操作，引用放在复制邻近位置；不新增“复制后再粘贴”的隐式流程。

- TextView 和 Diff 没有专用菜单 builder，使用现有外层 `div` 的 Kit `ContextMenu` 组合；范围只包正文，不接管文件导航或整个窗口。
- Editor 使用公开 `Editor::context_menu`。该接口**替换**默认 NativeMenu，必须保留适用于只读编辑器的既有复制、全选等标准项，不能仅返回引用一项。
- 同一个应用 Action 也放入既有命令面板，提供不依赖鼠标悬停的入口；打开面板前捕获当前受支持的选区。已有打开命令面板的键位保持不变，不给引用绑定新键。
- 无选区、输入框不可编辑、扩展问答占用输入框或 Shell 模式时禁用，理由由菜单所在区域/命令描述说明。引用不自动退出 Shell、不写入扩展问答，不自动新建或切换会话。
- 激活后显示对话输入框（必要时退出工作区单区前景），聚焦并滚动到新增文本之后。由既有布局/焦点入口完成，不直接改分隔比例。取消菜单恢复原焦点，不改选区或草稿。

捕获发生在菜单/命令面板接管焦点前：来源拥有者读取公开选区、真实来源标签和文本，生成一次性快照，同时记录目标会话 key/binding。菜单期间流式更新不改变已捕获的引用；若目标会话切换、断开重绑或变为不可编辑，取消插入，不能落到另一个会话。快照只随这次操作存活，不成为常驻“最近引用”缓存。原选区由 Kit 管理，应用不仿造第二套选择状态。

## 插入文本

**追加到普通草稿末尾**，不替换草稿中已有选区，不猜上次光标位置。保留所有已有文字、资源 token 和附件；只追加缺少的空行，使前后相邻段之间至少有一个空行。每次插入以两个换行结束，光标落在引用后的空白段，便于直接输入评论。重复引用不去重、不合并。

对话使用角色标题和 Markdown 引用块，每一逻辑行加 `> `，空行也留在引用中。取可见正文文本，不带隐藏推理、工具 payload 或图片原始数据；已有 Markdown 标记按普通引用文本呈现，不承诺还原原富文本样式。角色用现有消息真实语义，不把用户消息统一标成助手。

```text
助手：
> 先检查请求是否已经取消。
> 然后再提交状态更新。

这里是否有竞态？
```

```text
Assistant:
> Check whether the request was cancelled first.
> Then commit the state update.

Could this race?
```

源码和 Diff 使用来源标题加围栏代码块。路径优先使用相对于该会话 cwd 的真实路径，不能相对时使用绝对路径；不生成 `@` token、链接或自动读文件指令。路径展示必须可区分换行等控制字符；只是来源说明，不作为可执行命令或可回跳标记。行号描述范围内实际选中的部分，不意味着整行都被选中。

````text
文件：src/app.rs，12–14 行
```rust
if cancelled {
    return;
}
```

````

````text
File: src/app.rs, lines 12–14
```rust
if cancelled {
    return;
}
```

````

Diff 标题明确来源和侧别，例如“Git · 已暂存：src/app.rs，旧侧，12–14 行” / “Git · Staged: src/app.rs, original side, lines 12–14”；Pi 对应“本轮改动：…，新侧，…” / “Turn changes: …, modified side, …”。Git staged/unstaged/untracked 和旧/新侧按当前加载版本解释，不把 index 或历史内容称为当前磁盘文件。代码块内是所选侧的源码文本，因此语言使用该文件的已有语言信息，不标为 `diff`。

围栏复用现有 `messages/presentation.rs::code_fence` 的避碰规则，长度大于正文中可能关闭围栏的反引号序列；语言未知就省略语言，不伪造语言。正文不 trim 缩进、不补读上下文；包装需要的换行不冒充源文件内容。

输入 mutation 使用 TextareaState 的公开选区/插入 API：在末端设置空选区，一次 atomic `insert`，让现有 InputEvent/草稿同步链保存 `InputContent`。Base `input/base/state.rs:1159` 已把 insert 标为 Atomic 并移动光标；不得 `set_value(拼接后的全文)` 重建 token/撤销栈。一次撤销应只撤掉本次引用和分隔空行。IME 有未提交组合时不强行覆盖或提交组合，待输入框可安全编辑后再执行用户操作。

## 长选区、状态与本地化

不新增模型 token 限制或任意的“前 N 字”截断：引用完整的**已加载选区**，不静默丢尾部。文件/差异仍受各自现有加载边界约束，引用不会绕过它们；对话正文没有新增引用专用长度门槛。不在 render/鼠标移动时持续格式化大字符串，只在用户打开操作入口时捕获、执行时插入。若将来实际测得大选区卡顿，再按证据决定交互预算，不把显示预览上限当输入上限。

加载中、空选区、只有图片且无可选文本、二进制、读失败、没有可用 DiffBlock 时不可引用，不改成整篇复制。流式正文只引用捕获时已经显示的部分；来源的后续变化不更新草稿。已有当前会话草稿可编辑时允许引用，是否可发送仍由现有 Pi/会话状态判断，不自动等待后发送。

九语言 `de/en-US/es/fr/ja/ko/pt-BR/zh-CN/zh-TW` 同步操作名、禁用原因和来源标题的完整 Fluent 句式；参数为 path、起止行及必要来源，允许各语言安排顺序，不拼翻译碎片。文件路径和所选正文保持原文。插入时使用当前界面语言，之后切语言不改已经进入草稿的文字。角色可复用已有语义相同的本地化 key。

菜单使用 Kit 的焦点、方向键、Enter/Escape 和禁用语义；名称不依赖图标、颜色或 tooltip。命令面板是键盘可达入口，源码保持现有键盘选区；不为 Markdown/Diff 重新实现键盘选择。组件原生选区可访问性以其公开能力为限，不宣称本设计新增了完整屏幕阅读器选区支持。使用既有菜单主题、密度和焦点 token，不造引用卡片、成功 toast 或新 overlay 层。

## 所有权与改动位置

`home` 拥有“引用当前显示内容到当前会话草稿”的动作路由和一次性目标绑定；来源拥有者只交付文本与已知出处。输入框继续拥有编辑、撤销和光标，ConversationState 继续拥有草稿，Pi 继续拥有发送与历史。可用一个小型 `home/quote.rs` 组合格式与动作，不建立引用服务、持久缓存或全局剪贴板监听。

主要接入 `home/messages/markdown.rs` 及其正文调用处、`home/source.rs`、`home/messages/diff.rs`、`home/workspace/{changes,git_changes}` 的已加载来源投影、`home/actions` / 命令目录及 Home 的普通输入框入口；九语言增加必要文案。复用现有布局恢复、草稿同步和围栏 helper。`pi-rpc`、`gupi-pi-runtime`、Git 读取层、Pi 历史格式、依赖版本均不改。

## 必要验证

按 AGENTS.md 选择能证明新产品契约的少量验证，优先扩展现有组件路径；关键是实际选区与来源一致、菜单焦点后仍插入正确会话、已有 token/附件和一次撤销保留。通过代表性的正文、源码、单侧 Diff 手动试用确认可编辑结果及多语言菜单，不为字符串常量或框架选择行为另建矩阵。运行格式检查及实际受影响包的 `--locked` Clippy/测试；报告自动化与原生范围，不因文档列出边界就扩大成全平台验收。

## 参考与待确认

Codex 本机解包 `webview/assets/app-initial-f3e281fb9a71.js:1814–1826` 包含选中文本/回答批注的普通文本序列化及专用 annotation 指令；`base-file-tree-22c411648ed4.js:1` 有 Add to chat 上下文菜单；`use-diff-annotations-1611ca064634.js:1` 有评论加入对话操作。这里只借用“把当前内容加入草稿”的入口和快照语义，不复刻其结构化 UI 或标记。右键菜单选择遵循 Kit Design Guides 的 selection-scoped ContextMenu 与键盘备用入口要求。

已确认：首版 Diff 只支持**单侧行号选区**，字符选区与跨侧混选提示从行号选择同一侧；若以后需要字符选区携带精确侧别与行号，需先有 Kit 公开接口。
