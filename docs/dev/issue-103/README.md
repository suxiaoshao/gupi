# 文件与技能引用打开及 Files 预览

状态：打开路由、Files 图片预览及 Markdown 切换已实现。本地 Markdown 图片按最终选型交给 Kit base TextView 内置图片节点。

## 结果与边界

用户点击对话或输入框中的文件、目录、技能位置后，属于该会话项目的内容在现有 Files 中查看，项目外的内容交给系统默认应用。Files 保持一个文件树和一个只读预览位；增加常用位图预览及 Markdown「源码 / 预览」切换，不增加文件标签或编辑、保存能力。

不支持或超过现有文本限制的文件仍可在 Files 中选中，正文显示「无法预览」及「用默认应用打开」。点击文件树本身不启动外部应用。Pi 的 prompt、token 序列化、技能加载、会话历史和工具执行不变；打开资源只是桌面交互，不触发模型请求。

文件和编辑器行为以 Zed 为上限，取其中必要的只读子集。项目内外分流是本功能已确认的应用规则，不宣称它是 Zed 的规则。不引入 PDF、Office、音视频、CSV 表格、notebook 执行、hex、SVG 渲染、双栏同步滚动、文件监听或新的预览插件系统。

## 源码基线与可复用能力

调研基于本地 `main` **b29ddb45db5e6db1d378cca9b2cd83efb825767d**、锁定的 Kit **0.8.0**；没有用工作分支上的 settings 测试改动推导设计。

| 当前入口 / 能力 | 当前行为与本次接点 |
| --- | --- |
| `home/composer.rs:453`、`:484` | file token 直接 `open_with_system`；技能说明 Dialog 的「打开文件」也直接外部打开。保留 token 和 Dialog，只改打开命令。 |
| `home/messages/resources.rs:132–224`、`home/messages.rs:766` | Markdown 资源插件和技能位置直接外部打开。插件还在 parse 中调用 `path.is_dir()`；改为只携带路径及来源上下文，文件系统判定放到点击后的后台任务。 |
| `home/workspace.rs:561` | `open_source` 激活 Files、复用同路径预览、保存当前预览及订阅；继续作为内部文件入口。内部打开还需沿用工作区的显式显示意图，不能仅创建一个当前布局不可见的预览。 |
| `home/files.rs:465` | `reveal` 展开父目录，但最后 `focus_open` 以 `open_path` 为目标；目录引用不能直接调用它就宣称已定位。需要让树的 reveal 目标与当前预览文件独立。 |
| `home/source.rs:8`、`:127` | `Preview` 持有只读 Editor、文件加载任务、加载版本及引用高亮装饰。本次由它拥有各预览类型，不增加全局预览状态。 |
| `home/files/directory.rs:91` | 当前只读 UTF-8 文本：1 MiB、20,000 行；NUL/无效 UTF-8 拒绝，长行关闭语法着色。扩展名黑名单提前拒绝图片等类型，需改为文本读取与公开图片组件之间的类型分派。 |
| `home/image_preview.rs:42–173` | 现有图片查看器拥有缩放、适配和滚动，但 `PreviewHost` 是全窗模态层。复用图片主体与缩放计算；Files 内不套用模态宿主、全窗尺寸或空白点击关闭。 |
| `host.rs:27`、`src/app/external.rs:7`、`home/workspace/reader.rs:443` | 已有 `Task<io::Result<()>>` 的系统打开器及失败通知。复用服务和反馈方式，不再调用无返回值的 `open_with_system`。成功仅表示系统接受打开请求。 |

上表 `home/` 位于 `crates/gupi-conversation-ui/src/home/`；该包已有 `image 0.25.10` 的 png/jpeg/gif/webp/bmp 支持，本次不升级依赖。

### 参考与公开 API

