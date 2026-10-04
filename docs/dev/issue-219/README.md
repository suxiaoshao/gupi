# Gupi：Pi RPC 与进程生命周期

Gupi 使用用户本机的 Pi CLI，通过 `pi --mode rpc` 的 stdin/stdout JSONL 连接。安装、升级、登录及模型提供方配置由用户在 Pi 中完成。协议覆盖与当前版本差距统一见 [RPC/TUI 接入盘点](../pi-rpc-gaps.md)。

## 归属

| 位置 | 职责 |
| --- | --- |
| [pi-rpc](../../../crates/pi-rpc/README.md) | 纯命令探测；一个 client 的 Child、管道、请求关联、事件流、就绪与关闭结果 |
| [Pi 探测](../../../crates/gupi-pi-runtime/src/probe.rs) | 探测 Operation 与 GPUI 任务接入；本地化错误映射由应用 i18n 持有 |
| [PiState](../../../crates/gupi-pi-runtime/src/runtime.rs) | 应用内多个运行实例、client 与各自的事件消费任务，统一创建与退出 |
| [会话状态](../../../crates/gupi-conversation/src/conversation.rs) | 实例与会话绑定、事件投影、命令及扩展请求回复 |
| [启动与退出](../../../src/features/startup.rs) | 已有配置写入、草稿和布局保存、Pi 连接关闭完成后退出应用 |
| [gpui-tokio](../../../crates/gpui-tokio/src/lib.rs) | GPUI / Tokio 桥接；丢弃 GPUI Task 取消所持有的 future |

运行实例标识、Pi session ID 与 OS PID 各有用途。隐藏窗口或切换展示会话不关闭后台连接；已保存 Pi 命令的变更只影响之后建立的连接。同一会话文件在应用内复用连接。

## 连接与请求

- 版本探测只用于诊断。探测的 15 秒 deadline、16 KiB 输出限制和取消行为独立于长期 RPC 请求，不作为打开会话的前置门槛。
- 创建连接使用 executable、cwd、独立 argv 与环境快照；stdout/stderr 从启动时读取。事件流在 Ready 前即可消费，标准扩展启动请求不会藏在就绪之后。
- 首个带 ID 的 `get_state` 成功响应证明就绪。启动超时、协议失败或提前退出均报告真实失败。
- Client 分配请求 ID，先登记等待者再写入，按 ID 与 command 关联响应；乱序响应、单个 Pi 拒绝和整条连接终止分别处理。
- 丢弃请求 future 只结束本地等待，不隐式 abort、不重放请求、不声明命令未执行。扩展 UI 回复使用原请求 ID 的独立信封。
- `prompt` 成功表示 Pi 已接收或处理本次输入；执行结束仍由运行事件推进。Pi 0.99 的新结果字段尚未全部接入，不能把接受等同于模型任务完成。
- 保留有界请求、写入和事件队列。事件队列满时暂停 stdout 并保持顺序；传输层不设图片、请求或历史帧的固定字节上限。具体容量和错误语义由 crate README 单独维护。

## 关闭与应用退出

正常关闭先拒绝新请求并结算等待者，发送带保留 ID 的 abort。成功回应后释放 stdin，给 Pi 的 EOF / session-shutdown 路径执行机会；继续读取结束事件和观察退出。整个优雅阶段共用 2 秒 deadline，提前完成即返回。

通信失败或超时后，生命周期 owner 取消并等待自己的 pipe 任务结束，释放启用了 `kill_on_drop` 的 Child，交由 Tokio 终止与后台回收。不追加固定 sleep、SIGTERM 或回收等待。`Closed` 表示连接及其 I/O 资源已释放；`CloseReport.status` 仅在确实观察到退出时存在，不保证失败路径已同步回收任意后代进程。

应用退出先停止建立连接并取消尚未交接的启动任务，然后对全部已连接 client 同步发出关闭信号；各 owner 的截止时间并发开始。每个事件消费任务保留到其连接关闭。配置提交、非空草稿和布局保存继续完成，随后才调用 `cx.quit()`。视图隐藏不取消应用持有的退出协调任务。

Windows 使用相同协议和 Child Drop 边界；任意扩展自建进程或 shim 后代的退出不由直接 Child 的释放保证。

## 证据与验证边界

上述归属来自迁移时的当前源码。crate 的现行自动化覆盖为内存协议序列化与解析；真实进程、Shell、文件系统及墙钟超时集成测试已移除。它们不会证明当前安装版 Pi 的互操作或操作系统进程退出。

早期 Pi 0.85.1 的启动、标准扩展往返、双实例和关闭实验保留在 [原仓库固定快照](https://github.com/suxiaoshao/gpui/blob/ca2c45f9bd96d24e06acfb7f445dcd2f6227273d/docs/dev/issue-219/README.md#实现依据与已知限制)，不替代独立仓库当前提交的验证。第一阶段探测依据见 [进程证据](../issue-218/pi-evidence.md)，当前应用能力见 README 的[使用方法](../../../README.zh-CN.md#使用方法)。
