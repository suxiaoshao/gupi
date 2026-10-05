# 项目目录文件浏览（Gupi #31）

状态：目录浏览与会话/源码工作区已实现。布局、焦点、响应式与分栏偏好见 [会话与源码工作区](ui-redesign.md)。本文记录文件浏览、读取边界与语言支持。

目标：在会话右侧查看当前项目的目录和文件，通过展开、收起了解项目结构。
需求来源：[Gupi #31](https://github.com/suxiaoshao/gupi/issues/31)。

## 首期范围

- 浏览当前会话 `SessionInfo.cwd` 对应的本地目录；明确显示根目录。
- 展开、收起目录，选择文件或目录，查看其完整路径。
- 按需读取子目录，支持手动刷新、空目录和读取失败反馈。
- 用只读 Editor 预览文本文件，支持语法高亮、行号、选择、复制和查找。
- 与会话历史共用右侧面板，沿用现有宽度调整及窄窗口覆盖模式。

单击普通文件或按 Enter 打开工作区内的单文件快照。再次打开同一文件保留阅读位置、换行和查找；“重新读取”才更新快照。预览限文本源码，不提供编辑和保存。图片、PDF、音视频、二进制与超限文件展示无法预览的原因，不自动调用外部应用。

编辑、新建、重命名、删除、拖放、Git 状态、搜索、自动目录监听、多根目录和远程目录均不纳入首期。不修改 Pi 配置或会话文件，不与 #33 的本轮改动查看绑定。

## 当前实现与 Zed 参考

Gupi 基线为 `9f5e723`：

- [HomeView](../../../crates/gupi-conversation-ui/src/home.rs) 组合会话与右侧历史；[panes.rs](../../../crates/gupi-conversation-ui/src/home/panes.rs) 已负责面板宽度、拖动与窄窗口覆盖。
- [history.rs](../../../crates/gupi-conversation-ui/src/home/history.rs) 拥有历史列表、树视图和阅读选项；文件浏览不改变这些语义。
- [命令面板文件候选](../../../crates/gupi-conversation-ui/src/command_palette/files.rs) 当前使用 `ignore::WalkBuilder` 递归扫描供 `@` 搜索，默认过滤行为和完整扫描均不适合本功能。保留其原有用途，不把目录浏览接到这次扫描上。

参考本地 Zed 源码 `c770f3b182bc98fff2ec79484ab3d936a6a2f493`，并非对最新上游版本的声明：

| Zed 的做法 | Gupi 的取舍 |
| --- | --- |
| [按 worktree 保存展开项，左右键展开及收起](https://github.com/zed-industries/zed/blob/c770f3b182bc98fff2ec79484ab3d936a6a2f493/crates/project_panel/src/project_panel.rs#L1468) | 当前根目录拥有展开状态；切换根目录重置，只保留当前项目的浏览状态 |
| [行呈现区分层级、图标、选中与悬停](https://github.com/zed-industries/zed/blob/c770f3b182bc98fff2ec79484ab3d936a6a2f493/crates/project_panel/src/project_panel.rs#L5920) | 固定缩进、目录/文件图标、名称和选中状态；省略诊断、Git 标记、文件类型图标库 |
| [用 uniform_list 渲染可见行](https://github.com/zed-industries/zed/blob/c770f3b182bc98fff2ec79484ab3d936a6a2f493/crates/project_panel/src/project_panel.rs#L7507) | 使用 GPUI Kit 的虚拟 List，数据层只投影已展开目录 |

借鉴交互和可见行投影，不复制 Zed 的 Project/Worktree 系统、目录链压缩、文件操作或编辑器联动。

## 界面与正文预览

右侧面板使用真正的“历史 / 文件”页签，下面是当前页签工具栏和导航列表。文件工具栏显示项目名称、刷新及更多菜单，完整路径放在提示和复制路径动作中。目录单击展开/收起；上下键移动选择，左右键导航层级，Enter 打开普通文件。符号链接仅显示，不跟随。

正文采用 GPUI Kit 只读 Editor，提供行号、高亮、选择、复制、查找和手动换行。Markdown 文件也显示源码，不渲染正文或加载文内资源。快照持有在独立工作区区域中，与导航显隐、历史页签独立；宽窗口会话与源码并排，窄窗口提供会话/文件切换，输入区保留在底部。

HomeView 拥有一个源码槽。新文件替换旧文件；同 cwd 的会话切换保留快照，cwd 改变、无会话或主动关闭时释放。重新读取失败保留旧快照并显示原因。加载、错误及超限反馈位于正文区域；异步读取不能抢走已移到输入框的焦点。详细焦点契约见工作区文档。

### 文件边界

首期最多读取 **1 MiB**，按上限加一字节的方式读取以识别超限，不能仅依赖读取前的文件大小。仅打开普通文件，不读设备、FIFO 或符号链接目标。支持严格 UTF-8（允许 UTF-8 BOM）；含 NUL、非 UTF-8 或已知二进制格式显示“此文件暂不支持文本预览”，不做有损解码、不把二进制伪装为源码。

超过 **20,000 行**的文件提示超出预览范围；单行超过 **16 KiB**时使用不高亮的纯文本只读 Editor，并说明原因。这些是初始产品上限，不是组件性能保证。限制检查在后台读取阶段完成。首期不做尾部读取、分页或截断后伪装成完整文件。

### tree-sitter 与文件语言识别

根包和 `gupi-conversation-ui` 已启用 `gpui-kit` 的 **`tree-sitter-languages`**。该集合已经包含所有当前内置 grammar；JSON 由基础 `tree-sitter` feature 带入，Markdown 同时含 block/inline grammar。因此本功能**无需增加 tree-sitter 依赖或 Cargo feature**，也不借本次功能裁剪集合，避免影响现有回答代码块的语言覆盖。

实施需要增加的是文件名/扩展名到语言 ID 的映射。首批重点验证以下语言，其他已有语言继续复用内置注册表，未知语言回退纯文本：

| 文件类型 | 语言 ID / 已有 feature | 识别重点 |
| --- | --- | --- |
| Rust | `rust` / `tree-sitter-rust` | `.rs` |
| JSON | `json` / 基础 `tree-sitter` | `.json`；JSONC 不强行当作完整支持 |
| TOML | `toml` / `tree-sitter-toml` | `.toml`、Cargo.lock |
| YAML | `yaml` / `tree-sitter-yaml` | `.yaml`、`.yml` |
| Markdown | `markdown` / `tree-sitter-markdown` | `.md`、`.markdown`；inline grammar 自动随 feature 包含 |
| Shell | `bash` / `tree-sitter-bash` | `.sh`、`.bash`、`.zsh` 采用 Bash 高亮近似，不承诺 Zsh 语法完全匹配 |
| JavaScript / JSX | `javascript` / `tree-sitter-javascript` | `.js`、`.mjs`、`.cjs`、`.jsx`；现有 `jsdoc` 支持随集合保留 |
| TypeScript / TSX | `typescript`、`tsx` / 对应 feature | `.ts` 与 `.tsx` 分开映射；`.mts`、`.cts` 归 TypeScript |
| HTML / CSS | `html`、`css` / 对应 feature | `.html`、`.htm`、`.css`；保留已有嵌入语言支持 |
| Python / Go | `python`、`go` / 对应 feature | `.py`、`.pyi`、`.go` |
| C / C++ | `c`、`cpp` / 对应 feature | `.c` 与 `.cpp/.cc/.cxx/.hpp`；`.h` 首期默认 C |
| SQL / Diff / Make | `sql`、`diff`、`make` / 对应 feature | `.sql`、`.diff/.patch`、Makefile/GNUmakefile |
| 普通文本 | `text` | `.txt`、无已知映射的 UTF-8 文件 |

先按特定文件名匹配，再按扩展名匹配；路径展示与语言识别分开，不仅把扩展名直接传给 Editor。首期不加入内容猜测或项目语言服务器。

**当前上游限制：**0.7.1 的 Swift、C#、GraphQL、Proto、CMake 虽提供 grammar，但内置高亮查询为空，不能承诺可见语法着色；首期按纯文本预览。Nix、Vue、PowerShell 没有对应内置 language feature，同样回退纯文本。若要求这些语言高亮，需要另行接入 grammar/query 并核对许可证及版本兼容性，本次设计不默认升级依赖或复制一套高亮系统。

Editor 和消息 Markdown 共用现有语言服务及主题语法颜色；不在文件预览里另造配色。实施验证区分“可识别语言”“可解析 grammar”和“有可见高亮”三件事。

## 目录规则与反馈

| 情况 | 首期行为 |
| --- | --- |
| 首次打开 | 后台读取根目录的直接子项；子目录默认收起 |
| 展开未读取目录 | 在该目录下显示“正在读取…”；完成后替换成实际子项 |
| 空目录 | 展开后显示“空目录”，仍保留目录图标和收起能力 |
| 根目录读取失败 | 显示读取失败行，悬停显示路径与系统原因；点击该行或刷新重试 |
| 子目录读取失败 | 仅在该目录下显示错误与重试，其他目录继续可用 |
| 部分项读取失败 | 保留成功读取项，并提示本目录有项目未能读取，允许重试；不把部分结果标成完整结果 |
| 没有当前会话 | 显示“选择会话以浏览项目文件”，不读取应用进程目录 |
| 文件已被外部修改 | 直到手动刷新或首次读取该子目录时才反映变化 |

目录排在文件之前，各组按名称稳定排序，同名或大小写等价时用原始名称作次级排序。显示隐藏项及被 Git 忽略的项，不套用 `@` 候选的过滤规则；`node_modules`、`target` 等不会自动递归读取。

符号链接显示为链接项及其路径，首期不跟随展开；损坏链接保留可识别条目。不通过 `canonicalize` 把根目录替换成另一个展示路径。原始路径始终保留为操作及身份依据，名称的有损显示不作为条目 ID。

“刷新”只重新读取根目录和当前展开且可见的目录，不遍历收起子树。读取成功后保留仍存在的选择和展开项，删除项移到最近仍存在的父目录；收起子树的旧缓存失效，下一次展开重新读取。刷新期间同一根目录已有内容继续可见；根目录变化时立即清空旧内容。

## 所有权、异步与组件选择

在 `gupi-conversation-ui` 内增加项目文件面板模块及其 retained Entity，由 HomeView 持有。该 Entity 拥有当前根目录、目录结果、异步任务及目录投影；标准 ListState 拥有键盘选择和滚动。工作区源码 Entity 持有独立 EditorState 和文件读取任务，HomeView 负责其生命周期。HomeView 负责页签、面板显隐及将当前会话 cwd 传给面板。不新增 crate，也不把文件浏览状态放进 ConversationState 或 Pi RPC。

简单的单层目录枚举放在本功能模块，使用现有后台执行方式运行文件系统 I/O。当前没有第二个调用者，不为此新建通用文件服务，也不重构命令面板扫描。

已核对锁定的 GPUI Kit 0.7.1 源码：Tree 支持展开/收起事件和虚拟渲染，但 `TreeItem::is_folder()` 以子项非空判断目录，尚无独立目录类型或按需加载接口。首期用现有 List/ListDelegate 投影“路径、类型、层级、展开状态”的可见行，目录读取与树形导航由面板拥有；不使用虚假子项冒充目录，不升级依赖。List 复用标准选择、焦点及虚拟滚动，目录左右键行为由功能动作补齐。

- 每次根目录变化或刷新生成请求代次，结果带根目录、目录路径和代次回传；过期结果不得安装。切项目不会短暂展示上一项目条目。
- 同一 cwd 的会话间切换保留浏览位置；cwd 改变时重置缓存、选择、滚动和展开项。无当前会话时清空。切回旧项目重新读取，首期不维护多项目缓存。
- 目录加载状态明确区分未读、读取中、已读和失败；对同目录去重请求。收起期间完成的有效结果可缓存，但不重新展开。
- 隐藏面板后不发起新读取；当前有限请求可完成，关闭窗口释放任务。弱 Entity 与请求代次共同防止旧任务影响新视图。
- 枚举、排序及可见行投影在事件或结果安装阶段完成，不在 render 递归扫描、不逐帧重建完整树；对大目录结果的投影避免在 UI 线程做完整排序或大量克隆。
- 目录缓存和页签只在当前窗口寿命内保留。复用现有 `history_width` 存储右侧宽度，新增分栏比例单独存储，避免旧布局格式迁移。
- 可访问名称包含条目名称及目录展开状态，加载/错误有文字反馈；所有新增文案按现有 Fluent 契约同步九种语言。

## 实现与验证

实现位于 `home/files.rs`、`home/files/directory.rs`、`home/source.rs`、`home/workspace.rs` 和 `home/panes.rs`；HomeView 接入标题栏与命令面板，九种语言同步维护。分栏偏好位于 `gupi-settings/source_split.rs`。没有新增依赖或升级 GPUI revision。

验证结果和实际平台边界集中记录在 [工作区文档](ui-redesign.md#实施与验证)。语言核对中 Rust、TOML、Markdown、TSX 已观察到着色；TSX 示例的 JSX 标签/属性未完整着色，沿用组件 query，不承诺所有 token 都有高亮。
