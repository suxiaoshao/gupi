# #132 会话失败诊断日志

状态：已实现。核对基线：Gupi main `315a6b0`；本地 Pi `36be9c612`（1.4.0）。

## 现状与 Pi 行为

原有基础：`src/app.rs:190` 已安装诊断 writer，`src/app/logging.rs:1` 已有当前文件加三个 5 MiB 归档；保存失败和待命连接失败等已有零散记录。`gupi-conversation/src/conversation.rs:1500,1552,1592,2239,2601` 的会话读取、启动/断连、提交拒绝、最终重试失败主要更新界面状态，没有一致的会话与阶段关联。

Pi 通过 RPC error、assistant error 和重试事件报告失败；`packages/coding-agent/src/core/agent-session.ts:1147` 会将包括失败 assistant 在内的消息交给 SessionManager 保存，`session-manager.ts:1172` 在非持久模式直接跳过写盘。这是会话内容，不代替 Gupi 的进程启动/协议诊断；临时会话仍应保持 `--no-session`。

## 当前实现

- 在现有失败处理接点补 `tracing` 结构化记录，复用日志目录和轮转；不加独立日志系统、远程上报或自动重试，不复制 Pi 正文。
- 字段限于时间、Pi session ID（已有时）、运行时 instance、连接 binding、固定阶段、错误类别及已有退出状态。尚未取得 session ID 时用运行时 instance 关联启动失败；不要把内部 session key 原样写出，它实际是文件路径（`session_catalog.rs:42`）。
- 先覆盖连接启动/异常退出、核心状态与历史读取、输入提交拒绝、模型最终失败；记录发生失败的处理分支，不在 render 中记录。正常停止、主动关闭和过期请求丢弃不伪装成错误；Pi 自动重试由 Pi 继续负责，不把每个重试都记成新的最终失败。
- 在错误转为 String 前保留 `pi-rpc::Error` 的安全类别（`error.rs:2`）；仅有自由文本的 Pi 错误记录事件种类与失败标志。默认不记录原始 error 字符串、stderr、RPC payload、提示词、工具参数/输出、凭据和含认证信息的 URL。信息不足不靠“截短正文”补诊断，也不另建通用脱敏框架。
- 实现后用已有失败夹具确认日志可关联阶段，且带凭据/正文标记的错误不会写出这些值；不改变既有界面错误展示与会话持久化。

新增日志通过现有 tracing writer 默认写入本地轮转文件。`pi-rpc::Error::category()` 只返回固定类别；运行时在当前有效启动失败上记录 instance；Conversation 在会话文件/目录准备、连接失败/退出、状态/历史读取、提交失败与 agent_settled 最终模型失败分支记录安全元数据。尚无 Pi session ID 时不使用 draft ID 或内部路径替代。退出状态保留，stderr 不写入这些日志。

核心读取在保持类型错误的同时携带实际失败阶段；输入编码/提交结果在转换为 UI 字符串前保留安全类别。过期绑定、已中断提交及主动关闭的 Closed 不新增错误记录；自动重试中间结果不作为最终模型失败记录。界面原错误与 Pi 会话持久化行为不变。

回归捕获正式拒绝提交处理分支：确认 session/binding/阶段/类别可关联，UI 仍保留原错误，而凭据 URL、正文、命令原文和内部路径标记均不进入日志。不添加日志系统、正文复制或脱敏分类框架。

## 本轮验证

受影响五包（gupi、gupi-conversation-ui、gupi-conversation、gupi-pi-runtime、pi-rpc）fmt、Clippy（all-targets、locked、-D warnings）与 test（all-targets、locked）全通过：127 / 231 / 46 / 4 / 5；另有两条既有 installed-Pi 测试按原设置忽略。未运行真实模型请求或追加原生检查。
