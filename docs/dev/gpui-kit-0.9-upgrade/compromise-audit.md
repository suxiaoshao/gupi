# 历史妥协审计

范围已确认：P1（包页有界列表/详情）与 S1（保留高亮后端、单列缺失语种）均在本次升级之外；远程图片默认加载、D1、M2、R1 保持。

[返回升级计划](README.md) · [交互清单](http://127.0.0.1:8715/kit-0.9/review.html#audit)。审计不是“现有行为必须保留”清单：按原始原因、当前公开能力和正确行为重新分类。公开能力已到位的妥协不再以历史文档为约束；能力尚未到位的项也不写新绕行。

## 结论与范围

- **升级中移除**：RemoteImages / Base TextView 降级、focus-only Base Button；补上此前漏审的认证页 `focus_next` 间接焦点定位，以及 MessageScroller 的旧 viewport inset 扣减。截图窗口 activation refresh 保留，首帧光标交用户核对。
- **已修复的正确性接点**：共享 Form 文本和整数 adapter 已在 gpui#272 改为逐条消费不可变编辑快照；Gupi 统一采用该修复 revision，回归由共享库维护，不新增应用探针。
- **不再视为阻塞**：旧 Settings 空态/导航私有性、已接入的 Pi disposition/嵌套调用/完整 shell exports/诊断及设置能力。只清过时事实，不重复开发。
- **继续保留有明确原因**：defer_draw prepaint mask、macOS 活动窗口判定、A11y 事务回滚、部分 Diff/选区/InputGroup 能力。后端性能、资源预算与领域语义另列，避免混同为依赖缺陷。
- **已确认的后续范围**：P1，升级后独立修正包页有界滚动；S1，保持现有高亮后端，单列具体语种缺口。不新增入口/设置。

核对范围：`docs/dev` 全目录关键词检索（248 命中）、`src`/`crates` Rust 注释及降级关键词（602 命中），补查所有 `base::` 使用点、焦点遍历、升级计划全部“保留/维持”项、当前共享 Form/theme/lucide 和三个共享应用相关接点。去重按同一原因聚合；测试调度 `run_until_parked`、测试 private fixture、正常错误与业务 fallback 不逐字制造任务，相关边界集中在 D11。命中数是本次研究快照，不是稳定覆盖指标。

应用源码核对至 `941c78b43b95ddbabda1abee7e957df16989e729`，并读取升级 worker 的当前工作树；共享源码固定 `c3edcc047d42dd299a089655f0e2ea28b89384d7`。Kit/base/component 以发布 **0.9.0**、gpui-pre **0.3.11**（Zed `f816e1f9263e3a8ef63c4baaa02e1441ce1c6038`）为准。Pi 为本地 **1.4.0** 源码 `e7a0df27c09d11bf4161d3605246cbf57a129cb2`，没有启动 Pi 或读取用户配置。源码/API 与原生结果分开；旧测量注明来源，不声称新版重测。

## 逐项判断

### 可删除、已解除与待核实的适配

#### A01 · Files 远程图片与 Base TextView — 升级移除

- **原始原因**：#103 在未安装 HTTP client 时将远程图片改成链接；Base TextView 用于本地相对路径 image_source。不是禁止远程图片的产品规则。
- **当前能力**：0.8.1 #3694 将 image_source 开放给 component TextView；0.9 #3683 解析 ImageReference；gpui-pre-reqwest-client 0.3.11 可通过 App::set_http_client 或 Application::with_http_client 安装；当前实现使用后者。
- **正确行为**：本地相对路径继续解析；远程 inline 与引用式图片均按 Kit 默认方式加载显示。
- **处置**：按已确认决定删除 RemoteImages 及降级文案，改 component TextView，接官方 HTTP client；不再提引用式图片拦截接口需求。
- **必要验证**：worker 用虚构 Markdown 核对本地/远程 inline 与引用式图片、默认加载失败表现；网页图片示意不算原生结果。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-103/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/text/mod.rs)；[HTTP client API](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui/src/app.rs)。

#### A02 · 快捷键录制与集合 Add 的 Base Button — 升级移除

- **原始原因**：#138 为复用 retained FocusHandle，只能用 Base Button 并提供外观，保证清除回焦及 Add 的 Escape→Enter。
- **当前能力**：0.8.1 #3684、0.9 component Button::track_focus 已公开。
- **正确行为**：styled Button 持有同一外部句柄；录制、草稿确认/取消和胶囊共同 hover/focus 不变。
- **处置**：升级替换两个 focus-only Base 录制/添加面，删除其手写外观；不要删回焦行为。
- **必要验证**：worker 核对 Kbd→X、清除仅改草稿、Tab、取消/确认；Add Escape 后 Enter 重开。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-138/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/button/button.rs)。

#### A03 · 认证页通过 focus_next 找真实按钮 — 升级移除

- **原始原因**：auth.rs::focus_anchor 先聚焦非 Tab-stop 外层，再 focus_next，借 Button 的私有句柄让 Dialog 恢复焦点；Add、供应商行及菜单触发共用此路线。
- **当前能力**：0.9 Button::track_focus 可将保留句柄直接绑定到真实按钮；无需通过遍历推断子按钮。
- **正确行为**：打开编辑/确认前直接聚焦真实触发按钮；编辑器已有的来源消失回退保持，Root 正常恢复逻辑仍有效。菜单确认移除后的异步行消失是 main 既有缺口，需要 Kit Dialog 校验恢复目标并支持逻辑回退；0.9 没有对应公开钩子，本升级不处理、不加焦点时序绕行。
- **处置**：真实 Add、供应商行及菜单按钮直接 Button::track_focus；focus_anchor/focus_next 与外层焦点锚已删除。Editor 的来源消失回退保留；菜单移除提交后的异步行消失路径另有原生核对边界，见下文。
- **验证范围**：现有认证回归覆盖来源行消失后的 Add 回退；虚构认证条目的原生试用核对了鼠标打开、Escape 关闭、Enter 重开和 Add 回焦。菜单确认移除后焦点丢失已复现，为 main 既有缺口，本次不处理；未访问真实凭据。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/auth.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/button/button.rs)。

#### A04 · 截图窗口激活时额外 refresh — 保留，用户核对

