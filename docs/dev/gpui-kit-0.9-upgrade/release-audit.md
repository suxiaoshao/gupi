# 0.8.1 / 0.9.0 release 逐条审阅

[返回升级计划](README.md)。89 条（47 + 42），每条同时判断 Gupi 与共享仓库。随升级表示使用上游行为修复，不新增功能；保留表示没有语义等价替代或当前没有对应使用点。仅源码审查，待实现及 worker 验证。版本摘要中的 API 破坏、Shell publication、主题同步另见主计划。New Contributors 是署名，不是额外功能条目。

历史理由与当前代码的重新判断见 [45 项妥协审计](compromise-audit.md)，包括认证焦点、MessageScroller inset、Form 快照及不存在的 Combobox suppression 表述纠正。
## 0.8.1

| 官方条目 | 结论 | Gupi 去留与理由 | 共享 gpui 去留与理由 |
| --- | --- | --- | --- |
| [#3608 typos: Ignore the two release-notes contributor handles](https://github.com/longbridge/gpui-kit/pull/3608) | 保留 | 拼写词典维护，无应用适配可删。 | 只承接上游维护，不复制 contributor/葡语白名单。 |
| [#3610 dock: Read the legacy empty `StackPanel` as an empty split](https://github.com/longbridge/gpui-kit/pull/3610) | 保留 | 工作区是自有 pane 模型，不使用 Dock 的 StackPanel 持久化。 | 未发现旧 Dock JSON 迁移需求；不新增 Dock。 |
| [#3585 input: align accessibility semantics with editor focus](https://github.com/longbridge/gpui-kit/pull/3585) | 随升级 | 现有 Input/Textarea/Editor 自动获得正确焦点语义，保留 recorder 的业务焦点。 | Form 输入与 HTTP Editor 受益；保留 Form owner。 |
| [#3603 dialog: Animate the exit to match the entrance](https://github.com/longbridge/gpui-kit/pull/3603) | 随升级 | Dialog 退出动画交给 Kit，不重写关闭/确认生命周期。 | 共享应用 Dialog 同样承接。 |
| [#3604 input: Measure the horizontal scroll extent from laid-out lines](https://github.com/longbridge/gpui-kit/pull/3604) | 随升级 | Files/Shell 的长行及 Tab 对齐自动修复；不改文本或 Tab 策略。 | HTTP 可编辑代码受益；保留 tree-sitter 的已测性能选择。 |
| [#3606 button: preserve Tab traversal when the focused control becomes disabled](https://github.com/longbridge/gpui-kit/pull/3606) | 随升级 | 保存中按钮禁用仍可 Tab 离开；不删除业务禁用条件。 | Form 示例与共享应用按钮承接。 |
| [#3617 table: Apply Column::text_right and text_center in DataTable](https://github.com/longbridge/gpui-kit/pull/3617) | 随升级 | 设置自有行不是 DataTable，不为修复强迁表格。 | Feiwen 现有 DataTable 列对齐受益；保留 bounded fit_column。 |
| [#3618 diff: Let the application supply the syntax highlighter](https://github.com/longbridge/gpui-kit/pull/3618) | 保留 | Diff 已用 syntect；官方自定义 highlighter 不替代文本转 patch 领域适配。 | HTTP 无 Diff 高亮适配可删；不更换 tree-sitter。 |
| [#3621 scroll: Expose the last horizontal strip scroll step a mask claimed](https://github.com/longbridge/gpui-kit/pull/3621) | 保留 | 工作区 TabBar/ScrollableMask 已负责滚动；无理由新增 scroll claim 路由。 | gpui-heatmap vertical_wheel 已在 #261 采用；无需另写 edge 路由。 |
| [#3607 story: Add an application-owned completion menu to the Input story](https://github.com/longbridge/gpui-kit/pull/3607) | 保留 | Input story 的补全示例不是资源/命令目录；保留现有补全 owner。 | 只更新官方 skill/示例知识，不复制 story。 |
| [#3613 scroll: Scroll content past a scrollable's maximum size](https://github.com/longbridge/gpui-kit/pull/3613) | 随升级 | 嵌套滚动与大偏移修复自动生效，保留 transcript 锚点及各滚动 owner。 | HTTP 两编辑器/Feiwen 列表受益；不补应用 bounce。 |
| [#3623 input: Ask for the virtual keyboard when an input is tapped](https://github.com/longbridge/gpui-kit/pull/3623) | 随升级 | 沿用桌面输入；不新增触摸键盘开关。 | 平台行为自动承接，无共享适配替代。 |
| [#3624 sheet, notification: Keep out from under the system bars and the keyboard](https://github.com/longbridge/gpui-kit/pull/3624) | 随升级 | 已有 Sheet/通知保持原动作，只承接安全区域修复。 | 共享应用同样承接，无新通知能力。 |
| [#3648 scrollable_mask: Dispatch the overscroll steps of the scroll claim test within one frame](https://github.com/longbridge/gpui-kit/pull/3648) | 保留 | 上游测试时序修正，不搬入产品。 | 必要 scroll 测试参考同帧事件语义，不复制测试。 |
| [#3629 resizable: Read active drag state when mouse events arrive](https://github.com/longbridge/gpui-kit/pull/3629) | 保留 | 修复 #3619 但主 pane 生命周期仍不足；设置/包浏览已有 h_resizable 直接获益。 | 共享应用保留已有官方分隔；不从 Gupi 复制 pane 策略。 |
| [#3622 input: Lay out tabs to tab stops in the code editor](https://github.com/longbridge/gpui-kit/pull/3622) | 随升级 | Files/Shell 的长行及 Tab 对齐自动修复；不改文本或 Tab 策略。 | HTTP 可编辑代码受益；保留 tree-sitter 的已测性能选择。 |
| [#3625 input: Ask scroll containers to reveal a focused input](https://github.com/longbridge/gpui-kit/pull/3625) | 随升级 | 聚焦输入自动显露；只删证实重复的 scroll-to-focus，保留业务导航定位。 | Form 与 HTTP Editor 原生验证聚焦显露。 |
| [#3628 text: Release the shaped text of dropped text views at the next frame](https://github.com/longbridge/gpui-kit/pull/3628) | 随升级 | Markdown source/message TextView 释放缓存，无应用手工回收可删。 | 共享 TextView 使用者承接，不添加资源生命周期旁账。 |
| [#3638 locales: Add Brazilian Portuguese (pt-BR)](https://github.com/longbridge/gpui-kit/pull/3638) | 随升级 | Gupi 已有 pt-BR；组件翻译随 Kit 更新，应用 Fluent 不被替换。 | app-i18n 保留应用 locale owner。 |
| [#3640 color_picker: Round hex channels instead of truncating them](https://github.com/longbridge/gpui-kit/pull/3640) | 保留 | 未使用颜色编辑器，不因修复新增 ColorPicker。 | 无已识别的同类颜色绕行；只承接依赖。 |
| [#3632 scroll: Ignore rounding in the part of a packet the text did not use](https://github.com/longbridge/gpui-kit/pull/3632) | 随升级 | 嵌套滚动与大偏移修复自动生效，保留 transcript 锚点及各滚动 owner。 | HTTP 两编辑器/Feiwen 列表受益；不补应用 bounce。 |
| [#3631 scroll: Throw the edge on release only when it is stretched](https://github.com/longbridge/gpui-kit/pull/3631) | 随升级 | 嵌套滚动与大偏移修复自动生效，保留 transcript 锚点及各滚动 owner。 | HTTP 两编辑器/Feiwen 列表受益；不补应用 bounce。 |
| [#3635 dialog: add centered() to seat a dialog in the middle of the window](https://github.com/longbridge/gpui-kit/pull/3635) | 保留 | 全窗口图片预览用 Base Dialog.popup/focus_handle；component centered 仍是有 margin/max-height 的普通 surface，非全窗口画布替代。 | 现有 Dialog 位置保持，避免全局外观变化。 |
| [#3644 input: Let an editor supply its own scrollbar](https://github.com/longbridge/gpui-kit/pull/3644) | 保留 | 不新增自定义编辑器滚动条；网页与原生不同不能作为绕行理由。 | HTTP 保留默认 Editor scrollbar，未发现替代对象。 |
| [#3641 color_picker: Keep the first hex edit after a programmatic value](https://github.com/longbridge/gpui-kit/pull/3641) | 保留 | 未使用颜色编辑器，不因修复新增 ColorPicker。 | 无已识别的同类颜色绕行；只承接依赖。 |
| [#3645 website: Fix OtpInput examples and event documentation](https://github.com/longbridge/gpui-kit/pull/3645) | 保留 | OTP 文档修正无现有产品用途。 | 官方 skills 同步文档，不新增 OTP。 |
| [#3642 focus_trap: End the Tab search at the first repeated stop](https://github.com/longbridge/gpui-kit/pull/3642) | 随升级 | 引用弹层/命令面板 Tab trap 修复；不删除正确的 focus restoration。 | 共享 Dialog/Select 原有焦点流程承接。 |
| [#3651 toolbar: Grow with content taller than the bar](https://github.com/longbridge/gpui-kit/pull/3651) | 随升级 | 现有工具栏的高度/空白点击行为受益，不改变命令位置。 | 共享工具栏保留动作策略。 |
| [#3653 hover_card: Apply trigger_style to the trigger container](https://github.com/longbridge/gpui-kit/pull/3653) | 保留 | 仅 trigger_style 接点；不为引用复杂 hover/popover 行为换 owner。 | 没有已识别的 HoverCard wrapper 可直接删除。 |
| [#3654 command: Pad the empty list only while the empty state shows](https://github.com/longbridge/gpui-kit/pull/3654) | 随升级 | 命令面板空态间距交给 Kit，业务搜索/结果仍应用持有。 | 共享 Command 使用者承接。 |
| [#3652 toolbar: Keep focus where it was when blank toolbar space is clicked](https://github.com/longbridge/gpui-kit/pull/3652) | 随升级 | 现有工具栏的高度/空白点击行为受益，不改变命令位置。 | 共享工具栏保留动作策略。 |
| [#3661 scroll_bounce: Avoid false overscroll at large offsets](https://github.com/longbridge/gpui-kit/pull/3661) | 随升级 | 嵌套滚动与大偏移修复自动生效，保留 transcript 锚点及各滚动 owner。 | HTTP 两编辑器/Feiwen 列表受益；不补应用 bounce。 |
| [#3646 text: Keep a drag that starts in a table cell in that cell](https://github.com/longbridge/gpui-kit/pull/3646) | 随升级 | Markdown 表格选区/词段拖选更准确；不解除 #3669 几何阻塞。 | 共享文本视图自动承接。 |
| [#3655 time_field: Focus the first segment when the field is pressed outside a segment](https://github.com/longbridge/gpui-kit/pull/3655) | 保留 | 无现有 TimeField 需求，不新增时间设置。 | 无对应自有时间控件可迁。 |
| [#3664 typos: Preserve valid Portuguese translations](https://github.com/longbridge/gpui-kit/pull/3664) | 保留 | 拼写词典维护，无应用适配可删。 | 只承接上游维护，不复制 contributor/葡语白名单。 |
| [#3663 text: Support inline code style overrides](https://github.com/longbridge/gpui-kit/pull/3663) | 保留 | 可用公开样式细化，但本轮不改消息/引用设计；保留 resource plugins。 | app-theme 统一语法色不被三个样式 builder 替代。 |
| [#3667 table: Expose DataTable's structure and cells to assistive technology](https://github.com/longbridge/gpui-kit/pull/3667) | 随升级 | 自有设置列表保持原语义，不套 DataTable。 | Feiwen TableDelegate 补/核对 cell_label 与真实内容；保留自有数据层和有界列宽。 |
| [#3665 button: Expose accessible names in the script API](https://github.com/longbridge/gpui-kit/pull/3665) | 保留 | gpui-shell script Button 不在 Gupi 依赖入口，不启用 Shell JS。 | HTTP/Feiwen 是 Rust app，不迁 QuickJS。 |
| [#3673 text: Preserve sparse Markdown table rows](https://github.com/longbridge/gpui-kit/pull/3673) | 随升级 | 稀疏 Markdown 表格按官方渲染，无应用补空单元格代码可删。 | 共享文本同样承接。 |
| [#3672 test: Read accessibility descriptions and placeholders in element snapshots](https://github.com/longbridge/gpui-kit/pull/3672) | 随升级 | 必要原生测试可读 description/placeholder，不能代替行为验证。 | Form 示例测试使用正式快照 API，不造测试专用行为。 |
| [#3689 base: Report on_prepaint bounds from the element's corner](https://github.com/longbridge/gpui-kit/pull/3689) | 审查迁移 | 检查 panes/quote/navigator 的 on_prepaint 原点假设，删除仅补旧偏移的部分；不删真实坐标变换。 | heatmap/Feiwen fit 的测量按新 bounds 核对。 |
| [#3686 text: Publish selection snapshots once per frame](https://github.com/longbridge/gpui-kit/pull/3686) | 随升级 | 每帧 selection snapshot 优化不是选区几何；quote 绘制后读取边界不能盲删。 | 不改 store/form 事件时序。 |
| [#3681 text_selection: Extend double- and triple-click drags](https://github.com/longbridge/gpui-kit/pull/3681) | 随升级 | Markdown 表格选区/词段拖选更准确；不解除 #3669 几何阻塞。 | 共享文本视图自动承接。 |
| [#3684 button: Accept a caller-owned focus handle](https://github.com/longbridge/gpui-kit/pull/3684) | 迁移 | keys::key_area 与 Addition 改 styled Button.track_focus；认证页 focus_anchor/focus_next 间接定位也可删除。quote/diagnostic 的 aria_expanded 仍缺。 | 扫描 app 的 focus-only Base Button 可同迁；不删依赖 expanded 的语义。 |
| [#3687 text: Support list and blockquote style overrides](https://github.com/longbridge/gpui-kit/pull/3687) | 保留 | 可用公开样式细化，但本轮不改消息/引用设计；保留 resource plugins。 | app-theme 统一语法色不被三个样式 builder 替代。 |
| [#3688 diff: Parse truncated hunks through parse options](https://github.com/longbridge/gpui-kit/pull/3688) | 保留 | 当前 DiffFile::parse 对完整 Git patch 保持严格；超限 fallback 不能伪装截断 hunk，勿开启全局宽松解析。 | 无已有部分 patch 产品契约，不启用 permissive parser。 |
| [#3694 text: Expose image source resolver on component TextView](https://github.com/longbridge/gpui-kit/pull/3694) | 迁移 | Files Markdown 用 component TextView.image_source；保留本地相对路径与 ResourceLinks，删除 RemoteImages 插件。 | 无需新封装；app-theme 不承担图片加载策略。 |

## 0.9.0

| 官方条目 | 结论 | Gupi 去留与理由 | 共享 gpui 去留与理由 |
| --- | --- | --- | --- |
| [#3691 input: Reuse measured wraps during live resize](https://github.com/longbridge/gpui-kit/pull/3691) | 随升级 | Editor resize 复用测量结果；不以其代替 pane 拖动状态。 | HTTP 编辑器 resize 自动受益。 |
| [#3701 setting: Show an empty state for unmatched searches and follow scrolling in the sidebar](https://github.com/longbridge/gpui-kit/pull/3701) | 保留 | #3690 已修复，但 #138 自有设置拥有 15 页/作用域/草稿/零结果导航；保留 SettingsRow + Empty。 | 不以 Settings 聚合器代替共享 Form。 |
| [#3703 diff: Add line decorations and interactive gutter markers](https://github.com/longbridge/gpui-kit/pull/3703) | 后续功能 | DiffDecoration 支持 #110 D1，升级合并后独立 PR；#111 按 M2 整体等待，升级 PR 不做功能。 | 无通用引用业务；不把 Gupi Source 身份搬进共享库。 |
| [#3683 text: Render reference-style Markdown images](https://github.com/longbridge/gpui-kit/pull/3683) | 随升级 | 接入 gpui-pre-reqwest-client =0.3.11 与 App::set_http_client；inline/引用式远程图片均按 Kit 默认加载显示。 | 承接官方引用式图片解析；应用 HTTP client 由各 app 初始化，不复制 parser。 |
| [#3697 text: Fix application shutdown deadlock during layout cleanup](https://github.com/longbridge/gpui-kit/pull/3697) | 随升级 | 承接关窗死锁/paint 选区修复；不重写生命周期。 | 共享 app 退出与文本选区受益。 |
| [#3699 selectable_text: Avoid resolving window selection during paint](https://github.com/longbridge/gpui-kit/pull/3699) | 随升级 | 承接关窗死锁/paint 选区修复；不重写生命周期。 | 共享 app 退出与文本选区受益。 |
| [#3700 input: give editor popups POPUP_PRIORITY so they paint above dialogs](https://github.com/longbridge/gpui-kit/pull/3700) | 随升级 | 对话框内 Editor 补全位于上层；删除仅证实重复的层级补偿，不改 quote overlay。 | HTTP 弹窗内编辑器受益。 |
| [#3615 message_scroller: Let the scrollbar reach the end of padded content](https://github.com/longbridge/gpui-kit/pull/3615) | 迁移 | 已经使用官方 MessageScroller；0.9 padding 移入首尾行，删除旧 viewport inset 扣减，保留数据 splice/子卡片业务锚点。 | 无同类应用补偿可直接删。 |
| [#3713 input: Configure cursor blinking through the theme](https://github.com/longbridge/gpui-kit/pull/3713) | 迁移 | 主题结构构造改 default 后赋值；光标保持默认 500ms，不新增偏好。 | app-theme::adapt_material_scheme 必改；保留动态系统色与 syntax palette。 |
| [#3633 icon: Let the application resolve the path every icon is drawn from](https://github.com/longbridge/gpui-kit/pull/3633) | 保留 | 图标选择是 Dock/应用图标；gpui-lucide 按需嵌入 SVG，resolver 的运行时路径策略不是等价替代。 | 保留 gpui-lucide/build.rs 和 Assets 分工，不引入全图标目录或运行时主题功能。 |
| [#3714 menu: Make submenu rows activatable](https://github.com/longbridge/gpui-kit/pull/3714) | 随升级 | 现有菜单按公开 submenu 行为受益，不重写菜单导航。 | Feiwen/HTTP 菜单同受益。 |
| [#3720 virtual_list: Use the height constraint when a horizontal list infers its height](https://github.com/longbridge/gpui-kit/pull/3720) | 随升级 | VirtualList 几何刷新更正确；保留 transcript 数据拼接与跨视图锚点。 | Feiwen 列表仍由组件处理几何。 |
| [#3718 popover: Clear focus on close when nothing had it at open](https://github.com/longbridge/gpui-kit/pull/3718) | 随升级 | Popover 无初始焦点时清焦点；保留有明确触发器的回焦逻辑。 | 共享 Form/Select/Popover 焦点按实际流程验证。 |
| [#3717 test: Register Toolbar for snapshot queries](https://github.com/longbridge/gpui-kit/pull/3717) | 随升级 | 只在必要工具栏验证用公开 snapshot，不改产品。 | 同上；无需搬运 story tests。 |
| [#3716 toolbar: End the wrap search at the first repeated stop](https://github.com/longbridge/gpui-kit/pull/3716) | 随升级 | Toolbar wrap search 修复，不改工具栏内容。 | 共享 app 直接承接。 |
| [#3719 virtual_list: Rebuild cached geometry when the axis or gap changes](https://github.com/longbridge/gpui-kit/pull/3719) | 随升级 | VirtualList 几何刷新更正确；保留 transcript 数据拼接与跨视图锚点。 | Feiwen 列表仍由组件处理几何。 |
| [#3722 text: Continue alphabetic ordered-list numbering after Z](https://github.com/longbridge/gpui-kit/pull/3722) | 随升级 | Markdown 字母有序列表续号修复，不做内容预处理。 | 共享 TextView 直接承接。 |
| [#3721 motion: Sample directly built easing curves with nothing to sample as linear](https://github.com/longbridge/gpui-kit/pull/3721) | 随升级 | 保留已有 spring/easing 配置，空曲线由 Kit 线性处理。 | 不新增动画曲线或绕行。 |
| [#3728 command: Name each palette row after its item](https://github.com/longbridge/gpui-kit/pull/3728) | 随升级 | palette 条目 A11y 名称修复；应用仍提供准确命令名。 | 共享 Command 使用者承接。 |
| [#3723 input: Add a native touch selection opt-out](https://github.com/longbridge/gpui-kit/pull/3723) | 审查迁移 | set_selected_text_range 改善平台/IME 选区，不是富文本编辑器或 #3669 几何。#104 结论范围不改。 | Form Input/Textarea/Editor 原生 IME 受益；保留 native selection 默认。 |
| [#3704 editor: Add gutter and overview ruler decorations](https://github.com/longbridge/gpui-kit/pull/3704) | 保留 | editor gutter/ruler 是装饰能力，不替代文本/视口来源身份；不新增 Git gutter 功能。 | non_exhaustive HighlightTheme/SyntaxColors 初始化适配；语法色 owner 不变。 |
| [#3726 speech: Show the voice in SpeechButton while capturing](https://github.com/longbridge/gpui-kit/pull/3726) | 保留 | 未有语音输入，不因上游 waveform/privacy 修复增加 Speech。 | 共享 workspace 亦不新增语音能力。 |
| [#3725 speech: Add a liquid appearance to SpeechWaveform](https://github.com/longbridge/gpui-kit/pull/3725) | 保留 | 未有语音输入，不因上游 waveform/privacy 修复增加 Speech。 | 共享 workspace 亦不新增语音能力。 |
| [#3724 test: Observe painted text and alternative text](https://github.com/longbridge/gpui-kit/pull/3724) | 随升级 | 可用 painted/alt text 观察验证图片策略；不是截图替代或本轮原生验收。 | 仅更新必要测试接点。 |
| [#3730 speech: Disclaim the terminal's responsibility for privacy access on macOS](https://github.com/longbridge/gpui-kit/pull/3730) | 保留 | 未有语音输入，不因上游 waveform/privacy 修复增加 Speech。 | 共享 workspace 亦不新增语音能力。 |
| [#3727 table: Give the focused element the grid's role](https://github.com/longbridge/gpui-kit/pull/3727) | 随升级 | Gupi 自有列表保留语义；不为 Table A11y 修复替换它。 | Feiwen DataTable 焦点/行列 A11y 自动受益；cell_label 如实表达。 |
| [#3729 menu: Report a checked item's check to assistive technology](https://github.com/longbridge/gpui-kit/pull/3729) | 随升级 | 勾选/未勾选菜单按官方 A11y，无应用假 checkbox 可删。 | 共享菜单承接。 |
| [#3734 input: Hover the whole visible editor after scrolling](https://github.com/longbridge/gpui-kit/pull/3734) | 随升级 | 滚动后 Editor hover 范围修复；Kbd 胶囊是另一问题，保留已确认组合。 | HTTP 编辑器 hover 受益。 |
| [#3736 menu: Open a submenu on the left only without room on the right](https://github.com/longbridge/gpui-kit/pull/3736) | 随升级 | 现有菜单按公开 submenu 行为受益，不重写菜单导航。 | Feiwen/HTTP 菜单同受益。 |
| [#3732 table: Resize a column from where the drag started](https://github.com/longbridge/gpui-kit/pull/3732) | 随升级 | 不影响应用 pane 边界策略。 | Feiwen DataTable 拖列起点修复，保留 #262 bounded auto-fit。 |
| [#3741 a11y: Name the icon-only close buttons](https://github.com/longbridge/gpui-kit/pull/3741) | 随升级 | 官方关闭按钮有名称，应用业务 icon button 仍显式命名。 | 不移除应用提供的对象信息。 |
| [#3745 locales: Add Russian (ru)](https://github.com/longbridge/gpui-kit/pull/3745) | 保留 | 组件 ru 翻译可用；不新增 Gupi 俄语入口或改 nine-locale 应用文案。 | app-i18n locale 范围不扩张。 |
| [#3746 text: Draw a mark's text in a color that reads on its background](https://github.com/longbridge/gpui-kit/pull/3746) | 随升级 | Markdown mark 字色修复，不改 quote/find 优先级。 | app-theme 保留语义色。 |
| [#3739 diff: keep the vertical scrollbar at the viewport edge when soft_wrap is off](https://github.com/longbridge/gpui-kit/pull/3739) | 随升级 | Diff nowrap 竖滚动条回到视口边，无需应用移位补偿。 | 共享无自有 Diff scrollbar 可删。 |
| [#3751 input: Expose edits from the latest change](https://github.com/longbridge/gpui-kit/pull/3751) | 审查迁移 | 事件带有每次 edit 后文档，#104 可以减少文本差分但不能实现原子块；现有业务 whole-value 控件保留。 | 审计 A05 的同帧中间值丢失已由 [gpui#272](https://github.com/suxiaoshao/gpui/pull/272)（`c3edcc047d42dd299a089655f0e2ea28b89384d7`）修复；共享 adapter 按 InputEditsEvent 快照入场，应用不补绕行、不双订阅提交或重放 edits。 |
| [#3756 input: Restore backward selections through set_selected_range](https://github.com/longbridge/gpui-kit/pull/3756) | 随升级 | 反向选区恢复改进；Shell 历史/补全的原字节与 selection 契约保留。 | HTTP 编辑选区受益。 |
| [#3449 combobox: Correct filtered selection events and confirmation](https://github.com/longbridge/gpui-kit/pull/3449) | 审查迁移 | 模型/工具搜索确认用官方修复；不删目录重载后的权威值投影。 | FormCombobox 用 sync_selected_values_state→官方静默 set_selected_values；没有独立 suppression 层。保留权威值投影，过滤索引/确认由 Kit 修复。 |
| [#3758 table: Move assistive technology focus with the selected row or cell](https://github.com/longbridge/gpui-kit/pull/3758) | 随升级 | Gupi 自有列表保留语义；不为 Table A11y 修复替换它。 | Feiwen DataTable 焦点/行列 A11y 自动受益；cell_label 如实表达。 |
| [#3761 menu: Report an unchecked option as unchecked to assistive technology](https://github.com/longbridge/gpui-kit/pull/3761) | 随升级 | 勾选/未勾选菜单按官方 A11y，无应用假 checkbox 可删。 | 共享菜单承接。 |
| [#3759 text: Preserve list body width in fitted containers](https://github.com/longbridge/gpui-kit/pull/3759) | 随升级 | Markdown list 宽度修复，不自测复制排版。 | 共享 TextView 自动承接。 |
| [#3766 gpui: Upgrade to gpui-pre 0.3.11 and gpui-fast 0.1.13](https://github.com/longbridge/gpui-kit/pull/3766) | 迁移 | gpui-pre 精确 0.3.11；键盘 OS-aware、Scroll source、Rc text system/测试时钟统一适配。 | 全 workspace 同版本，不启用未使用的 gpui-fast；已有独立 feature 才同步对应后端。 |
| [#3769 chore: Bump v0.9.0](https://github.com/longbridge/gpui-kit/pull/3769) | 迁移 | Kit/component/assets 0.9 同族，更新 lock 与技能。 | 先全量升级合并，再给 Gupi 唯一 rev。 |
