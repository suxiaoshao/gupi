# Pi RPC/TUI 接入盘点与能力差距

本页集中维护 **33 个 RPC 命令、24 种事件名、9 类扩展 UI** 与 Gupi 的接入状态，以及 TUI 有而标准 RPC 未提供的能力。剩余工作的优先级、归属和验证边界统一见 [总待处理文档](follow-ups.md)，这里不再维护第二份排期清单。

2026-09-30 复核应用基线 `5baab1c5`、官方最新正式 **v0.99.1** `d86654abb8862e201933517d6f1fce9f88dd117f` 和 main 快照 `1b347794e2a630e4359f2584f4eea388145d0ddf`。本机 Pi 已安装 0.99.1；源码仓库 `/Users/sushao/Documents/code/pi` 从 `898ab804` 快进到上述 main，更新前后工作区干净。安装产物中 rpc-types/rpc-mode 的 source map 源码与 v0.99.1 完全一致。本次未修改全局 Pi 配置或安装版本，也未实现下述应用适配。

统计口径：传输层 `request_raw` 能发命令不等于产品有入口，fixture 调用不算应用接入；没有直接调用同名 RPC 也不等于缺功能。标准 RPC、TUI、实验 Harness 和社区方案分别判断。

## 正式版和 main 复核

| 检查 | 结论 |
| --- | --- |
| v0.87.1 → v0.99.1 | 命令名、事件名和扩展 UI 集合不变，仍为 33 / 24 / 9；`prompt`、`steer`、`follow_up` 成功响应新增 `data.disposition`。工具事件和历史消息新增字段，不能用命令数量不变推断协议没有变化 |
| v0.99.1 → 已核对 main | RPC 目录、SDK、AgentSession 和 TUI 内置命令表无差异；coding-agent 的未发行修改位于 ModelRuntime 默认模型/凭据处理，不增加下述 RPC 能力 |
| SDK 与 Harness | `sdk.ts` 仍构造 `new Agent(...)`，未切到实验 Harness。Harness 的队列 ID、完整消息与 cancelQueued 不能作为标准 RPC 可用接口 |
| durable / protocol / server | 这些底层和实验设施继续独立于 coding-agent stdio RPC；目录变化不解除 Gupi 的完整队列或树导航限制，也不要求迁移存储 |
| v0.99.1 发行变化 | 新模型目录及 OpenAI 登录打包修复；RPC 更新来自 v0.99.0。模型与思考选择继续读取 Pi 动态返回值 |

## Pi 0.99 新增内容与 Gupi 接入机会

