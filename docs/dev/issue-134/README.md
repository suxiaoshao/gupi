# #134 设置搜索零结果空态

状态：已核实 Kit 能力缺口，建议上游处理后接入。核对基线：Gupi main `3191bbf`、锁定 gpui-kit 0.8.0；另查 Kit main `7b14e03f`，该缺口仍在。本轮未改代码或发布上游 issue。

## 现状与 Kit 能力

问题仍存在。`src/features/settings/layout.rs:716` 直接组合 Kit `Settings`，搜索输入、过滤和页选择均归 Kit，而非 Gupi 自建搜索。

Kit 0.8.0 `gpui-component/src/empty.rs:14` 已有公开 `Empty`、`EmptyHeader`、`EmptyTitle`、`EmptyDescription` 等空态构件；缺的是 **Settings 的接入点**。`src/setting/settings.rs:112` 在无匹配页面时返回空 `div`；`:266` 的 SettingsState/search_input 和内部过滤结果没有公开访问接口，Settings builder 也没有零结果插槽。现有 Empty 因而不能直接由应用放入正确的搜索状态。最新 main 相同，不应在 Gupi 复制过滤规则、探查私有状态或覆盖空白区域补画。

## 推荐

- 向 `longbridge/gpui-kit` 报告 **Settings search renders a blank content area when no settings match**。最小复现是单页 Settings 输入不存在的关键词；预期内容区域显示零结果空态，实际返回空 div。建议 Settings 自己复用 Empty，提供本地化“未找到匹配设置 / 尝试其他关键词”，搜索框保持可编辑与现有焦点，不显示旧页面。
- 由 Kit 继续拥有匹配条件与零结果判断，保留已有“清空查询恢复原页选择”行为。最小修复不需要先开放全部 SettingsState，也不需要 Gupi 增加第二个搜索框或清空按钮；若上游选择公开空态 renderer，再通过该正式接口接入。
- Gupi 等待包含该能力的正式依赖版本，按获准升级流程接入，不在应用侧绕行或单独改 registry。必要验证放在 Kit 的现有 Settings 搜索回归中，覆盖零结果文案及清空后的页选择；Gupi 接入后核对现有 Gupi/Pi 两组设置和语言显示，不扩大为设置页重设计。

查重：检索公开 open/closed issue、PR，未找到专门的零结果空态项；已有 [#3089](https://github.com/longbridge/gpui-kit/issues/3089) / [#3104](https://github.com/longbridge/gpui-kit/pull/3104) 修的是搜索后选择身份，其测试保留零结果时的选择，但没有增加空态。上报时说明这一差别。

需要用户决定的产品选择：无。缺口应由 Kit Settings 修复，本轮只记录报告内容，不发布。
