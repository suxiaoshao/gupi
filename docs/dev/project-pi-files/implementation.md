# 项目 Pi 文件：D1 实现记录

状态：D1 已完成受影响构建、测试和主要 UI 路径验证，可交付试用。本文记录当前实现位置和与设计的差异；界面方向见[界面设计](interface.md)，写入契约见[数据与持久化](data-and-persistence.md)。

## 代码位置

| 能力 | 位置 |
| --- | --- |
| 项目扫描 | `crates/gupi-resources/src/pi_resources/discovery.rs` `scan_project`：只扫描 `<cwd>/.pi/skills`、`<cwd>/.pi/prompts`，应用项目 `settings.json` 中的启停覆盖；不创建 `.pi/` |
| 路径归属 | `crates/gupi-resources/src/pi_resources.rs` `project_root`、`project_owns`：按解析后的真实路径判断是否位于规范化 `cwd` 下的 `.pi/` 内；尚不存在的路径按最近的已存在祖先判断 |
| Pi 冲突名称 | `Resource::key`：模板取文件名；Skill 只有描述非空时才有名称，取 frontmatter `name`，缺省回退到所在目录名 |
| 保存基线 | `Baseline`、`save_text(path, text, create, baseline)`：保存前重读，内容与打开时不同就拒绝，错误用 `Error::is_changed` 识别 |
| 控制器目标 | `crates/gupi-resources/src/resources.rs` `Target::{Global, Project(cwd)}`。项目目标只接受 `Save` 和 `Delete`，并在写入线程中重新校验路径归属 |
| 共享作用域 | `src/features/settings/pi_config.rs`：`scope()`、`trusted()`、`set_active_project`、`canonical_scope`；`pi_config/page.rs`：`render_scope`（作用域菜单与信任状态）、`project_label`、`choose_project_folder` |
| 资源页面 | `src/features/settings/resources.rs`：`follow_scope` / `sync_scope` 为每个项目新建一个 `ResourceController`；`Row::{Global, Own, Inherited}` 决定行内操作；`relation` 计算同名标签 |
| 系统提示词 | `resources/prompts.rs` `render_system_prompt`、`confirm_use_global` |
| 编辑器 | `resources/editor.rs`：`Editor.owner` 和 `Editor.baseline` 在打开时固定；冲突时显示“重新读取”；`save_then` 在保存成功后继续离开操作 |
| 离开保护 | `src/features/settings/layout.rs` `confirm_unsaved`：先处理资源编辑器，再处理设置字段草稿。适用于切回 Gupi 区域、离开设置和项目入口 |
| 当前对话项目 | `src/features/startup.rs` `apply_settings_visible`：打开设置时记录当前对话的 `cwd` |

## 行为

- **作用域标识**：作用域以规范化目录为键。对话菜单、项目行和文件夹选择器指向同一目录时，合并为同一项。
- **作用域菜单顺序**：
  1. 全局；
  2. 当前对话的项目；
  3. 本次运行中打开或选择过的其他项目；
  4. 分隔线，然后是“选择项目文件夹…”。

  同名项目附带上级路径。
- **项目作用域下的 Skills 和模板页**：
  - 标题行使用共享的作用域菜单；包页仍显示静态的“全局”。
  - 先列出本项目组，再列出“继承自全局”组。
  - 继承行只读，菜单中有“在全局中编辑”，不显示开关和删除。
  - 本项目行不显示开关，也不提供“注册 Skill”：这两项会写 `settings.json`。
- **同名标签**：只有项目已信任、两边都已启用、且都能确定 Pi 名称时才显示。未信任时，本项目行显示“未加载（项目未信任）”。
- **SYSTEM / APPEND**：
  - 项目作用域下显示“项目文件”或“使用全局”。
  - “使用全局”把项目文件移入废纸篓。
  - 空白内容可以保存，保存后仍是一个存在的文件。
- **删除 Skill**：只移入列出的 `SKILL.md`（或单文件 Skill），确认框显示该文件路径。
- **晚到结果**：切换项目会丢弃旧项目的控制器及其任务，旧的读取或写入结果不会显示在新项目页上。写入线程已经开始时，文件仍会写入原目标，但不再显示反馈。

## 与设计的差异与限制

- **全局编辑器的变化**：保存基线检查同样作用于全局编辑器。文件在打开后被外部修改时，全局编辑也会拒绝覆盖，这是对现有全局行为的唯一改变。
- **项目路径项不列出**：项目 `settings.json` 中的显式路径项（`skills` / `prompts` 数组里的路径）不在本项目组中列出。路径数组仍属待设计范围。
- **祖先 `.agents/skills` 不扫描**：因此同名标签只比较 Gupi 已扫描的全局来源与本项目 `.pi/` 文件。
- **侧栏切页不拦截**：区域内的侧栏页面切换仍无法拦截。资源编辑器是模态对话框，打开时无法点击侧栏。

## 建议的验证（由协调者执行）

```sh
cargo fmt --all -- --check
cargo test -p gupi-resources --locked
cargo test -p gupi --locked settings
cargo clippy -p gupi --all-targets --locked -- -D warnings
```

新增测试：

- `pi_resources::tests`：路径归属、链接、扫描错误、Pi 名称、保存基线；
- `resources::tests::project_scope_never_writes_settings_or_packages`；
- `features::settings::resources::skills::tests::project_scope_lists_only_the_projects_own_text_resources`。

真实窗口验收场景见[界面设计](interface.md)“UI 验收”。

## 当前验证结果（协调者）

- `cargo fmt --all -- --check`、`cargo build -p gupi --locked`、`cargo clippy -p gupi --all-targets --locked -- -D warnings` 通过。
- `cargo test -p gupi-resources --locked`：32 通过；最终 `cargo test -p gupi --locked features::`：33 通过（含 settings 与 startup）。新资源测试已改为等待目录加载通知，避免把 GPUI 执行器暂时空闲当作后台扫描完成。
- 在临时 Gallery 配置和项目中启动本地构建：选择空项目未创建 `.pi/`；新建项目 Skill、SYSTEM 和模板成功；Skills / 模板共享项目作用域；项目 SYSTEM 保存没有改写全局文件。
- 模板打开后外部修改同一文件：保存拒绝覆盖，保留草稿；“重新读取”载入外部内容。
- 修复并复验 macOS“窗口 > 主窗口”的 Root 重入：有项目模板草稿时正常显示确认；取消保留草稿，放弃返回主窗口且不写入，保存写入原项目文件后返回。仅有对话字段草稿时，确认与放弃也通过。
- 修复并复验普通入口：从项目作用域返回主窗口，再点击普通“设置”，提示词页恢复全局。
- 项目 B 中指向临时全局文件的符号链接：显示“外部来源 / 只读”，可预览，菜单只有“显示位置”，没有编辑或删除。
- 本次没有重新验证已有对话的上下文入口、实际 Pi 会话加载、信任后的同名标签或 D2 包操作；这些不计入上述 UI 通过结果。

以上 UI 结果来自隔离配置的真实原生窗口，不代表实际 Pi 会话的资源加载验证。
