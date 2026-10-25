# Product screenshots / 产品截图

These are captures of Gupi's actual macOS UI. Sample answers illustrate the interface, not model benchmarks or real user work. The UI is not generated or retouched. Each capture group has its own baseline below.

这些图片来自真实的 Gupi macOS 界面。示例回答只用于说明界面，不代表模型评测或用户真实工作记录；界面未经生成或修饰。各组图片的拍摄基线分别记录如下。

| Files | Purpose / 用途 |
| --- | --- |
| `workspace-en.jpg`, `workspace-zh-CN.jpg` | Project sessions, Markdown and composer / 项目会话、Markdown 与输入区 |
| `history-en.jpg`, `history-zh-CN.jpg` | History list, branch graph and branch preview / 历史列表、分支连线与分支预览 |
| `files-en.jpg`, `files-zh-CN.jpg` | Workspace tabs, project tree, Markdown reading view and local image / 工作区标签页、项目文件树、Markdown 阅读视图与本地图片 |
| `git-en.jpg`, `git-zh-CN.jpg` | Read-only Git diff with staged, unstaged and untracked files / 只读 Git 差异及已暂存、未暂存、未跟踪文件 |
| `settings-en.jpg`, `settings-zh-CN.jpg` | Historical appearance captures; not used in the current README / 历史外观截图，当前 README 不使用 |
| `pi-settings-en.jpg`, `pi-settings-zh-CN.jpg` | Project-scoped conversation settings, Global sources and ordered model patterns / 项目作用域对话设置、全局来源与有序模型范围 |
| `packages-en.jpg`, `packages-zh-CN.jpg` | npm search results and published package details before installation / npm 搜索结果与安装前的包发布详情 |

Workspace/history/files/Git baseline: source commit `bf36bc9` (Cargo version 0.1.10, before the 0.1.11 release), Pi 1.4.0, macOS 27.0.1 arm64, 2026-10-25. All eight captures use the same local build, Default Light theme and 1240 × 850-point window, producing 2480 × 1700-pixel JPEGs. The isolated Gallery contains an authored Harbor project, a real disposable Git repository and authored Pi-format demonstration history. UI labels, conversation text and the unsent draft are paired in English and Chinese; project files are the same English example in both. Files and Git use the maximized workspace, with both tabs open. No model request was sent; displayed model selection and zero usage are demonstration state, not model output or a benchmark. Each capture was inspected for clipped controls/content, leftover menus, credentials, personal paths and private data. The Gallery was quit and its disposable data removed afterward. Capture-tool JPEG bytes are stored directly, without UI retouching or re-encoding.

工作区／历史／文件／Git 拍摄基线：源码 `bf36bc9`（Cargo 版本 0.1.10，0.1.11 发版前），Pi 1.4.0，macOS 27.0.1 arm64，2026-10-25。八张图使用同一本地构建、默认浅色主题及 1240 × 850 点窗口，输出 2480 × 1700 像素 JPEG。隔离 Gallery 中的 Harbor 项目、真实临时 Git 仓库和 Pi 格式演示历史均为自编内容；界面、会话与未发送草稿中英文对应，项目文件沿用相同英文示例。文件和 Git 图使用最大化工作区，同时保留两个标签。没有发送模型请求，模型选择与零用量仅为演示状态。逐张检查了控件／内容边界、菜单残留、凭据、个人路径及私有内容，拍摄后已退出 Gallery 并清理隔离数据；直接保存工具返回的 JPEG，未修饰界面或重新编码。

Historical appearance baseline (`settings-*` only): Gupi 0.1.0, source commit `3efcdca`, Pi 0.99.1, macOS arm64, 2026-09-30; sample project and authored demonstration sessions.

Pi settings/package baseline: public Gupi 0.1.5 arm64 ZIP, source commit `16fc415`, Pi 1.1.0, macOS, 2026-10-12; the Gupi source project and normal configuration, at 2216 × 1482 pixels. Package captures show the live npm search `pi-mcp` and `pi-mcp-adapter` 5.2.0 details. No package was installed, no Pi settings were saved and no model request was sent. The interface language was restored to Follow system after capture. The JPEG bytes returned by the capture tool are stored directly with `.jpg` extensions.

When refreshing the screenshots:

1. Build the current source and confirm the captured process belongs to it.
2. Use the environment requested for the task and public or authored content appropriate to the feature. Keep credentials and private content out of product screenshots.
3. Capture equivalent English and Chinese states at a consistent window size. Choose the screens needed to explain the feature.
4. Inspect the actual captures for truncation, transient menus, personal paths and private data. Keep each language's captions and filenames aligned with its README; place the history and settings captures beside their respective usage sections.
5. Restore the environment as appropriate for the task and update the relevant capture baseline. Do not present an older capture as evidence of a new interface.