- **原始原因**：gupi-screen-capture/overlay.rs 激活观察者为首帧十字光标额外 window.refresh；注释仍称 GPUI 在 key window 绘制时应用光标。
- **当前能力**：pre 0.3.11 Window::new 的 on_active_status_change 已在通知观察者后调用 refresh；reset_cursor_style 实际按 is_window_hovered 判断。旧说明已不准确。
- **正确行为**：依赖原生激活重绘；截图失活取消仍由应用拥有。
- **处置**：active 分支 refresh 保留。本轮未取得真实应用覆盖层首帧光标证据，不能确认这次 refresh 冗余或必要；交用户在真实应用核对，不满足删除条件。deactivation→capture::activation_changed 保持，不扩写补丁。
- **验证边界**：用户在真实应用核对鼠标静止打开截图覆盖层时的首帧十字光标，以及移动、Escape 和失活取消；worker 不另写测试程序。当前未取得删除依据，保留 refresh。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-screen-capture/src/overlay.rs)；[pre 0.3.11 激活/光标](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui/src/window.rs)。

#### A05 · Form 同帧输入快照 — 已在共享库修复

- **原始原因**：共享 input.rs / integer_input.rs 在 InputEvent::Change 回调读取 state.value，再排 Form 编辑。旧事件没有与每次修改绑定的不可变文本。
- **当前能力**：0.9 #3751 InputEditsEvent::edits 给出有序 (InputEdit,Rope)，Rope 对应每次 edit 后文档。共享 [gpui#272](https://github.com/suxiaoshao/gpui/pull/272)（`c3edcc047d42dd299a089655f0e2ea28b89384d7`）已让文本、映射文本及整数绑定逐条消费这些快照；整数步进与失焦仍按各自事件处理。
- **正确行为**：每个 native change 按顺序捕获其自己的文本。多次修改先发生、后分发事件时，不能把它们都变成最后一个值；Form tickets/drafts/admission 继续单一拥有。
- **处置**：Gupi 的 8 个直接共享 Git 依赖及传递 gpui-form-macros 已统一锁定 `c3edcc047d42dd299a089655f0e2ea28b89384d7`，不混用旧 rev；应用无需 adapter 绕行、双订阅或 edits 重放。
- **验证范围**：旧 revision 的同帧有效→无效→有效序列曾确认中间值丢失，证据仅作历史记录，保存在被忽略的 build/design/kit-0.9/probes/admission/。修复及 native edit 回归由 gpui#272 提供；本轮只运行 Gupi 全量检查，不重建探针或补原生截图，真实应用由用户验收。
- **依据**：[共享文本 adapter](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/crates/gpui-form-gpui-component/src/input.rs)；[共享数字 adapter](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/crates/gpui-form-gpui-component/src/integer_input.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/input/base/state.rs)。

#### A06 · FormCombobox 投影与所谓写回抑制 — 保留有据

- **原始原因**：升级计划误写 FormCombobox::sync_selection/写回抑制。当前及旧 rev 2d14af95 都是 sync_selected_values_state→官方 set_selected_values，没有独立的 suppression 状态机。
- **当前能力**：Kit 0.9 ComboboxState::set_selected_values 文档明确无 ComboboxEvent，更新 committed selection/snapshot；#3449 修的是过滤后选择与确认。
- **正确行为**：Form 控制权威值→控件投影，用户 Change→Form；官方静默 setter 已负责不回响。
- **处置**：保留正式投影，纠正计划的不存在符号和“保留 suppression”表述；不新增防回写层，也不把索引修复当作可删除 Form 的依据。
- **必要验证**：随升级核对过滤后选择/确认及权威值重投影；无需为不存在的 suppression 写测试。
- **依据**：[共享 Combobox adapter](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/crates/gpui-form-gpui-component/src/combobox.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/combobox.rs)。

#### A07 · #54 的 Invalid、基线与手写重合并 — 已解除

- **原始原因**：#54 用 Slot::Invalid 存坏输入，并自存基线、计算字段改动和手动重合并；#139 已迁走这些适配。
- **当前能力**：共享 `c3edcc047d42dd299a089655f0e2ea28b89384d7` 提供 editor drafts、DraftPolicy、EditorKey、原子 rebase_preserving_edits、field merge hook、EditResult::Update。当前 Pi map_controls 通过正式映射编辑。
- **正确行为**：共享管理编辑事务；应用只负责 Unset/Unreadable、业务 key 的 B/L/R 合并、文件和继承。
- **处置**：不再按 #54 恢复旧适配。#54/#2 中的旧理由是历史设计，现行入口应指 #139；Operation/Store 是否精简不是 Form 本轮阻塞。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-54/README.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-139/README.md)；[当前 Form 契约](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/crates/gpui-form/docs/guide.zh-CN.md)。

#### A08 · 旧 Settings 的空态、侧栏插槽与切页不可拦截 — 已解除

- **原始原因**：#2/#3/#134/project-pi-files 将私有 Settings 导航、无 Empty/分区/有界插槽当成限制；follow-ups 仍写零结果空白。
- **当前能力**：#138 已用自有 SettingsView 管理导航和 Empty；0.9 #3701 也补官方 Settings 空态。旧组件不再拥有当前页面切换。
- **正确行为**：保留已确认的标题栏域切换、零结果清查询、按页草稿及保护规则；这些是当前产品决定，不能再解释为无法拦截。
- **处置**：升级文档清理应删除“等待 Settings 才能有空态/导航”的现行待办；不退回聚合 Settings，也不自动新增切页确认。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-134/README.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-2/implementation.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/project-pi-files/interface.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/layout.rs)；[0.9 #3701](https://github.com/longbridge/gpui-kit/pull/3701)。

#### A09 · 包浏览整页滚动 — 已确认

- **原始原因**：#3 明确要搜索/页脚固定、列表滚动，但旧 Settings MinContent 槽只好 Mount::Content；当前 browse.rs::render_list_pane 仍写 Results and paging share the page scroller。
- **当前能力**：当前自有 SettingsView 已能给有界 flex_1/min_h_0 槽，Kit 0.9 公共 ScrollableElement / ScrollHandle 与 h_resizable 足够。无需再等 SettingPage::content。
- **正确行为**：包页采用有界列表/详情，搜索与分页固定；保留搜索、选择、分页和窄窗返回状态，普通设置页不变。
- **处置**：P1：升级合并后以独立应用布局 PR 修正包页有界列表/详情，使用公开布局与滚动组件，不改共享库；本次升级不包含该布局变更。
- **必要验证**：独立布局 PR 核对长结果/长详情、固定搜索页脚、窄窗返回及分栏。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-3/implementation.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-138/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/resources/browse.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/layout.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/scroll/scrollable.rs)。

