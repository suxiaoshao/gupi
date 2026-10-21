# Shell 命令交互

Gupi 为 Pi 的直接 Shell 命令（`!` / `!!`）提供桌面界面，遵循[职责边界](../../gui-boundary.md)：调用 RPC、展示实时输出和历史。命令执行、扩展处理、模型上下文和会话写入均由 Pi 负责。当前已接入输出与历史展示；Shell 输入框尚未接入，输入框中的 `!` 仍按普通消息发送。

## 实时输出与历史

实时行显示命令、输出和运行状态，并提供“停止命令”（`abort_bash`）；收到确认后显示“正在停止”，最终状态以命令响应为准。模型的停止按钮仍只调用 `abort`。复用现有命令/工具详情的文本、折叠、复制和滚动呈现，不伪造模型工具调用。输出按普通等宽文本展示，处理 ANSI 与换行；长输出沿用 GUI 显示截断及展开方式，用户向上查看时不强制滚到底部。

完成后用响应结果结束实时状态，并调用现有历史刷新。历史中的 `bashExecution` 按 Pi 的真实条目 ID 和顺序直接渲染，显示：

- 命令与输出；`!!` 显示“不加入模型上下文”。
- 已完成、退出码或已取消；缺失退出码不当作 0，RPC 成功不等于命令成功。
- Pi 的 `truncated` 提示，以及存在 `fullOutputPath` 时的查看完整输出入口，复用文件预览与错误提示。

历史尚未返回时只保留必要的实时结果展示。模型运行期间 Pi 可能延后加入条目，此时沿用 `agent_settled` 等现有历史刷新时机，刷新到完整历史后撤下临时展示。刷新失败沿用现有错误处理，不提前丢掉屏幕上的结果。这里是临时显示与历史视图的切换，不建立命令回执账本、按内容/出现次数匹配、请求 ID 到历史 ID 的映射或额外同步状态机。

切换会话时输出仍归属原连接，不能串到当前会话。断连显示已有连接错误，不推断命令已取消，也不自动重跑。会话保存和恢复完全采用 Pi 返回的历史；不探测磁盘来判断 Shell 是否持久化，不增加保存提示或自行写会话文件。

## 接入范围

| 位置 | 改动 |
| --- | --- |
| `pi-rpc` | 增加 `bash` / `abort_bash` 命令、`bash_execution_update { id, delta }` 事件与结果类型；让调用方及时取得请求 ID，以正确接收流式输出。 |
| `gupi-conversation` | 调用 RPC，保存当前界面所需的命令状态与输出；响应后触发现有历史刷新。复用连接归属检查。 |
| `gupi-conversation-ui` | 实时输出、停止操作及 `bashExecution` 历史展示，接入现有复制、详情和查找。 |

结果与历史取自 Pi，不在界面再存一套执行事实。不新增执行器、持久化层或通用请求生命周期框架。具体私有 API 在实现时选择最小改动。

所有新增文案、提示和无障碍名称覆盖现有九种语言：`de`、`en-US`、`es`、`fr`、`ja`、`ko`、`pt-BR`、`zh-CN`、`zh-TW`。使用 Fluent 完整消息及变量；命令与输出原文不翻译。组件、样式和状态遵循仓库的 GPUI Kit 与 i18n 指南。

## 必要的 Pi 接口事实

依据 Pi 1.3.0 的 `docs/rpc-commands.md`（Bash）、`docs/json.md`（RPC 事件）和 `docs/message-types.md`（BashExecutionMessage）：

- `!` / `!!` 解析位于交互 TUI，RPC `prompt` 不解析；RPC `bash` 已负责执行、扩展钩子和记录结果。对应实现为 `dist/modes/interactive/interactive-mode.js` 与 `dist/modes/rpc/rpc-mode.js`。
- 流事件的可选 ID 对应 RPC 请求；缺省 ID 的事件不归属当前直接请求，忽略其显示而不关闭连接。历史记录有自己的 ID。空闲命令完成不发送 `agent_end` 等消息事件，因此响应后需要主动刷新历史。
- `abort_bash` 停止该实例的直接 Shell 命令，不停止模型；其确认早于最终结果。UI 同时只提供一个活动 Shell 命令。
- `bashExecution` 包含 `command`、`output`、`exitCode`、`cancelled`、`truncated`、`fullOutputPath`、`excludeFromContext`，缺省字段按协议处理。

这些事实用于 UI 接入，不在 Gupi 复制 Pi 内部的执行、排队或落盘规则。

## 实现与验证

已接入：`pi-rpc` 的 Bash 请求、流事件和停止接口；会话连接所有的实时 Shell 展示与历史刷新；独立的 Shell 消息行、停止按钮、输出折叠/展开、复制、查找与完整输出入口；九种语言文案。普通 composer、前缀输入和 Escape 未改动。

历史读取捕获开始时已完成的临时结果数量。读取期间新增完成结果，或仍有待替换结果且 Pi 的新状态为 streaming 时，暂缓应用整份历史，保留现有历史与实时消息；新增完成会合并触发下一次读取，streaming 则沿用 settled 等刷新时机。只有同一会话绑定的成功、空闲快照能撤下其覆盖的临时结果；读取失败保留结果。没有按命令内容匹配、伪造历史 ID 或新增持久化。

已通过格式检查、受影响的 `pi-rpc` / `gupi-conversation` / `gupi-conversation-ui` 全目标 Clippy（拒绝警告）、受影响包测试（conversation 39、conversation-ui 170、pi-rpc 4，以及 1 个 doctest；另有两个默认忽略项），以及后续聚焦的 Shell 回归。真实组件测试覆盖长输出展开、完整复制、查找展开和模型/Shell 行并存。实时缓存保留 UTF-8 安全的最近 64 KiB，紧凑预览显示最近 80 行并提示显示截断；最终响应恢复 Pi 返回的输出。断连先于结果回调、晚到结果、缓存边界和重连复用请求 ID 均有聚焦回归；临时显示 ID 包含连接 binding，流匹配仍使用原请求 ID。九种语言的 12 个键与变量一致。安装的 Pi 1.3.0 隔离测试验证了真实流 ID、重复命令的不同历史 ID、取消结果、缺省退出码、合法无 ID 流事件，以及无需 agent_end 的完成后历史读取；测试目录已清理。

原生应用构建与隔离启动成功。但 Computer Use 两次无法启动 native pipe（`Sky Computer Use native pipe startup failed`），因此浅色/深色、窄窗口、原生点击/滚动、完整输出入口和会话切换尚未验收。没有用 headless 测试替代这些原生结论，隔离 gallery 数据已清理。
