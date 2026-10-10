# 配置格式与覆盖规则

研究版本和上游文件链接见 [README](README.md#研究依据)。

## 文件位置与职责

`agent-dir` 默认为 `~/.pi/agent`，可由 `PI_CODING_AGENT_DIR` 环境变量或 SDK 的 `agentDir` 参数改变。项目目录是 Pi 的工作目录，不应直接假定为 Git 根目录。

| 路径 | 用途 |
| --- | --- |
| `<agent-dir>/settings.json` | 全局偏好、模型默认值、资源路径、包声明 |
| `<cwd>/.pi/settings.json` | 项目级设置覆盖、资源路径、包声明 |
| `<agent-dir>/models.json` | 自定义供应商、端点、模型及模型属性覆盖 |
| `<agent-dir>/auth.json` | API Key 与 OAuth 凭据 |
| `<agent-dir>/keybindings.json` | Pi 终端快捷键 |
| `<agent-dir>/mcp.json`、`<cwd>/.pi/mcp.json` | 全局与项目 MCP 服务 |
| `<agent-dir>/SYSTEM.md`、`<cwd>/.pi/SYSTEM.md` | 替换系统提示词 |
| `<agent-dir>/APPEND_SYSTEM.md`、`<cwd>/.pi/APPEND_SYSTEM.md` | 追加系统提示词 |
| `<agent-dir>` 与项目 `.pi` 下的 `extensions/skills/prompts/themes` | 自动发现的资源目录 |

同名 `SYSTEM.md` 或 `APPEND_SYSTEM.md` 由可信项目文件优先，不合并全局与项目两份同名文件。上下文指令文件 `AGENTS.md` 等有独立的父目录发现规则，不能套用 `settings.json` 的两层定位规则。

## JSON 格式

使用标准 JSON，不支持注释或尾逗号；读取时允许 UTF-8 BOM。设置字段均可省略。示例仅展示格式，模型应从实际可用列表选择：

```json
{
  "defaultProvider": "anthropic",
  "defaultModel": "claude-sonnet-4-5",
  "defaultThinkingLevel": "medium",
  "compaction": {
    "enabled": true,
    "reserveTokens": 16384,
    "keepRecentTokens": 20000
  },
  "retry": {
    "enabled": true,
    "maxRetries": 3
  },
  "images": {
    "autoResize": true,
    "blockImages": false
  }
}
```

Pi 提供 TypeBox 定义与生成的 JSON Schema，顶层允许未知字段。当前 `SettingsManager` 读取使用 `JSON.parse` 和旧格式迁移，并未统一执行完整 Schema 校验；部分值由 getter 验证、限制范围或回退。因此 Schema、文档与运行时消费方式需要一起核对。

## 继承与覆盖

普通设置按项目值、全局值、内置默认值解析。CLI、SDK 覆盖与会话恢复可能进一步影响运行值，不应把合并后的配置直接当成当前会话状态。

- 嵌套对象递归合并。项目仅设置 `retry.maxRetries` 时仍继承全局 `retry.enabled`。
- 普通数组整体替换，例如 `enabledModels`。
- 恢复继承应删除目标字段。写入默认值仍是显式覆盖，`null` 也不是通用的继承标记。
- 项目配置通常在获得项目信任后加载；`sessionDir` 为定位会话而在信任解析前读取。
- `defaultProjectTrust`、`httpProxy`、`cacheWarming` 只取全局值。内部 `deviceId` 也只使用全局值。

### 资源列表的例外

资源加载器分别读取全局与项目配置，组合加载 `packages`、`extensions`、`skills`、`prompts`、`themes`，不直接依赖普通数组覆盖后的结果。资源选择还受包过滤和资源类型的冲突处理规则影响。

全局资源相对路径基于 `agent-dir`，项目资源相对路径基于项目 `.pi`；支持绝对路径和 `~`。资源数组支持 `!pattern` 排除、`+path` 精确包含、`-path` 精确排除。

`packages` 元素可为来源字符串，或包含 `source` 的对象；对象可带 `autoload` 和 `extensions/skills/prompts/themes` 过滤数组。npm、Git 与本地来源继续复用已有资源管理能力。

内置扩展名称为 `builtin:mcp`、`builtin:llama.cpp`、`builtin:codemode`、`builtin:tool-search`，默认加载。`-builtin:<name>` 可禁用；项目的显式启用或禁用优先于全局。

### 工具列表的例外

`defaultTools` 中的普通名称替换继承选择；只有 `+name`、`-name` 时修改继承选择。一个列表同时有普通名称和修饰项时，先用普通名称构成选择，再按顺序应用修饰项。

默认工具为 `read/bash/edit/write`。空数组禁用内置工具，但不等于禁用所有扩展或 SDK 工具。CLI 工具选项另有覆盖规则。

## 保存与错误行为

Pi 自身通过 `proper-lockfile` 加锁，写入前重新读取文件，只合并被标记修改的字段或子字段，最后以两空格缩进序列化 JSON。它保留未修改字段，但不会保留原始排版；旧字段可能在迁移时改变。

加载有错误的作用域不会被正常保存路径直接覆盖；重载失败会保留已有内存配置并记录错误。Gupi 接入需要保留未知字段、区分加载失败与空配置，并考虑和 Pi 同时写入的协调，不能用旧的完整快照覆盖文件。

这些是源码行为核对，不构成 Gupi 文件读写实现已完成或并发安全已验证的结论。