### 组件与窗口能力边界

#### B01 · defer_draw 的 Masked 包装 — 仍受阻

- **原始原因**：workspace/strip/layer.rs 额外在 prepaint/paint 恢复裁剪，防止不可见拖动标签命中相邻窗口控件。
- **当前能力**：pre 0.3.11 的 prepaint_deferred_draws 仍不恢复每个 deferred draw 的 content mask；#66079 尚未修复。
- **正确行为**：绘制和命中必须受同一 strip/祖先边界裁剪。
- **处置**：保留现有最小 Masked；不是永久规则。包含正式修复后删包装及专用补偿测试，不新增另一路裁剪。
- **必要验证**：升级运行现有 clipped_lifted_hitbox 回归；不把测试通过当上游修复。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/workspace/strip/layer.rs)；[pre 0.3.11](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui/src/window.rs)；[Zed #66079](https://github.com/zed-industries/zed/issues/66079)。

#### B02 · macOS 活动窗口通知防护 — 仍受阻

- **原始原因**：notifications.rs::focused_window 遍历 Window::is_window_active，避免将记忆中的 main window 当当前前台窗口。
- **当前能力**：pre 0.3.11 固定快照仍读 NSApplication.mainWindow 且只接受 WINDOW_CLASS。#66080 是未能复现单窗场景后关闭，不是修复发布；后续两窗 panel 复现与此不同。
- **正确行为**：通知/已读应依据实际活跃窗口；普通窗口和临时 panel 不能混认。
- **处置**：保留公开 per-window 活动检查，修正“关闭即修好”的推断。静态证据不证明所有单窗切后台场景仍复现。
- **必要验证**：worker 核对普通窗→临时 panel→外部应用，比较 getter 和每窗状态；无需读取对话内容或发系统真实消息。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/app/notifications.rs)；[该发行快照 macOS getter](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui_macos/src/window.rs)；[Zed #66080 与两窗证据](https://github.com/zed-industries/zed/issues/66080)。

#### B03 · 长回答 reveal 的无障碍事务缺口 — 仍受阻

- **原始原因**：#242/follow-ups：虚拟列表重试 prepaint 没回滚已登记 A11y 节点，可能重复节点断言/丢节点。应用没有通过关闭 A11y 绕过。
- **当前能力**：pre 0.3.11 Window::transact 仍仅截断 hitbox/tooltip/deferred/dispatch/element/text-layout，未回滚 a11y；Kit #3295 仍开放。
- **正确行为**：深处查找应保留可访问树且无重复节点。
- **处置**：继续列真实上游阻塞；不能因版本从 0.3.7 升级就宣称解除。无需新增应用 workaround。
- **必要验证**：需要确认当前可达性时交 worker：A11y 激活、超高消息内 reveal_range；普通 headless 不算此场景。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/follow-ups.md)；[pre 0.3.11 transact](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui/src/window.rs)；[Kit #3295](https://github.com/longbridge/gpui-kit/issues/3295)。

#### B04 · Disclosure 按钮仍使用 Base — 仍受阻

- **原始原因**：quote/attachments.rs 与 diagnostic.rs 要求按钮本身 aria_expanded；旧 themed Button 缺 builder。
- **当前能力**：0.9 track_focus 已到位，aria_expanded 仍未公开，#3678 仅部分解除。
- **正确行为**：展开/折叠语义留在真实触发按钮的 A11y 节点。
- **处置**：保留这两个 Base 组合；不把属性移到无关父容器，不因统一外观删除语义。
- **必要验证**：无需本轮新增实测。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/diagnostic.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/quote/attachments.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/button/button.rs)；[Kit #3678](https://github.com/longbridge/gpui-kit/issues/3678)。

#### B05 · Windows/Linux 标题栏菜单宽度计算 — 仍受阻

- **原始原因**：#30/chrome.rs 为 TitleBar 私有内容行的 intrinsic 最小宽度手算菜单 max_w，扣平台控件和拖动区。
- **当前能力**：0.9 title_bar.rs 内部 bar 仍 flex_shrink_0 + flex_1；没有公开内容槽 min-width/style builder。外部 Styled 只作用根。
- **正确行为**：长菜单应收缩滚动且不挤掉原生控制；平台按钮仍由 Kit 提供。
- **处置**：现有有限宽度组合保留；建议单独提交最小 TitleBar 内容收缩能力 issue（本审计不发布，无已知跟踪号），到位后删几何耦合。
- **必要验证**：worker 如验证此项只需原有 Windows/Linux 窄窗长菜单路径；不以 macOS 通过代替。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-30/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/chrome.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/title_bar.rs)。

#### B06 · 延迟悬停预览侧栏 — 保留有据

- **原始原因**：#220 用户允许首版不做：Sheet 会抢焦点/清正文选择，关闭直接移除，不能表达非打断式滑出预览。
- **当前能力**：0.9 Root::open_sheet_at 仍 focus 并清选择，close_sheet 仍取走 active_sheet。HoverCard 的 trigger_style 修复不等于完整侧栏预览生命周期。
- **正确行为**：若以后增加预览，应不破坏正文选择/焦点；不能仅换组件名宣称等价。
- **处置**：继续保留明确取舍，不借升级新增此功能。将来产品重启时单独设计，并为真实缺口提上游；不是已有功能被错误降级。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-220/titlebar.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/root.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/hover_card.rs)。

#### B07 · SettingsRow 没用 Field — 保留有据

