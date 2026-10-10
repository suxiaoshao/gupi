# Pi 设置与项目级覆盖

为 [Gupi #2](https://github.com/suxiaoshao/gupi/issues/2) 整理的上游配置契约与实施设计。用户已确认首批推荐范围、按页保存、手动重连及明确的“信任项目…”操作，指定设计师开发。当前开发以 [首批实施方案](implementation.md) 为准；研究中的高级字段和备选不属于本期授权范围。

## 文档导航

- [首批实施方案](implementation.md)：已确认范围、保存/信任/生效契约、实现顺序与必要验证。
- [配置格式与覆盖规则](configuration.md)：文件位置、JSON 格式、继承、资源路径与写入行为。
- [设置清单](settings.md)：字段、类型、默认值、终端专用选项与内部字段。
- [配置覆盖与缺口统计](coverage.md)：58 个 Schema 顶层键的逐项覆盖、独立文件与资源缺口、后续建议及待确定问题。
- [MCP 配置盘点](mcp-configuration.md)：内置 MCP 配置字段、覆盖/认证规则、现有缺口及接入待调研项。
- [接入限制与待确定问题](integration.md)：RPC 能力、持久化与运行时生效边界、推荐方案。
- [设置页面与导航讨论](information-architecture.md)：Gupi / Pi 分区方案的比较与已采用的决定。
- [图形设置的字段筛选](field-selection.md)：哪些字段常用、适合页内高级、保留文件编辑，或不应提供控件及其源码依据。
- [页面设计](page-design.md)：当前 Pi 区域的页面、字段、作用域、保存、信任与错误交互。

## 研究依据

2026-10-09 将本地 `/Users/sushao/Documents/code/pi` 的 `main` 快进更新到上游 `origin/main`，以提交 [`6fb2e7815167e6b19006fc526d1a5d0f5f998787`](https://github.com/earendil-works/pi/tree/6fb2e7815167e6b19006fc526d1a5d0f5f998787) 为研究快照，`packages/coding-agent/package.json` 版本为 `1.1.0`。

这里只核对上游源码、文档与协议，未升级 Gupi 依赖或本机安装的 Pi，也未验证 Gupi 中的运行时生效行为。后续接入需核对实际使用的 Pi 版本，不能把此快照的全部能力视为已受 Gupi 支持。

主要上游依据：

- [configuration.md](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/docs/configuration.md)
- [settings.md](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/docs/settings.md)
- [settings-schema.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/settings-schema.ts) 与生成的 [settings.schema.json](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/schemas/settings.schema.json)
- [settings-defaults.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/settings-defaults.ts)
- [settings-manager.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/core/settings-manager.ts)
- [rpc-types.ts](https://github.com/earendil-works/pi/blob/6fb2e7815167e6b19006fc526d1a5d0f5f998787/packages/coding-agent/src/modes/rpc/rpc-types.ts)

