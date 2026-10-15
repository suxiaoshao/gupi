# Pi 工具与环境、按模型覆盖与高级运行设置（[Gupi #49](https://github.com/suxiaoshao/gupi/issues/49)、[#50](https://github.com/suxiaoshao/gupi/issues/50)）

状态：Ready。架构师审查意见已并入，用户已确认范围与三项决定。

## 目标

两个 Issue 在一个 PR 中完成。改的都是 `settings.json` 的字段，沿用 #2 / #54 已有的 Pi 设置页机制：
- 作用域与继承、来源列、字段菜单；
- 按页 Form 草稿、按页保存、离开保护；
- 锁内重读、只写变更叶字段、叶冲突检测；
- 保留未知字段，手动重连后生效。

不升级共享 gpui revision。存储层只做一处扩展：叶路径支持运行时键（见“存储”）。

## 依据

- 本机 Pi 为 1.2.0，源码对照本地 `/Users/sushao/Documents/code/pi` 的 `v1.2.0` 标签，路径相对于 `packages/coding-agent/`。
- 与 #2 的 1.1.0 研究快照相比，`settings.schema.json` 和 `docs/settings.md` 只改了 `$id` 和一行说明。
- 字段类型和默认值见 [设置清单](../issue-2/settings.md)。本文的取舍替代 [字段筛选](../issue-2/field-selection.md) 中这些字段“首批不做”的结论。

| 字段 | Pi 的读取方式（v1.2.0） |
| --- | --- |
| `defaultTools` | `mergeDefaultTools`：上层列表全是 `+name`/`-name` 时（包括 `[]`）追加到下层列表后面，否则整体替换下层。`resolveDefaultTools` 再从普通名（没有普通名时用默认 `read/bash/edit/write`）开始，按顺序应用增减。因此全局 `["read","bash"]` 加项目 `[]`，结果仍是 `read,bash` |
| `codemode.*` | 内置 codemode 扩展读取；要求该扩展已启用并且 `codemode` 工具处于启用状态 |
| `shellPath`、`shellCommandPrefix`、`npmCommand` | 读取合并值。`npmCommand` 是 argv 数组：`[]` 回退为 `npm`，`[""]` 会报错（`core/package-manager.ts:1787-1796`） |
| `enableInstallTelemetry` | 读取合并值。只要设置了环境变量 `PI_TELEMETRY`，就以它为准：只有 `1`、`true`、`yes`（不区分大小写）表示开启，其他任何值都表示关闭（`core/telemetry.ts`）。Pi 子进程继承 Gupi 自身的环境，登录 shell 只补充 PATH |
| `defaultProjectTrust` | 只读全局文件。它是没有适用信任决定时的回退值；明确的上级目录决定和信任扩展的决定都优先（`core/project-trust.ts:47-82`） |
| `modelThinkingLevels` | 启动和恢复时：显式等级 > 会话记录 > 按模型设置 > 默认值（`sdk.ts`）。会话中切换模型时，按模型设置优先于默认值，已有会话同样适用（`agent-session.ts:2694-2705`）。`enabledModels` 中带 `:thinking` 后缀的显式等级又优先于按模型设置（`core/model-resolver.ts:664-670`） |
| `thinkingBudgets` | 启动时传给支持预算的供应商和模型，不是对所有模型都生效的通用预算 |
| `compaction.modelOverrides` | 对象递归合并后，每个 token 参数分别解析：合并后的模型覆盖值 > 合并后的普通值 > 默认值（`settings-manager.ts:63-79,818-843`）。数值上限是 `Number.MAX_SAFE_INTEGER` |
| `cacheWarming` | 只读全局文件 |
| `transport`、`httpIdleTimeoutMs`、`websocketConnectTimeoutMs` | 两个超时接受数字、数字字符串和 `"disabled"`（不区分大小写），`0` 和 `disabled` 都表示关闭，有限小数向下取整（`core/http-dispatcher.ts:19-34`）。发请求时也会读取 |
| `retry.provider.*` | 读取合并值。上游建议 Provider 层重试次数保持 `0` |
| `branchSummary.reserveTokens` | 只在 `summarize` 为真时使用（`agent-session.ts:4082-4090`）。Gupi 自带的树导航固定传 `summarize: false`（`crates/gupi-pi-runtime/src/extension.ts:12`） |

## 范围决定