- **原始原因**：#138 普通行要求左说明使用余宽、右控件自然宽；Field 私有 label/content 分配不合适。
- **当前能力**：0.9 Field 仍 label_width/intrinsic、content w_full+flex_1；label_fn 不改变 wrapper。h_flex/v_flex、Kit controls 是正式组合接点。
- **正确行为**：普通行布局归应用，Form 状态不复制；适合 Field 的编辑器继续使用 Field。
- **处置**：保留纯展示 SettingsRow，这是组件组合而非补写 Field 行为；不为减少包装退回固定控件列。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-138/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/composition.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/form/field.rs)。

#### B08 · 全窗口图片预览 Base Dialog — 保留有据

- **原始原因**：image_preview.rs 用 Base Dialog 的 popup + focus_handle 承载全窗口画布、平移缩放、顶栏及底部控制。
- **当前能力**：0.9 component Dialog::centered 只改变对话框定位，仍有窗口 margin、max-height、默认 surface/body；focus_handle 是内部根适配，不是公开构建器。
- **正确行为**：保留全窗口图片阅读与 Escape/回焦，不能缩成普通表单弹窗当作等价替换。
- **处置**：保留公开 Base Dialog composition。以全屏画布需求为理由，不再仅写“特殊尺寸/焦点所以不能替换”的模糊判断。
- **必要验证**：无需本轮新增实测。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/image_preview.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/dialog/dialog.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/dialog.rs)。

#### B09 · 主窗格自有拖动路由 — 保留有据

