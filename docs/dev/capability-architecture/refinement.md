# 接口与依赖收敛

状态：Done。更新日期：2026-10-04。

六个能力 crate 的边界见[当前架构](README.md)。本轮按照已安装的官方 [gpui-kit skill](../../../.agents/skills/gpui-kit/SKILL.md) 与 [Coding Guides](../../../.agents/skills/gpui-kit/references/coding-guides.md) 完成接口收敛，没有增加 crate、升级依赖或改变配置/Pi RPC 格式。线上指南本轮未能读取，不宣称安装内容与线上最新版本完全一致。

## 状态写入与查询

Session 和 ConversationState 的行为字段私有。UI 通过只读查询、明确动作和 ReadView / ReadStatus / CatalogView 消费状态；查询投影不暴露运行任务。绑定标识与内容版本仍可只读查询，用于目标新鲜度和渲染缓存校验。execution、loading、catalog、content 等实现模块隐藏，必要的展示类型从 conversation 入口明确导出。

附件读取的任务、结果安装及通知归模型。界面发起加载、移除或错误反馈；模型拒绝重复读取，任务随会话删除或退出取消，切换选中会话不会改变结果的归属。快捷任务通过 begin_preparation / complete_preparation / finish_preparation 更新，完成时一次安装 Pi 快照、附件、模板、名称和草稿；取消准备后拒绝迟到的完成。准备允许从尚未得到 Pi 快照的新会话开始。

ConfigController 的 Store、Form 弱引用和关闭标识私有。ConfigReader 只提供读取与订阅，配置变更由控制器协调；Pi Form 仍作为明确的编辑入口共享。HomeView 的引用字段不再跨 crate 可写。现有输入 binding、来源抑制、IME、撤销与滚动归属保持不变。

新的公共记录 PreparedTemplate、PendingTemplate、Notice、NoticeContent 使用非穷尽声明和构造入口。测试所需的状态构造与通知位于 test-support feature，生产调用方无法通过这些夹具修改模型。未恢复旧 state/foundation 转发层。

## 应用接口与寿命

三个 host 的回调字段不再对外公开，根包通过构造函数安装。会话与配置 owner 创建时要求接口已经存在；其他入口通过必需的 Global 访问调用，漏装不能被解释成成功。无原生能力的 crate 测试显式安装 headless 实现；应用集成测试使用正式组合入口。

- 快捷键 prepare 同步返回失败并阻止写入；apply 只使用成功保存或恢复后的值。
- 会话关闭同步请求应用取消快捷任务准备；通知订阅在 owner 创建后延迟接入，继续由应用通知服务持有。
- 配置准入直接查询应用更新 owner 的权威状态。根包建立更新观察，回调弱引用配置控制器，订阅由控制器持有并随其释放；不额外复制 installing 布尔值。
- 原生窗口显隐、回填、通知呈现和有序退出仍在根包；host 回调不得同步重入调用中的同一 Entity。

## 依赖方向

| 能力 | 当前方向与变化 |
| --- | --- |
| conversation | 依赖 resources、pi-runtime 和 pi-rpc；已去掉 settings 与 Fluent 依赖 |
| conversation-ui | 依赖 conversation、settings、resources；工具图标映射位于 UI |
| settings | 生产依赖 resources、pi-rpc；更新禁用与跳过版本通过应用接口查询，updates 仅用于回归测试 |
| updates | 已去掉 resources；Windows 安装日志路径由应用调用方传入 |
| resources | 模板调用数据、参数引用与展开规则集中在 composer_resources；运行环境继续复用 pi-runtime |

会话通知携带语义 Kind、Severity 和操作反馈 Feedback，由根包映射提醒偏好与本地化文案。模板校验、删除和导出反馈不在模型中提前翻译。Pi 返回的原始消息继续保留原文；重连未确认使用语义错误，由 UI 本地化。

安装与配置写入仍互斥，“跳过此版本”仍可在安装会话内保存；Windows helper 参数与错误处理语义保持不变。设置和更新 UI 留在根包，模型继续使用 GPUI Entity/Task，不引入 common/core 包。

## 菜单同步

