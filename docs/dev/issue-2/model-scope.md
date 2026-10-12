# 第二批：模型范围

状态：已实现；自动化与隔离 Pi 验证通过，原生界面验收待完成（Computer Use 连接故障）。本批仅包含 `enabledModels`，等待用户试用。首批契约（作用域、继承、按页保存、冲突、锁、信任、手动重连）见[首批实施方案](implementation.md)和[页面设计](page-design.md)，本文只写新增内容。

## 范围

- 对话页“默认模型”组新增“模型范围”字段，读写 `enabledModels`，全局与项目作用域都可编辑。
- 不做：按模型思考等级 `modelThinkingLevels` 及其他按模型覆盖、高级运行参数（[#50](https://github.com/suxiaoshao/gupi/issues/50)）；Gupi 内模型轮换；按范围过滤会话中的模型选择器；本地复刻 Pi 匹配器做命中预览。
- 其余拆出的后续工作见[剩余工作](#剩余工作)。

## Pi 契约

依据研究快照 `6fb2e78`（Pi 1.1.0），`packages/coding-agent/` 下：

- 字段是字符串数组，每项一个模式（`src/core/model-resolver.ts` `resolveModelScopeFromModels`）：
  - 含 `*`、`?`、`[` 时按 glob 匹配 `provider/id` 或单独的 `id`，不区分大小写；可带 `:<thinking>` 后缀。
  - 否则先按完整引用匹配，再逐级剥离冒号后缀，因此含冒号的模型 ID（如 OpenRouter `model:exacto`）可用。
  - 多个模式命中同一模型时保留首次出现，顺序有意义；未命中只产生 Pi 启动警告。
- 空数组或未设置都表示不限制范围（`src/main.ts` 中 `modelPatterns.length > 0` 才解析）。
- 合并：`deepMergeObjects` 对数组整体替换。项目设置 `[]` 会覆盖全局范围为“不限制”；项目非空数组完全替代全局数组，不做并集。
- 启动选择（`src/main.ts` `buildSessionOptions`）：仅在未传 `--model` 且会话没有消息时生效。已保存默认模型在范围内则用它，否则用范围中第一个模型；思考等级优先取对应模式的 `:thinking` 后缀。
- RPC `set_model` 不持久化，不会把模型追加进 `enabledModels`；`get_available_models` 返回全部可用模型，不按范围过滤。

## 在 Gupi 中的实际作用

- Gupi 新建会话（主窗口、临时窗口）不传 `--model`，因此范围决定新对话的初始模型和思考等级，规则同上。
- 重连会显式传当前 `--provider/--model/--thinking`，已有会话的模型不受范围影响；带模型的模板任务通过 `set_model` 指定模型，也不受影响。
- Gupi 没有模型轮换，会话内模型选择器仍列出全部可用模型。
- 文案必须据此描述，不得称为“允许的模型”或白名单。

## 界面

放在对话页“默认模型”组，默认思考等级之后。行内为有序列表，组件优先复用 GPUI Kit 现有列表/输入/菜单，具体布局按 `gpui-kit-design-guides` 处理。

- 每行一个模式，可直接编辑文本，可上移、下移、删除。
- “添加模型…”复用默认模型的可搜索选择器（同一个 `--no-session` 查询实例），追加精确的 `provider/id`。
- “添加模式”追加一行空文本，用于 glob 或带 `:thinking` 后缀的模式。
- 文件中已有的任意字符串原样显示和保存，不改写成精确 ID，不做命中或可用性校验。
- 来源列、字段菜单（撤销更改、设为项目值/全局值、使用全局值/默认值）沿用首批规则；整个数组视为一个值。
- 列表为空时显示“不限制”。项目作用域中显式为空时注明“不限制（覆盖全局）”，与“继承全局”区分。
- 说明文字（中文基准，九种语言同步）：“新对话从此范围选择初始模型：默认模型在范围内时使用默认模型，否则使用第一个匹配的模型。不限制会话中可选的模型。”同时更新默认模型行的说明，提示范围可能改变新对话的初始模型。

## 读写规则

- `Field` 新增模型范围，单叶字段 `["enabledModels"]`，值为 JSON 字符串数组；默认值为未设置。
- 校验：仅含空白的行阻止保存，提示修正；其余文本原样保存，不 trim、不去重。
- 删除最后一行：项目作用域写 `[]`（显式覆盖为不限制）；全局作用域删除该键。“使用全局值/默认值”仍删除当前作用域的键。
- 草稿相等判断、保存冲突、锁内重读、保留未知字段与首批一致。冲突比较整个数组；Pi 交互模式 `/scoped-models` 也会改写该字段，属于正常的外部修改。
- 文件中该键不是字符串数组时，显示为无法编辑的原始值并提示在文件中修复，不覆盖。

## 生效

保存只表示文件已写入。新建的 Pi 进程读取新范围；已运行会话不变，手动重连也不会改变已有会话的模型。不新增自动重启或 RPC 同步。

## 验证

协调者负责以下检查；新增测试只覆盖正式代码中的新规则。

- 单元测试：读取/保存字符串数组；项目 `[]` 与继承的区分；删除最后一行在两个作用域下的写入；空白行阻止保存；非数组值不被覆盖；未知字段保留；数组冲突检测。
- 实际 Pi 1.1.0 隔离配置：全局范围首项不是默认模型时，新对话的初始模型为首项；默认模型在范围内时保持默认模型；带 `:thinking` 后缀时思考等级生效；项目 `[]` 覆盖全局后恢复默认模型；已有会话重连后模型不变。
- 原生界面（computer use）：添加/编辑/排序/删除、项目与全局切换、草稿离开保护、窄宽度布局、九种语言中至少中英文案。
- `cargo fmt --all -- --check`、`cargo clippy -p gupi --all-targets --locked -- -D warnings`、`cargo test -p gupi --locked`。

完成后更新 [coverage.md](coverage.md)、[field-selection.md](field-selection.md) 与[页面设计](page-design.md)中 `enabledModels` 的状态，并把本文状态改为已实现。

## 实现与验证记录

- `pi_config/model_scope.rs` 实现有序模式列表，复用 `PiConfig` 的作用域、草稿、字段菜单和按页保存；模型选择器复用默认模型查询结果，没有新增 Pi 查询进程。
- `fields.rs` 添加 `enabledModels` 单叶字段；九种语言同步模型范围与默认模型说明。
- 首批实现的 `cargo test -p gupi --locked`：81 个测试通过。新增数组读写、项目空数组/继承、异常类型、空白行、未知字段保留和冲突测试；补充带模式列表的 480/1200px 渲染检查；当前页面草稿由按页 `Form` 持有，模型范围列表以 `ItemPath::key()` 绑定行控件；同列表重排保留身份，外部重建后旧路径退役。迁移实现与本轮验证见 [gpui-form 迁移](../issue-54/README.md)。这是自动化渲染检查，不代表原生视觉验收。
- `cargo build -p gupi --locked`、Clippy（`-D warnings`）、格式检查通过。
- 已安装 Pi 1.1.0 使用隔离的 agent/project/HOME、自定义离线模型进行 RPC `get_state` 实测，没有发送模型请求：默认模型不在范围时选 beta/high；默认模型在第二项时仍选 alpha/medium；项目 `[]` 恢复 alpha/low；加载含一条消息的会话文件并按 Gupi 重连参数传入 alpha/low 后，仍为 alpha/low。
- 原生界面：当前构建已通过 `script/gupi-ui-gallery --no-build --keep` 启动。Computer Use 绑定测试 app 时两次失败，报 `Sky Computer Use native pipe startup failed`（含重置后重试）。未完成实际添加/编辑/排序/删除、离开保护、窄宽度和中英文视觉检查，不能记为通过。

## 剩余工作

以下内容从 #2 拆出，各自在对应 Issue 中确定范围，不随本批实施：

| Issue | 内容 |
| --- | --- |
| [#47](https://github.com/suxiaoshao/gupi/issues/47) | 项目资源路径数组、启停/过滤写入与包 delta 编辑 |
| [#48](https://github.com/suxiaoshao/gupi/issues/48) | 供应商、自定义模型 `models.json` 与认证 |
| [#49](https://github.com/suxiaoshao/gupi/issues/49) | 工具与执行环境、遥测、默认信任策略 |
| [#50](https://github.com/suxiaoshao/gupi/issues/50) | 按模型覆盖（含 `modelThinkingLevels`）与高级运行/网络参数 |
| [#51](https://github.com/suxiaoshao/gupi/issues/51) | MCP 全局与项目配置、认证 |
| [#52](https://github.com/suxiaoshao/gupi/issues/52) | 仅更新项目包的契约调研 |
| [#54](https://github.com/suxiaoshao/gupi/issues/54) | Pi 配置页迁移到 gpui-form；#47、#50 在其后开始 |

扩展/主题代码编辑不单独立项，所需深度另行讨论。Pi 终端主题、终端快捷键与其他终端专用字段不复制到 Gupi。