- **原始原因**：#81 因 #3619 丢快速拖动保留自有 route；后来 workspace-layout 明确内外独立、release/Escape 一次提交、打断回滚。
- **当前能力**：#3619 已在 0.8.1 #3629 修复。0.9 ResizableState 的 start/cancel/active drag 仍非公开，on_resize 是结束通知，mouseup 不重新应用释放坐标。
- **正确行为**：可用官方外观，保留现有保存/打断语义；不能把快速事件旧 bug 当唯一现行理由。
- **处置**：遵循已确认 R1：主窗格保留应用事务；设置与包分栏使用官方并获得修复。产品契约等价前不迁主路由。
- **必要验证**：已有 release/Escape/interrupt、无中间帧 move/up 契约回归即可；不新建自有 drag。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/workspace-layout/README.md)；[Gupi #81](https://github.com/suxiaoshao/gupi/pull/81)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/resizable/panel.rs)；[Kit #3619](https://github.com/longbridge/gpui-kit/issues/3619)。

#### B10 · Git 两段文本先转 unified patch — 仍受阻

- **原始原因**：#86 的 Prepared::new 用 similar 文本差异→完整 patch→DiffFile::parse，因 Kit 没有 text-pair 构造。
- **当前能力**：0.9 DiffFile::parse_with 支持 truncated_hunks，仍非 from_texts；#3670 未修复。
- **正确行为**：原文件身份和换行归应用，官方 Diff 渲染；正式 text-pair 能力到位后删除临时桥。
- **处置**：保留最小桥，不用宽松 parser 冒充构造器。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-86/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/workspace/git_changes/text.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/diff/mod.rs)；[Kit #3670](https://github.com/longbridge/gpui-kit/issues/3670)。

#### B11 · Diff 引用只支持同侧行号选区 — 仍受阻

- **原始原因**：#95 首版不能从字符/跨侧选区获得可靠来源坐标，限制为同侧行号选区。
- **当前能力**：0.9 装饰没有开放 selected_source_range/selection 的精确来源；#3670 仍缺字符范围及 viewport anchor。
- **正确行为**：不从鼠标 X 或搜索相同字符串猜出处。
- **处置**：#107 继续等待；现有受限提示保留，不能将 DiffDecoration 当作选区协议。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-95/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/diff/selection.rs)；[Kit #3670](https://github.com/longbridge/gpui-kit/issues/3670)。

#### B12 · 引用工具条锚在指针松开点 — 仍受阻

- **原始原因**：#95 因 TextView/Editor/Diff 缺一致可见选区矩形，采用 release point 并夹入可见边界。
- **当前能力**：0.9 #3723/#3756 改善选区恢复，仍不提供 #3669 的可见片段几何。
- **正确行为**：有正式几何后才贴选区；键盘入口不能假造指针。
- **处置**：#108 等待 #3669；保持现有最小锚点，不包装第二套文字测量。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-95/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/quote/toolbar.rs)；[Kit #3669](https://github.com/longbridge/gpui-kit/issues/3669)。

#### B13 · Diff 评论不高亮 — 已解除

- **原始原因**：#95 原来省略 Diff 评论高亮，因为没有公开独立装饰层。
- **当前能力**：0.9 #3703 发布 Diff::decorations / DiffDecoration，对应 #3671。
- **正确行为**：正在编辑评论的同侧行号引用临时高亮，关闭或来源失效即清除，不改变选择/复制。
- **处置**：已确认 D1：v0.1.13 升级后独立 PR；删除“Diff 不高亮”的现行限制。回跳和持久恢复仍待 #3670，不整体关闭 #110。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-95/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/diff/mod.rs)；[Kit #3671](https://github.com/longbridge/gpui-kit/issues/3671)。

#### B14 · 引用来源旁标记/编辑入口 — 仍受阻

- **原始原因**：#95/#111 等待三种来源的范围几何和 Diff gutter 插槽。
- **当前能力**：0.9 已有 Diff gutter_markers/render_gutter_marker，Editor gutter/ruler；TextView/统一几何仍缺 #3669。
- **正确行为**：不能以一个 Diff 图标冒充跨来源完整交互。
- **处置**：已确认 M2 整体等待；明确是部分解阻，不再声称 Diff 无任何 marker API。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-95/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/diff/mod.rs)；[Kit #3669](https://github.com/longbridge/gpui-kit/issues/3669)；[Kit #3671](https://github.com/longbridge/gpui-kit/issues/3671)。

#### B15 · Shell Editor 不进入 InputGroup — 仍受阻

- **原始原因**：#82 需要真正 Bash Editor，但 InputGroupControl 不接受 Editor。
- **当前能力**：0.9 仍只有 Input/Textarea 转换，#3620 开放，#3657 未合并。
- **正确行为**：保持真实编辑器与命令/输出语义。
- **处置**：保留公开布局组合；不用 dummy Input、私有 mode 或复制 InputGroup。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-82/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/input/group.rs)；[Kit #3620](https://github.com/longbridge/gpui-kit/issues/3620)。

#### B16 · 多行引用仍在 composer 附件区 — 仍受阻

- **原始原因**：#104/#109 目标是正文原子块；现有 InlineToken 单行且保留文本约束，不能塞任意引用块。
- **当前能力**：0.9 InputEditsEvent/原生选区改进没有自定义可编辑 payload/block 文档。editor_host 有渲染几何，但编辑事务/IME 属外部宿主。外部 BlockEditor 固定版仍有载荷/无损序列化边界。
- **正确行为**：不要把现有附件区写成永久规则，也不以事件快照宣称富文本已解决。
- **处置**：维持 #104 独立路线设计与 #109 合并实施边界，本审计只标影响，不改该文档或实施编辑器。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-104/README.md)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/input/base/inline_tokens.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/input/base/state.rs)。

#### B17 · ASCII ! 与 IME/斜杠引用边界 — 保留有据

- **原始原因**：#82 不预知 IME 最终标点；#95 对需要 Pi 展开的命令+引用拒绝提交，避免把引用误当参数。
- **当前能力**：0.9 native selection API 不预知输入法输出；Pi 1.4 RPC prompt 仍单一 message+images，没有“命令后另附上下文”的原子接口。
- **正确行为**：普通文本/全角标点不被偷改；无法无损表达命令引用时保留草稿并说明。
- **处置**：保留两个准确限制；#104 可重审编辑结构，但不能自行展开 Pi 命令或拆成两次发送。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-82/README.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-95/README.md)；[Pi 1.4 RPC](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-types.ts)。

### 性能、资源与内容呈现

#### C01 · Gupi syntect / HTTP tree-sitter — 保留有据

- **原始原因**：#81 为体积/语种选 syntect（历史 release-fast 约减 49.5%）；共享 #262 的 HTTP 64 KiB JSON 插入对比为 syntect 205ms、tree-sitter 2.4ms。不是“官方不支持所以永远不能换”。
- **当前能力**：0.9 仍正式提供两后端；#3604/#3622/#3752 是布局/Tab/换行改进，未承诺 syntect 解析成本变得等价。
- **正确行为**：Gupi 读者和 HTTP 编辑器按各自任务取舍；旧测量不能冒称新版实测。
- **处置**：本次保留已选择后端，记录证据类型；不要为凑迁移统一后端。若要反转选择，须新测而非只读 release 推断。具体语言降级另见 C02。
- **必要验证**：只有拟更换后端时，worker 用相同 release/MBX 构建、64 KiB 编辑和 Gupi 阅读材料测体积/延迟/语种；本次不要求重做全量性能工程。
- **依据**：[Gupi #81](https://github.com/suxiaoshao/gupi/pull/81)；[共享 #262](https://github.com/suxiaoshao/gpui/pull/262)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/highlighter/mod.rs)。

#### C02 · Astro、PowerShell、HTTP XML 的无色降级 — 已确认

- **原始原因**：Gupi directory.rs 将 .astro 映射 HTML；two-face 的 fancy 版排除 PowerShell。旧 #81 称文件映射不用 PowerShell，但现在工具详情明确使用 powershell。HTTP #262 将 XML/SVG 正确标 xml，tree-sitter 没有该内置 grammar。
- **当前能力**：two-face 0.5.2+bat-0.26.1 仍无 Astro、fancy 仍排除 PowerShell；XML 在 syntect 中有。Kit 0.9 有 tree-sitter-astro，但 syntect_backend 在启用 tree-sitter 时不编译，不能只加一个 feature 就混用官方后端。
- **正确行为**：源码内容/复制完整，高亮可降级但要准确记载；不得把 HTML-only 说成完整 Astro，或说不存在 PowerShell 使用点。
- **处置**：S1：本次保留现有官方高亮后端，单独跟踪具体语种及正式后端组合支持；不在应用写补色 parser 或私有桥。
- **必要验证**：worker 用实际 .astro frontmatter/表达式、PowerShell 工具输出、HTTP XML/SVG 样本确认呈现；现有 language lookup 测试只证明映射存在，不能证明语法全着色。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/files/directory.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/messages/tool_details.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/highlighter/mod.rs)；[two-face 0.5.2 语法清单](https://docs.rs/crate/two-face/0.5.2+bat-0.26.1/source/README.md)。

#### C03 · Feiwen 有界 fit_column — 保留有据

- **原始原因**：#262 全结果同步测量曾导致秒级阻塞，改为可见行和表头有界测量、缓存、自动宽度上限。
- **当前能力**：0.9 TableDelegate::fit_column 仍是应用 delegate；#3732 修拖动起点，#3667/#3758 修 A11y，未提供免费全数据宽度索引。
- **正确行为**：显式双击适配当前可见内容，不自动扫描全部结果或无故改变用户手动宽度。
- **处置**：保留有界策略，是已确认响应性能契约；若日后要求全数据 fit，单独设计后台索引和交互，不借升级增加。
- **必要验证**：无需本轮新增实测。
- **依据**：[共享 #262](https://github.com/suxiaoshao/gpui/pull/262)；[当前实现](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/app/feiwen/crates/feiwen-query/src/results_table.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/table/table.rs)。

#### C04 · Transcript 留白适配与业务滚动锚点 — 升级移除

- **原始原因**：#81 已迁 logical_scroll_top，但仍保持跨消息 splice 与展开子卡片的来源锚点；计划误称完全自有虚拟列表。
- **当前能力**：当前 messages.rs 已用官方 MessageScroller；0.9 #3615 将 padding 放入边界行，应用原先从 viewport 扣 MESSAGE_INSET_REM 的坐标修正不再成立。VirtualList 修复不提供 Gupi 数据身份。
- **正确行为**：组件负责滚动几何，应用负责替换消息、跟随底部和来源存活后的阅读位置。
- **处置**：删除旧 viewport inset 扣减，使用完整 MessageScroller viewport（worker 工作树已迁移）；保留业务 anchor。纠正“未用 MessageScroller、无适配可删”的升级审阅结论。
- **必要验证**：worker 沿正式查找/展开/新消息入口核对阅读位置与到底部；保留现有 splice 回归，不测 Kit 的内部 padding 数值。
- **依据**：[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/messages.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/message_scroller.rs)。

#### C05 · gpui-lucide 与官方 icon resolver — 保留有据

- **原始原因**：共享 gpui-lucide 为按需 SVG 嵌入和构建时目录生成；不是缺运行时图标路径 resolver 才存在。
- **当前能力**：0.9 #3633 set_icon_resolver 选择现成图标路径，不负责将调用到的 SVG 字节编入二进制或 dead stripping。
- **正确行为**：资源打包和运行时解析分层；应用/Dock 图标与 UI glyph 主题不混用。
- **处置**：保留共享资源生成及 Assets；不新增图标主题功能。
- **必要验证**：无需本轮新增实测。
- **依据**：[共享资源](https://github.com/suxiaoshao/gpui/blob/c3edcc047d42dd299a089655f0e2ea28b89384d7/crates/gpui-lucide/src/lib.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/icon.rs)。

#### C06 · ResourceLinks 与消息内容投影 — 保留有据

- **原始原因**：#243 的 Markdown plugin 将明确文件/Skill 身份投影成可打开资源；#103 的本地图片路径需绑定文档目录。
- **当前能力**：0.9 inline/list/blockquote styles、图片节点均不理解 Pi 资源身份或 cwd；component image_source 已足够删除 Base 外观适配，但不替代这些业务解析。
- **正确行为**：保留原文/复制与真实来源；无法还原身份时显示实际文本，不猜资源。
- **处置**：只删 RemoteImages；保留 ResourceLinks、本地路径 resolver、合法消息投影。样式 builder 不能替代业务 plugin。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-243/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/source.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/text/mod.rs)。

#### C07 · 严格 Diff、文本/文件大小限制和错误 fallback — 保留有据

- **原始原因**：#31/#86 限定普通文件、UTF-8、大小/行数；损坏 patch 或过大内容用可复制纯文本/显式外部打开，不伪造来源。
- **当前能力**：0.9 parse_with(truncated_hunks) 是接收已知截断补丁，不是修复任意损坏数据；新编辑器布局优化也不承诺任意大文件的 I/O/内存可控。
- **正确行为**：正文与复制不篡改；超限、二进制、读取失败明确区分，原始文件不修改。
- **处置**：保留资源与正确性边界；不把性能上限称永久 Kit 限制。要调整上限单独测实际峰值/延迟，不随升级放宽解析。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-31/README.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-86/README.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/messages/diff.rs)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/diff/parser.rs)。

#### C08 · 旧 CJK/IME 等待清单 — 已解除

- **原始原因**：follow-ups 仍以 Kit 0.7/pre 0.3.7 记录 #13 输入、#14 中文混排为等待新版本。
- **当前能力**：0.9 inline_flow 已包含 shaped-width 修正（#3293），输入另有 #3723/#3756 等正式修复；#13/#14 已关闭。关闭状态不单独证明每种 IME 场景通过。
- **正确行为**：当前文档不应继续把已随正式版本获得的修复当未发布；未实测只标验证边界。
- **处置**：清理旧等待条目，承接官方修复；不添加 CJK 手测宽、插字回滚或输入代理。B03 的 A11y 问题仍独立保留。
- **必要验证**：worker 用原报告的中文标点+行内代码和 IME 换行/反向选区作一次升级验证；结果只覆盖实际操作。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/follow-ups.md)；[CJK 修复 #3293](https://github.com/longbridge/gpui-kit/pull/3293)；[Kit 0.9 公开源码](https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/base/src/text/inline_flow.rs)；[Gupi #13](https://github.com/suxiaoshao/gupi/issues/13)。

### Pi 与平台边界

#### D01 · Pi disposition / 嵌套工具结果 — 已解除

- **原始原因**：follow-ups/pi-rpc-gaps 仍写成功响应 Ok(_) 丢 disposition、子调用 parentToolCallId/nestedCalls 未消费。
- **当前能力**：当前 Client::prompt→InputDisposition、conversation.rs 的 Handled 分支，以及 execution.rs/nested_calls.rs 都已接入；Pi 1.4 正式数据仍提供这些字段。
- **正确行为**：按真实提交结果和父子身份显示，不把“已传 JSON”当完整渲染；历史子输出仍受 Pi 持久化上限限制。
- **处置**：删除过时待接入条目，不重复开发 typed disposition/嵌套卡片；保留完整输出未持久化的真实边界。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/pi-rpc-gaps.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/pi-rpc/src/client.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation/src/conversation.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/messages/nested_calls.rs)；[Pi 1.4 RPC](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-types.ts)。

