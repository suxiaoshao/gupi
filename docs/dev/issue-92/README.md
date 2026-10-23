# 界面错误提示本地化

## 目标与边界

Gupi 的界面错误用当前语言说明发生了什么，沿用已有恢复操作；系统、进程和 RPC 的原始错误作为诊断详情保留。调整对话正文/历史、输入区、模型选择器、会话搜索、命令/文件候选面板，以及原始 `$error` 文案调用的呈现，不改变 Pi 执行、重连、持久化或重试策略，不新增错误框架。

Pi 工具、模型回答、Shell 输出和扩展提供的会话内容保持原文。`SessionError::Response` 已与 `Runtime` 分开，不能把消息里的错误拿来套通用界面文案。

## 逐处设计

以下位置按 `4939da8` 核对；示例表达中英文意图，实施时九种语言同步。

| 位置与当前表现 | 主文案及恢复动作 | 技术细节与复用 |
| --- | --- | --- |
| `home/content.rs:60–79` 直接显示 `error`。正文和历史面板共享此函数，分别由 `messages.rs`、`history.rs` 调用；已有内容刷新失败时保留旧内容。 | 首次失败：“无法加载对话内容。” / “Could not load conversation content.”；刷新失败：“无法刷新对话内容，当前显示的内容可能不是最新的。” / “Could not refresh conversation content. Displayed content may be outdated.” 历史面板使用“会话历史 / conversation history”。保留现有“重新连接 / Reconnect”按钮及其 connect + refresh 行为，不把它改名为只重试读取。 | 新增对应加载/刷新文案 key，由调用处传入语义，不解析原始错误字符串；详情采用下述折叠呈现。`conversation-history-failed` 当前意为“执行失败”，不能用于历史读取。临时窗口现有按钮禁用规则不变，文案不承诺可重试。 |
| `home/composer.rs:153–183` 将 `runtime_error()` 原文放在输入区上方；`ReconnectUnconfirmed` 已有专用本地化。 | 普通 Runtime：“Pi 运行出现问题。” / “Pi encountered a runtime problem.”，旁边保留“重新连接 / Reconnect”。不推断一定是网络、鉴权或进程退出。`conversation-reconnect-unconfirmed` 保持原有“重启 Gupi 后再连接”的准确说明，不降级为通用提示。 | Runtime 原文进折叠详情；已本地化的 ReconnectUnconfirmed 不再加一份重复详情。保留 BodyState 的显示门控，避免正文加载失败与输入区重复报同一故障。 |
| `home/composer.rs:190–194` 将 `conversation-save-error` 与原始 `storage_error` 拼接显示。 | “无法访问本地会话数据。请查看详情，检查相关文件的位置和权限。” / “Could not access local conversation data. See details and check the file location and permissions.” 不新增保存/重试按钮，不声称草稿已保存或必定保留。 | 新增语义准确的本地存储错误 key，原文进折叠详情。现有 `storage_error` 也承载启动读取 `conversations.toml` 和临时目录准备失败，不只保存失败；本轮不为精准区分而重构错误类型。`conversation-save-error` 只有确认无其他调用后才删，不全局改写无关使用者。 |
| `home/pickers.rs:598–607` 遍历 model_error、thinking_error、change_error，原文直接显示。 | 分别为“无法加载模型列表。” / “Could not load models.”；“无法加载思考强度选项。” / “Could not load thinking options.”；“无法确认模型或思考强度的切换结果。” / “Could not confirm the model or thinking-level change.” 保留已有刷新入口和当前可用选项；使用现有刷新查询确认状态，不自动重复切换命令，不声称切换肯定未生效。 | 分别绑定本地化 key 与各自原始错误，详情不丢失错误来源。复用 Picker 的反馈区、刷新 Button 和 Tooltip；不新增弹窗。不复用带原始错误插值的 key；无调用的 `pi-models-failed` 按下文删除。 |
| `home/palette.rs:443–465` 会话目录错误直接显示原文，带 Retry；另有 `self.error`。 | 目录复用 `conversation-scan-failed`：“会话目录加载失败” / “Could not load the conversation catalog”，保留 `action-retry` 与 `scan`。此处“目录”是会话目录，不是文件选择器的目录。 | 目录原文进折叠详情。`self.error` 当前唯一赋值为 `command-target-changed`（会话已不可用，请选择其他会话），已经本地化且有明确恢复提示，保持原文案，不再包装为“动作失败”，不为不存在的技术详情造入口。 |

## 呈现与所有权

