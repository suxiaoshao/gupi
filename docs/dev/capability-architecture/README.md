# Gupi 能力架构

Gupi 已按六个内部 crate 拆分。根包负责应用组合、原生窗口和退出；能力 crate 不依赖根包。配置格式、会话数据和 Pi RPC 契约保持不变。

拆包后的接口封装与依赖收敛见[接口与依赖收敛](refinement.md)，同时记录官方 skill 核对结果与文档冲突。

## 约定与依据

通用架构、状态所有权和文件组织遵循 [GPUI Kit Coding Guides](https://gpui-kit.com/docs/coding-guides/)。官方 skill 通过 `npx skills` 安装维护；项目 skill 只补充产品边界和入口，不维护另一套框架规则。依赖 API 以 Cargo.lock 为准，本次没有升级外部依赖。

拆分参考本地拉取的 Zed 官方 main `a84689073d296dfd39987bc7dd478e43ef76d83a`（2026-10-03）：[terminal_view](https://github.com/zed-industries/zed/blob/a84689073d296dfd39987bc7dd478e43ef76d83a/crates/terminal_view/src/terminal_view.rs#L131) 持有终端 Entity，[agent_ui](https://github.com/zed-industries/zed/blob/a84689073d296dfd39987bc7dd478e43ef76d83a/crates/agent_ui/Cargo.toml) 与 [settings_ui](https://github.com/zed-industries/zed/blob/a84689073d296dfd39987bc7dd478e43ef76d83a/crates/settings_ui/Cargo.toml) 依赖各自模型。采用能力模型与复杂 UI 分离的方式；模型允许依赖 GPUI 来持有 Entity、订阅和 Task。

## 代码归属

| owner | 职责 |
| --- | --- |
| `gupi-pi-runtime` | Pi 实例、连接事件、启动/关闭、探测、执行环境和 Shell PATH helper；底层传输继续复用 `pi-rpc` |
| `gupi-resources` | Pi 资源发现与编辑、资源操作 owner、输入资源标识和参数引用；提供统一的目录发现、文件读取和原子持久化支持 |
| `gupi-updates` | 版本查询、缓存、检查/安装状态和平台安装驱动；拥有 Objective-C bridge 的构建及 Windows helper 协议 |
| `gupi-settings` | 配置与提交、快捷键及任务配置、提醒策略、窗口布局、语言/主题/图标偏好、共享动作定义；集中提供应用展示资源 |
| `gupi-conversation` | 会话 Entity、历史投影、消息/工具语义、草稿、附件、提交、队列和取消；拥有会话专用测试 |
| `gupi-conversation-ui` | HomeView、输入、消息、工具详情、图片预览、导航、命令面板和共享窗口栏；持有会话 Entity |
| 根包 `src/app` | 实例、日志、菜单、Tray、通知投递、全局快捷任务、临时窗口、原生更新退出协调与应用寿命 |
| 根包 `src/features` | 启动组合、设置界面、临时窗口组合和更新弹窗 |
| 根包 `src/components` | 应用共用的恢复控件 |

`src/state`、`src/foundation` 的实现已全部迁出，旧模块入口已删除。附件、历史和工具语义归会话能力，没有分别建包；设置和更新界面规模有限，保留在根包。`pi-rpc`、`gpui-tokio`、`gpui-lucide`、`window-ext`、`xtask` 保持原有独立职责，不增设通用 core/common crate。

## 依赖与公共接口

核心方向如下；所有边均由 Cargo 检查，不存在能力 crate 对根包的反向依赖：

- `conversation-ui → conversation / settings / resources`；`conversation → pi-runtime / resources / pi-rpc`。
- `settings → resources / pi-rpc`：配置持久化和探测失败的本地化映射；安装期间的提交互斥由根包注入，updates 只作为 settings 的测试依赖。
- updates 接收应用提供的 Windows 安装日志路径；`resources → pi-runtime`：资源扫描与包管理使用相同的执行环境。
- 根包组装全部能力，并在创建配置和会话 owner 前调用 `init_capability_hosts`。

三个 `host::Host` 是应用注入的同步函数接口，不持有 View 强引用：

| 接口 | 由根包实现 | 保持的时序 |
| --- | --- | --- |
| settings prepare/apply shortcuts | 全局快捷键准备与应用 | 准备失败阻止写入；完成后按已保存或恢复的配置应用 |
| settings refresh menus | 原生菜单重建 | 快捷键或活动窗口命令变化立即更新菜单 |
| conversation attach | 订阅会话通知事件 | owner 创建后延迟接入，维持原有一次订阅行为 |
| conversation cancel preparation | 取消快捷任务准备 | 关闭会话时同步取消，防止准备完成后继续发送 |
| conversation-ui present/hide/paste | 通知可见性、隐藏临时窗口、回填 | UI 仅发起对应动作，平台对象和寿命留在根包 |

host 必须显式安装。独立模型/UI 测试使用 headless 实现；应用集成测试使用与正式启动相同的组装入口。原生 updater 通过事件交给根包适配器；适配器检查配置状态，完成有序退出后才继续安装，驱动不调用根包私有函数。

Pi 实例 ID 不透明，探测与资源操作只提供只读查询和操作方法。Session 和 ConversationState 的字段私有，UI 通过只读投影与语义动作访问；附件任务与快捷任务准备结果由模型安装。配置通过 ConfigReader 提供只读操作投影，更新准入查询由应用注入。跨边界回归所需的消息回放和夹具构造放在 `test-support` feature 下，不为测试公开 transcript 内部类型。

## 版本、资源与验证入口

根产品及本次新增的五个能力包使用 `[workspace.package].version`，确保更新比较、请求 User-Agent 和 Windows updater 取得产品版本。`xtask` 支持读取根 manifest 的继承版本。`bundled` 转发到 settings/updates；`performance` 转发到 runtime/conversation/conversation-ui，应用仍持有 trace writer，UI crate 仅记录渲染区段。

图片、语言文件和测试回放仍使用仓库中的唯一资源源文件，include 路径随模块迁移更新。测试按实现归属迁移；跨窗口和原生菜单/通知测试留在根包。CI 已使用 `--workspace`，因此自动包含新包。

本地开发继续使用 Nix；当前 `target` 链接的缓存目录不可用，验证使用 `CARGO_TARGET_DIR=/tmp/gupi-architecture-target`，不修改用户缓存配置。发行包仍必须在 Nix 外构建。

2026-10-03，macOS Apple Silicon 的本轮结果：

- Nix 开发构建通过，全部受影响 crate、根包及 xtask 的 200 项独立测试通过（包含新增配置预检失败/恢复回归、版本继承及原有跨窗口回归）。
- 全目标、全 feature 的 Clippy 通过；覆盖 `bundled`、`performance` 与测试入口。
- `cargo fmt --all -- --check`、`git diff --check`、cargo-shear 和本轮修改文档的本地链接检查通过。Cargo.lock 外部包的名称、版本和来源集合未变。
- 最新开发程序使用独立 gallery 配置/日志/数据和 Pi 夹具：主窗口、设置、Pi 探测、project-a 会话输入区及临时对话正常打开；正常退出码为 0，日志包含 `managed quit completed`。Shell PATH helper 实际执行返回预期的 NUL 分帧数据。

未在 Windows/Linux 运行原生界面，也未构建或安装发行包。截图功能未混入此次结构改造。
