# D2 项目包：Pi 契约调研

状态：In progress。协调者负责实现与验证，设计师 review 样式。用户已选择保留 Pi 扩展的信任判断；项目命令不传 `--approve` 或 `--no-approve`。

依据：本地 Pi 1.1.0 研究快照 `6fb2e7815167e6b19006fc526d1a5d0f5f998787`，路径相对 `packages/coding-agent/`。行号为约数。以下都来自源码阅读，没有运行 Pi 命令验证。

## 目录：项目与全局

| 对象 | 规则 | 依据 |
| --- | --- | --- |
| 项目 | 包命令以进程的 `cwd` 为项目，以 `getAgentDir()`（即 `PI_CODING_AGENT_DIR`）为全局目录 | `src/package-manager-cli.ts` 约 951–953 行 |
| 项目包存储 | npm 装在 `<cwd>/.pi/npm`，git 装在 `<cwd>/.pi/git/<host>/<path>`；local 来源相对 `<cwd>/.pi` 解析 | `src/core/package-manager.ts` `getNpmInstallRoot`（约 2085 行）、`getGitInstallRoot`（约 2166 行）、`getBaseDirForScope`（约 2195 行） |
| 全局包存储 | `<agentDir>/npm`、`<agentDir>/git`；只有全局 npm 会回退到系统全局 `node_modules` | `getNpmInstallPath`（约 2145 行） |

**Gupi 现状**：`package_action` 把 `PI_CODING_AGENT_DIR` 和 `cwd` 都设为全局目录，不带 `--local`。这只适用于全局，不能靠传入 `.pi` 变成项目模式。

**项目命令的条件**：`cwd` 为规范化的项目目录，`PI_CODING_AGENT_DIR` 保持真实的全局目录。

## 命令能力

| 命令 | 项目作用域 | 行为 |
| --- | --- | --- |
| `install <source> -l` | 支持 | 先安装到项目存储，再把来源加入 `.pi/settings.json` 的 `packages`；local 路径改写为相对 `.pi` 的路径（`installAndPersist`、`normalizePackageSourceForSettings`） |
| `remove <source> -l` | 支持 | 只从 `.pi/npm` 或 `.pi/git` 卸载，并移除项目设置中匹配的条目；全局安装不受影响（`removeAndPersist`） |
| `update --extension <source>` | **不支持 `-l`** | 更新全局**和**项目设置中所有同一身份的条目（`update()` 约 1090–1120 行）；CLI 没有只更新项目副本的方式 |
| `list` | 不需要 | 信任时列出全局和项目的包；Gupi 读文件即可 |

`-l` 只被 `install` 和 `remove` 接受（`package-manager-cli.ts` 约 431 行）。

## 信任语义

**CLI 判定**（`createCommandSettingsManager` 约 765–810 行、`src/core/project-trust.ts` `resolveProjectTrusted`、`src/core/trust-manager.ts`）：

1. 传了 `--approve` 或 `--no-approve` 时直接使用该值，只对本次命令有效，不写 `trust.json`。
2. 项目没有需要信任的资源时视为信任（`hasTrustRequiringProjectResources`）。判断依据是 `.pi` 中的相关条目，以及祖先目录中的 `.agents/skills`。
3. 否则，先加载**全局**扩展并发出 `project_trust` 事件。扩展代码会运行；扩展可以返回 `remember`，这时会写入 `trust.json`。
4. 再按最近祖先读取 `trust.json` 中的记录（`findNearestTrustEntry`）。
5. 都没有时，按 `defaultProjectTrust` 处理：`always` 为信任，`never` 为不信任，`ask` 在没有 UI 时为不信任。

**命令差异**：

- `update` 只用已保存的记录（`useSavedProjectTrustOnly`），不运行扩展。
- `install -l` 和 `remove -l` 在不信任时拒绝执行，项目存储路径本身也有检查（`assertProjectTrustedForScope`）。

**与 Gupi 首批信任读取的差异**：首批只看最近祖先的明确记录和 `defaultProjectTrust = always`。Pi 的 CLI 还有两条额外路径：

