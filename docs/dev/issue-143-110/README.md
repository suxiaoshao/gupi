# 包列表统一分栏与 Diff 评论高亮（#143 / #110 D1）

包页采用 O2 布局：“已安装”和“浏览”使用统一的有界列表、详情面板及底部操作条。对话中的 Diff 引用在编辑评论时，通过 Kit 行装饰临时高亮对应来源；退出编辑或来源失效后清除，不改变原有选择、复制和滚动。

设计来源为复用设置外壳的本地交互样张 `build/design/issue-143-110/`；以下描述以产品实现为准。

## 包页布局与操作

入口为 **设置 → Pi → 包**，保留同一页内的“已安装 / 浏览”标签。页头只有标题与作用域、共同工具行两层：工具行把标签、当前视图操作和刷新放在同一排，宽度不足时自然换行。浏览提供目录搜索，已安装提供“从来源安装”，没有新增本地搜索、过滤或分页。

选择包之前显示列表；选择后，宽内容区显示可调整的左列表/右详情，窄内容区以详情替换列表并提供返回。两页共用 `packages.rs` 中的 `package_row`、`package_panes` 和 `package_detail`：

| 区域 | 已安装 | 浏览 | 共同规则 |
| --- | --- | --- | --- |
| 列表行 | 包名、版本、来源、资源数量及项目/继承等状态 | 包名、版本、描述和当前作用域已安装标记 | 整行选择，统一字体、间距、分隔线和选中样式；更新/移除放在详情中 |
| 详情正文 | 当前版本、安装来源、按类别分组的实际包内资源与逐项启停 | 目录版本、发布信息、声明资源、来源与安装状态 | 只使用各自实际数据，不为已安装额外请求目录信息，不把目录声明视为已加载资源 |
| 详情操作 | 全局更新、可变包移除、定位目录、复制来源；继承包可编辑全局 | 安装，或按当前作用域更新/移除；目录链接和来源操作 | 固定在详情底部，复用现有包操作与权限判定 |
| 滚动 | 列表和详情正文各自滚动 | 相同；结果列表底部另有固定分页 | 页头、工具行、详情标题和操作条不随正文滚走 |

独立扩展/主题继续位于所属作用域的独立资源分组，随已安装列表滚动，保留定位、复制、启停和只读规则。包内资源在详情中管理，不增加包级总开关。项目、继承、delta、覆盖、文件缺失及未信任状态继续沿用资源层语义；继承包不能直接更新或移除，继承及 delta 资源不提供可编辑开关。

### 布局所有权与状态保留

[layout.rs](../../../src/features/settings/layout.rs) 只为包页启用有界内容区；[composition.rs](../../../src/features/settings/composition.rs) 的 `SettingsPage::bounded_body` 复用设置页头，把主体约束在剩余高度内。普通设置页和设置搜索的空结果页继续整页滚动，包页仍通过原 section 的关键词参与设置搜索。

列表与详情分别使用公开的 `ScrollHandle`、`overflow_y_scroll`、`track_scroll` 和 `vertical_scrollbar`；宽窗分栏使用 `h_resizable` 与保留的 `ResizableState`。搜索不属于结果滚动区，分页不属于结果正文。来源安装表单在已安装工具行下展开，安装协议和目录分页大小不变。没有修改共享库或复制滚动实现。

[packages.rs](../../../src/features/settings/resources/packages.rs) 的 `Installed` 保存选择、窄窗详情状态、列表/详情滚动句柄、分栏与焦点；[browse.rs](../../../src/features/settings/resources/browse.rs) 的 `Browse` 单独保存目录查询、请求序号、分页、选择和对应展示状态。

- 切换标签不重建各自状态。窄窗返回保留选择与列表位置，焦点回到所选行；恢复宽窗后仍显示所选详情并使用保留的分栏状态。
- 已安装以继承标记和来源字符串区分当前 scope 内的安装身份；切换作用域时重建 `Installed` 并重置 `Browse` 的作用域状态，旧目标不会投影到新 owner。
- 选择另一个包时重置详情滚动；关闭详情清除该标签选择。数据刷新后已安装的所选包消失时，清除失效选择并把焦点移回包页操作区。
- 目录翻页和新搜索重置结果滚动；新搜索保留原有过期请求防护并清除旧选择。加载、失败及重试留在所属结果或详情区域。
- 资源页的“打开所属包”入口调用 `reveal_package`，选择正确的自身/继承来源并打开已安装详情，不再依赖原位展开集合。

### 移除确认

两个详情面板中的“移除…”均使用 `Button::danger()`，调用同一个 [resources.rs](../../../src/features/settings/resources.rs) `confirm_remove`。确认框复用 Kit `Dialog` / `DialogFooter`；[settings.rs](../../../src/features/settings.rs) 的 `DialogCommit::Danger` 同样使用危险样式。

确认标题指明包名，正文说明“从所选 Pi 作用域中移除；本地来源目录会保留”，提供取消与移除。提交前检查 busy 状态及捕获的 controller 是否仍为当前作用域；取消不执行包操作。不增加第二层确认，也不改变安装/移除语义。

## 设置页导语

包页及其他 13 个设置页的重复页头导语已经删除：通用、外观、通知、快捷键、关于、对话、网络、工具与环境、模型与供应商、认证、MCP、Skill、提示词。连接页保留对 Pi 可执行文件的说明。