沿用 Git 阅读器“本地化摘要与原始诊断分离”的方式；阅读器当前以 `diff-read-failed` 显示失败并在异步失败处记录日志，刷新按钮也可携带诊断 Tooltip。本轮不扩大为整个应用的统一错误系统。

行内错误保留现有布局和主题 `danger` 文本，删除失败保留现有通知承载；不增加 Alert 卡片、新通知投递或自动弹窗。原始错误可能很长，采用 Kit 既有 `Collapsible`，标题新增共用本地化 key：“技术详情 / Technical details”，默认收起；内容以普通可换行文本显示，不当 Markdown/终端控制序列执行。只有展开的原始诊断区域设置 `max_h_48`（12rem）、`min_h_0` 和 Kit 垂直滚动；父级继续约束实际可用高度，摘要、恢复按钮与详情入口留在滚动区域外，不能覆盖输入区。用户可用键盘展开、收起，不把唯一详情入口藏在 hover 中；在窄 Picker 中也不得让长路径撑宽面板。现有恢复按钮保持可见，展开详情不改变动作目标。

错误状态仍由 Conversation / Session / Picker 查询结果拥有；UI 只选择文案和呈现诊断，不保存第二份错误状态、不按英文子串分类。详情使用现有错误原文，不额外附加命令、环境变量或会话内容。若实现选择补日志，只在实际失败路径记录，不在 Render 中重复写入；可展开详情已经满足诊断可获取的要求，不要求另建日志通路。

`t` / `t_with_args` 在显示时读取语言；完整句子由 Fluent 负责，不再拼“本地化前缀 + 英文错误”。新增 key 与参数在 `de/en-US/es/fr/ja/ko/pt-BR/zh-CN/zh-TW` 保持一致。按钮继续复用 `conversation-reconnect`、`action-retry` 和已有刷新标签。使用现有 Kit 组件、主题及字号，不引入依赖、持久化字段或自定义错误组件框架。

## 改动位置

实现集中在 `crates/gupi-conversation-ui/src/home/{content,composer,pickers,palette}.rs`，以及传递正文/历史加载与刷新语义所需的 `messages.rs`、`history.rs` 调用处；还包括 `crates/gupi-conversation-ui/src/command_palette.rs` 和根包 `src/app/notifications.rs` 的删除失败通知组合；语言资源为九份 `locales/*/main.ftl`。错误生产、Pi 消息内容和恢复流程不在改动范围。

## 命令面板与 `$error` 调用全量清单

扫描九份 `locales/*/main.ftl`，含 `$error` 的 key 只有下表两个；仓库生产 Rust 中实际设置 Fluent `error` 参数的调用只有删除失败。没有需要翻译的 Pi 会话内容插值调用。不按同名 `error` 字段批量替换工具、模型、Shell、扩展通知或会话正文。

| 位置与错误来源 | 主文案与恢复动作 | 详情位置与实施处理 |
| --- | --- | --- |
| `command_palette.rs:775–791`：`commands().error()` 来自 `ConversationState::read_commands` 的 Pi `get_commands` RPC 读取失败，不是命令执行结果。 | “无法加载命令列表。” / “Could not load commands.”；保留 `action-retry` 调用 `read_commands`，不执行用户命令。 | 同一状态区新增默认收起的技术详情 Collapsible，原始错误不再直接作为摘要。保留连接不可用、加载和空列表等原有状态判断。 |
| `command_palette.rs:799–805`：`files.result` 的 Err 来自 `command_palette/files.rs` 文件遍历，在无候选且发生遍历错误时返回原始文件系统错误。无效查询正则当前返回空结果，不是错误。 | “无法查找文件。请检查路径后重新搜索。” / “Could not search for files. Check the path and search again.”；保留修改查询触发既有搜索，不新增扫描、文件读写或重试框架。 | `@` 状态区放本地化摘要与默认收起的技术详情，保持当前查询/会话绑定和旧请求丢弃规则。 |
| `conversation-delete-failed` → `src/app/notifications.rs:518–526` 的 `feedback_text`，上游 `conversation/deletion.rs` 发 `Feedback::DeleteFailed(String)`：可能是无法确认 Pi 退出，也可能是删除会话文件失败。 | “无法删除会话。请查看详情后再试。” / “Could not delete the conversation. See details before trying again.”；保留原有会话删除入口及确认流程，不在通知新增直接重试删除按钮，不断言仅为权限故障。 | 九语言去掉 `$error` 参数。现有错误通知通过 Kit `Notification::content` 组合摘要和默认收起的 Collapsible；保留关闭入口，为用户查看详情将此类通知设为不自动消失。仅为 DeleteFailed 携带已有原文到通知呈现，不改变其他 Feedback/通知投递，不把正文或扩展内容包装成技术错误。没有可投递窗口时，在现有事件处理处记录失败原文，避免诊断随通知丢失；不在 render 写日志。 |
| `pi-models-failed`：九语言均含 `$error`，但生产代码和测试均无调用。 | 无新增文案或恢复动作；删除九语言死 key，不为它新造界面。 | 实际设置页 `src/features/settings/pi_config/map_controls.rs:465` 已使用 `pi-models-unavailable`、既有重试及原始详情 Tooltip；不属于原始错误插值，保持现状。 |

