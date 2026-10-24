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
- Kit base [`text/text_view.rs:275`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/text_view.rs#L275) 有公开 `image_source`，返回值同时用于绘制和尺寸测量，且不会回退原 URL；但 0.8.0 的 component TextView 未转发这个方法。因此阅读面直接使用 base TextView；远程图片入口单独通过公开 MarkdownPlugin 接入。
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

同一份有界文本创建保留的 `TextViewState`，用公开 base TextView 阅读/选择能力显示；切换不重新读取磁盘、不改文件。源码和阅读态各保留自己的局部滚动，不承诺段落与源码行同步。重新加载时统一更换快照并失效旧引用来源；只在阅读态启用阅读控件，在源码态保留 Editor 搜索与选区引用。首版不为 Markdown 阅读态新增引用工具条/来源高亮，避免把 #103 扩展成另一套 #95 选区集成。

点击 Markdown 中的本地路径仍走有来源的 Home 路由；HTTP(S) 链接交浏览器。纯文档锚点可用已有公开能力处理，做不到就不增加自制大纲/定位系统。只做 Markdown 阅读，不引入 HTML 浏览器、脚本执行或 MDX 组件执行。

内嵌普通图片使用 Kit base `TextView::image_source`：本地相对路径以 Markdown 文件父目录为基准，绝对路径及 `file://` 显式转换为文件源，交给内置图片节点测量与显示。本地图片不按会话 cwd 再限制。Kit 初始化安装的 `TextViewDefaults` 已提供主题和代码高亮，不复制 component wrapper 样式。远程 HTTP(S) 图片由小型 `RemoteImages` 插件转成替代文字及浏览器打开入口，不自动加载、不接入全局 HTTP 客户端。

引用式图片 `![alt][ref]` 不做特殊支持，沿用 Kit 默认表现；不建立定义表、AST 桥、源文改写或应用图片缓存。该选择替代最初由图片插件渲染全部图片的方案：自定义行内元素按 MaxContent 测量，无法可靠得到正文宽度，而内置图片节点由 Kit 处理尺寸。
首次阅读安装远程图片插件后，大文档可能在异步重解析完成前短暂沿用默认图片节点；当前 Gupi 的 GPUI HTTP 客户端为空，旧节点既不会实际联网，也不会把 URI 转为本地文件读取，因此接受这段缺图过渡，不增加空 state / 布局后装文本的 bootstrap。过渡以解析提交为界，不承诺一帧完成。

#### 历史研究：Kit 0.8.0 图片探针

2026-10-24 用当前锁定依赖编译并运行两个临时 headless 探针：真实 `component::text::TextView::markdown` 经正常布局/绘制，公开 `ImageCache` 只观察并转交 Kit 的默认加载器；HTTP 客户端记录请求，对 HTTP/HTTPS 图片返回固定 1×1 PNG，对失败项返回 404。本地绝对路径指向真实存在的同一 PNG。第二个探针同时渲染 component 的 `MarkdownPlugin` 与 base 的 `image_source` 两个入口，将相对地址解析到模拟 Markdown 文件父目录的临时目录，共享图片缓存成功读取 PNG。两项通过；探针及临时图片随后删除，依赖、锁文件和产品代码未修改。这是组件加载/解码实测，不是公网 HTTP/TLS 或原生视觉验收。

| 默认 component wrapper 输入 | 实测结果及含义 |
| --- | --- |
| `./img.png`、`../x.png` | 保持 `Resource::Uri`，请求构造报 `invalid format`，没有进入 HTTP handler，更没有本地文件读取。不以进程 cwd 或 Markdown 文件目录为基准。 |
| `/绝对路径/image.png` | 保持 URI，进入 HTTP handler，探针返回 404；即使文件存在也不读取。实际错误取决于宿主 HTTP 客户端，不能将探针的 404 当作本地文件不存在。 |
| `file:///绝对路径/image.png` | 保持 URI，请求构造报 `invalid format`；不会转换为文件路径。 |
| `http://…`、`https://…` | 发起 HTTP 请求；返回 PNG 后均成功解码为 1×1。宿主必须配置 HTTP 客户端，组件不自行提供网络服务。 |
| HTTP 404 | 缓存返回 `BadStatus`，没有可绘制图片。 |

失败显示由源码进一步确认：[`base/text/node.rs:2799`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/node.rs#L2799) 的图片元素没有安装失败替代内容；行内分支 [`inline_flow.rs:285`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/inline_flow.rs#L285) 也没有把 alt/title 画成错误文字，未获得固有尺寸时保留紧凑占位尺寸。因此默认失败是没有图片的空白位置，不是浏览器式断图加 alt，也没有应用级重试/打开按钮。此处是绘制实现判断，没有宣称原生截图验收。

默认路由可由 [`base/text/node.rs:795–806`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/node.rs#L795) 复核：除内嵌 data URL 外，文档地址全部保留为 URI；锁定 `gpui-pre 0.3.9` 的 `src/elements/img.rs:623–659` 仅对 `Resource::Path` 读文件，对 `Resource::Uri` 调 HTTP。当前 Gupi `src/app.rs:81` 只设置 assets，没有配置 HTTP 客户端；GPUI 默认 `NullHttpClient` 返回 `No HttpClient available`（`src/app.rs:3492–3505`）。因此当前 Gupi 不能仅加 Markdown wrapper 就获得远程图片加载。

不改依赖可用的公开控制点：

- **保留 component wrapper：** `.plugin(MarkdownPlugin)`（[`markdown_ext.rs:45`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/markdown_ext.rs#L45)，[`format/markdown.rs:764`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/crates/base/src/text/format/markdown.rs#L764)）可以在默认 Image AST 转换前截获图片，携带 Markdown 文件父目录，返回公开 `img(PathBuf)`；也可以只返回替代文字/显式打开入口。探针证实本地 PNG 经插件成功解码且没有 HTTP 请求。解析只生成路径/数据，实际加载交给组件。这一路线的自定义行内布局后来在大图窄窗复测中不满足尺寸要求，已放弃本地图片的插件绘制，不保留引用式 AST 特判。
- **使用 base TextView：** `gpui_kit::base::TextView::image_source` 可公开访问；探针证实解析相对地址后返回 `ImageSource::Resource(Resource::Path)` 能加载本地 PNG。此 resolver 同时服务尺寸测量和绘制，失败不会回退原 URL。component wrapper 没有转发此方法、也没有公开 inner；最终采用 base；component 在 Kit 初始化时已安装 `TextViewDefaults` 的主题和 highlighter，无需复制 wrapper 样式。
- **网络：** 宿主可通过公开 `Application::with_http_client` / `App::set_http_client` 接入 HTTP 实现；已有 `on_link_click` 只控制链接激活，不能拦截图片自动请求。本设计不接入远程图片加载，由插件生成交给浏览器的显式打开入口。

Zed 的 `resolve_preview_image`（[`markdown_preview_view.rs:1516–1581`](../../../../zed/crates/markdown_preview/src/markdown_preview_view.rs)）显式解析 HTTP(S) 和本地图片：普通相对地址按源文件目录解析，带工作区上下文的根路径另有工作区规则。Kit 默认 wrapper 没有这层解析。最终选型用 base `image_source` 显示本地普通图片，远程插件仅提供浏览器打开入口；引用式图片不扩展 Kit 的默认能力，无需引入全局 HTTP 接入。图片自动展示与点击引用的项目内外路由是两种行为；不擅自把 cwd 边界套成新的图片安全沙箱。不增加应用自定义图片尺寸或内存限制。

### 保留状态的插件安装时序

保留的 TextViewState 先持有同一文本快照，base 阅读面再安装远程图片插件和本地 `image_source`。锁定 Kit 的完整替换同步阈值为 4 KiB；较大文档的插件解析异步提交前，可能暂时缺图，且不保证只持续一帧。当前宿主只有 assets、未配置 HTTP client，默认 NullHttpClient 直接拒绝请求；默认图片 URI 也不会作为本地路径读取。因此接受这一过渡，不新增一次性 bootstrap、拒绝 URI 的生产 cache 或监听。未来实际接入 HTTP client 时再同步审视该路径。

### 状态、文案和可访问性

主文案说明结果及可做的操作：`无法预览 / Preview unavailable`、`用默认应用打开 / Open with default app`；读取失败沿用可重试状态，文本超限说明现有文本大小/行数限制，不称文件损坏。不支持与超限不弹 modal，不自动外部打开。复用 `files-source`、`files-preview`、`files-reload`、`files-readonly` 等 key；`files-unsupported` 需要去掉“文本预览”的限制措辞，图片加载失败不能误报文本的 1 MiB/20,000 行限制。九语言保持语义一致。

内部打开是 Button/Action，外部网页才使用 Link。键盘激活后焦点落在可见文件正文或选中的目录树，鼠标操作遵守现有预览焦点规则；从技能 Dialog 打开时不把焦点留在已关闭层。缩放、切换、外部打开均有可访问名称，状态不能只靠颜色表达。窄窗不让长路径推开正文或把操作挤出可见范围。

## 实现顺序与改动归属

1. **统一资源打开入口。** 在 conversation-ui 的 Home 范围内实现后台路径分类及来源校验，将 composer token/技能 Dialog、历史资源插件与技能位置接入；复用 root Host 的可失败打开服务。不要更改 Pi 层、token 数据格式或技能协议。移除这些入口的直接 `open_with_system` 与 parse 阶段文件 I/O。
2. **打通内部目标。** 复用 Files 激活与单区显示，修正目录 reveal 与当前预览目标混用；保持会话隔离和已打开文件状态。先使不支持类型也能显示状态与显式外部打开。
3. **引入图片正文。** 在 `home/source` / `home/files` 的既有归属内组织读取和分类，复用 `image_preview` 的显示主体；只抽取实际共享部分，不建立全局媒体框架或新 crate。
4. **完成 Markdown 切换。** 用保留的文本快照、公开 base TextView 与来源回调实现；本地普通图片由 `image_source` 解析成文件源，远程图片插件只呈现浏览器打开入口，不做引用式图片桥接或自动网络请求。
5. **收尾界面与证据。** 更新受影响九语言和必要使用说明，按实际行为调整现有正式入口回归，记录验证结果及限制。实现工作不包含当前独立进行的 #101 settings 测试修复。

本次实现仅同步本文件，不扩大开发索引或其它功能文档范围。

## 必要验证

新增测试前说明阻止的产品失败。这里真正需要保护的是：规范化路径/符号链接误分流、异步结果打开到另一会话、目录引用选错目标，以及图片加载失败时仍可显式外部打开。优先扩展现有资源打开、Files 与 Preview 正式入口测试；仅在现有测试无法观察独有契约时新增少量测试。不测试 Kit/std 的实现，不以格式枚举、控件数量或固定像素构建矩阵。

实现后完成 `cargo fmt --all -- --check` 和实际受影响包的 `--locked` 检查、`--all-targets` Clippy（`-D warnings`）及测试；共享图片主体被修改时要覆盖其现有使用方。原生验证从一段真实引用完成内部查看/外部失败反馈，检查窄窗定位及图片/Markdown 切换能完成任务；按实际验证的平台和主题汇报，不把源码审查或 headless 结果称作三平台原生通过。具体结果见下节；组件研究不代替实现或原生验收。


## 当前验证结果

- 当前画布版本的 `cargo fmt --all -- --check`、UI/root 的 `cargo clippy --all-targets --locked -- -D warnings` 及测试通过：UI 226、root 127 项。测试未新增数量，删除 1 项旧滚动锚点专用测试；既有 Files 与弹窗交互回归覆盖小图不放大、两侧平移可达和指针锚点缩放。settings 20 项是此前本地化改动时的通过结果，本次未改设置包。
- 新增或扩展的正式入口回归覆盖组件边界/符号链接、旧会话及旧打开任务失效、目录定位、真实 composer token 指针激活、已加载 PNG 的按钮/滚轮/捏合、Markdown 同快照切换与 Reload。临时窗口 cwd 内文件仍走外部 opener 的分支已补入同一资源入口回归并单独通过。
- 隔离 gallery 的中文浅色实测：对话文件、目录及历史技能位置进入 Files；目录保留原正文；PNG 显示并可操作缩放；Markdown 源码/阅读及本地链接可用；远程图片为显式链接；不支持文件显示「无法预览」和外部打开按钮；删除外部测试目标后显示本地化打开失败通知。深色查看了图片与 Markdown。外部打开只确认系统请求/错误反馈，不声称目标应用完成加载。
- 最终 base 图片路线的新构建已在隔离 gallery 复测：800×400 的双色块/圆形图片在窄正文中完整按 2:1 缩小（约 700×350 屏幕像素），浅色与深色均确认；远程图片仍显示为显式链接。未加入自定义尺寸、缓存或引用式图片桥。
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