#### D02 · 仅捕获 PATH、无失败诊断 — 已解除

- **原始原因**：follow-ups 仍称登录 shell 只补 PATH、会话关键失败无诊断日志。
- **当前能力**：当前 Environment/Snapshot 和 shell_environment 捕获导出变量；conversation/diagnostics.rs 已分类记录失败且避免凭据/正文。
- **正确行为**：环境以正式捕获快照给各 Pi 子进程；捕获失败回继承环境并解释。系统代理不是 shell export，不自动声称已转换。
- **处置**：旧 PATH-only 与无日志清单应删除；保留失败回退和系统代理来源边界，不增第二套环境机制。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/follow-ups.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-pi-runtime/src/environment.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation/src/conversation/diagnostics.rs)。

#### D03 · 无设置/资源/认证 RPC 因而无 GUI — 已解除

- **原始原因**：旧 pi-rpc-gaps/follow-ups 将配置、认证/信任、Shell、部分资源管理列为独立待做或外部 Pi 专属。
- **当前能力**：当前 Pi 配置文件读写、auth、MCP、包、Skill、提示词、信任及 Shell UI 已存在；文件/官方 CLI 是正式接点，不要求每项都有 RPC。
- **正确行为**：按领域拥有者操作实际文件或 Pi CLI，守住信任、未知字段与保存边界；不把“无同名 RPC”写成产品缺功能。
- **处置**：更新旧能力索引的现行判断，不再次实现这些页面；OAuth 完整登录仍见 D07。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/pi-rpc-gaps.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/layout.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/src/features/settings/auth.rs)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-conversation-ui/src/home/shell.rs)。