上述诊断是 UI 操作故障。`Feedback::Message` 等其他原文通道不因此转为通用错误；工具/模型错误仍由消息拥有并原样显示。

## 必要验证

本轮文档不代表实现验收。实施后按 issue 要求在中英文下查看受影响错误摘要、可访问的详情和现有恢复操作，确认长诊断不破坏布局、已有内容与错误状态不被改写；九语言检查 key/参数一致。执行 `cargo fmt --all -- --check`，以及受影响各包的 `cargo clippy -p <包> --all-targets --locked -- -D warnings` 和 `cargo test -p <包> --all-targets --locked`；当前涉及根包与 conversation-ui，按最终实际改动确定其他包。优先复用现有覆盖；只有实际行为缺口才补少量产品断言，不为翻译常量、折叠组件或测试夹具另建测试矩阵。

实现经源代码审查后，做一次简短原生检查：中英文各观察几处代表性错误摘要与详情展开，并确认长路径不撑破布局。Computer Use 本身故障立即向协调者回报原始错误并停止该验证路径；调用参数或输入错误自行纠正。未完成的原生观察如实说明，不扩大为全量验收。

## 实现与验证记录

- 已按上表区分加载、刷新、运行、存储和各类查询摘要；错误与恢复动作仍由原 owner 管理。`diagnostic.rs` 仅组合 Kit Collapsible 与按钮，保留临时展开状态，不分类或缓存错误。
- 诊断按钮使用 `GupiErrorDetails` 焦点上下文，临时窗口的普通 Enter→粘贴回答规则仅排除此上下文，让 Enter 交给 Kit 按钮激活，避免查看详情时误粘贴；其余快捷键及用户配置机制不变。
- 删除失败沿用原通知，追加诊断详情并关闭自动消失；无目标窗口时在事件处记录原文。其余通知和 Pi 会话内容不变。九语言已删除不用的 `pi-models-failed`、`conversation-save-error`，不再插值原始 `$error`。
- 九语言 key/参数一致性、`cargo fmt --all -- --check`、`git diff --check` 通过。最终 `cargo clippy -p gupi -p gupi-conversation-ui -p gupi-settings --all-targets --locked -- -D warnings` 通过；同三包 `cargo test --all-targets --locked` 通过：根包 127、conversation-ui 221、settings 20。此前一次根包运行中，未修改的 auth 夹具 `empty_ids_keep_their_identity_through_actions_and_candidate_edits` 曾失败；未为此改动代码或测试，诊断布局修复后的上述最终完整运行通过，不据此断言已解决该偶发问题。
- 限定源码审查无阻塞。现有 Response 错误回归扩展验证 Runtime 详情点击/Enter、长路径限高，以及 480×340 实际失败正文中原始区域、重新连接与 composer 的上下边界；新增删除失败通知回归验证默认收起、长时间保留、展开及原关闭入口。没有为翻译常量建立额外测试矩阵。
- 隔离 gallery 原生观察：中英文对话/历史加载摘要、模型与思考选项查询摘要及详情；详情默认收起，点击展开、Enter 收起。24 段无空格长路径在 History 挤窄的正文、历史卡片与 256pt picker 中换行并独立滚到末尾；模型/思考两条详情同时展开仍限高，恢复入口保留。首次观察发现正文详情覆盖 composer，修复约束链、重建后复看已不覆盖；不以修复前截图作为通过证据。原生窗口缩放尝试未改变尺寸，矮窗口父级约束由上述集成回归覆盖，不宣称原生极矮窗口验收。
- 原生范围是上述代表性故障，不包含所有错误来源、删除通知真实失败、临时窗口粘贴或九语言逐一视觉验收。未调用真实模型、认证或用户会话写入；gallery 及故障 RPC 夹具均随退出删除，无保留日志/截图。