`layout.rs::pages` 仅为连接页附加该描述。九种语言同步移除了失效的页头文案，以及已安装旧标题和展开/收起按钮文案。作用域、继承、权限、未信任、未保存、错误和具体选项帮助仍按原来的位置显示。

## Diff 评论临时高亮

### 捕获与渲染

Git 工作区和轮次改动阅读器沿用现有引用与评论入口。指针选区工具条、Diff 右键菜单和键盘引用统一通过 [quote.rs](../../../crates/gupi-conversation-ui/src/home/quote.rs) 的 `capture_diff` 同时取得引用文本及临时来源；仍要求单侧行选区，并通过原有引用许可和会话绑定检查。

[messages/diff.rs](../../../crates/gupi-conversation-ui/src/home/messages/diff.rs) 的每个有效 `DiffBlock` 拥有一个 `DiffOwner`；[quote/highlight.rs](../../../crates/gupi-conversation-ui/src/home/quote/highlight.rs) 定义其来源契约：

- `DiffOwner` 持有真实 `DiffState` 的弱引用、数据 revision 和当前装饰范围。
- 捕获的 `DiffSource` 保存 owner 弱身份、当时的 revision，以及 `selected_lines()` 给出的真实 Diff path、side 和归一化起止行。行号为 1-based 闭区间，不从选中文字反查位置。
- `DiffBlock::update` 仅在权威内容变化时推进 revision 并清空高亮；等值数据更新和分栏、折叠、换行等展示变化不改变来源版本。替换为另一个 block 会产生不同的 owner。

引用来源保存在会话展示层 `Attachments.sources` 中，不写入 `QuoteAttachment`、草稿文件、历史或 Pi 请求。会话 binding 改变时清除对应 Diff 来源。

[quote/attachments.rs](../../../crates/gupi-conversation-ui/src/home/quote/attachments.rs) 在打开评论编辑时创建 `highlight::Active`，在正常渲染流程中按来源有效性和可见性投影范围。`DiffBlock` 的有界和完整 renderer 都通过 `Diff::decorations` 接收 `DiffDecoration::new(range)`；没有活动范围时传空集合。颜色和选择背景优先级使用 Kit 默认行为，不新增主题设置。

高亮更新只触发来源的正常重绘，不调用 `set_selected_lines`、滚动定位或以 `set_files` 替代装饰，不重建选择模型。

### 生命周期与边界

| 事件 | 行为 |
| --- | --- |
| 编辑引用评论 | 来源有效且当前可见时，高亮对应 path/side/lines |
| 保存、取消、Escape 或引用被移除 | 清除活动装饰；评论保存或弃稿仍走已有流程 |
| 同源切换分栏、折叠、软换行 | 保留来源行坐标，不替用户展开、选中或滚动 |
| 来源暂时不可见 | 不显示高亮、不自动切页；仍存活且有效的来源再次可见时可继续显示 |
| 内容 revision 改变、owner 释放/替换或 binding 失效 | 丢弃过期来源并清除高亮，保留引用文本快照和评论编辑内容；旧来源不会匹配同路径的新数据 |
| 重启或恢复草稿 | 只有引用快照和评论，没有临时来源，因此不恢复高亮 |

来源回跳、重启后恢复高亮、持久来源、字符级或跨侧引用、viewport anchors、gutter 标记、批注线程、自动修改文件及正文内联引用均不在本次范围。实现不调用 Kit 私有布局、不猜测字符几何、不复制选择行为，也不修改 Pi 协议。

## 验证

以下为 2026-11-06 已完成的本地实现验证，依据实现交付记录及测试日志；网页样张验证不作为原生实现验收证据。

| 检查 | 实际结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check`，覆盖 `gupi`、`gupi-conversation-ui`、`gupi-settings` | 通过 |
| `cargo clippy --all-targets -- -D warnings`，覆盖上述三个包 | 通过 |
| `cargo test --locked`，覆盖上述三个包 | 分别 138、235、22 项通过，0 失败；包含九语言键及参数一致性检查 |
| 本机原生界面 | 用户通过本地 `cargo run` 试用并验收；这是用户反馈的原生验收记录，不扩写为未记录的逐场景覆盖 |

已实现的针对性回归：

| 测试 | 入口与实际覆盖 |
| --- | --- |
| `packages_bound_scrolling_and_retain_navigation` | 挂载正式 `SettingsView` 并调用 `open_page`，通过 Kit 点击和滚动检查已安装列表/详情的固定区域、浏览分页位置、窄窗返回后的选择/列表位置/焦点，以及切标签和恢复宽窗后的详情。布局测试使用已完成的目录详情数据，不初始化 Tokio，不触发真实目录网络请求 |
| `diff_comment_highlight_ends_with_editing_or_source_owner` | 挂载正式 `HomeView`，从 Git 阅读器捕获引用并打开评论；渲染后检查传给 Diff 的装饰范围、原选区和选中文字不变；Escape、保存清除装饰；同路径数据替换使来源失效，引用快照及评论编辑仍保留 |
| `operation_quote_revision_tracks_data_only` | 直接验证轮次 `DiffBlock`：等值内容及展示配置不使来源失效，真实内容变化清除高亮，旧来源不能再次激活 |

包页回归测试的已完成详情夹具在本机 macOS 通过了针对性复测，并执行了 `cargo clippy -p gupi --all-targets --locked -- -D warnings`。以上自动检查覆盖组件几何、装饰投影及交互状态，不等同于逐像素截图比较或全部平台/所有手工场景的验收。
