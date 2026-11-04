# #139 迁移共享 Form 编辑会话契约

状态：迁移与约定验证已完成。一个 Gupi PR 交付，先于 [#138](https://github.com/suxiaoshao/gupi/issues/138) 的设置页组合调整。本文记录当前契约、领域边界与实际验证范围。

## 依据与范围

Gupi 实施分支从 `ff638fcb986e67381c8df1ddaeff5a660260346b` 创建，统一锁定共享库正式合入 revision `2d14af959a72850a456876267c01faf81637bc4d`，包含编辑会话契约、字段级领域三方合并 hook、接纳时映射更新，以及编辑器范围的弃稿与状态查询。

公开用法以该 revision 的 [Form 指南](https://github.com/suxiaoshao/gpui/blob/2d14af959a72850a456876267c01faf81637bc4d/crates/gpui-form/docs/guide.zh-CN.md)、[组件绑定指南](https://github.com/suxiaoshao/gpui/blob/2d14af959a72850a456876267c01faf81637bc4d/crates/gpui-form-gpui-component/docs/guide.zh-CN.md)与[宏指南](https://github.com/suxiaoshao/gpui/blob/2d14af959a72850a456876267c01faf81637bc4d/crates/gpui-form-macros/docs/guide.zh-CN.md)为准。

本项保留当前 Settings 界面与三个按页 Form。Operation 正交化、Store 精简未完成，不迁移、不预留适配；不升级 Kit，不复制共享库实现，也不为缺失的事务或绑定能力另写绕行。设计契约与本轮实际验证分别记录。

## 适配去留

| 当前 owner / 适配 | 迁移结果 |
| --- | --- |
| `pi_config/forms.rs`：`Slot::Invalid(String)`、decode/encode 分支；`inputs.rs` 与 `map_controls.rs` 的无效文本 | 删除 Invalid。解析失败返回 `EditResult::Draft`，Form 按路径和稳定 `EditorKey` 持有原文、问题与版本。`Unset / Set / Unreadable(Value)` 保留。 |
| `PiConfig.baseline: Snapshot`、`Snapshot::pending` 和普通字段叶值比较 | 删除常驻副本；从 typed path 的 `baseline/is_dirty` 读取。动态路径使用 `try_baseline/try_is_dirty`，新 occurrence 的 None 不等于 Unset。字段组合（如 provider/model）聚合正式路径状态。 |
| `Snapshot::merge`、`PiConfig::reconcile` 中 rebase 后 replace | 删除普通字段手写三方选择和两阶段安装，改 `rebase_preserving_edits`。两个 map 的领域算法交给字段 hook，在 Form 同一次事务内运行。 |
| `inputs.rs::bind`、手写刷新 input、token 的 `try_bind_control_in` | 用 `FormMappedInput` 或核心 mapped binding；应用只留下 ControlMapping 的格式、解析和上下文。删除同步订阅、回声抑制和手工 setter 投影。 |
| `Snapshot::invalid` | 删除对 Invalid 的检查；解析失败由 prepare 拦截。资源原值不可读时的编辑限制、空模型模式、空命令首参数等业务规则保留，按 typed dependencies 接入规则或保留明确的保存前领域检查。 |
| `model_maps.rs`：按业务 key/叶合并、未知 extra 与 raw 保留、顺序 | 保留为纯领域函数，接入新 hook；不扩展为通用集合框架。磁盘顺序优先，本地新增项追加；同 key 的 reserve/keep 独立处理。 |
| `Snapshot::project/read/value/leaf_values`、`fields.rs`、`resource_paths.rs` | 保留文件投影、继承、业务叶定位和写盘差量所需部分。删除后不再有调用方的辅助方法；不为了“删除 Snapshot”破坏领域桥接。 |
| `Files`、`Document`、`Scope::commit`、`gupi-resources::pi_settings` | 保留原始文件、未知键、文件基线、文件锁与叶冲突检测。Form 基线不能替代原文件，Form 冲突不能替代写盘检查。 |

写盘准备需要的基线 Snapshot 在操作当下从三个 Form 根路径读取，不另存第二个可变基线。实际提交快照和 FormVersion 是单次保存凭据，不属于要删除的常驻重复状态。资源刷新中 `preserve_dirty_resource_bases` 的文件保留策略仍是领域责任，dirty 来源换成 Form。

## 映射、草稿与动态行

映射上下文是应用计算的继承值、来源、领域限制和格式文本的只读快照；不得捕获 PiConfig 再从投影回调反读模型。`format` 保持 Unset 的有效继承展示和 Unreadable 的原始展示；`parse` 先判断回到有效基线/当前展示是否应恢复原 Slot，再执行计数、timeout、shell words 等现有领域解析。返回 Draft 时不改最后成功解析的存储值。

上下文更新调用 `refresh(context, cx)`，保留无效草稿；业务有效性变化另请求 External 校验。不能直接 `set_value` 覆盖草稿，也不能把继承变化写成显式覆盖。现有计数上限、timeout 的 off 表达、shell path 空白与 npm 参数语义不改。

EditorKey 在同一 Form 会话和同一编辑器重挂时稳定，同路径不同编辑器使用不同 key；动态身份仍由 Form 提供。恢复整个字段、设为继承等明确替换整个字段的动作先对 typed subtree 弃稿，再执行结构操作并处理 Result。map 行/叶操作则只弃掉对应稳定 EditorKey 的草稿与排队输入，不能扩展到整个 map。普通重绘、搜索、控件卸载均不清稿。

`map_controls.rs` 的 inherited token 没有 local path。绑定 map 根路径，每个业务 key / token 叶使用独立稳定 `EditorKey`。映射返回 `EditResult::update`，在原生入口解析并捕获目标叶和必要的基线/继承片段，在接纳时只修改 latest storage 中这一叶；回到继承/基线同样只恢复目标叶。部分编辑器不能返回入口时的整个旧 map。失败交给 Form 持有原文 Draft，不创建应用票据队列或迁移/伪造动态 PathKey，也不为显示继承值创建会写盘的覆盖行。

### 编辑器范围的弃稿与反馈

map 行/叶的恢复、继承和删除使用绑定的 `discard_draft(cx)`；控件已卸载时使用 `Form::discard_editor_draft(typed_path, editor_key, cx)`。它们只清除目标编辑器的草稿、问题和待接纳票据；整行删除明确处理该 key 的两个 token 编辑器，不影响邻行。

token 反馈来自 `Form::editor_status(typed_path, editor_key)`，读取已接纳的草稿与 issues；不从原生文本重做解析，不保存平行错误状态。仍有草稿的继承行即使在外部文件消失也保持可见，以便用户恢复或弃稿。根路径错误仅用于字段级展示。

## 生命周期选择

| 入口 | 策略与结果 |
| --- | --- |
| 首次加载到空会话 | `rebase(loaded, Discard, cx)`，处理 Result；没有旧用户草稿。 |
| 已确认的作用域切换 | 保留原未保存确认；切换代次后 Discard 安装新作用域，旧读取/保存结果不得落入新会话。 |
| 整页/全部放弃 | 对选中 Form `reset(Discard, cx)`，范围外不动；显式清稿和待接纳票据。 |
| 字段恢复、删除行、切换结构 | 对明确 typed subtree 弃稿；map 行/叶使用编辑器范围弃稿后修改结构。仅清这一范围，不能把生命周期失败当作全表放弃授权。 |
| 同作用域外部重读、资源刷新 | `rebase_preserving_edits(remote, cx)`，没有 Discard 参数；原子安装远端基线并保留本地值和草稿。两 map 通过新 hook。 |
| 仅继承源/catalog 更新 | 刷新 mapping context；不 replace/rebase 本地作用域。若本地文件也更新，再走上一行。 |
| 保存成功 | 每个已提交 Form 保留 `Prepared.version()`，用 `rebase_if_current(version, saved, cx)`；该入口无策略参数，必须检查 bool。未提交页按外部刷新处理。 |

`Preserve` 适用于确需替换模型但明确保留有效草稿的生命周期，不把 `reset(Preserve)` 当作放弃，也不把 `rebase(Preserve)` 当作保留已解析编辑的刷新。`WouldRetireDrafts` 保持原状态并显示可恢复问题；禁止自动重试 Discard。`EditsPending` 等原生编辑接纳完成后再处理同一刷新，并复核作用域代次；不使用 sleep、不丢弃票据。

三个 Form 是三个事务；文件快照和界面状态只能在对应安装成功后推进。不能先替换 Files，再忽略某页生命周期失败，使页面和磁盘基线错位。刷新遇到 pending 可先保留读取结果，待正式接纳后再统一处理本轮结果；不新增长期平行基线。

保存按钮及编辑禁用仍按现行规则，但禁用不代替版本保护。迟到保存成功只证明提交快照已写盘：版本拒绝时保留当前会话，不执行自动离开，不以旧 Form 基线调用 preserving rebase 强行清除 dirty。记录实际提交凭据与已写盘事实，提供现有重读/放弃恢复路径并提示仍有新编辑。若必须自动合并保存期间的新编辑，需要以实际提交快照为祖先的公开事务能力；#265 不提供，不能偷偷恢复旧手写 reconcile。正常可操作性由保存期间禁用与待接纳输入保护保证。

## 冲突与现有界面

保留当前页面错误区、字段状态与 Save/Discard/重读动作，不在 #139 引入新设置页面。Form `LifecycleReport.conflicts()` 的 PathKey 经本页正式路径映射到 Field；map hook 的冲突至少定位整个 map，具体业务叶仍由应用说明。报告带 `version()`，安装后立即确认 `is_current` 并转成稳定 Field 冲突状态，不保留会过期的动态行地址。`DraftChanged` 更新 dirty/错误展示，`SessionChanged` 更新基线相关状态，不拿这些事件无条件重设原生控件。

内存刷新冲突和 `pi_settings::Error::Conflict(leaves)` 分开记录。前者表示远端基线已推进而本地值保留，后者表示磁盘写入未成功。刷新冲突保持可见到用户明确放弃相关值或明确再次保存；再次保存代表采用当前本地选择，仍走文件锁及新一轮并发检测。不能在刷新成功时把保留下来的本地修改静默自动保存。字段 hook 若因草稿保护保留整个 map，界面说明该字段仍待处理，不能宣称所有独立键已合并。

## revision 带来的调用方迁移

升级只为 Form 契约；根 manifest 中同一共享库来源的包保持兼容的统一锁定，审核 Cargo.lock 变化，不顺带做 Operation/Store 或 Kit 重构。根包已从同一 revision 引入 `gpui-form-gpui-component`，不复制 adapter。

- 所有 total path/descriptor `set` 改为 `Result<bool, ResolveError>`：迁移 Pi、普通设置与 `gupi-settings/src/config.rs` 的调用方。固定路径保持 total，结构操作处理无法解析的路径，不把失败当成成功投影。
- `reset/replace/rebase` 显式 policy 和 Result；特别审核 ConfigController 的读取、恢复和保存分支，已有保存版本保护继续保留。
- FormEvent 新增 `DraftChanged` 和 `SessionChanged`。当前 Gupi 源码未找到直接穷举消费者，既有 observe 仍须确认只刷新页面状态，不造成输入回声。
- `FormIntegerInput::new/try_new` 新增 EditorKey 和 build error。当前 Gupi 未直接使用它；新增/迁移时按公开签名接入，不为清单虚构现存改点。Slot 数字优先 mapped input，不能把继承值强行改为裸整数。
- 旧 Validator 改显式 ValidationRule/RuleId/dependency；当前未找到旧自定义 Validator 实现。新增领域规则必须声明实际读取路径。FormSchema derive 随 revision 升级；不手写隐藏 `__merge` 或访问 `__private::MergePlan` 实现应用 hook。

## 实施与适度验证

先升级含字段 hook 的正式 revision 并解决编译期契约，再迁移映射和草稿、删除 Invalid/重复基线，最后接入原子刷新及保存凭据并删除旧 reconcile。领域映射、资源文件差量、model map 合并随正式调用方一起调整，避免留下新旧两套运行入口。一个 Gupi PR 交付，不包含 #138 换界面。

验证围绕实际风险：非法文本在重挂或继承变化后丢失、回到继承值却产生覆盖、刷新吞掉独立修改、放弃后仍有离线草稿，以及迟到保存错误标记 clean。优先调整现有 `pi_config/tests.rs` 和文件服务回归；新增测试前说明所防止的用户失败，通过正式 Form/绑定/保存入口验证，不在夹具里重写 merge 算法。不重复共享库内部票据与规则全矩阵。

## 当前实现与本轮验证

已删除 `Slot::Invalid`、常驻 `PiConfig.baseline`、`Snapshot::pending/merge`、旧 `reconcile`、手工 Input 同步和继承 token 的延迟转绑。普通输入与 token 都通过正式 mapped binding 接纳输入；部分 map 编辑只在 latest storage 上应用目标叶。两个 map 的领域函数接入字段 hook，未知内容、文件锁与叶冲突仍由文件服务负责。

首次加载和确认作用域切换显式 Discard；页面放弃只作用于选中页。外部刷新保存读取结果并逐页安装，`EditsPending` 在已有接纳回调之后重试，并检查读取代次；全部对应安装成功后才推进 Files 和继承上下文。模型/草稿事件只更新页面反馈，不手工重设输入。刷新冲突和写盘冲突分开存放；刷新冲突直到用户明确放弃相关字段或成功保存选中页才解除。

每次保存保留实际提交快照与各页 FormVersion，按版本条件安装。迟到成功保留当前编辑、阻止自动离开，并保留真实已写盘 Document 与提交凭据作为单次 receipt；该 receipt 在资源差量基线投影之前记录。显式放弃相关已保存页后，从已写盘结果恢复；其余页仍按保留编辑刷新处理，不用旧 Form 基线模拟保存期间合并。

本轮实际在 `nix develop` 内执行，构建缓存使用独立目录 `/private/tmp/gupi-139-target`，未改动仓库的 target 链接：

- `cargo fmt --all -- --check`：通过。
- `cargo check -p gupi --locked`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过，包含根包及受影响成员。
- `cargo test --workspace --locked`：整个 workspace 及 doc tests 通过，其中根包 133 项、资源 crate 95 项、设置 crate 21 项。保留原有两个需要 installed Pi CLI 的 ignored 集成测试，未修改 ignore。
- `cargo build -p gupi --locked`：通过，用于下述原生试用。
- `git diff --check`：通过。

新增产品级保障覆盖：同批原生输入编辑两个继承键后保存，文件只出现目标 override；两个 map 的实际 reload hook 合并独立业务键/叶并保留未知原值；外部刷新冲突持续可见；迟到保存不会清除新编辑或自动离开，即使回到旧基线仍保留已写盘 receipt，显式放弃后恢复真实保存结果；已接纳、排队或控件卸载后的目标 token 恢复不会清掉邻行草稿；恢复 Unreadable token 的原始展示会清稿而不改原值；新本地 token 与远端不可读 map/row 相交也显示刷新冲突。迁移旧测试通过 native Change 入口提交无效文本并断言 Form 状态，不再把坏文本写入 typed model，也不重复共享库票据/更新回归矩阵。九种语言的 Fluent 解析和 key/参数一致性通过；未新增 trybuild 快照。

### 一次隔离原生试用

在 macOS 使用当前 Nix 构建的真实应用与 Pi 1.5.0 完成一次试用。临时 app 的编译引导器与 bundle 环境均固定 `HOME`、Gupi config/log/data 和 Pi agent/session 目录到 `/private/tmp/gupi-139-native-pdo_z43a`，启动目录固定为其空 project；目录不存在则拒绝启动，不回落默认路径。命令环境不继承凭据，Pi 使用 offline/no extensions/skills/context 等参数。修正隔离 fixture 最初漏掉的 Node PATH 后，重新启动同一个隔离 app；未读取或修改用户真实 Pi 配置。

实际操作与结果：

1. 两个 `trial/a`、`trial/b` map 行中，给 a 的 reserve 输入无效文本，同时把 b 的 keep 从继承值改为 22000。只在 a 显示整数错误，b 的编辑保留。
2. 外部修改隔离 settings.json 中两个 reserve 与全局 keep，再通过页面 Reload 重读。a 的无效原文和 b 的已解析编辑都保留，继承的 keep 更新为 21000。界面明确说明带草稿的整个 map 被保留，需要修正/弃稿后再合并。
3. 对 a 执行 reserve 的 Undo change：恢复远端 150 并清掉该编辑器错误，b 的 22000 仍在。字段继续说明本地选择与外部变更的冲突，直到用户明确保存。
4. Save 成功后页面 clean；直接审核隔离文件确认 a=150、b.reserve=200、b.keep=22000，两个未知键均保留。b.reserve=200 是整字段草稿保护保留下来的本地选择；本次 Save 明确采用它。a 的继承 keep 没有被写成显式覆盖。
5. 再给 a 的继承 keep 输入无效文本，执行页面 Discard，恢复显示 21000、错误消失、页面 clean，文件内容未改变。随后关闭隔离 app。

本次试用覆盖原生无效输入、继承、map 冲突、外部刷新及保存/放弃；不代表 Linux/Windows 原生验收、所有设置页或签名发行验证。试用未使用真实模型凭据、发起模型请求或覆盖 #138 的界面调整。

主要原生流程在最后的 Unreadable token 恢复及 opaque parent 冲突修正前完成。这两个边界通过最终 133 项根包测试验证；尝试在同一隔离 fixture 补查原生 raw token 时，CUA 持续返回 `cgWindowNotFound`，未能取得窗口，故不宣称该补查通过。最终二进制构建成功；此工具捕获限制与上面已完成的原生操作分开记录。
