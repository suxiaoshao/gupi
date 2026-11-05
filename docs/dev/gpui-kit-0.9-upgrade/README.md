# GPUI Kit 0.9 全量升级与迁移

状态：已确认。

Gupi 使用 GPUI Kit `=0.9.0` / gpui-pre `=0.3.11`。共享服务统一固定到 `c3edcc047d42dd299a089655f0e2ea28b89384d7`；本次升级保留现有功能和数据边界，仅调整所需 API 与已确认的官方实现接点。

[交互评审站](http://127.0.0.1:8715/kit-0.9/review.html) · [设计 hub](http://127.0.0.1:8715/)。源码与网页分别在忽略的 `build/design-src/upgrade-*`、`build/design/kit-0.9/`；网页展示迁移前后、完整审阅表和已确认范围，不能证明原生能力已经验收。

[历史妥协审计](compromise-audit.md) 重新核对 45 项适配、降级与旧文档限制；[交互清单](http://127.0.0.1:8715/kit-0.9/review.html#audit) 可按处置筛选。审计已确认。P1：升级合并后以独立应用布局 PR 修正包页有界列表/详情，固定搜索与分页；S1：本次保持现有高亮后端，单独跟踪具体语种缺口。远程图片默认加载、D1、M2、R1 保持。

## 依据与目标

以官方 [0.8.1 release](https://github.com/longbridge/gpui-kit/releases/tag/v0.8.1) 的 47 条、[0.9.0 release](https://github.com/longbridge/gpui-kit/releases/tag/v0.9.0) 的 42 条 What's Changed 为去重覆盖单位；摘要中的破坏性变更和不在自动列表中的 Shell publication 另外列出。完整的逐条判断见 [release 审阅](release-audit.md)，全量直接依赖见 [依赖清单](dependencies.md)。先读 release notes，再以对应 tag 源码确认具体接点，不用源码 diff 代替 release review。

上次 [Gupi #81](https://github.com/suxiaoshao/gupi/pull/81) 依赖共享 [#261](https://github.com/suxiaoshao/gpui/pull/261) / [#262](https://github.com/suxiaoshao/gpui/pull/262)：先共享版本与应用适配，再固定合并 rev。Gupi syntect、HTTP tree-sitter、Feiwen 有界可见行 fit_column 经[审计 C01–C03](compromise-audit.md)重核后仍有体积、编辑延迟和有界计算依据；旧测量不是新版实测，也不是永久豁免。Astro、PowerShell、XML 的具体降级独立列出，不用后端选择掩盖语言覆盖缺口。

共享服务包含编辑会话、字段级合并、入场更新以及 editor-scoped 草稿查询/放弃契约。Gupi 使用 #138 的自有设置页与官方设置分栏。共享库版本与应用版本在同一 GPUI 类型体系内，不混用旧 revision。

目标是 Kit/component/assets 同族 `=0.9.0`、默认 GPUI `=0.3.11`，全 workspace 所有库、官方 skills、Nix、toolchain、Actions 一起审查并更新。最新稳定且兼容的项不制造无意义改动；跨主版本不能只刷新 lock 后宣称完成。没有新设置、语音、图标主题、编辑器或后台服务。不给 Kit 缺口写应用绕行，不改变 Pi 数据或配置格式。

## 升级范围

根及各成员的 normal/dev/build/target-specific manifests 与 lock 统一更新。内部 path 包保持产品版本；共享 Git 服务固定到同一 SHA。官方 skills、skills-lock、Nix 输入、toolchain 与 Actions 一并核对；没有新增设置、语音、图标主题、编辑器或后台服务。

## 必须适配的破坏性表面

| 变化 | 当前落点与处理 |
| --- | --- |
| Theme 系列 `#[non_exhaustive]` | 共享 `app-theme/src/material.rs::adapt_material_scheme` 的 `ThemeConfig { ..Default::default() }` 必改为 `let mut config = ThemeConfig::default();` 后赋值。扫两仓库全部 Theme/ThemeConfig/ThemeSet/ThemeColor/ThemeToken、Semantic*Config、HighlightTheme/HighlightThemeStyle/SyntaxColors、MotionTokens、Sheet/NotificationSettings，以及 Base 的 Theme、SemanticThemeTokens、Color/Radius/Spacing/Typography/TextStyle/Shadow/ResizableTheme/ListSettings；外部 struct update literal 也不合法。保留动态系统色与语法色语义，不另拷一套结构体。 |
| 0.8.1 GPUI 键盘 API | `Modifiers::secondary/secondary_key` 传 App；动态 `Keystroke::parse`/`KeyBinding::new` 不再借泄漏字符串解决 static 要求，按 `cx.platform_info().os` 的官方解析/`KeyBinding::load(..., cx)` 迁移。`unparse` 传 OS。重点 `gupi-settings/keybindings.rs`、`shortcuts.rs`、settings keys/global_keys、tray。保留持久快捷键跨 OS 的原意、冲突判定、录制草稿和全局注册边界。 |
| 滚轮事件 | `PlatformInput::ScrollWheel` → `Scroll`；显式事件构造补 `source`。测试里精准 wheel/line/触摸来源不混写。 |
| GPUI 0.3.11 文本与动画 | `Window::text_system()` 为 Rc，WindowTextSystem 不再 Send/Sync；测量留在 UI executor，不用 unsafe 恢复 Send。Animation 跟 executor clock；相关测试用 advance_clock，不睡墙钟。 |
| DisplayMap unwind traits | 不再 UnwindSafe/RefUnwindSafe。搜索 catch_unwind 边界，不能为通过编译无条件包 AssertUnwindSafe；没有调用则无迁移。 |
| Shell publication | 0.8.1 的 QuickJS 0.14.1 registry 发布属于 gpui-shell，Gupi 的 Bash Shell 是 Kit Editor，两者不是同一能力；不引入 QuickJS 或 JS runtime。 |

0.8.1 还明确延后了 #3616 的 MotionTokens 新字段和 #2842 的 Textarea associated-type 变更以保 patch 兼容；不把这些“延后项”当作可迁移的新能力。0.9 以实际公开结构及 non_exhaustive 规则适配，不据它们推导 Editor 可成为 InputGroupControl。

## 有收益的替换与保留

### styled Button 接回保留焦点

0.8.1 [#3684](https://github.com/longbridge/gpui-kit/pull/3684) / v0.9.0 `component/src/button/button.rs::track_focus` 已公开。迁移 `settings/keys.rs::key_area`（全局录制也复用）、`config_files/editor/collections.rs` 的 Addition：把 focus-only Base Button 的边框、hover、pressed、disabled、focus 样式交回 styled Button。保留原 retained handle、录制 Action/事件、Escape→Add→Enter 再打开与保存草稿；#138 胶囊的共同 hover/focus 容器和内部 X 仍是应用组合。

认证页通过公开 `track_focus` 直接绑定真实 Button 的 retained handle；外层锚点与 `focus_next` 间接定位已删除。保留 Dialog 正常回焦和编辑器来源行消失后回 Add 的业务规则；菜单确认移除后的既有焦点缺口不在本次修复范围，验证边界见审计 A03。

**不能全部迁移 Base Button**：`home/quote/attachments.rs` 和 `diagnostic.rs` 使用 `aria_expanded`；v0.9.0 仍没有对应 themed builder，#3678 只部分满足。保留这些 Base 组合与语义，不删 expanded、不把状态放到错误的父节点、不访问私有 keyed ID。

### 文件 Markdown 回到 component TextView

0.8.1 [#3694](https://github.com/longbridge/gpui-kit/pull/3694) 允许 `TextView::image_source`。`home/source.rs` 的 Base TextView 换成 styled component，复用现有 TextViewState、ResourceLinks 与本地相对路径解析，主题文本样式回归官方。

**Files Markdown 接入官方 `gpui-pre-reqwest-client =0.3.11`，由应用初始化通过 `Application::with_http_client` 安装。** 删除 `source/markdown.rs::RemoteImages` 插件及其接入，远程 HTTP(S) 图片按 Kit 默认方式加载显示，不转成“打开远程图片”链接。0.9.0 [#3683](https://github.com/longbridge/gpui-kit/pull/3683) 提供 `ImageReference` 支持，inline 与引用式图片统一走 Kit 的图片解析、加载和呈现；保留本地图片相对文档目录解析与现有资源链接行为。此路径使用官方公开能力。客户端关闭其 reqwest 分支的默认 features，启用官方 rustls 配置；lock 中与现有 reqwest 0.13 共用 rustls 0.23 / rustls-platform-verifier 0.7，没有引入 native-tls 或 OpenSSL。

### 分隔条：修复不等于契约等价

[#3619](https://github.com/longbridge/gpui-kit/issues/3619) 在 **0.8.1 #3629** 修复，不在最初 0.8.0 release。v0.9.0 源码确认事件到达时读 drag index；但 `ResizableState::resizing_panel_ix/done_resizing` 仍私有，公开 `on_resize` 只有结束通知，mouse-up 分支不使用释放位置重新应用尺寸。主窗口 `home/panes.rs` / workspace 的既有行为还要求：嵌套两级、自动单区 vs 手动最大化、outer 不挤 inner、拖动不落盘、release 与 Escape 各提交一次、打断回滚、陈旧 move/up 无效。修复 #3619 单项不能证明以上全部成立。

**当前决定 R1：保留主窗口应用拖动路由，设置页继续官方分栏。** 主 pane 的保存、取消、打断和嵌套布局契约保持；本升级不迁移主窗口到 ResizablePanelGroup，不新增拖动绕行。

设置页 `layout.rs`、包浏览 `resources/browse.rs` 已用 h_resizable/resizable_panel，保留；不能把自有 SidebarMenuItem 列重新塞回 Kit Settings。主窗口原生比较交 worker，结论只可能确认现有边界或发现新的正式接点，不以牺牲保存语义换组件数量。

### 其它共有边界

- `gpui-form` 的 DraftPolicy、EditorKey、入场票据、B/L/R field hook、Update 的 admission-time patch、基线/冲突仍为共享 Form 所有。共享文本与整数 adapter 已在 [gpui#272](https://github.com/suxiaoshao/gpui/pull/272)（`c3edcc047d42dd299a089655f0e2ea28b89384d7`）改为逐条消费 `InputEditsEvent` 的修改后不可变快照，修复同帧编辑丢失中间值的问题，详见审计 A05。应用不双订阅提交、不在 Form 重放 edits。FormCombobox 实际使用 `sync_selected_values_state` → 官方静默 `set_selected_values`；没有单独的 `sync_selection`/programmatic suppression 层需要保留，正式权威值投影仍必要（A06）。
- `set_icon_resolver` 负责运行时图标路径，不替代 `gpui-lucide` 按需嵌入 SVG、build-time 目录元数据与 dead stripping；保留。应用图标/Dock 选择不是组件图标主题，不新增入口。
- Diff `parse_with(truncated_hunks)` 不等于 `from_texts`。保留 Git 文本转 patch 以及完整 patch 严格解析，保留超限纯文本 fallback；不自动用宽松解析掩盖损坏数据。
- `Settings` 的零结果已修复（#3690 → #3701），保留 #138 自有导航、搜索语义、作用域、草稿操作和 Empty。Theme/Field 变更不要求重做 SettingsRow。
- 发布的 gpui-pre 0.3.11 指向 Zed `f816e1f9263e3a8ef63c4baaa02e1441ce1c6038`：`Window::prepaint_deferred_draws` 仍未恢复 content mask，保留 `workspace/strip/layer.rs` 的既有补做（#66079）。[该快照 macOS active_window](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui_macos/src/window.rs) 仍读取 `NSApplication.mainWindow` 而不检查应用是否活跃，保留 `app/notifications.rs` 的窗口活动判定；#66080 关闭不等于这个发行快照已修复。不新增绕行。

## 主题同步

已将当前 `assets/themes` 与 [v0.9.0/themes](https://github.com/longbridge/gpui-kit/tree/v0.9.0/themes) 的文件集合和 Git blob SHA 逐项比较：**同为 21 个 JSON，新增 0、删除 0、内容差异 0**。此次执行已再次核对，仍无差异；不为制造“同步改动”重新格式化。Kit 默认主题随 dependency 更新，和复制到 assets 的目录不是同一来源。共享 app-theme 动态生成主题仍适配新字段默认值，不能用复制 assets 替代动态 scheme。

## 可解除阻塞的 issue（v0.1.13）

升级 PR 不包含下面新的功能实现。本版拆分 #110 的 Diff 评论高亮，在升级合并后独立 PR；#111 整体等待，其他未满足的部分继续等待，不扩张公共库/应用职责。

| Issue | 上游到位情况 | 当前执行范围 |
| --- | --- | --- |
| [#105](https://github.com/suxiaoshao/gupi/issues/105) | #3619/#3676/#3677 已发布；#3678 仅 focus；其余见本计划 | 已纳 v0.1.13；升级 PR 完成审查和已选迁移后关闭，不表示全部被跟踪功能完成。 |
| [#106](https://github.com/suxiaoshao/gupi/issues/106) | 汇总依赖不全 | 保持跟踪，不整体关闭；只登记本版本完成的子范围。 |
| [#107](https://github.com/suxiaoshao/gupi/issues/107) | #3670 未修复 | 继续单侧行号引用；字符/跨侧源范围、from_texts、viewport anchors 继续等待。不得搜索字符串猜位置。 |
| [#108](https://github.com/suxiaoshao/gupi/issues/108) | #3669 未修复 | 指针松开锚点与绘制后读取保留；InputEditsEvent / 反向选区恢复不是可见选区几何。不承诺本版解除。 |
| [#110](https://github.com/suxiaoshao/gupi/issues/110) | #3671 的 DiffDecoration 已在 #3703 发布；#3670 仍缺 | 按 D1 纳入 v0.1.13：升级合并后独立 PR 补单侧行号引用的评论高亮；持久来源、回跳与 viewport 恢复继续等待。 |
| [#111](https://github.com/suxiaoshao/gupi/issues/111) | Diff gutter marker 已有，Editor gutter/ruler 也已有；#3669 仍缺 | 按 M2 整体等待：本版不加局部 Diff marker，待统一范围几何与必要锚点到位后再实施。 |
| [#82](https://github.com/suxiaoshao/gupi/issues/82) | #3620 未修复，#3657 未合并；v0.9 InputGroupControl 仍仅 From<Input/Textarea> | Shell 保持独立 Bash Editor 与现有布局，不用 dummy Input 或私有 mode 塞 InputGroup；已实现的 #82 不重开重做。 |
| [#109 / #104](https://github.com/suxiaoshao/gupi/issues/109) | 此次编辑器改进未提供原子块文档 | 维持独立设计，下面只列影响，不改 #104 文档或范围。 |

### 已确认的功能范围

- **D1：#110 拆分先做 Diff 评论高亮，纳入 v0.1.13。** 升级合并后独立 PR，用 `Diff::decorations` + `DiffDecoration`，按当前单侧 path/side/lines、数据 revision、source owner 建立临时高亮。编辑评论时显示，关闭或失效时清除，不改变选择和复制。回跳、重启恢复高亮与持久来源继续等待；#110 不整体关闭。不添加批注线程、自动改文件或 Pi 协议。
- **M2：#111 整体等待。** 本版本不加局部 Diff marker；待 #3669 与必要锚点到位后统一三类来源，保留现有引用入口。

## #104 的 0.9 影响复核（不改该文档范围）

1. `InputEditsEvent::edits()` 给出按发生顺序的 `(InputEdit, Rope)`，每个 Rope 为对应 edit 后的文档；比在 Change 回调里读可能已变的 state 更可靠。它可减少文本 host 的重算/差分，但不是可拦截的事务或自定义节点 API，不提供引用块载荷、原子选择/删除或跨块 undo。
2. `EntityInputHandler::set_selected_text_range` 现在实现 UTF-16 选择与方向恢复/显露，改善 native touch/IME；这是现有 InputMode 的实现，不会自动给自有 editor_host 实现 IME，也不暴露 #3669 的可见片段几何。`set_selected_range` 反向恢复、resize wrap 复用及弹层层级均是输入行为修复。
3. v0.9.0 InlineToken 仍禁止 text/label 换行，InputGroup 仍不能接 Editor；不能据此用多行 token 或 dummy Input 替代 #104 的引用块设计。Editor gutter/ruler 也不等于内嵌可编辑块。
4. #104 记录的外部 BlockEditor 固定版依赖 pre 0.3.9 / Kit 0.8，不能直接与升级后同进程混用两套 GPUI entity/types；若后续选它，须另核对上游版本/公开载荷接口/无损序列化，而非本 PR fork。E1/E2 的结论没有因此反转，本轮不选路线、不实现富文本、不改 #104 文档。

## 实现与验证

- 动态快捷键使用运行时 OS 解析、加载和序列化；配置后台归一化使用宿主 OS。键盘录制与 Addition 使用 styled Button 的公开 retained focus；胶囊边界、清除草稿和确认/取消语义不变。
- 滚动事件迁移到 `ScrollEvent` / `on_scroll`，模拟指针滚动显式带 `ScrollSource::Pointer`。MessageScroller 的留白已由列表移到首尾行，阅读锚点按完整 viewport 计算，避免沿用旧 inset 造成偏移。
- MCP 弹窗测试推进 executor clock 完成退出动画；文件选择测试关闭与其插入/取消契约无关的动画。
- msi 使用 `LanguageId`，保持已有语言号；sysinfo 初始化失败时保留 gallery 数据，避免把无法检查误当作没有运行进程。
- Markdown 使用 component TextView，保留本地路径解析与资源链接；远程 inline/reference 图片和加载失败样式由 Kit 拥有。
- 主窗格保留 R1 的拖动、提交、取消和持久化逻辑；保留 Form 权威值投影、Diff 严格解析、遮罩与后台通知的现有边界。

本地检查直接运行 cargo，通过 MBX 共享缓存，不设置 CARGO_TARGET_DIR。CI 仍为集成结果的依据；不以本机通过代替其它平台。当前命令与日志保存在被忽略的 `build/checks/gpui-kit-0.9-shared-272/`：

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo test --workspace --all-features --locked --doc
cargo build --workspace --all-targets --all-features --locked
```

2026-11-05 在共享 revision `c3edcc047d42dd299a089655f0e2ea28b89384d7` 下、Nix 外通过以上全部命令：578 个测试及 1 个 doctest 通过，2 个依赖已安装 Pi CLI 的既有测试仍被忽略。Clippy 使用 `-D warnings` 通过；build 成功但有 macOS linker 的 `__eh_frame` compact-unwind 表大小警告，另有传递依赖 `block 0.1.6` 的未来兼容提示。本地检查不代替其它平台的 CI 或用户原生验收。初次全量测试的 17 项失败来自既有测试使用静默 set_value 后手工发送 Change；改为正式原生编辑入口后，42 项 Pi 设置测试及最终全量检查均通过，产品断言未削弱。

原生试用使用仓库已有的 `cargo xtask gallery ui` / `runtime` 入口和隔离数据；不手工组装 app bundle，不在 /tmp 自建构建目录。用户可在真实应用验证的项目交给用户，不另外制作测试程序。此前原生截图只是对应构建的历史运行证据，不作为可重新启动的隔离环境。

### 原生核对

此前 `cargo build -p gupi --locked` 通过，使用 MBX 托管 target。中文浅色 1440×900 的隔离原生核对完成：Files 的 inline/reference HTTP 图片、本地相对路径图片和真实 HTTPS 图片显示；404 沿用 Kit 默认空白失败表现。Git Diff、主窗格拖动释放后持久化、Cmd+, 打开设置、录制/Escape/清除草稿/确认取消，以及 MCP Addition 的 Escape→Add→Enter 回焦均已核对。截图与逐项证据在被忽略的 `build/design/kit-0.9/native/README.md` / `manifest.json`。

全局快捷键录制、保存和注册接受已验证；CUA 注入按键未能证明系统级唤起，因此不将唤起计为通过。临时注册已清除，隔离应用与 loopback 图片服务已停止。本轮没有原生 IME、捏合或 Windows/Linux 的验证结论；对应既有事件回归/CI 不等同于原生验收。


### 妥协审计核对

认证页的 Add、供应商行和菜单按钮通过公开 Button::track_focus 直接绑定保留焦点句柄，删除非 Tab-stop 外层锚点与 focus_next 遍历。现有认证回归通过；隔离原生已核对鼠标打开、Esc 关闭和 Enter 重开同一编辑器，以及 Add 的键盘重开。菜单确认移除后的异步行消失焦点是 main 既有 Dialog 目标校验/逻辑回退缺口，本次不处理、不加时序绕行。

截图激活 refresh 保留：本轮未能在真实应用取得覆盖层首帧证据，不能证明冗余，留给用户本机核对。旧注释的 key-window 假设已去除，不改变刷新或失活取消逻辑。

Form 同帧输入缺口已在共享库 [gpui#272](https://github.com/suxiaoshao/gpui/pull/272) 修复；Gupi 的所有共享 Git 包统一锁定 `c3edcc047d42dd299a089655f0e2ea28b89384d7`。文本、映射文本与整数绑定按 `InputEditsEvent` 中每次修改后的不可变快照顺序入场，整数步进和失焦继续使用各自事件；共享库维护同帧 native edit、映射更新等回归，应用不补 adapter 绕行，也不重跑一次性探针。既有 Pi 设置测试改用 `EntityInputHandler::replace_text_in_range` 驱动真实编辑，替换静默 `set_value` 后手工发送 Change 的旧方式；保存、草稿、继承和冲突断言不变。旧 revision 的失败证据保存在被忽略的 `build/design/kit-0.9/probes/admission/`，不代表当前 revision 的行为。用户自行 cargo run 验收，本轮不补原生截图。

清理过时的 Settings 空态/导航阻塞、旧 Form Invalid/重复基线说明以及已接入 Pi disposition、嵌套调用、完整 Shell 环境、诊断和管理页面的待办。P1/S1 不实现，#104 不改。
