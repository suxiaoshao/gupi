# Pi 配置页迁移到 gpui-form（[Gupi #54](https://github.com/suxiaoshao/gupi/issues/54)）

## 目标与范围

Pi 配置页（`src/features/settings/pi_config`）的草稿从 `drafts: BTreeMap<Field, Edit>` 改由 gpui-form 持有；模型范围列表行使用 `ItemPath::key()`，删除 `scope_rows` / `next_scope_row` 等手写行身份。为 [#47](https://github.com/suxiaoshao/gupi/issues/47) 的路径数组和 [#50](https://github.com/suxiaoshao/gupi/issues/50) 的按模型映射提供同一模式。

用户可见行为不变，保持首批与模型范围契约（[首批实施方案](../issue-2/implementation.md)、[页面设计](../issue-2/page-design.md)、[模型范围](../issue-2/model-scope.md)）：

- 全局 / 项目作用域与继承、来源列、来源与直接恢复默认/继承操作。
- 按页保存与离开保护（含保存中排队离开）；锁内重读、只写变更叶子、叶冲突检测、锁占用提示；未知字段保留。
- 继承来的模型范围列表可直接编辑、排序、删除，首次编辑即成为本作用域值。
- 信任状态与信任动作、仅全局字段（`httpProxy`）在项目作用域只读跳转。
- 保存只写文件，已运行会话靠手动重连生效；不新增自动同步。

不做：新增字段、改变界面布局或文案、改动 `gupi-resources::pi_settings` 存储契约。

## 当前 Form 契约

编辑会话、无效草稿、EditorKey、生命周期与保存凭据统一见 [#139](../issue-139/README.md)。应用不再使用 Invalid 模型槽、自存重复基线、继承输入延迟交接或手写 reconcile；共享 Form 拥有草稿、入场更新和原子 rebase，应用保留 Unset/Unreadable、文件锁与业务 key 合并语义。