HomeView::render 不再同步原生菜单。同步由模型/视图通知、窗口激活、焦点变化、图片预览以及窗口 Root 的展示状态通知触发，复用现有 action_enabled 条件。Root 订阅覆盖弹窗打开与关闭；只监听焦点不足以保证即时禁用，回归测试已覆盖该时序。

隐藏的 HomeView 不发布菜单状态，避免后台会话变化覆盖设置界面的禁用投影；恢复显示时通知订阅重新同步。已有布局测量、虚拟列表、Markdown 缓存和版本失效机制保持原样，没有新增缓存或宣称性能提升。

## 文档一致性与官方 skill 的歧义

项目自有文档本轮已修正：

| 文档 | 修正 |
| --- | --- |
| 第一阶段设计 issue-218 | 删除旧目录树及“不得增加 Gupi 专用共享 crate”，统一链接当前架构，保留行为契约 |
| 设置设计 issue-231 | 当前九页包含通知，当前依赖为 0.7.0；0.6.4 和八页验证明确属于历史证据；更新资源、配置 owner 路径及快捷任务接入描述 |
| 临时窗口设计 issue-221 | 实现表改为拆分后的仓库相对路径 |
| 标题栏设计 issue-220 | 旧版组件源码明确标为初次设计证据 |
| 项目 gpui-i18n skill | 修正模块归属、翻译入口和当前语言 → 英文 → key 的回退顺序 |
| 项目 gpui-app-development skill | 链接本设计，区分当前实现与待收紧接口，保留项目边界，不复制通用规范 |

官方安装文件保持原样；以下是已安装内容内部的冲突或过度概括，不是新增项目框架规范：

| 位置 | 不一致 | 本项目采用的解释 |
| --- | --- | --- |
| [入口 Non-negotiables](../../../.agents/skills/gpui-kit/SKILL.md) 与 [Coding Guides / Public API design](../../../.agents/skills/gpui-kit/references/coding-guides.md) | 入口禁止跨边界公开字段；详细指南允许特意设计的数据记录 | 行为状态私有；记录按详细指南保留字段并提供非穷尽构造入口 |
| [Entity best practices](../../../.agents/skills/gpui-kit/references/gpui/entity-best-practices.md) 正文与末尾 checklist | 正文允许更新不同 Entity，checklist 禁止所有嵌套更新 | 检查同一 Entity 的借用重入及调用环，不能把顺序改写或 defer 当成所有嵌套访问的必需品 |
| 同文 Render Callbacks 章节 | 将外部 Entity 的 read/update 一概描述为 panic | 分析回调实际持有的借用及重入路径；必要的列表快照是只读投影，不复制整个业务 owner |
| 同文强引用 checklist 与 Coding Guides 的异步寿命规则 | checklist 禁止闭包中所有强引用，详细指南按是否应延长寿命选择弱引用 | 防止引用环、避免任务意外延长已关闭视图寿命；明确的拥有关系可以保留强 Entity |

“通过 gpui-kit 接入框架”不禁止已有 gpui-form、app-theme 或产品能力 crate；“稳定公共模块路径”也不要求恢复已删除的全局转发层。这两类措辞按各自范围解释，不构成需要删除产品边界的理由。

## 验证

2026-10-04，macOS Apple Silicon / Nix 开发环境：

- 分批覆盖 190 项受影响测试：根应用 45、conversation 32、conversation-ui 78、resources 7、settings 19、updates 9。修复后复测受影响场景；包括附件结果归属/取消/退出拒绝新任务、快捷任务取消、缺失 host、配置/安装互斥、弹窗菜单和隐藏视图回归。
- 受影响六个包的全目标、全 feature Clippy（`-D warnings`）、默认 feature 的 Gupi 构建、全 workspace 格式及 diff 检查通过。
- 本地原生试用使用独立 gallery 目录：主窗口、九页设置入口、Pi 探测就绪、project-a 会话连接、图片附件添加/移除、正常退出；进程退出码 0，日志包含 `managed quit completed`。最后补充的退出读取边界通过定向单元回归验证。
- 修改文档的本地链接检查通过；三个目标生产依赖已移除，Cargo.lock 外部包名称、版本和来源未变；官方安装 skill 未修改。

Windows/Linux 原生运行、真实模型请求和发行包未验证。未进行性能基准，不宣称此次结构调整带来性能提升。
