# D2 项目包与标题栏入口：实现与验证

状态：已实现并通过受影响构建、测试及设计师源码 review；可交付试用。GUI 本地包安装/移除闭环已验证；浏览详情等限制见下文。项目更新与过滤规则编辑未纳入本批。

## 实现位置

| 位置 | 当前行为 |
| --- | --- |
| `crates/gupi-conversation-ui/src/home/titlebar.rs` | 右侧 Files 之前的 FolderCog；有 cwd 的非临时会话显示，不依赖已发送消息；派发现有项目设置入口 |
| `crates/gupi-resources/src/pi_resources/packages.rs` | 项目包身份比较、delta 安装/移除限制、规范化 cwd 与实时信任检查；Pi 命令使用真实 agent-dir 和项目 cwd，只允许 install/remove --local |
| `crates/gupi-resources/src/pi_resources/discovery.rs` | 文件扫描项目包与资源；项目 npm 不使用全局目录回退；delta 不冒充独立安装 |
| `crates/gupi-resources/src/resources.rs` | 项目控制器接受包操作；执行后重读目录，包括失败路径 |
| `src/features/settings/resources.rs` | 按 cwd 保留控制器；命令固定原目标；活动页面接收其自身结果，跨作用域完成通知包含原项目完整路径；退出停止全部控制器 |
| `src/features/settings/resources/{packages,browse}.rs` | 项目、delta、继承三类行；项目无更新或资源开关，delta/继承只读；安装按钮明确项目，来源展开按钮保持“从来源安装…” |
| `src/features/settings/layout.rs` | Packages 接入共享作用域及信任入口 |
| `locales/*/main.ftl` | 九种语言的项目入口、安装目标及包关系文案 |

项目包操作先重读 Gupi 已保存信任，再由 Pi 最终判定。命令不传 `--approve` 或 `--no-approve`，保留全局扩展 `project_trust` 拒绝/记住决定的能力。保存设置不会授予信任。

保留的控制器使切换作用域不取消任务，也不会把结果写入新目标。移除确认回调校验原控制器，防止确认期间切换后误删新作用域。退出沿用全局包任务的停止语义，不承诺安装过程可事务回滚。

## 协调者验证（2026-10-10）

- `cargo test -p gupi-resources --locked`：34 通过，覆盖项目目录扫描、身份比较、delta 限制、信任前置检查和项目命令参数。
- `cargo test -p gupi --locked features::`：33 通过。
- `cargo check -p gupi --locked`、`cargo build -p gupi --locked`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check` 通过。最后的标签布局微调再次通过 build/Clippy/fmt。
- 真实 Pi CLI、独立临时 agent-dir/project、本地无脚本包：`install <path> --local` 与 `remove <path> --local` 成功，项目未知字段保留，全局 settings 字节不变。加入返回拒绝的全局 `project_trust` 扩展后，已保存信任项目仍被 Pi 拒绝，未写入包声明。此结果属于 CLI 验证。
- 原生临时应用：未选择项目时标题栏无项目按钮；新建项目对话在首次发送前已有按钮，点击进入准确项目作用域。项目 Packages 列表显示项目/delta/继承来源与替换关系；delta 菜单没有移除，继承菜单只有定位/复制/在全局中编辑；来源表单主按钮显示目标 project-b。Browse 可加载列表。
- 设计师两轮源码 review；修复退出遗漏项目控制器、未知信任误标“未加载”、来源按钮含义丢失及标签/菜单样式问题。

- 用户授权后完成原生 GUI 本地 fixture 安装/移除：安装后项目列表新增条目，移除后消失；仅项目包声明改变，全局 settings 逐字节不变，原有包及 delta 保留，本地来源目录保留。
- GUI 移除验证发现确认弹窗缺少显式页脚按钮、文案误称个人 Pi；已复用现有 `dialog_buttons` 并同步九种语言的作用域文案。修复后 build、Clippy、fmt 通过，实际确认移除与列表刷新通过。

## 验证限制

- 自动审批也拒绝 Browse 行的详情点击，原因是无障碍按钮无名称、无法确定目标；详情中的项目安装按钮未完成运行验证。
- 未做真实 npm/git 网络安装、安装中切换作用域或退出的运行验证；这些路径有源码实现和相应命令契约检查，不能据此声称完整 UI 验收。
- 不读取或修改个人 Pi 配置或凭据；测试应用已退出。

进一步试用发现问题时，仅复测受影响路径。完整 `.pi/` 管理仍有 extensions、themes、路径数组等独立待设计范围，见本目录索引。