- Zed 本地源码 **56934ec056**：[`image_store.rs:249`](../../../../zed/crates/project/src/image_store.rs) 的 `is_image_file` 排除 SVG；[`image_viewer.rs:1083`](../../../../zed/crates/image_viewer/src/image_viewer.rs) 注册图片项目 item。只借鉴图片适配、缩放及文件内容查看，不复制完整编辑器工作区。
- Zed [`markdown_preview_view.rs:136–219`](../../../../zed/crates/markdown_preview/src/markdown_preview_view.rs) 提供 Markdown 预览打开及侧边预览；本次只在同一个预览位切换。`:1516` 的图片解析支持 HTTP(S) 和相对文件路径，说明图片加载是额外行为，不能把第三方默认加载策略当成已经批准的范围。
- Kit [`component/input/editor.rs`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/component/src/input/editor.rs) 与当前 `EditorState` 的 `set_readonly`、`set_value`、搜索/折行接口继续用于源码。
- Kit [`base/text/state.rs:175`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/state.rs#L175) 的 `TextViewState::markdown`、`:328` 的 `set_text`，以及 [`base/text/text_view.rs`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/text_view.rs) 的 `TextView::new`、`scrollable`、`on_link_click` 可构成保留状态、可选择复制的 Markdown 阅读面。`MarkdownPlugin` 是公开的内容扩展点；不访问 `inner` 或私有解析状态。
- Kit 0.9 component TextView 公开 `image_source`，同时控制图片测量和绘制；本地路径解析由应用提供，网络请求由 GPUI 官方 HTTP 客户端执行。
- GPUI 的公开 `img`、`ImageSource` 和 `canvas` 用于图片显示与画布平移/缩放；Kit 不提供统一文件类型查看器。图片直接交给公开组件加载、解码和显示，不建立应用自己的解码流程。

## 所有权与打开路由

### 一个语义入口

Home 接收「在此会话中打开资源」命令。输入包含资源 `PathBuf`、来源会话 key/连接 binding、该会话 cwd，以及键盘或指针激活信息。普通消息、历史预览、输入 token、技能 Dialog 都将同一语义交给 Home；Markdown 插件不读取全局 `current()`，也不自行决定开哪个窗口。

本地文件链接继续使用语义 Button 或现有资源 token。HTTP(S) 等已有网络链接保持原入口。相对资源路径按其明确的基准解析：消息/草稿资源使用所属会话 cwd，Markdown 文件正文的相对链接使用该文件父目录，然后仍按所属会话 cwd 分流。不要把 URL 的 fragment、百分号编码或 `file:` 当作普通文件名；沿用 URL/Path 的解析能力，不添加 shell 展开或自己猜 `~`、行号后缀。

技能说明 Dialog 保留；点击「打开文件」后关闭该说明层，再交给 Home。源 key 在打开 Dialog 时捕获，不能在按钮点击后重新拿另一会话的当前目录。历史技能展示继续读 Pi 已保存的说明，不重新执行技能、替换历史正文或加载新指令。

临时窗口没有 Files 工作区：其中的文件、目录和技能引用统一使用系统默认应用打开，包括 cwd 内的目标。复用同一个可失败 opener，不新建主窗口、不迁移临时会话，也不添加工作区。以下项目内外分流仅适用于普通窗口。

### 后台路径解析

点击后在后台把 cwd 和绝对目标分别 `canonicalize`，通过路径组件的 `strip_prefix` 判断归属，目录自身与 cwd 相等也属于项目内。绝不使用字符串前缀、Git 根或自行 lowercase 的路径规则。符号链接以真实目标为准：项目内链接指到外部则外部打开，外部别名指到项目内则内部打开。技能按同样规则处理；`~/.pi` 与 `.pi/skills` 只是常见位置，不是类型特判。

解析任务属于发起它的 Home/会话。返回时确认来源 key、binding、cwd 仍匹配，且该会话仍显示；同一窗口后发的打开意图不能被旧任务覆盖。失效结果直接丢弃，不转投新会话，也不擅自切回旧会话。此检查应在外部打开副作用之前完成。

| 解析结果 | 行为 |
| --- | --- |
| 目标在 cwd 内且为普通文件 | 显示 Files，调用既有 `open_source`，由预览加载器判断是否能显示。 |
| 目标在 cwd 内且为目录 | 显示 Files 树、展开祖先并选中该目录；不把目录交给文本加载器，不替换已有文件预览。cwd 自身定位树根。 |
| 目标在 cwd 外 | 调用可失败系统打开器。 |
| cwd 或目标无法解析、断链、已删除、权限错误 | 不猜项目内归属；将捕获时构造的绝对目标交给系统打开器尝试。失败显示本地化通知。 |
| 目标是特殊文件等非普通对象 | 不读取内容；按无法预览状态提供显式系统打开动作。 |

真实路径用于归属判定和目标身份，原路径用于用户可理解的标题/tooltip。现有 Files 根可能保留 cwd 的符号链接拼写；内部定位可用「规范化目标相对规范化 cwd 的分量」映射回现有树根，避免 canonical 路径与 `Files::root` 字符串不同导致定位失败。不要为此改写会话 cwd 或重建另一棵树。归属判断是导航规则，不是抵抗并发替换文件的安全沙箱；打开时失败正常走错误状态，不增加持续监听和反复校验框架。

### 外部打开

统一调用 `crate::host::open_file`。失败复用现有本地化摘要与日志方式；`diff-external-failed` 的文案是通用打开失败，可在所有调用者一并改名为资源级 key，九语言同步，不额外创建错误框架。技术错误留日志；按钮可再次尝试，成功不弹 toast。不预先用 `exists()` 禁用打开，也不把系统接受请求当作目标应用已加载文件。

Files 的「用默认应用打开」始终直接走外部服务，不能再次进入项目内路由形成循环。技能、普通文件与目录共享此服务。网络链接不改成文件打开器。

## Files 预览

### 布局和状态

沿用每会话的 `SessionView.source`、Files 标签、导航收起状态和 workspace allocator。宽窗是同一 Files 标签内的树与正文；窄窗按现有单区前景/临时列表规则先显示目标正文或目录树，返回仍用既有动作，不新增浮动文件窗口、分隔比例或标签体系。目录定位需要允许 reveal 一个显式目标，而不是用它冒充 `open_path`。

`Preview` 拥有路径、加载任务/版本以及互斥正文：源码、图片、Markdown 阅读态或不可预览状态。文本读取在后台，图片异步加载由 Kit 公开组件负责；结果只更新仍匹配的预览实例；切换文件释放上一个大图和任务。不在 render 中读文件或重复解析。保留显式 Reload，不加文件监听；同路径复用仍遵守现有 reload 行为。

标题栏继续显示文件名、路径 tooltip、只读状态、Reload、树切换和关闭。折行仅在源码态显示；图片显示适配/缩放控制；Markdown 显示「源码 / 预览」的 Kit ToggleGroup 或同等级互斥控件。Markdown 切换与图片控件位于标题栏下的紧凑工具行，避免挤出窄正文中的 Reload、树切换和关闭。所有控件使用主题 token 和已有尺寸，不给正文再套卡片。正文区域 `min_h_0/min_w_0`，文本模式只保留一个主滚动拥有者，图片使用独立画布。

### 内容分类

先确认打开的对象是普通文件，再分派到文本加载器或 Kit 图片组件。文本继续做有界读取和 UTF-8/NUL 检查；图片路径交给公开 `img` / `ImageSource`，由组件解码验证。扩展名只用于选择入口，不能保证实际内容可解码；图片加载失败显示不可预览状态，不降为乱码文本。

| 内容 | 首版显示 |
| --- | --- |
| UTF-8 文本（含 BOM） | 现有只读 Editor；保留 1 MiB / 20,000 行，以及长行降低着色成本的行为。 |
| `.md` / `.markdown` 的有效文本 | 同一加载快照的源码与 Markdown 阅读切换，初始默认源码，保持当前打开习惯；当前文件的切换状态留在 Preview，不写设置或历史。 |
| PNG、JPEG、GIF、WebP、BMP | 直接由 Kit 公开图片组件显示；帧播放沿用组件现有行为，不另建播放器。格式集合与现有依赖相同。 |
| SVG、CSV/TSV、ipynb | 在现有文本限制内按源码显示，不建立图形、表格或 notebook 查看器。 |
| PDF、Office、音视频、归档、其他二进制、解码失败或超限 | 保留标题和文件树位置，显示不可预览状态及显式外部打开按钮。 |

Files 与附件弹窗复用同一图片画布，和历史树共享 `Camera` 的 screen/world/zoom_at 几何；缩放限额由调用方负责（历史树上限 4 倍，图片上限 8 倍）。图片按 natural × zoom 绘制并用 pan 定位，不使用 ScrollHandle。首次 prepaint 取得真实画布尺寸后在同帧适应并居中，小图不放大；拖动或普通滚轮平移，修饰键滚轮/捏合按指针缩放，按钮按中心缩放。窗口尺寸改变时，适应态重新适应，手动视图保持中心。Files 保留适配/原尺寸控件，弹窗保留减/百分比/加，不新增快捷键或百分比输入。不要为了复用把文件预览转成草稿图片附件或创建临时发送数据。

### Markdown 切换与资源

同一份有界文本创建保留的 `TextViewState`，使用 Kit 0.9 component TextView 阅读与选择；源码和阅读态保留自己的局部滚动，切换不重新读取磁盘。重新加载更换快照并失效旧引用来源；源码态保留 Editor 搜索与引用能力。

本地图片使用公开 `TextView::image_source`：相对地址以 Markdown 文件父目录为基准，绝对路径与 `file://` 转为文件源，内置节点负责尺寸测量和绘制；不按会话 cwd 增加限制。其它地址保留 URI。图片来源解析同时用于测量和绘制，失败不回退原 URL。

应用通过 `Application::with_http_client` 安装官方 `gpui-pre-reqwest-client =0.3.11`。其 rustls 0.23 与平台证书验证和现有 reqwest 0.13 使用同一 TLS 后端；不启用 native-tls/OpenSSL。行内图片和引用式图片统一按 Kit 默认方式加载、显示及处理失败，没有 RemoteImages 插件、定义表复制、源文改写、应用图片缓存或自定义失败样式。图片加载会请求文档中的远程 URL，不发送文件内容给模型。

Markdown 本地链接继续走有来源的 Home 路由，HTTP(S) 链接交浏览器。阅读面不执行脚本或 MDX。主题、高亮、图片布局和缺图反馈由官方组件拥有。

### 状态、文案和可访问性

主文案说明结果及可做的操作：`无法预览 / Preview unavailable`、`用默认应用打开 / Open with default app`；读取失败沿用可重试状态，文本超限说明现有文本大小/行数限制，不称文件损坏。不支持与超限不弹 modal，不自动外部打开。复用 `files-source`、`files-preview`、`files-reload`、`files-readonly` 等 key；`files-unsupported` 需要去掉“文本预览”的限制措辞，图片加载失败不能误报文本的 1 MiB/20,000 行限制。九语言保持语义一致。

内部打开是 Button/Action，外部网页才使用 Link。键盘激活后焦点落在可见文件正文或选中的目录树，鼠标操作遵守现有预览焦点规则；从技能 Dialog 打开时不把焦点留在已关闭层。缩放、切换、外部打开均有可访问名称，状态不能只靠颜色表达。窄窗不让长路径推开正文或把操作挤出可见范围。

## 实现顺序与改动归属

1. **统一资源打开入口。** 在 conversation-ui 的 Home 范围内实现后台路径分类及来源校验，将 composer token/技能 Dialog、历史资源插件与技能位置接入；复用 root Host 的可失败打开服务。不要更改 Pi 层、token 数据格式或技能协议。移除这些入口的直接 `open_with_system` 与 parse 阶段文件 I/O。
2. **打通内部目标。** 复用 Files 激活与单区显示，修正目录 reveal 与当前预览目标混用；保持会话隔离和已打开文件状态。先使不支持类型也能显示状态与显式外部打开。
3. **引入图片正文。** 在 `home/source` / `home/files` 的既有归属内组织读取和分类，复用 `image_preview` 的显示主体；只抽取实际共享部分，不建立全局媒体框架或新 crate。
4. **完成 Markdown 切换。** 用保留的文本快照、component TextView 与来源回调实现；本地图片由 `image_source` 解析成文件源，远程行内/引用图片经官方 GPUI HTTP 客户端按 Kit 默认行为加载。
5. **收尾界面与证据。** 更新受影响九语言和必要使用说明，按实际行为调整现有正式入口回归，记录验证结果及限制。实现工作不包含当前独立进行的 #101 settings 测试修复。

本次实现仅同步本文件，不扩大开发索引或其它功能文档范围。

## 必要验证

新增测试前说明阻止的产品失败。这里真正需要保护的是：规范化路径/符号链接误分流、异步结果打开到另一会话、目录引用选错目标，以及图片加载失败时仍可显式外部打开。优先扩展现有资源打开、Files 与 Preview 正式入口测试；仅在现有测试无法观察独有契约时新增少量测试。不测试 Kit/std 的实现，不以格式枚举、控件数量或固定像素构建矩阵。

实现后完成 `cargo fmt --all -- --check` 和实际受影响包的 `--locked` 检查、`--all-targets` Clippy（`-D warnings`）及测试；共享图片主体被修改时要覆盖其现有使用方。原生验证从一段真实引用完成内部查看/外部失败反馈，检查窄窗定位及图片/Markdown 切换能完成任务；按实际验证的平台和主题汇报，不把源码审查或 headless 结果称作三平台原生通过。具体结果见下节；组件研究不代替实现或原生验收。


## 图片画布验证（升级前）

- 当前画布版本的 `cargo fmt --all -- --check`、UI/root 的 `cargo clippy --all-targets --locked -- -D warnings` 及测试通过：UI 226、root 127 项。测试未新增数量，删除 1 项旧滚动锚点专用测试；既有 Files 与弹窗交互回归覆盖小图不放大、两侧平移可达和指针锚点缩放。settings 20 项是此前本地化改动时的通过结果，本次未改设置包。
- 新增或扩展的正式入口回归覆盖组件边界/符号链接、旧会话及旧打开任务失效、目录定位、真实 composer token 指针激活、已加载 PNG 的按钮/滚轮/捏合、Markdown 同快照切换与 Reload。临时窗口 cwd 内文件仍走外部 opener 的分支已补入同一资源入口回归并单独通过。
- 隔离 gallery 的中文浅色实测：对话文件、目录及历史技能位置进入 Files；目录保留原正文；PNG 显示并可操作缩放；Markdown 源码/阅读及本地链接可用；不支持文件显示「无法预览」和外部打开按钮；删除外部测试目标后显示本地化打开失败通知。深色查看了图片与 Markdown。外部打开只确认系统请求/错误反馈，不声称目标应用完成加载。
- 最终 base 图片路线的新构建已在隔离 gallery 复测：800×400 的双色块/圆形图片在窄正文中完整按 2:1 缩小（约 700×350 屏幕像素），浅色与深色均确认；未加入自定义尺寸、缓存或引用式图片桥。
- 画布新构建的中文浅色原生检查：Files 与弹窗的大小图首次适应、按钮放大、两侧边缘拖动到达、普通滚轮平移通过；深色查看两种画布。初始化后的控件可自行启用，无需用户额外点击。修饰键滚轮/捏合缩放由正式 GPUI 事件回归覆盖；Computer Use 没有公开捏合接口，未宣称原生捏合验收。图库退出并清理。
- 原生临时窗口在工具随后观测时失活并按既有设计隐藏，该路径改由上述正式入口回归覆盖；未改失活行为。未做 Windows/Linux 原生验证；Windows 绝对路径断言仅在对应平台编译执行。gallery 已退出并清理测试数据，无模型请求。


先前本地图片路线运行中 root 失败的原始断言如下（本次画布门禁未复现）。未修改认证产品或测试；协调者已确认另行跟进，不将其推断为本轮图片改动导致，也不宣称已确定认证失败根因。

```text
---- features::settings::auth::tests::editor_retains_masked_keys_on_busy_and_saves_without_exposing_existing_values stdout ----

thread 'features::settings::auth::tests::editor_retains_masked_keys_on_busy_and_saves_without_exposing_existing_values' (29773268) panicked at src/features/settings/auth/tests.rs:107:9:
assertion `left == right` failed
  left: Some("")
 right: Some("free text provider")
```


## Kit 0.9 Markdown 核对

官方 HTTP 客户端与 component TextView 接入后，全 workspace 的 Nix 检查通过，原生隔离 Files 已显示 inline/reference HTTP 图片、本地相对路径图片及 HTTPS 图片；404 使用 Kit 默认空白失败呈现。当前升级验证见 [GPUI Kit 0.9 升级](../gpui-kit-0.9-upgrade/README.md#原生核对)。图片请求不发送文件内容给模型。