#### D04 · 队列只能取回文字/整队清空 — 仍受阻

- **原始原因**：#222 因 RPC 只有字符串数组与整队 clear，不能无损恢复图片或按稳定 ID 编辑/删除。
- **当前能力**：Pi 1.4 rpc-types/rpc-mode 仍无 get_queue 与单项原子编辑/删除；disposition 不携带条目身份。实验 Harness 的能力未接到 coding-agent RPC。
- **正确行为**：只承诺取回实际返回的文字，不用 clear/requeue 冒充原子修改、不维护第二套执行队列。
- **处置**：保留明确降级；待正式 RPC 能力，既有 #222 记录作为跟踪，不虚构新的已提交上游 issue。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-222/queue-composer.md)；[Pi 1.4 RPC 类型](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-types.ts)；[Pi 1.4 分发](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-mode.ts)。

#### D05 · 插件补全、任意 TUI UI 与输入回读 — 仍受阻

- **原始原因**：旧文档因没有 RPC 表达而只实现标准问答/文字 widgets；不能运行远端 TUI 组件函数。
- **当前能力**：Pi 1.4 get_commands 仍无 getArgumentCompletions 查询；onTerminalInput/addAutocompleteProvider/setEditorComponent 为空，custom 未传输，getEditorText 空串，pasteToEditor 仍为替换。
- **正确行为**：展示标准 RPC 真有的数据；不猜问卷 schema、命令参数或执行 TUI renderer。
- **处置**：参数补全继续 #19 / Pi #7621 方向；其它按正式协议等待。Kit 升级不能解除传输层缺口。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/pi-rpc-gaps.md)；[Pi 1.4 降级实现](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-mode.ts)；[Gupi #19](https://github.com/suxiaoshao/gupi/issues/19)；[Pi 补全提案](https://github.com/earendil-works/pi/pull/7621)。

#### D06 · 刷新资源只能重启 Pi — 保留有据

- **原始原因**：#226 的“重连”重启进程恢复会话；旧索引容易把无直接 reload RPC 理解成没有任何公开 reload 能力。
- **当前能力**：Pi 1.4 RPC 模式的扩展 command context 已将 reload 绑定到 session.reload；Gupi 已有公开扩展通道用于 navigateTree。
- **正确行为**：能力并非不存在，但“资源重载”和重建进程/环境/恢复连接是不同动作，不能静默替换现有重连。
- **处置**：本升级保留已确认重连语义；若要资源-only reload，单独 issue 明确入口、busy、插件内存和失败语义。删除“绝无公开路径”的表述，不自动新增功能。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-226/reconnect.md)；[Pi 1.4 扩展 context](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-mode.ts)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-pi-runtime/src/extension.ts)。

#### D07 · OAuth 由 Pi TUI 完成、特殊 ID 只复制 /login — 仍受阻

- **原始原因**：#48 没有可复用的 GUI OAuth RPC，采用登录指令；无法安全表示的 ID 只复制 /login 让用户在 Pi 选择。
- **当前能力**：Pi 1.4 标准 rpc-types 仍无 login/logout 授权交互；TUI 命令参数不是 shell quoting 协议。已有文件认证管理不能冒充 OAuth 登入。
- **正确行为**：API Key 管理正常；OAuth 流程由实际 Pi 提供，不自行拼授权 URL、刷新凭据或暴露秘密。
- **处置**：保留此界限；用户之前删除上游需求草稿的决定仍有效，本审计不重新发布 issue。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-48/README.md)；[Pi 1.4 RPC](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/modes/rpc/rpc-types.ts)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/gupi-resources/src/pi_auth.rs)。

#### D08 · 项目包不能单独更新 — 仍受阻

- **原始原因**：project-pi-files/d2-packages 因 update 可能触及全局/信任而禁止项目更新，不能用 remove/install 替代。
- **当前能力**：Pi 1.4 PackageManager::update(source?) 仍遍历 globalSettings 和 projectSettings，无 scope 参数；CLI --extensions 选择更新对象类型，不是项目限定。
- **正确行为**：项目操作不能悄悄更新全局包/信任；install -l 不等价 update。
- **处置**：继续等待正式限定作用域能力，按 Gupi #52 现有证据跟踪；不通过 cwd 或参数名字推测隔离。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/project-pi-files/d2-packages.md)；[Pi 1.4 update](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/core/package-manager.ts)；[Pi 1.4 CLI](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/package-manager-cli.ts)；[Gupi #52](https://github.com/suxiaoshao/gupi/issues/52)。

#### D09 · Pi Changes 合成补丁/写入无基线 — 仍受阻