- 第 2 步：没有需要信任的资源时直接视为信任。全新文件夹因此可以直接 `install -l`。
- 第 3 步：由扩展决定，Gupi 无法预先得知。

**已选择的信任策略**：Gupi 在每次命令前重新读取已保存信任（最近祖先记录和全局默认策略），未信任则拒绝启动。对已信任项目，不传 `--approve` 或 `--no-approve`，交给 Pi 的扩展判断；Pi 仍可能拒绝，也可能根据扩展返回值写入 trust.json。执行后重新读取信任。安装不会由 Gupi 隐式授予信任。

## 身份与合并

依据：`getPackageIdentity`（约 1721 行）、`dedupePackages`（约 1741 行）、`docs/packages.md` “Understand scope and identity” 一节。

**身份**：

- npm 包按包名；
- git 包按 host 与路径，不含 ref；
- local 包按解析后的绝对路径，项目中相对 `.pi` 解析。

**同一身份同时出现在全局和项目**：

- 普通情况下，项目条目替换全局条目。
- 项目条目是对象且 `autoload: false` 时，它是对全局包的**过滤增量**：两条都保留，增量在前。它不是一次独立安装。

**列表因此需要区分三类行**：

1. 项目包：替换同名全局包；
2. 项目增量：过滤一个全局包，本身无安装；
3. 仅全局的包：继承，只读。

**列出项目包**：可以复用 `discovery::package_path`，以 `root = <cwd>/.pi` 映射 npm、git 和 local。但**不能**使用 npm `root -g` 回退：Pi 只在全局作用域回退，而且这个回退会以 `PI_CODING_AGENT_DIR = root` 运行 npm。列出项目包只读文件，不启动 Pi 或 npm。

## 本轮实施范围

- 包页接入现有共享作用域，保留已安装 / 浏览布局。
- 项目包支持文件读取、安装和移除；项目浏览页的安装目标为当前项目。
- 全局继承包只读；允许安装同身份的项目副本，并显示配置替换关系。
- 既有 `autoload:false` 过滤增量只读，不提供移除，也不允许同身份安装覆盖。展示原始配置关系，不声称当前会话已加载。
- 项目更新暂不提供。`install -l` 是否可安全更新仍需核对 npm 行为及声明保留，不以先移除再安装替代更新。
- 包内资源启停过滤保持只读；本轮不增加过滤编辑器。
- 项目目录与全局 agent-dir 分离；项目枚举只读文件，不调用 npm 的全局路径回退。

## 所有权与实施顺序

1. 对话标题栏右侧新增项目设置入口，复用 `ShowProjectPiSettings(cwd)` 与现有草稿保护；不依赖对话已落盘或实例已启动。具体界面见 [D2 界面](d2-interface.md)。
2. `gupi-resources` 承载项目包枚举、npm/git/local 身份与过滤增量识别。保持原有全局扫描行为；项目文件读取不授权代码执行。
3. 实现项目安装/移除：启动前重新读取信任和项目声明，拒绝已出现的同身份增量；操作绑定规范 cwd 和真实 agent-dir，Pi 负责安装与持久化。
4. 设置资源页复用现有 Installed / Browse。按钮状态来自当前作用域控制器，切换作用域清除旧的操作反馈。已运行命令继续归属原项目；不能因丢弃页面控制器而把安装进程中止并误报为取消。具体保留任务与回访刷新由实现时确定。
5. 协调者负责构建、核心测试与主要原生路径；设计师只 review 样式与交互。

## 剩余决定


- **项目更新后续方案**：不阻塞本轮查看、安装和移除；验证局部重装语义后再设计。

## 验证范围

- 项目声明、普通包与增量分类、npm/git/local 同身份、无全局目录回退。
- 临时 agent-dir / cwd 验证命令参数与环境分离，未信任及增量冲突在执行前拒绝，全局设置不被项目操作改写。
- 切换项目时安装归属不变，晚到结果不污染当前页，回访展示当前目录状态。
- 原生窗口检查右侧按钮、目标项目、未发送消息时的入口、包列表来源和浏览安装目标。
- 不把源码研究计为真实安装通过；真实 Pi 包命令验证使用临时配置和本地测试包，不读取个人凭据。