| 内容 | 决定 |
| --- | --- |
| #49 字段 | 全部纳入，内置扩展开关除外 |
| 内置扩展开关 | **移到 [#47](https://github.com/suxiaoshao/gupi/issues/47)**。它是 `extensions` 数组里的 `+/-builtin:<name>` 条目（`core/package-manager.ts:979-990`），由 #47 统一负责这个数组。#49 关闭时注明这一项转到 #47，不算本次交付 |
| #50 字段 | 全部纳入，`branchSummary.reserveTokens` 除外 |
| `branchSummary.reserveTokens` | **不提供**：Gupi 自带的树导航不生成分支摘要，这个控件对它没有效果；只有第三方扩展的命令可能用到。#50 关闭时说明 |

## 页面

Pi 区域新增“工具与环境”页，放在“网络”之后：

```text
连接 · 对话 · 网络 · 工具与环境 · 包 · Skills · 模板
```

**工具与环境（新页）**

| 组 | 字段 | 控件 |
| --- | --- | --- |
| 工具 | 默认工具（`defaultTools`） | 有序条目列表和启动选择预览，见下文 |
| 工具 | Codemode 模式、声明预算 | 选项 `on`/`only`；非负整数。说明：需要已启用的内置 codemode 扩展 |
| Shell | Shell 路径、命令前缀 | 文本；清空表示不设置 |
| 包管理 | npm 命令（`npmCommand`） | 有序 argv 列表，见下文 |
| 隐私与信任 | 安装遥测 | 复选框，环境变量覆盖状态见下文 |
| 隐私与信任 | 默认项目信任（仅全局） | 选项 `ask`/`always`/`never`，见下文 |

**对话页**：按模型的设置靠近相关的组，不单独排到最后。
- “默认模型”组：加入按模型思考等级、思考预算。思考预算附一句说明：只对支持预算的供应商和模型生效。
- “上下文”组：加入按模型压缩参数。
- 缓存预热（仅全局）也放在“默认模型”组，附一句“预热请求会产生用量”。

**网络页**新增“高级”组：
- 传输方式：选项 `auto/sse/websocket/websocket-cached`。
- 两个超时：见下文。
- Provider 层的请求超时、重试次数、最大等待：单独成组，标题注明“Provider 层”，与 Agent 重试分开。

仅全局字段在项目作用域的显示方式与现有 HTTP 代理一致：显示全局值，并提供“在全局中编辑”。

## 存储：运行时键的叶路径

`pi_settings::Leaf` 目前是 `&'static [&'static str]`，`Document::get`、`Change` 和 `Error::Conflict` 都用它。按模型映射的键（`provider/modelId`）只有运行时才知道。

- 叶路径改为支持自有字符串段，锁内重读、只写变更叶、按叶检测冲突的语义保持不变。
- 已有的静态 `Field::leaves` 照常转换为新的路径类型使用。
- 投影、变更计算、对账和“冲突映射回界面”都要能枚举动态键：冲突要能定位到具体的模型行和具体的 token 字段。

## 关键交互

### 默认工具

- 一个有序列表，条目原样保存，不 trim。普通名和 `+/-` 修饰可以混用，按 Pi 的规则解析。
- 提供已知工具名补全：内置 `read/bash/powershell/edit/write/grep/find/ls`，以及 `codemode`、`tool_search`。也可以输入扩展工具名。
- 列表下方显示“**启动时的工具选择**”：按 Pi 的合并和解析规则算出结果。文案说明扩展默认启用的工具和 MCP 会另外加入，所以这不是完整的实时工具列表；也不暗示这是安全限制。
- “未设置”和 `[]` 是两种不同的状态：
  - 全局作用域删除最后一项时保留 `[]`，因为 `[]` 表示关闭全部内置工具。“恢复默认”才删除这个字段。
  - 项目作用域的 `[]` 不会关闭工具，它只是一个空的修改列表。预览如实显示继承的结果，Gupi 不另行解释为“项目清空”。
- 列表控件独立实现，只借用模型范围列表的交互方式。模型范围列表删除最后一项会删掉字段，而且会拒绝空白内容，这两条都不适用于这里。

### npm 命令

- argv 每一项原样保存，包括第一项之后有意义的空参数；保持顺序，不 trim。
- 第一项（命令）不能为空。
- “恢复继承 / 默认”删除这个字段。界面不产生 `[]`；文件里已有的 `[]` 显示为“使用 npm”并原样保留。

### 超时

- 草稿值有三种类型：数字毫秒、关闭、未设置（继承）。读到的原始写法（`0`、`"disabled"`、`"DISABLED"`、`"300"`、`1.5` 等）保留在基线中。
- 只有用户实际修改这个字段时才改写原始值；保存其他字段、打开字段菜单、撤销都不会把原始写法规范化。
- 控件：一个毫秒输入框，加上“关闭超时”选项。用户新设置“关闭”时写入 `0`。
- 文件中无法被 Pi 接受的值按 `Unreadable` 显示，原样保留。

### 按模型思考等级

- 每个模型键对应一个叶字段：`["modelThinkingLevels", key]`。
- 新增行时从现有模型列表中选择模型，键为 `provider/modelId`。同一个键不能重复添加。行的键不可修改，要换模型就删除后重新添加。
- 列表中没有的已存键照常显示和编辑，并标注“当前不可用”；值格式不对的行按 `Unreadable` 原样保留。
- 项目作用域同时显示从全局继承的行，同键时以项目行为准。“设为项目值”把该行写入项目文件；删除项目行后，继承的全局值重新显示。
- 等级选项按所选模型的能力计算，与默认思考等级相同。
- 说明文字：启动和恢复会话时，显式等级和会话记录优先；会话中切换到这个模型时使用这里的等级；模型范围里带 `:thinking` 后缀的等级优先于这里。

### 按模型压缩参数

- 每行两个独立的叶字段：`[..., key, "reserveTokens"]` 和 `[..., key, "keepRecentTokens"]`，各自有草稿值和来源。
- 例：全局 `M = {reserveTokens: 100, keepRecentTokens: 200}`，项目 `M = {reserveTokens: 300}`，生效值是 `{300, 200}`。
  - 在项目中编辑预留 token，不会把继承的保留值写进项目文件，也不会覆盖同一对象里的未知成员。
  - 按叶恢复继承。
- 删除项目覆盖后，继承的全局值重新显示。继承来的行不能用 `{}` 屏蔽。
- 数值限制在 `0..=Number.MAX_SAFE_INTEGER`，这条也适用于现有的计数字段，因为现在的 `parse_count` 接受任意 u64。

### 安装遥测

- 复选框编辑保存的偏好值。
- 只有 Gupi 进程的环境里设置了 `PI_TELEMETRY` 时，才额外显示一行“环境变量 PI_TELEMETRY 生效：开启/关闭”，按 Pi 的判定规则计算。
- 说明它不影响更新检查，也不是所有分析数据的总开关。

### 默认项目信任

- 选择 `always` 时先确认：没有适用信任决定的项目都将被 Pi 信任，会加载项目设置并执行项目扩展；已有的明确决定和信任扩展仍然优先。取消则草稿不变。
- `ask`、`never` 不需要确认。
- 这个设置只写 `settings.json`，不写 `trust.json`。
- 项目信任状态只按已成功保存的文件计算；草稿、保存失败或取消都不改变显示。全局作用域本身没有项目信任标记。

## Form 接入

- 新增 `ToolsDraft`。快照、prepare、reset、对账、全部保存、dirty 判断和离开保护都纳入第三页。保存返回整份文档时，与 #54 一样保留其他页的草稿。
- `Invalid` 文本和 `Unreadable` 原始值都保存在模型里，只校验已改动的叶字段。
- 动态行有两种身份：Form 的 `ItemPath` 用于控件，持久的模型键用于存储和冲突映射。
- 思考等级映射和压缩映射的继承方式不同，各自实现，不预先抽象成通用的映射编辑器。

## 生效

- 所有字段只保存文件，现有会话通过手动重连读取新配置，不新增自动同步。
- 重连会启动新的 Pi 进程，启动时读取的字段在重连后完整生效。
- 重连时显式传入当前模型和思考等级，所以按模型思考等级在重连后不会立即改变当前等级，之后切换模型时才会用到。

## 实施顺序

1. 存储层：支持运行时键的叶路径，加测试。
2. `Field`、默认值、字段类型；新增 `Page::Tools` 和 `ToolsDraft`；三页的对账与保存。
3. 实现各控件：两种模型映射、argv 列表、默认工具列表与预览（移植 `mergeDefaultTools` / `resolveDefaultTools`）、超时三态。
4. 页面、仅全局字段、`PI_TELEMETRY` 状态、信任确认，以及九种语言的文案。
5. 完成必要验证后交付试用。

## 必要验证

- 纯测试：
  - 动态键叶路径的读取、写入、冲突，以及冲突映射回具体的行和字段；
  - `defaultTools` 的合并与解析，至少覆盖：
    - 全局非空 + 项目 `[]`；
    - 全局 `[]` + 项目 `[+codemode]`；
    - 普通名与修饰混用；
    - 全局删除最后一项后保留 `[]`；
  - 压缩映射的按叶继承：上文的 `{300, 200}` 例子，以及未知成员保留、项目删除后显示全局值；
  - 计数字段上限 `MAX_SAFE_INTEGER`；
  - 超时：`"disabled"`、数字字符串和小数保持原样，直到用户修改该字段；
  - argv 原样保留：空参数、顺序、不 trim，第一项校验；
  - `PI_TELEMETRY` 判定表；
  - 三页对账与无效草稿互不影响；
  - 仅全局字段在项目作用域不写项目文件。
- 原生验证（worker 用 Computer Use，在临时配置中）：
  - 三页新控件的基本编辑、保存和离开保护；
  - 映射行增删；
  - `always` 信任确认；
  - 保存后的文件内容正确，未知字段保留。
- 抽查一项实际生效：设置 `defaultTools` 后重连，确认 Pi 的工具列表变化。