- **原始原因**：#33 Pi edit 有 details.patch；write 只给新内容，事后无法补出旧版本，合成失败按逐次操作显示。
- **当前能力**：Pi 1.4 edit.ts 仍生成基于去 BOM/LF 的 patch，write.ts 仍 details:undefined；Kit Diff 不能创造执行前的文件基线。
- **正确行为**：只对可证明的记录合成净差异，不称完整工作区 diff；失败保留原操作和错误。
- **处置**：保留领域合成与准确 fallback。等待 Pi 正式 checkpoint/baseline 才删除限制；不偷偷执行前读盘或引入应用快照协议。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-33/README.md)；[Pi 1.4 edit](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/core/tools/edit.ts)；[Pi 1.4 write](https://github.com/earendil-works/pi/blob/e7a0df27c09d11bf4161d3605246cbf57a129cb2/packages/coding-agent/src/core/tools/write.ts)。

#### D10 · 原生 defer、存活标记与平台降级 — 保留有据

- **原始原因**：#58 的存活标记/defer 避免 GPUI 借用重入和迟到原生调用；#221 另记录非 macOS window level 无效果。
- **当前能力**：pre 0.3.11 没有取代原生窗口层级/跨平台定位的统一公开 API；当前 window-ext 已属 Gupi crate。Windows 0.62.2 原生 API 能设置 topmost，但不等价 macOS 多层级，Kit 未提供统一契约。
- **正确行为**：存活/回调边界必须保留；非 macOS 层级调用成功不代表已置顶，不能隐藏差异。
- **处置**：保留生命周期保护。非 macOS no-op 是尚未实现的平台能力，建议单独平台 issue 设计明确层级映射/不支持结果；不借升级添加原生绕行。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-58/README.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-221/temporary-window-comparison.md)；[应用代码](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/crates/window-ext/src/platform.rs)；[pre 0.3.11 Window](https://github.com/zed-industries/zed/blob/f816e1f9263e3a8ef63c4baaa02e1441ce1c6038/crates/gpui/src/window.rs)。

#### D11 · 读取失败/未知值/信任未知等降级 — 保留有据

- **原始原因**：文档大量“无法”指坏文件、未知 JSON、信任状态、OS 阻塞操作、异步来源过期，并非库没有控件。
- **当前能力**：Kit 0.9 控件与共享 Form 不拥有 Pi 文件语义、锁、版本/信任、进程退出或操作系统同步调用取消。
- **正确行为**：Unreadable 原样保留、未知不冒充生效、失败不当空文件、迟到结果不串会话；取消只承诺实际可取消边界。
- **处置**：保留这些正确性规则；旧字段未纳入首期、分享/外部编辑器/额外导航未选择等是产品范围，不因 API 存在自动增加。
- **必要验证**：无需本轮新增实测。
- **依据**：[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/project-pi-files/data-and-persistence.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-218/runtime.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-2/field-selection.md)；[原设计](https://github.com/suxiaoshao/gupi/blob/941c78b43b95ddbabda1abee7e957df16989e729/docs/dev/issue-139/README.md)。

## 已确认的范围

### P1 · 升级后独立修正包浏览滚动

升级合并后，以独立应用布局 PR 将包页改为有界列表/详情，搜索与分页固定；保留搜索、选择、分页和窄窗返回状态，普通设置页不变。使用 Kit 公开布局与滚动组件，不改共享库。本次升级不包含该布局变更，独立 PR 核对长列表、长详情和窄窗返回。

### S1 · 保持高亮后端，单列语种缺口

本次保持 Gupi syntect 与 HTTP tree-sitter 后端，单独跟踪 Kit 正式语言与后端组合支持。Astro 仅 HTML、fancy PowerShell / HTTP XML 无语法色的现状如实记录，内容与复制保持完整。不在应用添加补色 parser 或私有桥；旧性能数据不作为新版验证或永久保留依据。

## 升级计划中其余“保留”的逐项回查

上面的条目覆盖所有真实适配。其余 release 条目按下表解释，不能用“保留”笼统暗示存在需要维护的降级实现。

| Release 条目 | 重新判断 |
| --- | --- |
| #3610 | 当前未用旧 Dock StackPanel 持久化；无同类兼容层可删，不新增 Dock。 |
| #3621/#3648 | 官方 TabBar/ScrollableMask 已承担滚动；共享 Heatmap 已采用 vertical wheel。没有另一套 edge routing 要保留；测试修正不搬入产品。 |
| #3607 | 官方补全 story 是例子，不是 Pi 命令/文件目录所有者；应用补全保留真实业务数据。 |
| #3640/#3641、#3655、#3645 | 无 ColorPicker/TimeField/OTP 使用点，属于“不适用”，不是旧妥协。 |
| #3665、Shell publication | gpui-shell JS API 与 Bash Editor 不同；不引入 JS runtime。 |
| #3644 | 未自定义 Editor scrollbar；没有可删除的 scrollbar 适配，继续官方默认。 |
| #3653/#3718 | HoverCard trigger_style / Popover 无初始焦点关闭修复自动承接；不替代有真实来源的回焦与引用数据生命周期。认证间接回焦另按 A03 删除。 |
| #3663/#3687/#3746 | 文本样式/mark 对比修复自动承接；资源身份插件仍按 C06，主题语义色不能由 style builder 取代。 |
| #3604/#3622/#3613/#3632/#3631/#3661/#3720/#3719/#3752/#3734 | 布局、滚动、wrap、hover 官方修复自动承接。源码搜索没有发现对应的应用 bounce/Tab-width 补偿要保留；C04 的业务锚点和 C01 的后端选择独立。 |
| #3617/#3667/#3727/#3732/#3758 | Feiwen DataTable 使用官方修复；保留数据 delegate 与 C03 有界 fit。设置页是任务行，不能仅为使用 DataTable 改产品结构。 |
| #3713/#3721 | 主题 non_exhaustive 适配与默认光标/动画行为；动态系统色是产品能力，无重复 cursor timer 或 easing engine 可删。 |
| #3704 | Editor gutter/ruler 已公开；不因有 API 新增 Git gutter，#111 已确认 M2。 |
| #3726/#3725/#3730、#3745/#3638、#3608/#3664 | 语音、未选应用语言、拼写维护不产生产品迁移；已有 pt-BR 组件翻译自动承接，不新增入口。 |
| #3694/#3683/#3684/#3629/#3635/#3688/#3701/#3615/#3633/#3751/#3723/#3756/#3449 | 分别按 A01–A06、A08、B07–B16、C04–C07；不得继续引用本表之外的旧保留理由。 |

## 交给 worker 的验证与文档收尾

原始静态审计与本次运行验证分开；本次已执行 A03 认证回归与原生回焦、旧 revision 的 A05 同帧输入探针；A05 现已由 gpui#272 修复并统一接入。A01/A02/A03 的既有验证范围保持；A04 首帧光标交用户核对，A05 同帧回归由共享库维护。若验证发现正式能力不满足契约，报告共享/上游缺口，不补应用 workaround。本地检查直接 cargo/MBX，不设置 CARGO_TARGET_DIR。后续原生使用现有 gallery 入口；不手工组装隔离包、不为用户可验证的原生行为另写程序。A04 交用户在真实应用核对，refresh 保留。

文档收尾应同步当前事实：`follow-ups.md`、`pi-rpc-gaps.md` 的已接入功能和旧等待项；#2/#3/#134/project-pi-files 的旧 Settings 限制；#54 被 #139 替代的适配；#138 的焦点构建器版本说明；#103 远程图片当前设计。历史验证保留原日期/范围，不改写成新版本验收。已按获准范围清理各入口文档的过时阻塞与旧适配说明；历史协议研究保留其日期，不改 #104 范围。
