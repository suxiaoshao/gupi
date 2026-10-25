# #130 区分输入提交结果

状态：已实现。核对基线：Gupi main `315a6b0`；本地 Pi `36be9c612`（1.4.0）。

## 现状与 Pi 行为

原行为：`pi-rpc/src/client.rs:371` 保留了 `Response.data`，但 `gupi-conversation/src/conversation.rs:2217` 对成功统一走 `Ok(_)`，没有消费 `disposition`。现有提交回执只表示是否接受；运行和队列另有 Pi 事件驱动。

Pi `packages/coding-agent/src/modes/rpc/rpc-mode.ts:395` 在预检成功后返回结果，`core/agent-session.ts:2092` 定义实际分支：

| 结果 | Pi 的含义 | 界面反馈 |
| --- | --- | --- |
| `handled` | 扩展命令或 input handler 接管了输入 | 已交由扩展处理 |
| `queued` | 输入进入 steer / follow-up 队列 | 沿用已有队列状态，不另弹提示 |
| `started` | 本次输入开始运行 | 沿用已有运行状态，不另弹提示 |

`steer` / `follow_up` 只返回前两种。`handled` 不保证扩展工作成功结束，也不表示整个会话空闲；扩展可以自行发起模型工作。三种结果都不是完成回执、消息 ID 或队列 ID。

## 当前实现

- RPC 层解析结果，ConversationState 处理本次提交，仅 `handled` 复用现有非阻塞反馈提示“已交由扩展处理”。无需新增会话消息、持久状态栏或队列模型。
- 三种均按“输入已接受”清理本次提交的草稿和附件；拒绝仍保留。运行状态、停止按钮与队列内容继续以 Pi 事件/快照为准，不用 `handled` 清空已有运行，也不用 `started` 覆盖已经先到达的结束事件。
- 缺失或未知 `disposition` 的成功回执仍视为已接受，不弹处理结果提示，不猜测、不自动重发、不因解析该字段失败而诱导重复提交。
- 先接通结果解析与既有提交入口，再验证三种回执及与事件先后顺序的组合；沿用普通、队列和临时模板的共享提交路径，不复制 Pi 分派规则。

回归通过 RPC 客户端提交入口覆盖三种、缺失和未知回执；接受后的草稿消费不改变事件拥有的运行状态，也不清掉更新过的草稿。

## 本轮验证

受影响五包 fmt、Clippy（all-targets、-D warnings）与 test 全通过；RPC 5 项通过，1 项既有 installed-Pi 回归忽略，conversation 46 项、root 127 项通过。
