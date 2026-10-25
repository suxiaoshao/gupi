# #133 codemode/MCP 嵌套工具调用归属

状态：已实现。核对基线：Gupi main `3191bbf`；本地 Pi `36be9c612`（1.4.0）。参考行为来自源码核对；实现及本轮验证见文末，未执行模型或 MCP 工具。

## 原行为与 Pi 参考

原行为：`crates/gupi-conversation/src/conversation.rs:137,2763` 将工具事件按 toolCallId 存入 ToolActivity，但没有保留 `parentToolCallId`。UI `home/messages/tools.rs:102` 主要配对 assistant toolCall 与 toolResult；仅有实时子调用事件不会因此生成独立工具行。`tool_details.rs:261` 可显示 codemode 的原始 details JSON，但没有归属清单，也没有专门投影 toolResult 顶层 `nestedCalls`。

Pi 的两种数据不能混为一谈：

- **实时事件**：`core/nested-tool-calls.ts:110,176` 为 `ctx.executeTool()` 发出 start/update/end，带自己的 toolCallId 和直接调用方 `parentToolCallId`。RPC 原样转发会话事件（`modes/rpc/rpc-mode.ts:355`）。是否嵌套以该字段为准，不以 codemode/MCP 名称判断。
- **历史摘要**：`core/agent-session.ts:1108` 把 `nestedCalls` 放到最外层模型工具调用的 toolResult；共享 recorder 收集其所有后代。`packages/ai/src/types.ts:618` 的记录只有 ID、名称、可选参数、状态、耗时和截短错误，**没有子调用结果正文，也没有显式的直接父 ID**。`core/nested-tool-calls.ts:24` 限制 256 条、单条参数 8 KiB、总参数 32 KiB、错误 500 字符；`complete=false` 也可能表示参数省略或调用未结束，不等于父工具失败。
- **Pi 展示**：TUI 在 `interactive-mode.ts:3563` 跳过子调用独立工具行；codemode renderer 在父行内部展示 `details.calls` 的状态/参数/耗时，折叠时最近 8 条、展开看更多（`extensions/codemode/renderer.ts:105`）。这份专用详情还含模型调用，运行中工具 ID 可暂为 `<parent>/?`（`execute.ts:529`），不能当成通用稳定事件 ID。Pi HTML 导出另用 `nestedCalls` 在父工具内显示可展开摘要和 incomplete 提示（`core/export-html/template.js:1031`）。

## 当前设计

- ConversationState 保留实时事件的父 ID；现有 ToolIndex 在当前会话/分支内投影到父工具，UI 在父工具详情增加“嵌套调用”清单，不新增顶层消息或另一棵会话树。多级实时关系沿明确父 ID 找到所属顶层调用；恢复后只展示父结果自带的扁平摘要，不拆 ID 字符串猜树形关系。
- 首版使用通用事件与 `nestedCalls`：运行时显示已收到的名称、状态和参数；父结果到达后以其有界摘要展示，重开会话仍可还原。复用父工具现有展开入口，默认最多显示最近 8 条，展开看已记录条目；耗时、错误和参数只显示 Pi 实际提供的值。未完成与记录不完整分别说明，参数被省略就说明省略，不伪造成功或完整调用数。
- `details.calls` 保留现有附加详情展示，不再与通用清单合并出重复行，也不按占位 ID/名称猜匹配。本项只解决工具归属，不扩展 codemode 模型计费展示，不持久化子结果正文，不增加子调用重试/停止操作。找不到明确父调用时不挂到相邻工具。
- 在既有工具投影回归中覆盖并行父调用隔离、实时到最终摘要及重开恢复、不完整摘要；防止错误归属、重复行和把子调用当独立模型调用。新增文案覆盖现有九语言，状态同时有文字，不只靠颜色。

## 实现

- `ToolActivity` 记录明确的直接父 ID 和 Pi 提供的耗时；工具事件由原 Session owner 接收，不增加消息或 RPC。
- `messages/nested_calls.rs` 只沿实时父字段找到当前投影中的顶层调用，保留接收顺序；未知父调用不猜测归属。ToolIndex 复用同时考虑所属子调用快照，子调用更新不使并行父工具的内容失效。
- 父结果的 `nestedCalls` 替换实时清单，不与 `details.calls` 合并；历史仍是无父 ID 的扁平记录。缺失参数、未完成状态和记录不完整分别显示，不以记录不完整推断父工具失败。
- 父工具详情默认显示最近 8 条，可展开全部已记录条目；共用现有详情滚动区域。九语言文案同步，状态有文字，原附加详情 JSON 继续保留。

## 验证范围

新增工具投影回归覆盖并行父调用、多级实时归属、不透明 ID、未知父调用、实时到最终摘要、重开恢复和不完整记录；真实详情组件回归覆盖默认最近 8 条与展开/收起入口。只检查这些产品行为，不调用模型或 MCP，不增加原生验收矩阵。

本轮结果：`cargo fmt --all -- --check` 与三包 `cargo clippy -p gupi-conversation -p gupi-conversation-ui -p gupi --all-targets --locked -- -D warnings` 通过。初轮受影响包测试中 root 127 项、conversation 46 项通过；新增详情回归补齐 Kit 静态行观察后，最终 `cargo test -p gupi-conversation-ui --all-targets --locked` 233 项全部通过。未增加测试矩阵、依赖或原生检查；未执行真实模型/MCP 工具。
