# D2 项目包：Pi 契约调研

状态：调研 / Draft。本文只记录 Pi 的现有行为和待选方案，不授权 D2 实现。下文三项产品选择及 `--approve` 策略**均未选定**。工作范围见 [README](README.md)，写入契约见[数据与持久化](data-and-persistence.md)。

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

**`--approve` 策略（未选定）**：

| 方案 | 说明 | 风险 |
| --- | --- | --- |
| A. Gupi 自行读取信任，确认后传 `--approve`；否则禁用项目包操作 | 不运行扩展，不写 `trust.json` | 必须先与 Pi 语义对齐：最近祖先、`never`、`ask`。另需决定第 2 步的“无资源即信任”是否照搬，例如全新文件夹首次安装时 |
| B. 不传覆盖项，交给 Pi 判定 | 与 Pi 完全一致 | 会运行全局扩展代码，可能写入 `trust.json`；结果无法提前展示 |
| C. 项目写入一律要求“信任项目…”的明确记录 | 最保守 | 比 Pi 严格：`always` 策略或无资源的项目也会被拦下 |

无论选哪种方案，都**不能**用 `--no-approve` 执行项目写入：命令会直接失败，而且不代表信任。

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

## 待选产品决定（均未选定）

1. **项目“更新”的含义**：
   - a. 更新时注明会同时更新全局同一身份的包；
   - b. 项目中不提供更新，改为移除后重新安装；
   - c. 重新运行 `install -l` 作为更新。未固定版本的 npm 包和 git ref 的实际效果需要另行核实。
2. **增量编辑**：D2 是否包含项目对全局包的过滤增量（`autoload: false`），还是只管理完整的项目包。
3. **浏览页安装目标**：作用域为项目时，浏览页的“安装”是否安装到项目。

另需先选定上文的 `--approve` 策略。

## 验证设想（未执行）

- 使用临时的全局目录和项目目录，不读取个人配置。
- 用 local 来源验证 `install -l` 和 `remove -l` 只修改项目设置与项目存储，全局设置和全局安装不变。
- 验证未信任时操作被禁用，或 Pi 拒绝执行；`trust.json` 不被写入。
- 网络安装（npm、git）与 D1 的必要检查分开，按 D2 范围另行安排。
