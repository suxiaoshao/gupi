# Pi 配置页迁移到 gpui-form（[Gupi #54](https://github.com/suxiaoshao/gupi/issues/54)）

状态：In progress。对账后保存语义按用户决定保持现状；下文验证计划与本轮实际结果分开记录。

## 目标与范围

Pi 配置页（`src/features/settings/pi_config`）的草稿从 `drafts: BTreeMap<Field, Edit>` 改由 gpui-form 持有；模型范围列表行使用 `ItemPath::key()`，删除 `scope_rows` / `next_scope_row` 等手写行身份。为 [#47](https://github.com/suxiaoshao/gupi/issues/47) 的路径数组和 [#50](https://github.com/suxiaoshao/gupi/issues/50) 的按模型映射提供同一模式。

用户可见行为不变，保持首批与模型范围契约（[首批实施方案](../issue-2/implementation.md)、[页面设计](../issue-2/page-design.md)、[模型范围](../issue-2/model-scope.md)）：

- 全局 / 项目作用域与继承、来源列、字段菜单（撤销更改、设为本作用域值、恢复继承/默认）。
- 按页保存与离开保护（含保存中排队离开）；锁内重读、只写变更叶子、叶冲突检测、锁占用提示；未知字段保留。
- 继承来的模型范围列表可直接编辑、排序、删除，首次编辑即成为本作用域值。
- 信任状态与信任动作、仅全局字段（`httpProxy`）在项目作用域只读跳转。
- 保存只写文件，已运行会话靠手动重连生效；不新增自动同步。

不做：新增字段、改变界面布局或文案、改动 `gupi-resources::pi_settings` 存储契约、升级 gpui-form revision。

## 依据（固定 revision `9d7eb49`）

- `#[form(items)]` 只接受 `Vec<Item>` 且 `Item: FormSchema`；`Option<Vec<_>>` 不能直接标 `items`，`String` 不能作为 item。`#[form(child)]` 接受 `Child`、`Option<Child>` 和 FormSchema 枚举（变体为单元或单个具体载荷）。普通叶字段只要求 `T: 'static`，可以是泛型值类型。
- `is_dirty` 只比较模型与基线（`form.rs`）；`defer_set_issue` 只发布校验问题，不改模型、不推进 `FormRevision`。
- `replace` / `reset` / `rebase` 重建拓扑与校验，并对所有绑定排队投影；绑定投影成功后清除该控件的问题（`control.rs` `schedule_drain`）。投影在 `cx.defer` 中执行，回调读取 `PiConfig` 不重入。
- 本方案的 schema 不声明 `required`、不安装校验器、不使用控件问题和异步校验，因此 `prepare` 不会失败，只用于取得带 `FormVersion` 的快照；这不是 gpui-form 的通用保证。
- 应用已有自定义绑定先例：`src/features/settings.rs` 的 `bind_control_in` + `ControlWriter::defer_set`。

由此得出两条约束：所有需要离开保护、在重读/保存后保留的编辑状态都必须在**模型**里；不能依赖原生控件问题或 `FormVersion` 表示草稿。

## 设计

### 表单所有权：每页一个 Form

`PiConfig` 持有 `Form<ConversationDraft>`、`Form<NetworkDraft>`，字段按 `Field::page()` 划分且互不相交。按页拆分保证一页的无效草稿不影响另一页保存。保存全部（离开对话框）时两页一起检查和写入。

`Files`（全局/项目原始 `Document`）、继承解析、默认值、信任、冲突列表、模型目录仍由 `PiConfig` 持有；表单只表示**被编辑作用域文件中**已知叶子的值，不含继承值。

### 草稿模型

```rust
/// One leaf of the edited scope file.
#[derive(Clone, Debug, PartialEq)]
enum Slot<T> {
    /// Absent here, so the value is inherited.
    Unset,
    Set(T),
    /// Present on disk but not representable; kept verbatim unless replaced.
    Unreadable(serde_json::Value),
    /// Text typed here that is not a valid value yet. Never read from disk.
    Invalid(String),
}

#[derive(Clone, PartialEq, FormSchema)]
struct ConversationDraft {
    default_provider: Slot<String>,
    default_model: Slot<String>,
    thinking: Slot<String>,
    #[form(child)]
    enabled_models: ModelScopeSlot,
    compaction: Slot<bool>,
    reserve_tokens: Slot<u64>,
    keep_recent_tokens: Slot<u64>,
    steering: Slot<String>,
    follow_up: Slot<String>,
    auto_resize: Slot<bool>,
    block_images: Slot<bool>,
}

#[derive(Clone, PartialEq, FormSchema)]
enum ModelScopeSlot {
    Unset,
    Set(ModelScope),
    Unreadable(RawValue),
}

#[derive(Clone, PartialEq, FormSchema)]
struct ModelScope {
    #[form(items)]
    patterns: Vec<Pattern>,
}

#[derive(Clone, PartialEq, FormSchema)]
struct Pattern {
    text: String,
}

#[derive(Clone, PartialEq, FormSchema)]
struct RawValue {
    value: serde_json::Value,
}
```

`NetworkDraft` 同理包含 `http_proxy` 和 `retry` 四个叶子。

- `Slot::Invalid` 取代 `Edit::Invalid` 和原生控件问题：无法解析的数字文本作为模型值写入，因此 `is_dirty`、离开保护、重读/保存后的保留、撤销都按普通模型值处理；撤销写回的值与 `Invalid` 不同，不会成为空操作，投影随之重置输入框。
- `Unset` 与 `Unreadable` 的区分保持首批"类型不符原样显示、不覆盖"契约。
- 模型范围的"项目显式 `[]`"为 `Set(ModelScope { patterns: [] })`，与 `Unset` 区分；全局作用域删除最后一行写 `Unset`。空白模式是 `Pattern` 的文本值，本身就在模型中。
- 默认模型仍是两个叶子；选择器一次写两个字段，部分设置的组合保持现有来源判定。
- `Field` 枚举保留，作为叶子路径、页面、种类、标签、默认值和冲突映射的唯一表；在其上增加读取、比较、复制对应草稿字段的 `match`。`Edit`、`drafts`、`page_drafts`、`scope_rows`、`next_scope_row`、`model_scope_ids`、`edit_model_rows` 删除。

投影与回写：

- `project(document) -> Draft`：按叶子读取；类型匹配为 `Set`，缺失为 `Unset`，类型不符为 `Unreadable`，从不产生 `Invalid`。模型范围非字符串数组时为 `Unreadable`。
- 某字段**有待保存更改**，当且仅当当前值 ≠ `project(edited)` 中该字段的值。
- `changes(document, current)`：只遍历该页 `Field` 有待保存更改的字段。`base` 取当前 `Files` 中的原始 JSON；`next` 为 `Unset → None`、`Set → 序列化值`、`Unreadable → 原值`。未知字段从不出现在 `Change` 中，因此保留。

### 保存阻止条件

不安装 Form 校验器。保存前由应用对**有待保存更改的字段**检查：

- 任一字段为 `Slot::Invalid` → 阻止，提示 `pi-invalid-count`。
- 模型范围为 `Set`，且其 `patterns` 中存在仅含空白的文本 → 阻止，提示 `pi-model-scope-blank`。

没有待保存更改的字段不检查：磁盘上原有的空白模式、`Unreadable` 值都不阻止保存同页的其他字段。字段下方的错误提示用同一判定在渲染时计算，与现状一致。

### 对账：保留草稿地推进基线

不变式：两个表单的基线始终等于当前 `Files` 中被编辑文件的投影。

`reconcile(form, next, since)`：若 `project(next)` 与该页当前基线相同，表单不动（列表 `ItemPath` 不退役）。否则对每个 `Field`，当前值 ≠ `since` 时保留当前值，否则取 `next`，得到 `merged`；先 `rebase(next)`，若 `merged != next` 再 `replace(merged)`。`Invalid` 和模式文本都在模型中，因此随之保留；被保留字段的绑定投影的是保留值本身。整表 `rebase` 会让该页所有列表行的 `ItemPath` 退役并重建适配器：值保留，但行输入框的焦点和光标可能重置。

#### 对账后的保存语义（保持现状）

对账后，被保留的草稿以新读到的文件为基准：用户保存时写入的就是界面上看到的值。另一页保存成功、或显式重新读取后，即使外部也改过该字段，下次保存仍写入用户草稿，这与现有代码一致。叶冲突检测只针对"读取之后、本次保存之前"磁盘被改动的情况，`commit` 返回 `Conflict` 时的提示与处理不变。

| 事件 | 处理 |
| --- | --- |
| 首次加载、切换作用域（已确认丢弃） | 两页 `rebase(project(edited))` |
| 放弃本页 / 离开时丢弃 | 该页 `reset` |
| 显式重新读取（保留草稿，冲突后比较再保存） | 两页 `reconcile(new, since = 旧投影)`；清空冲突标记 |
| 保存成功（返回整份最新 `Document`） | 已保存的页 `since = prepared 值`，未保存的页 `since = 旧投影` |
| 冲突 / 锁 / 写入失败 | `Files` 与表单不变；冲突字段按 `Field::leaves()` 标记 |

已保存页的 `since` 用 prepared 值而非 `FormVersion`：保存期间晚到的编辑（含 `Invalid` 文本）都表现为当前值 ≠ prepared 值，不依赖版本号。`generation` 继续用于丢弃旧作用域/旧读取的异步结果；保存中控件仍禁用。

### 页面状态与字段菜单

- `has_unsaved` = 任一页 `is_dirty`；`has_page_drafts(page)` = 该页 `is_dirty`；字段"未保存"标记 = 该字段有待保存更改。在基线不变式下三者一致。
- 撤销更改：把字段写回 `project(edited)` 中的值。
- 设为本作用域值：写入 `Set(当前有效值)`；模型范围无有效值时写 `Set([])`。
- 恢复继承/默认：写 `Unset`。
- 有效值与来源列由 `PiConfig` 结合 `Files` 与草稿计算。

### 控件绑定

控件显示有效值（含继承值），表单只存本作用域的值，因此使用 `bind_control_in` 自定义绑定，不直接用 `gpui-form-gpui-component` 的 `FormInput` / `FormSelect`。

- 文本/数字输入：投影 `Slot` → 显示文本（`Unset` 时为继承值，`Invalid(text)` 时为 `text`）；`InputEvent::Change` 时解析并 `defer_set`：数字可解析为 `Set(n)`，否则为 `Invalid(text)`；代理文本空为 `Unset`。输入文本等于当前显示值时不写入（程序回显）。
- 布尔、选项菜单、模型选择器：渲染时读表单，回调中通过总路径写入。模型目录刷新后按表单当前值重新投影选择，不改写表单。

### 模型范围列表

行输入适配器由 `PiConfig` 持有在按行键索引的表中（适配器指输入框状态与其绑定），不用 `use_keyed_state`。

- `Set`：枚举 `ModelScope::PATTERNS` 的 `ItemPath`，以 `ItemPath::key()` 为键绑定行输入；上移/下移用 `move_before`，删除用 `remove`，添加用 `append`。路径退役（重读、放弃、作用域切换、恢复继承）时丢弃对应适配器，不复用。
- `Unset`（显示继承列表或"不限制"）：继承列表行按 `(files 读取代次, 序号)` 为键创建未绑定的输入适配器，显示继承文本。继承列表在 `Unset` 期间只随重读变化，代次变化即丢弃这些适配器。渲染和加载不写表单。
- 首次编辑继承行（改文本、上移/下移、删除、添加）：先写 `Set(ModelScope { patterns: 继承列表 })`，按序号把继承行的输入适配器移到新 `ItemPath` 键下并绑定到对应路径（同一输入框实体，保留焦点和光标），再对新路径执行该编辑。全局作用域删到空仍写 `Unset`。交接时取消继承行适配器的旧订阅；旧代次排队的回调发现代次不符即丢弃，不得再次物化或写入其他行。原生输入到表单的写入保持延迟，投影前比较文本，交接后不覆盖已输入的文本与光标。
- `Unreadable`：只读显示原始值与修复提示，不参与保存检查。

### 外部调用方

`set_scope`、`reload`、`activate`、`trusted`、`confirm_leave`、`request_scope` 的调用语义不变。`has_unsaved` / `needs_leave_guard` 增加 `&App` 参数以读取 Form；布局与资源页的两处调用同步传入 `cx`，不缓存第二份 dirty 状态。

## 实施

1. `pi_config/forms.rs`：`Slot`、两页草稿、模型范围 schema、投影、字段访问、保存阻止判定和字段合并；`fields.rs` 保留字段定义、继承解析和叶变更生成。
2. `pi_config.rs`：两个表单的创建与生命周期、保存（prepare → commit → 两页 reconcile）、离开保护与放弃；删除 `drafts` 相关代码。
3. `pi_config/page.rs`：字段菜单与各控件改为读写表单；输入改用自定义绑定。
4. `pi_config/model_scope.rs`：行适配器表、`ItemPath` 编辑、继承行首次编辑物化；删除 `scope_rows`、`next_scope_row` 及其测试。
5. 更新受影响单元测试（`tests.rs`、`fields_tests.rs`），删除只服务于旧 `Edit`/行身份的测试。

## 验证

- 单元测试（纯函数）：
  - 投影/回写对 `Unset`、`Set`、`Unreadable` 的往返，未改动的 `Unreadable` 不产生写入，未知字段保留，项目 `[]` 与继承的区分。
  - 保存阻止判定：磁盘原有空白模式或 `Unreadable` 不阻止保存其他字段；新增空白行和 `Invalid` 数字阻止保存。
  - `reconcile`：保留用户改过的字段、采用其余字段的新值；新文档投影与基线相同时表单不动。
- GPUI 测试：
  - 数字输入 `-` 后离开或切换作用域触发离开保护。
  - 重读后 `-` 文本与错误仍在；撤销后输入框恢复。
  - 对话页保存时网络页干净：外部改动的网络字段被采用。
  - 网络页 `maxRetries` 草稿 3、外部改为 4，对话页保存成功后草稿仍为 3；随后保存网络页写入 3。
  - 网络页保存后又切到对话页保存：模型范围行适配器重建后值不变（不断言焦点保留）。
  - 模型范围重排、删除后重读，行控件不与其他模式配对（覆盖重排与外部替换后的行状态错配场景）。
  - 项目 `Unset`、全局 `[a, b]`：通过真实 `InputEvent` 与延迟写入编辑第二行，项目草稿为 `[a, b']`，同一输入框保留焦点和光标；重排、删除同理；仅渲染不产生草稿；重读后旧代次回调不再物化。
  - 一页有无效数字时另一页仍能保存。
- 检查：`cargo fmt --all -- --check`、`cargo check -p gupi --locked`、`cargo test -p gupi --locked`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`。
- 原生界面快速试用：全局/项目各编辑并保存一次，确认文件只改动对应叶子。


## 实现与本轮验证

- `forms.rs` 定义两页 schema、四态 `Slot`、模型范围枚举/结构化列表及投影、字段访问和合并；`PiConfig` 的 Form 为草稿来源，文件投影只作基线。`Edit`、`drafts` 和手写列表行身份已删除。
- `inputs.rs` 保留原生输入实体和自定义表单绑定，数字的无效文本进入模型；重读全局继承来源时也刷新输入投影。`model_scope.rs` 使用 `ItemPath` 操作列表，继承适配器首次编辑时转交同一输入实体；旧回调同时核对读取代次和原生实体身份。
- 成功保存同时对账两页；未编辑字段采用返回文件，保留草稿以新文件为基准。失败保留草稿，锁与叶冲突检测仍由现有 `pi_settings` 执行。
- 新增 2 个纯函数测试（异常值投影/待保存字段校验、保留更新编辑的合并），5 个 GPUI 回归测试（跨页保存对账、绑定的无效输入在重读和另一页保存后保留/撤销、继承行的真实 `InputEvent` 与延迟交接、连续输入及旧回调拒绝；数字拼写纠正后恢复默认/继承/原始异常 Slot；无变化重读保留本地输入实体）。原有行身份测试改为正式 `ItemPath` 的排序/退役行为；文件/信任/失败回归继续执行。
- 全部 Gupi 测试 88 项通过；最终加强旧继承回调的实体身份检查，并修正连续原生输入改回原值的延迟顺序后，受影响的 Pi 配置测试 31 项通过。`cargo fmt --all -- --check`、`cargo check -p gupi --locked`、`cargo clippy -p gupi --all-targets --locked -- -D warnings` 与 `cargo build -p gupi --locked` 通过。
- 原生验证由协调者使用 `cua_repl` 操作本轮构建的 `Gupi UI Gallery.app`，只用隔离 agent 与 project-a。观察并核对文件：全局模型范围重排/编辑/保存；无效数字 `-` 的离开保护、取消、重读保留和字段撤销；项目继承行首次编辑后继续输入（文本/焦点交接）、排序/删除/添加/保存；网络页有无效草稿时对话页独立保存，修正数字后网络页保存；最终构建重新启动后复测数字由 `-` 改回原值，离开时不再提示草稿。项目未知字段保留，全局文件未被项目操作更改，未创建 trust.json。
- 这些结果不是用户验收；整表重建时焦点可能重置的设计边界仍保留。未进行新的平台或发行包全量验收。

- 最终评审修复：数字修正到有效基线值时恢复基线 `Slot`，不固定默认或继承值；无变化重读保留本地行适配器；删除测试专用的写入复制路径，现有测试改用正式绑定事件及列表动作；`prepare` 错误正常显示，变更生成使用已有基线。继承按钮目标在物化后退役，不能按旧索引修改新列表。
- 上述评审修复后的 88 项测试、编译、Clippy 与构建已通过；最新构建的拼写纠正原生补测尚未完成：进程运行中，但 `cua_repl` 返回 `cgWindowNotFound`，重连与重置后仍无法读取窗口。前述原生场景是初版迁移与连续输入修复构建的实际结果，不能代替这项最新补测。