| 更新 | 正式源码与实际传输 | 对遗留工作的影响 |
| --- | --- | --- |
| 每次输入的处理结果 | `prompt` 返回 `handled` / `queued` / `started`；独立 `steer`、`follow_up` 返回 `handled` / `queued`，都在 `data.disposition` | 过去无法从成功响应区分插件消化和实际入队的限制已解除。Gupi 的 `Client::prompt` 保留 Response，但提交分支使用 `Ok(_)`，尚未消费结果；可补 typed 结果和提交语义，保留会话绑定、草稿版本和附件清理保护 |
| 嵌套工具的实时父子关系 | `ctx.executeTool()` 的三类 `tool_execution_*` 事件带 `parentToolCallId`，独立 `toolCallId` 关联该子调用 | 可给 codemode/MCP 的子调用增加归属和实时展示。Gupi 当前 ToolActivity 未保存父 ID，正文工具投影依赖 assistant 中的顶层 toolCall，不能把收到子事件算作已显示子调用 |
| 嵌套调用的历史记录 | 父工具的 toolResult 消息保存 `nestedCalls: { calls, complete }`；记录 ID、名称、参数、状态、耗时和截断后的错误，不保存子工具的完整结果 | 可在重新打开历史后显示有限调用记录；最多 256 次、单项参数 8 KiB、总参数 32 KiB，`complete:false` 时必须保留不完整语义。Gupi 保留原 JSON，但尚未将此字段投影到工具详情；不能承诺重启后恢复完整子输出 |
| 工具结构化输出与费用 | 实时工具结果可带 `structuredContent`、`isError` 和 usage；MCP 工具提供结构化 CallToolResult，嵌套模型用量计入父结果及 `get_session_stats` | 可补工具详情对结构化输出的读取；现有详情主要读 content/details。`structuredContent` 未由普通 ToolResultMessage 持久化，不能承诺它与实时事件在历史中一致；累计费用继续以 Pi stats 为准，不重复累加 nestedCalls |
| 内置 MCP / codemode / tool_search | Pi 负责连接、工具发现、执行和权限钩子；`get_commands` 可返回 `sourceInfo.path = builtin:mcp` 的 `/mcp` 扩展命令。RPC 下无参数 `/mcp` 降级为 notify，login/logout/reconnect 子命令支持无 TUI 调用；图形管理器仍属 TUI | 已配置的 Pi 能通过既有 RPC 执行这些工具，无需 Gupi 自建 MCP runtime。管理配置可在 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 选字段时另行评估；安装包市场 [Gupi #3](https://github.com/suxiaoshao/gupi/issues/3) 的目录/元数据前提没有因此解决。`builtin:*` 是来源标识，不能当成文件路径 |
| 首条用户消息及时落盘 | SessionManager 在第一条 user 或 assistant 消息出现时创建文件，setup 元数据独自不落盘；不再等待第一条 assistant 回复 | 解除普通持久会话首次回复前退出导致输入未保存的问题。Gupi 沿用 Pi 存储即可；临时会话的 `--no-session` 及缺诊断日志不受此修复影响 |
| 虚拟模型、图片/分类模型与 HTML 导出 | 虚拟模型可经既有聊天模型选择，实际回答记录物理模型；图片生成/分类是 ModelRuntime 与扩展能力，没有新增对应 RPC 命令。HTML 导出新增 display:false 插件消息的手动显隐 | 模型选择和 export_html 可复用已有入口；实际模型展示、图片/分类专用界面是可选范围，不能视为新增 stdio API。新的 provider_stream_event 仅发给扩展，不新增客户端事件 handler |

`disposition` 描述这一份输入的结果：`handled` 不要求这份输入产生 agent_start/agent_settled，但插件可以独立启动其他工作；`queued` 不保证条目仍留在队列，也不携带 ID；`started` 表示启动已获接受，不代表模型回答完成。运行状态继续由实际事件和 `agent_settled` 决定，不能从 handled 响应强制将已有运行设为空闲。

2026-09-30 使用本机安装的 0.99.1、隔离 agent/cwd/session 目录、`--offline`、显式临时扩展和本地模拟 provider 完成协议检查，无真实模型或 MCP 网络请求：验证扩展命令/input handler 的 handled、输入变换后的 steer/follow-up queued、普通 prompt started、运行中 prompt queued，以及 clear_queue 返回变换后的文字；验证首条 user 在 assistant 回复前已落盘；验证子调用事件的 parentToolCallId/structuredContent 和 get_entries 中父结果的 nestedCalls。此为 Pi 协议验证，不是 Gupi 原生界面或应用适配验收。

## RPC 命令接入

33 个命令：17 个直接调用，10 个有替代或主要能力，6 个未接入。

下表按命令名计数，每个名字只归入一类。替代并不表示逐项行为完全相同，差异写在相应行；未接入不自动等于必做。

| RPC 命令 | Gupi 当前接入 | 未覆盖内容／处理判断 |
| --- | --- | --- |
| `prompt` | 直接调用；文字、图片及 `streamingBehavior` | Skill/模板展开由 Pi 负责；新增 `data.disposition` 已保留在 Response.data，但应用尚未消费，见上方 0.99 接入机会 |
| `abort` | 直接调用；停止当前会话 | Pi 的 `abort()` 同时取消重试、压缩和分支摘要；不能把缺专用停止按钮算作完全不能停止 |
| `get_state` | 直接调用；握手、模型与会话状态、排队数量 | `steeringMode`、`followUpMode`、`messageCount` 留在 extra 中，未提供配置/统计展示；自动压缩状态已有上下文 tooltip |
| `get_commands` | 直接调用；插件命令、模板、Skill 候选，含加载的内置扩展命令 | 参数补全和插件键位不在返回值内；`builtin:*` 来源不代表本地资源文件；不返回 TUI 内置命令表 |
| `get_available_models`、`set_model` | 直接调用；模型选择器及模板任务覆盖 | 模型范围管理不等同于模型选择器 |
| `get_available_thinking_levels`、`set_thinking_level` | 直接调用；模型对应思考等级及模板任务覆盖 | TUI 保存默认思考等级是另一项配置行为，不由当前会话选择自动完成 |
| `compact` | 直接调用；手动压缩及停止 | 未暴露可选 `customInstructions`；自动压缩失败详情另见事件表 |
| `get_entries` | 直接调用；完整条目、历史树、消息投影 | 未用可选 `since` 增量读取；这是优化候选，不是缺少历史功能 |
| `get_fork_messages`、`fork` | 直接调用；指定用户消息分叉 | 不等同于在原文件任意树节点继续，见 RPC 缺口 |
| `clone` | 直接调用；复制当前会话 | 已有，不重复列为待实现 |
| `export_html` | 直接调用；系统保存窗口指定 `outputPath` | 未用 Pi 自动选默认输出路径；无需为此另加入口。JSONL 导出属于文件能力 |
| `set_session_name` | 直接调用；离线会话也先连接 Pi，再执行改名 | 已有，不重复列为待实现 |
| `get_session_stats` | 直接调用；token、费用、context 用量 | 身份、路径及消息/工具计数已在会话信息弹窗展示 |
| `steer`、`follow_up` | 未直接调用；通过 `prompt.streamingBehavior` 接入运行中 steer/follow-up | 已有主要提交能力；独立命令返回 handled/queued，不返回 started；始终入队语义不与 prompt 的空闲直接执行混同 |
| `new_session`、`switch_session` | 未直接调用；Gupi 新会话/恢复使用独立或复用实例，以及 `--session` | 已有多会话新建/恢复；不为用满 API 而强切一个 runtime。`parentSession` 也未作为普通新建参数提供 |
| `cycle_model`、`cycle_thinking_level` | 未直接调用；已有显式模型/思考选择器 | 无循环切换动作；是否需要快捷操作待选，不影响现有选择能力 |
| `get_tree` | 未直接调用；由 `get_entries` 在本地构建历史树 | 已有历史预览，不重复新增读取；树导航语义缺口仍存在 |
| `get_messages` | 未直接调用；消息界面由 entries 和实时事件构建 | 已有消息展示；若要查看 Pi 精确模型上下文再评估此接口，历史视图不等同于模型当前上下文 |
| `get_last_assistant_text` | 未直接调用；从实际执行分支的原始历史提取最后回答 | 0.87 的此 RPC 从 Pi 的上下文投影取值，受 context_edit 省略/替换影响，不再假定与界面最后可见回答始终一致。现有复制/回填继续以用户看到的回答为准 |
| `abort_retry` | 未直接调用；通用 `abort` 已会取消重试 | 仅缺“只取消重试”的专用动作，不能列为无法停止重试 |
| `clear_queue` | 直接调用；清空全部、全部文字取回草稿 | 返回值仅含两类文字数组，排队图片无法恢复；逐条操作继续延后 |
| `set_steering_mode`、`set_follow_up_mode` | 未接入 typed 方法和设置 UI | 由后续 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 选择字段与生效范围。Pi setter 会写其设置，不能当作纯临时会话字段 |
| `set_auto_compaction` | 未接入开关；只展示已有状态 | Pi 自动压缩仍正常工作；修改入口归后续 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 的字段选择，setter 会写 Pi 设置 |
| `set_auto_retry` | 未接入开关 | Pi 自动重试仍工作；修改入口归后续 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 的字段选择，setter 会写 Pi 设置 |
| `bash`、`abort_bash` | 未接入用户命令执行及独立中止 | TUI 的 `!` / `!!`、`excludeFromContext`、输出流和取消需要一起定范围。已有模型工具的 bash 卡片不是此能力；不能通过普通 prompt 冒充执行 |

## 实时事件接入

`pi-rpc::Event::Agent` 会保留原始 JSON，但 Gupi 只对部分 kind 更新状态。下表覆盖 24 种事件名（含 `extension_error`），不把原样传输当作界面已消费。

| 事件 | 当前行为 | 剩余内容／判断 |
| --- | --- | --- |
| `agent_start`、`agent_end`、`agent_settled` | start 更新运行状态；settled 定向校准并收尾；end 不单独触发回读 | 已有；不把 prompt 接受当作任务结束 |
| `turn_start`、`turn_end` | 没有单独 handler；消息和工具使用更细粒度事件 | 暂无独立轮次 UI 需求，不仅为了消费事件增加界面 |
| `message_start`、`message_update`、`message_end` | 接入消息增量、思考、文字、工具调用及结束校准 | 已有；特定消息种类的展示不能因此一概视为完整 |
| `tool_execution_start`、`tool_execution_update`、`tool_execution_end` | 接入顶层工具状态、部分结果及结果卡片；原始 JSON 保留新增字段 | 0.99 子调用的 parentToolCallId、结构化结果及历史 nestedCalls 尚未消费；不继承插件 TUI renderer |
| `compaction_start`、`compaction_end` | 更新压缩状态并刷新历史；#241 已保留未取消且不再重试的自动压缩失败详情 | 手动压缩以 RPC 结果提示，避免与事件双报；中间重试/取消不增加最终失败通知 |
| `auto_retry_start`、`auto_retry_end` | 已更新 retrying、尝试次数、等待倒计时及原因 | 重试等待不回读正文；最终错误提醒已纳入 #241，中间重试不逐次通知 |
| `queue_update` | 已消费，实时同步两类文字和 pending 数量 | 队列可折叠查看；状态快照仅能补充数量，逐条操作和完整附件仍受协议限制 |
| `entry_appended` | 已接入来源会话的历史/用量同步 | Pi 0.87.0 的边界钩子还可追加 custom_message、context_edit、compaction；已有定向校准保留。display:true 插件消息已接入正文，不将所有结构条目都当成聊天消息 |
| `session_info_changed`、`thinking_level_changed` | 已接入名称、思考等级与定向状态校准 | 保留清空名称语义及模型设置请求与事件的竞态保护，不因此扫描全部目录 |
| `summarization_retry_scheduled`、`summarization_retry_attempt_start`、`summarization_retry_finished` | 已接入独立的摘要重试等待阶段 | 本地倒计时；实际尝试开始/结束清理本阶段，不误清普通模型重试或将其当作整个任务结束 |
| `bash_execution_update` | 未消费 | 与用户 bash 功能共同评估，非模型 `tool_execution_update` |
| `extension_error` | 来源会话保留错误详情并按插件提醒路由 | 不改变任务结果；实现与原生验证边界见 [通知设计](issue-241/README.md) |

### 事件同步、错误提示与局部刷新

上述接入盘点继续作为能力索引；事件契约、局部刷新实现和验证边界统一见 [#236 实现说明](issue-236/README.md)。名称/思考等级/entry 同步、压缩状态、两类重试进度及过宽刷新修正已完成受影响验证，不再列为待处理项。外部会话由用户手动刷新；首次新项目定向发现、后台删除保留选择、设置资源首次按需加载均已落实。

错误、插件提示、待答和回答完成已接入应用级投递，阅读计数与业务状态分离；原生验证边界见 [通知设计](issue-241/README.md)。提交阶段失败保留应用内错误反馈，不额外发系统通知；点击已有通知仅回来源，不隐式重连。`queue_update` 已接入，逐条操作仍受协议限制；turn 事件不另建 UI，直接 Bash 仍另定范围。状态通知 `cx.notify` 与应用内/系统用户提醒是不同层次，不据通知次数推断所有 Pi 实例重新加载。

## 扩展 UI 接入

| 请求／回复 | 当前行为 | 剩余内容 |
| --- | --- | --- |
| `select`、`confirm`、`input`、`editor` | 都已接入，回复带请求 id，使用 value/confirmed/cancelled；editor 支持 prefill，前三者处理 timeout | 组件映射与受影响交互验证已完成，见 [#243](issue-243/README.md)，复用 #222 体验入口；保留 input 空值与 editor 多行语义，不能记成四类功能都没实现 |
| `notify` | 保留来源/request id 和 info/warning/error，统一应用内/系统路由 | 来源保留多条临时提醒，默认不发系统通知，也不直接增加未读数字 |
| `setStatus`、`setWidget` | 已展示/更新/清理文本状态及上下方文字 widget | 只支持 RPC 提供的字符串数组；组件工厂不是应用漏接 |
| `setTitle`、`set_editor_text` | 已更新扩展标题和会话输入文字 | set_editor_text 作为纯文本替换，清除旧标签并保留附件 |
| `extension_ui_response` | 已支持三种回复形态 | 它是 UI 应答，不额外计入 33 个命令 |

## 队列边界

- **已经提供**：运行中提交 steer/follow-up、每份输入的 disposition、`queue_update` 两类排队文本、`clear_queue` 清空并返回文本、两个队列模式 setter。用户已确认逐条返回草稿、编辑和删除的设计目标，详见[队列交互草稿](issue-222/queue-composer.md)；disposition 不提供队列身份，现有接口尚不足以直接实现完整目标。
- **没有专用 RPC**：无副作用主动读取队列的 `get_queue`、按消息 ID 修改/删除某一项、包含图片等附件的完整队列快照。事件和清空结果只有字符串数组，不能声称能够无损恢复排队图片。
- TUI 的“编辑排队消息”实际是将全部队列取出、拼接进编辑器，不是任意逐条在线编辑 API。不要把候选功能写成上游已经提供。
- 继续调研发现本地 Pi 新 Harness 已有 entryId、完整消息与 cancelQueued，但 coding-agent RPC 尚未接入；公开扩展 API 也不能操作已排队单条。具体可行性、消费竞态和推荐契约见[队列交互草稿](issue-222/queue-composer.md#逐条操作的可行性与实现边界)，不据此维护第二套客户端队列。
- TUI 在压缩期间另有界面层 `compactionQueuedMessages`；Gupi 当前压缩时限制发送。若要同等体验，属于新增客户端队列策略，尚未授权实现，也不恢复先前删除的失败输入恢复队列。

## Pi 0.87 语义变化与当前接入

| 变化 | 当前代码核对与结论 |
| --- | --- |
| `context_edit` / canonical session context | Pi 用 append-only 记录省略或替换模型上下文贡献，原始历史不改。Gupi 的开放 SessionEntry 类型保留字段，历史“全部”中作为通用事件，正文不应用这些编辑，符合原始对话阅读语义。不要把它误当成删除聊天消息，也不另造上下文调度器；专门展示“对模型已隐藏/替换”属于可选产品范围。 |
| 可执行插件边界 | `turn_end` 新字段、`agent_before_settle` 和 `context_with_system` 属于扩展层；后两者不是新增的 stdio RPC 客户端事件。RPC 继续转发 AgentSessionEvent，不能因为 release 的 ExtensionEvent union 增加成员就给客户端虚构 handler。Gupi 继续以 agent_settled 收尾，符合插件可能延续运行的语义。 |
| `entry_appended` 的更多来源 | 边界提交可能包含 custom、custom_message、context_edit、compaction，失败恢复也会追加 context_edit。Gupi 已为任意 entry_appended 定向回读历史和用量；需区分“数据已同步”与“正文是否显示”。display:true custom_message 已独立展示，display:false 保留原历史但不显示正文；普通结构元数据不应一律变成气泡。 |
| retain-none 压缩 | Pi 将压缩记录自身 ID 作为 firstKeptEntryId。Gupi 展示原始历史和压缩摘要，不在 GUI 复刻 Pi 上下文裁剪；不据此删除压缩前消息。此新形态尚未单独进行运行中兼容验收。 |
| 按模型图片处理 | AgentSession 已根据当前模型 inputLimits.images.resize 和 images.autoResize 归一化 RPC prompt 图片，之后才构建持久用户消息；CLI/read/工具图片也复用 Pi 的策略。Gupi 继续发送原始字节、保留原图预览，不复制这些限制。恢复历史中的图像可能已由 Pi 处理，不能承诺一定等于导入原图。 |
| TUI `/bug` | 内置命令从 23 增至 24，没有对应 RPC。诊断上传/本地 ZIP 导出属于独立产品与数据范围，不自动给 Gupi 增加上报功能。 |

## 原生 RPC 的主要缺口

| 能力 | 源码事实 | Gupi 的影响 |
| --- | --- | --- |
| 插件子命令／参数补全 | `get_commands` 仅返回名称、描述、来源；没有 `getArgumentCompletions` 查询命令，`addAutocompleteProvider` 为空实现 | 已能补全命令名，参数仍需手动输入 |
| 插件自定义快捷键 | RPC 不枚举、不触发 `registerShortcut` 的回调；`onTerminalInput` 为空实现 | 无法自动继承插件快捷键；Gupi 自己的 action/keybinding 不受影响 |
| 插件自定义 UI | `custom` 返回 undefined；header/footer/editor 和 widget 组件工厂不传输 | 无法直接复用任意 TUI 弹层、阅读器和组件；插件需提供标准 RPC 降级路径 |
| 插件自定义消息／工具渲染 | `renderCall`、`renderResult`、message renderer 返回本地 TUI Component，RPC 传输事件数据 | 可显示文本、工具数据，无法自动继承插件渲染函数 |
| 组合问卷 | `select` 是字符串选项和单字符串返回，没有原生多选、附加说明、多字段 schema | 不能通用推断问卷结构；只能由插件顺序调用标准交互或解析输入文本 |
| 插件读取输入、粘贴语义 | `getEditorText` 返回空字符串，`pasteToEditor` 降级到 `setEditorText`；AbortSignal 取消问答不发送专用撤销事件 | 插件读不到宿主正文；粘贴退化为替换文字；当前处理已传 timeout 和来源关闭，不能即时感知未传输的插件主动撤销 |
| 插件控制显示细节 | 工作提示／动画、隐藏思考标签、主题查询／切换、工具展开 API 在 RPC 下为空或不支持 | 插件设置无法驱动宿主；Gupi 自己实现对应显示能力仍可行 |
| 同文件树节点续聊 | 有 get_entries/get_tree/fork，没有直接 navigate_tree RPC | 历史预览和用户消息 fork 已有；任意节点原地续聊及导航总结／标签未接入 |
| 进程内资源重载 | 没有直接 reload RPC | Gupi 采用重启连接，不能完整保留原进程插件内存及生命周期 |

树导航和 reload 已在 RPC 模式的扩展命令上下文中绑定到 Pi 核心。因此“缺直接 RPC”不等于核心不支持；额外桥接扩展可以调用，但目前未采用私有桥接。

源码依据（相对于 Pi 仓库根）：

- `packages/coding-agent/src/modes/rpc/rpc-types.ts`：客户端命令及九类扩展 UI 消息。
- `packages/coding-agent/src/modes/rpc/rpc-mode.ts`：空实现、get_commands，以及扩展上下文的 navigateTree/reload 绑定。
- `packages/coding-agent/src/core/extensions/types.ts`：插件补全、快捷键和 TUI 渲染函数契约。
- `packages/coding-agent/src/modes/interactive/interactive-mode.ts`：TUI 补全与快捷键接入。

## 缺管理接口，但可以由应用另行接入

| 能力 | 可用基础与边界 |
| --- | --- |
| 插件／包管理 | 已由 #231 使用 Pi CLI 接入个人包安装、更新、移除及资源启停；无需把无 RPC 等同为无法管理 |
| Skill、模板、全局提示词管理 | #231 已接入个人资源文件、配置及 SYSTEM.md/APPEND_SYSTEM.md；包内资源只读，修改后由用户手动刷新会话 |
| Pi 设置、模型范围管理 | 部分设置有专用 RPC；其余读写由独立后续 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 确定，没有通用设置编辑 RPC |
| MCP、codemode 配置 | 已有 Pi 的 mcp.json、defaultTools/codemode/内置扩展设置和 `pi mcp` CLI；配置管理没有新增通用 RPC。[Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 实施时可评估字段、继承/项目信任和生效规则，不将其自动纳入已确认范围 |
| 登录／退出登录、信任管理 | 无对应 RPC 工作流，沿用外部 Pi 管理；若改变此范围，需要另行确定认证／信任机制 |
| JSONL 导入导出、分享 | 可由应用提供文件管理或发布能力；不属于复用一个现成 RPC 的工作 |
| 参数表单、必填项及忙碌可用性 | get_commands 缺少结构化元数据，无法自动生成可靠 UI；不自行猜测 |

## TUI 客户端能力差距

| 功能 | Gupi 当前情况 | 判断 |
| --- | --- | --- |
| 输入历史、外部编辑器、路径补全 | `@` 文件/目录候选和原子路径标签、系统选择器/拖入/粘贴、图片附件均已接入；输入历史和外部编辑器未接 | 后两者是未选择的客户端能力，不是缺少 RPC，也不自动加入本轮 |
| 工具统一展开、思考显示切换、历史树筛选 | 已有逐项折叠、三级历史详情与树预览；TUI 的特定全局动作/仅用户/仅标签筛选不完全对应 | 现有能力可用，是否增加这些操作待选，不因快捷键名字不同判缺失 |
| 会话选择器命名筛选、排序切换、路径显示 | 已有目录分组、搜索、时间排序、路径操作 | 不需新 RPC；额外筛选/排序选项属于体验候选 |
| 正文搜索、会话信息页、`hotkeys` 别名、Pi changelog 入口 | 当前分支正文搜索、会话信息弹窗和快捷键设置均已有；别名与 changelog 入口未接 | 后两项为未选择的入口完善，见 [24 项内置命令对照](issue-226/builtin-commands.md)；搜索的正式依赖缺陷另见总清单 |
| 终端挂起、终端主题/按键协议/全屏与滚屏控制 | 原生桌面窗口已有自身的窗口、主题和剪贴板机制 | TUI 专属机制，不列为桌面缺陷 |

## 有保留价值的上游参考

当前可用性以以上固定源码为准。以下仅提供与缺口直接相关的历史方案入口，不承诺当前审查状态或上线时间。

| 缺口 | 历史方案／讨论 | 使用边界 |
| --- | --- | --- |
| 参数补全 | [#7621](https://github.com/earendil-works/pi/pull/7621) | get_argument_completions 的实现参考，正式 0.99.1 和已核对 main 仍无该命令 |
| 同文件树导航 | [#1762](https://github.com/earendil-works/pi/pull/1762) | 区分导航与另建文件 fork；当前没有公开 navigate_tree RPC |
| 更广的 Web GUI/RPC 接口 | [#8840](https://github.com/earendil-works/pi/pull/8840) | 社区私有协议参考，不作为官方能力或替换依赖依据 |
| 输入处理结果 | [#9098](https://github.com/earendil-works/pi/issues/9098)、[#9803](https://github.com/earendil-works/pi/issues/9803) | 正式 0.99 已提供 disposition，以发布源码为准；该字段不携带逐项身份或完整队列，不作为单条编辑/删除接口 |

## 实验协议的边界

Pi 的 `packages/protocol`、`packages/server`、`packages/durable` 和实验 Harness 是另一层架构。标准 coding-agent stdio 仍走 RPC mode；研究这些设施时应检查实际 SDK 接线、发行入口与协议契约，不以目录存在、包版本号或底层 JSONL 存储支持推断 Gupi 已可使用。当前继续按用户确认使用标准 RPC，不采用社区 fork 或自建桥接。

## 固定源码依据与后续复核方法

应用入口：[typed RPC](../../crates/pi-rpc/src/protocol.rs)、[Client](../../crates/pi-rpc/src/client.rs)、[会话事件与提交](../../src/state/conversation.rs)、[定向回读](../../src/state/conversation/reads.rs)、[输入区](../../src/features/home/composer.rs)、[会话信息](../../src/features/home/session_info.rs)、[全局模板任务](../../src/app/shortcuts.rs)。

- 正式 [v0.99.1 release](https://github.com/earendil-works/pi/releases/tag/v0.99.1)、[v0.99.0 release](https://github.com/earendil-works/pi/releases/tag/v0.99.0) 与 [CHANGELOG](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/CHANGELOG.md)。旧协议比较基线为 [v0.87.1](https://github.com/earendil-works/pi/tree/f07218c4d4bbc12bef056a7058c3dd49dfe41abe)。
- [RPC 类型](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/modes/rpc/rpc-types.ts)、[分发/扩展降级](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/modes/rpc/rpc-mode.ts)、[SDK](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/core/sdk.ts)。
- [AgentSession](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/core/agent-session.ts)、[SessionManager](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/core/session-manager.ts)、[扩展 API](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/core/extensions/types.ts)。
- [嵌套调用与记录上限](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/core/nested-tool-calls.ts)、[工具结果消息构造](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/agent/src/agent-loop.ts)、[MCP 扩展与 RPC 降级](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/src/extensions/mcp/index.ts)。
- [已核对 main](https://github.com/earendil-works/pi/tree/1b347794e2a630e4359f2584f4eea388145d0ddf)。官方协议文档为 [RPC commands](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/docs/rpc-commands.md) 和 [Extension UI](https://github.com/earendil-works/pi/blob/d86654abb8862e201933517d6f1fce9f88dd117f/packages/coding-agent/docs/rpc-extension-ui.md)；字段可用性另以类型、实现和运行结果核对。

升级复核时先比较正式标签与 main 的 rpc-types/rpc-mode、SDK、事件及历史格式，再核对 Gupi 实际调用与消息投影。版本号、PR 合并日期和社区自述不能替代发布源码和针对性运行证据。
