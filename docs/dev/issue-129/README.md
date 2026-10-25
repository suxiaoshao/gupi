# #129 从 Dock/Finder 启动时传递登录 Shell 环境

状态：已实现。核对基线：Gupi main `315a6b0`；本地 Pi `36be9c612`（1.4.0）、Zed `79fe6a645f`。

## 原行为与参考

`crates/gupi-pi-runtime/src/shell_path.rs` 已在 macOS GUI 启动时运行用户登录 Shell，但 helper 只返回 PATH；`environment.rs:16` 也只向 Pi 启动参数补 PATH。用户在 Shell 中导出的代理、API key、Node 配置等变量没有随这次捕获一起传入。

Zed `crates/util/src/util.rs:393` 的 `load_login_shell_environment` 在用户家目录捕获完整登录 Shell 环境，跳过 `SHLVL`，避免捕获用 Shell 的层级污染后续进程。Gupi 沿用这一思路，扩展已有捕获入口，不另建环境来源。

## 当前设计

- 将现有 PATH helper 的有边界字节帧扩展为完整的已导出环境变量集合，保留启动脚本输出隔离、非 UTF-8 值和现有超时/清理行为。跳过 `SHLVL` 和 Gupi 捕获 helper 自用变量，不把变量名限制在代理或 PATH 白名单。
- Environment 持有捕获结果，启动 Pi 时使用；普通、临时与待命连接复用同一入口。沿用现有 GUI 启动判定、环境缓存与失败处理，终端启动保留继承环境，不新增监听或自动重启。
- 捕获对应在用户家目录启动登录 Shell 得到的导出环境，使 GUI 启动的 Pi 获得同样的 Shell 配置；不承诺复制某个已打开终端中临时 export 或进入项目后才生效的环境。Pi 的 cwd 仍由会话决定。
- 在子进程启动环境中应用捕获结果，不修改 Gupi 进程全局环境。代理、API key、Node 配置等由用户 Shell 决定；环境变量与 `httpProxy` 的处理继续由 Pi 自己负责（`packages/coding-agent/src/core/http-dispatcher.ts:45`），Gupi 不读取或合并 Pi 配置来决定优先级。
- 实现验证围绕既有捕获与子进程入口：完整变量传递、排除捕获专用变量、噪声/字节值保留，以及 Dock/Finder 启动。环境可能含凭据，不打印捕获内容或变量值。

## 实现

捕获入口为 `gupi-pi-runtime/src/shell_environment.rs`，内部 helper 使用 `--gupi-print-shell-environment`。NUL 字节帧携带完整导出变量，排除 `SHLVL` 与 helper 自用变量；`Environment` 缓存快照，现有 Pi 启动、探测、临时及待命入口共用它。失败保留继承环境，不修改 Gupi 全局环境，不记录变量值。

已有帧与缓存回归扩展为多变量、非 UTF-8、噪声、空值和 helper 排除。原生 Dock/Finder 的真实配置未额外操作。

## 本轮验证

macOS 本地受影响五包 fmt、Clippy（all-targets、-D warnings）与 test 全通过；runtime 4 项通过，1 项既有 installed-Pi 回归按原设置忽略。本轮未追加真实 Dock/Finder 启动验证。
