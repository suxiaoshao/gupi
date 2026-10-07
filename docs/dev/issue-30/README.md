# Windows / Linux 窗口菜单布局

状态：Ready。用户已于 2026-10-07 确认菜单置顶、完整展开的两行布局，可按本方案在 Windows 上实施。尚未修改产品代码或进行 Windows / Linux 原生验证。

对应 [Gupi #30](https://github.com/suxiaoshao/gupi/issues/30)。Windows 与 Linux 共用窗口内菜单布局，平台窗口行为继续交由 GPUI Kit 处理。

## 已确定的设计与交接

- 顶部第一行常驻显示 Gupi、会话、编辑、显示、窗口、帮助六个菜单，以及拖动留白和系统窗口按钮。用户明确选择可直接发现命令的完整菜单，不折叠成单个按钮，也不增加菜单显示模式设置。
- 第二行放当前会话/页面的标题与操作，直接连接侧栏和正文；Windows/Linux 第二行不再作为窗口拖动区。macOS 保持现有系统菜单与单行标题栏。
- 使用现有 GPUI Kit 组件，具体组合见下文。常规布局尺寸和平台适配细节在 Windows 实窗中确定，无需重新讨论已确认的两行结构。
- 本轮交付是设计文档与[交互示意](window-menu.html)，示意图中的“推荐 · 菜单置顶”为已确认方案，“当前 · 菜单在下”仅供比较。示意图的内容、窄屏适配及点击反馈不作为产品行为或尺寸验收依据。
- Windows 开发从“开发步骤”开始，先核对真实窗口，再实现并完成受影响检查。文末参考源码已摘要，Windows 接手者无需访问本机 Zed 路径或 Electron 临时解包目录才能开工。

## 目标与当前依据

让全局应用菜单、当前会话操作和窗口控制各有明确位置，保留菜单命令、启用状态、快捷键提示及键盘操作。正文仍是窗口的主要工作区域。

当前源码与固定依赖 `gpui-component 0.7.1` 表明：

- `crates/gupi-conversation-ui/src/chrome.rs` 在非 macOS 平台提供 `AppMenuBar`。每个窗口保留自己的菜单 Entity 和 `MenusChanged` 订阅。
- `home/titlebar.rs` 将侧栏开关、会话标题、会话操作放在 46 逻辑像素的 `TitleBar`；`home.rs` 在其下追加 `h_8()` 菜单行。设置、引导等页面在 `src/features/startup.rs` 使用相同顺序。
- 菜单内容由 `src/app/menus.rs` 统一生成，顶层为 Gupi、会话、编辑、显示、窗口、帮助；`gupi-settings::commands` 提供动作和会话启用状态。
- `AppMenuBar` 保留打开状态、原动作焦点及菜单间切换，内部使用横向滚动；没有公开的紧凑模式、溢出菜单或 Alt/F10 激活接口。不能把新增这些能力当成纯布局调整。
- `TitleBar` 默认高 34 逻辑像素；其窗口按钮宽度使用同一常量，高度随标题栏拉伸。Windows 使用系统控制区域，Linux 根据 CSD/SSD 和窗口管理器支持决定按钮呈现与行为。
- 主窗口当前最小尺寸为 800 × 600，默认尺寸为 960 × 740；实际菜单有九种语言，主题字体或基础 rem 变化也会改变宽度。当前未发现面向用户的独立字号缩放设置。

当前菜单行夹在会话工具栏与正文之间，打断二者关系；菜单起点缺少与上方控件一致的留白，侧栏色带也被全宽菜单行截断。Windows/Linux 的系统控制按钮随 46 高标题栏拉伸。这些是源码上的布局依据，尚未用真实 Windows 截图确认观感。

## 已确认布局

```text
┌ Gupi  会话  编辑  显示  窗口  帮助 ── 拖动留白 ── 最小化 最大化 关闭 ┐
│ 侧栏开关              │ 会话标题 …              导出 刷新 文件面板 │
├───────────────────────┼─────────────────────────────────────────┤
│ Gupi / 搜索           │                                         │
│ 新建会话              │                 正文                    │
│ 项目与会话            │                                         │
└───────────────────────┴─────────────────────────────────────────┘
```

1. **第一行为窗口标题栏。** 使用现有 `TitleBar`，左侧嵌入现有 `AppMenuBar`，右侧保留组件提供的窗口控制。菜单前导边缘与下一行侧栏开关的控件边缘使用共同的布局留白。采用组件默认窗口高度，不另造 32px 窗口按钮规格。
2. **第二行为页面工具栏。** 保留现有侧栏开关、标题、更多、导出、刷新、文件面板入口及其条件。左区宽度继续来自现有面板布局，与下方侧栏相连；设置页面对应为返回和页面标题。Windows/Linux 的这一行作为普通工具栏，不再启动窗口拖动或标题栏双击操作。
3. **主题与边界。** 第一行采用当前主题的平面背景；第二行保持侧栏与主内容各自的背景。第一行不加分隔线，第二行负责与内容之间的细边界，避免重复描边。控件沿用组件菜单的悬停、打开、焦点、禁用和弹层样式。
4. **高度。** 默认字号下，两行约为 34 + 46 = 80 逻辑像素，与当前约 46 + 32 = 78 接近；这只是源码基准估算，不是 Windows 实测尺寸。页面工具栏应随内容和字号保持可用，避免把默认高度变成大字号裁切上限。系统控制几何沿用平台组件。
5. **窗口状态。** 主窗口、临时会话以及设置/引导/恢复页面使用同一顶部次序。主窗口退出、临时窗口隐藏等生命周期不随布局变化。系统窗口标题继续同步当前会话/页面；不在菜单行重复放一份会话标题。

macOS 继续使用系统菜单栏和现有单行会话标题栏。

## 宽度与交互

菜单、可拖动留白、系统控制区是独立布局区域：`[菜单区域：按内容占宽、允许收缩] [拖动留白：吸收剩余宽度、保留最小宽度] [组件系统按钮]`。菜单区域不参与剩余宽度扩张，外壳用 `.occlude()` 阻断其下方的窗口拖动命中，留白保持可拖动。不能让 `AppMenuBar` 的 `size_full()` 占满整行后再覆盖窗口按钮，也不能让整个标题栏外壳遮蔽系统控制。组件的百分比宽度与菜单外壳的实际宽度如何配合，需在 Windows 布局中核实，不能仅凭 `min_w_0()` 宣称解决。留白采用相对布局尺度，具体尺寸在 Windows 实窗中确定。

这个处理参考了 Zed 菜单触发器的 `.occlude()` 与 Electron 的 `no-drag`。已核对固定 `gpui-pre 0.3.8` 的命中测试：`BlockMouse` 会停止收集下层 hitbox，窗口控制命中则从收集结果查找。仅调用 `stop_propagation()` 是事件分派层处理，不等于从原生窗口拖动命中区排除。当前未在 Windows 复现确认缺陷，菜单区域遮蔽是实现方案中的防误触措施，仍须原生验证。

默认尺寸及 800 最小宽度下，优先让六个菜单完整显示，菜单标签不缩写、不换行、不裁字。会话工具栏优先省略标题，保留原有操作入口。大字号或长语言导致空间不足时，先使用组件已有横向溢出能力；必须实测隐藏菜单能否通过鼠标和键盘到达、焦点是否可见。若现有能力不能满足，不以裁切或增加最小窗口宽度掩盖问题，需据具体限制再确定组件层的最小修正。

菜单直接复用既有 Entity、菜单定义和弹层：

- 菜单触发与弹层点击不能误触窗口移动；只有第一行空白区承担标题栏拖动及平台双击行为。
- 保留键盘进入、确认、左右切换、Escape 关闭和焦点恢复；动作仍面向打开菜单前的编辑器/输入框等上下文，不强制聚焦 composer。
- 页面切换、语言/快捷键变化及会话启用状态变化继续走现有菜单刷新机制。
- 不增加汉堡菜单、菜单自动隐藏设置或新的 Alt/F10 行为，不复制菜单状态机。

Zed 提供 F10，Electron 提供 Alt/F10 与助记键；这些能力是参考差距，本次未纳入实现范围。另一个现有差异是 `AppMenuBar` 在没有展开菜单时左右键处理直接返回，Zed 会继续传播；本布局调整不将其误报为新增回归。

Linux CSD 使用组件提供的窗口控制；SSD 下让窗口管理器负责系统标题栏和按钮，应用内仍保留菜单行。不要复制 Windows 控制或额外实现 Linux 窗口管理协议。各模式的最终拖动、右键系统菜单和全屏表现以对应平台验证为准。

## 取舍

| 选择 | 结论 |
| --- | --- |
| 全局菜单/窗口控制在上，会话工具栏在下 | 已确定；层级清晰，侧栏和内容重新连续，菜单有独立宽度预算 |
| 保持当前顺序，只调整菜单行留白、背景和分隔线 | 改动较小，但全局菜单仍隔在会话操作与正文之间 |
| 六个菜单与全部会话操作挤进同一行 | 最小窗口下空间紧张，当前组件没有成熟的紧凑/溢出菜单接口 |
| 单个汉堡入口 | 用户明确不采用；常驻完整菜单更直观 |

## 后续实施顺序

### GPUI Kit 组件选择

以下接口已对照本机 Cargo registry 的 `gpui-component 0.7.1` 源码，应用统一通过 `gpui_kit` 使用。

| 位置 / 职责 | 组件与接口 | 本次用法及边界 |
| --- | --- | --- |
| 第一行窗口框架 | `component::TitleBar`、`TitleBar::window_options()` | 沿用系统控制、装饰模式、移动和平台行为；通过 `Styled` / `ParentElement` 组合菜单与留白 |
| 全局应用菜单 | `component::menu::AppMenuBar::new(cx)`、`reload(cx)` | 复用每窗 Entity 与 `MenusChanged` 订阅；读取现有统一菜单定义，保留焦点上下文与弹层状态 |
| 菜单弹层 | `component::menu::PopupMenu` | 由 `AppMenuBar` 内部构造，已有快捷键、禁用项、子菜单和焦点处理；应用不再组装第二套全局菜单 |
| 页面工具栏布局 | `h_flex()` / `div()`、现有面板宽度 | 复用目前控件树，分配侧栏区、可收缩标题和尾部动作；背景与分隔线由该行拥有 |
| 页面操作 | `component::button::Button`、`ButtonVariants`、`Sizable` | 保留已有 `ghost().small()`、启用条件、加载状态、动作和可访问名称 |
| 会话更多菜单 | `component::menu::DropdownMenu` 扩展 trait | 沿用 Button 的 `dropdown_menu` 与现有 `session_menu`，不拿它替代顶层 `AppMenuBar` |
| 标题与图标提示 | `component::tooltip::Tooltip`、现有 tooltip 扩展 | 保留长标题完整内容、图标动作名称和快捷键提示 |
| 图标 / 局部分隔 | `component::Icon`、`component::separator::Separator` | 复用现有图标及侧栏收起后的短竖线，不新增图标风格 |
| 主题 | `component::ActiveTheme` | 使用当前主题背景、侧栏色与边框 token；应用间距使用既有相对尺度 |

已评估 `component::toolbar::{Toolbar, ToolbarGroup}`：它们提供工具栏语义、方向键游走焦点和尺寸传播；`Toolbar::child` 还会把 Button 准备为紧凑 ghost 控件。这会改变当前页面标题行的键盘和几何契约，因此本次保持普通布局容器；文中的“工具栏”指页面区域，不隐含必须迁移到 `Toolbar`。`AppMenuBar` 也不应再套一层会捕获方向键的 `Toolbar`。

当前组件限制：`AppMenuBar` 没有公开密度/间距 builder、菜单压缩/溢出入口或 Alt/F10 激活 API；内部菜单按钮不能由 Gupi 直接单独定制。`TitleBar` 的系统按钮同样由组件私有实现。先使用现有组合接口；若 Windows 命中测试、放大字号或溢出验证显示组件契约不足，应在拥有该行为的组件层修正，不能在应用里复制整份菜单/窗口实现。

### 开发步骤

1. 在 Windows 打开当前版本，记录主窗口、设置和临时窗口的实际顶部及现有键盘路径；确认 DPI、字号、主题和窗口尺寸。
2. 在 `chrome.rs` 分开菜单内容与菜单行外壳，保留每窗 Entity/订阅；组合非 macOS 窗口标题栏与菜单。第一行不能直接沿用当前 `chrome::title_bar` 的 `.h(HEIGHT)`（46）；使用组件默认高度，并明确设置 `.border_b_0()` 去掉组件默认下边框。复用现有 `TitleBar`，不复制平台实现或升级依赖。
3. 在 `home/titlebar.rs`、`home.rs` 将非 macOS 会话行改成普通工具栏；在 `startup.rs` 同步页面标题行。保持面板宽度来源、会话状态与动作路由。临时窗口的 `on_close_window` 隐藏回调随 `TitleBar` 一起移到第一行；该组件回调仅在 Linux 生效，Windows 继续使用既有原生关闭处理。
4. 完成受影响构建、关键回归和真实 Windows 启动检查，交付试用。若发现固定组件的命中测试或溢出契约不足，记录具体失败，再决定组件修正范围。

本功能不需要新增 Pi RPC、持久化配置或公共业务接口。现有 [统一标题栏说明](../issue-220/titlebar.md)记录当前实现；开发落地后再同步其非 macOS 布局描述。

## 验证与交付依据

Windows 实窗重点：

- 菜单打开、菜单间切换、禁用项、快捷键、Escape 与原焦点恢复；切换语言及当前会话后菜单状态正确。
- 第一行空白拖动、双击最大化/恢复、最大化后拖动恢复；窗口按钮和 Win11 Snap 布局、系统菜单保留原平台行为。菜单及跨到第二行的弹层不会拖动窗口。
- 800 × 600 与默认窗口，侧栏开合、长会话标题及较长菜单语言；系统控制、菜单和留白不重叠。增大主题基础 rem 后第一行按钮不被固定标题栏高度裁切，所有菜单仍可到达；必要时让行高容纳内容，保留组件窗口控制实现，同时复查固定宽度窗口按钮被拉高后的比例和命中效果。
- 100% / 125% / 150% / 200% 显示缩放下的按钮命中、顶部裁切、边缘对齐和单一分隔线；有多显示器时补查跨 DPI 移动。
- 主窗口、设置/引导及临时窗口，浅色/深色主题；确认关闭主窗口与隐藏临时窗口的既有差别保留。

代码检查选择 `cargo fmt --all -- --check`、受影响的 `gupi-conversation-ui` 与根包构建/关键回归，按实际 CI 执行必要 Clippy；不为仅改文档运行构建，也不自动扩展为全量平台验收。

Linux 复用代码需保留平台编译正确性；实际 CSD/SSD 的显示和操作单独报告，Windows 通过不能代替 Linux 验证。目前只有源码与设计审阅，尚无原生运行、构建或测试结论。

## 本地参考实现

参考用于检验布局选择，源码观察和本项目的原生验证分开记录。没有修改参考仓库或已安装应用。

### Zed

2026-10-07 检查本地 `/Users/sushao/Documents/code/zed`，HEAD 为 `c770f3b182bc98fff2ec79484ab3d936a6a2f493`。路径与行号均对应该快照。

| 源码位置 | 源码观察 | 对 Gupi 的启示 |
| --- | --- | --- |
| `crates/title_bar/src/title_bar.rs:412` | `show_menus=true` 时第一行为 `PlatformTitleBar + ApplicationMenu`，第二行为普通 `h_flex` 页面操作，不标记窗口拖动区域 | 与推荐两行结构及第二行不拖动一致 |
| `assets/settings/default.json:628`、`crates/title_bar/src/title_bar.rs:318`、`crates/title_bar/src/application_menu.rs:176` | 默认 `show_menus=false`；单菜单图标展开为完整菜单时，会让出项目/分支标题空间，可访问模式另行处理 | 单行模式依赖专用状态和布局能力，Gupi 当前 `AppMenuBar` 没有等价接口；不照搬该模式 |
| `crates/title_bar/src/application_menu.rs:185`、`:220`、`:133` | 菜单触发区域使用 `.occlude()`；打开菜单时保留原焦点作为动作上下文 | 菜单区排除拖动命中与保留动作目标均是实质交互要求 |
| `crates/ui/src/utils/constants.rs:17`、`:21` | Windows 标题栏固定 32；其他平台为 `max(1.75rem, 34)` | 平台高度不同有明确原因；Gupi 先沿用 Kit 默认 34，不复制其数值或原生按钮实现 |
| `crates/platform_title_bar/src/platform_title_bar.rs:218` | 平台标题栏处理拖动、双击、Linux 窗口菜单及装饰模式 | 保留 Kit 的平台职责；不在页面工具栏重写这一套行为 |
| `assets/keymaps/default-windows.json:662`、`assets/keymaps/default-linux.json:669`、`crates/title_bar/src/application_menu.rs:273` | F10 激活，菜单内左右切换；未打开菜单时左右键继续传播 | 记录与 Kit 的能力差距，保持本次布局范围 |

Zed 的 Linux 标题栏还处理窗口管理器按钮布局、失焦色和装饰细节；这些不因参考研究自动进入本次开发范围。结论：保留推荐两行方案，补充菜单区域的原生命中边界。

### Codex / ChatGPT Electron

2026-10-07 只读查看 `/Applications/ChatGPT.app/Contents/Resources/app.asar`，本机应用版本为 `26.930.61225`（build `13232`）。仅将相关 JS/CSS 提取到 `/tmp/gupi-issue-30-electron` 供分析，不将第三方 bundle 放入项目。

| 包内文件 / 可检索标识 | 源码观察 | 对 Gupi 的启示 |
| --- | --- | --- |
| `.vite/build/main-C7cfj__D.js`：`primary`、`quickChat`、`titleBarOverlay`、`j9` | Windows/Linux 使用 `titleBarStyle: hidden` 与原生窗口控制 overlay；overlay 高度由 `round(36 * zoom)` 计算，macOS 使用 `hiddenInset` | 平台窗口控制与应用布局分工；36 是该版本 Electron 参数，不照搬为 GPUI 尺寸 |
| `webview/assets/app-initial-341eb5dd9ad5.css`：`ApplicationMenuTopBar` | 独立顶部菜单行使用小工具栏高度、左右安全留白，背景可拖动，内部 button 标为 `no-drag` | 支持全局菜单/系统控制在最上方、页面操作在下方的层级 |
| `webview/assets/app-initial-69cd8dbddec5.js`：`ApplicationMenuTopBar`、`windowControlsOverlay` | 单独渲染菜单行；通过 `getTitlebarAreaRect()`、`geometrychange` 和 resize 获取控制区留白 | 窗口控制区应由真实平台几何决定，不能靠覆盖层猜测位置 |
| `webview/assets/app-shared-6fb15e58cd7f.css`：`data-app-shell-application-menu-bar=true` | 此模式将下层 `.draggable` 取消为 `no-drag` | 推荐方案中第二行改为普通页面操作区有明确参考 |
| `app-initial-69cd8dbddec5.js`：`windowsMenuBar`、`applicationMenu.getSnapshot`、`F10` | File/Edit/View/Help 四个顶层菜单通过 bridge 取快照和执行动作；另行实现 Alt/F10、助记键与原焦点恢复 | 菜单定义与视觉呈现分离值得复用；四菜单宽度不能证明 Gupi 六菜单可用，其自建键盘能力也不等于 GPUI Kit 已具备 |

这些是 macOS 安装包内保留的平台分支证据，没有运行该应用的 Windows/Linux 版本。设计采用其结构和职责划分，保留 Gupi 自己的菜单项目及组件行为。用户对折叠菜单直观性的评价记录为 Gupi 的产品选择，不据此推断其他版本或平台上 Codex 的默认菜单呈现。
