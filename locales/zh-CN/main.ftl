app-title = Gupi
menu-settings = 设置
menu-show-main = 主窗口
menu-quit = 退出 Gupi
startup-welcome = 欢迎使用 Gupi
temporary-setup-required = 请在设置中完成配置，再开始临时对话。
startup-checking = 正在检查…
startup-quitting = 正在完成操作…
settings-pi-command = Pi 可执行文件
settings-path-help = 留空以自动查找 pi，或填写可执行文件的绝对路径，不包含参数。
settings-theme = 外观
settings-language = 语言
settings-save-pi = 保存 Pi 路径
settings-reload = 重新读取
settings-write-current = 将已应用设置写回磁盘（保留草稿）
theme-system = 跟随系统
theme-light = 浅色
theme-dark = 深色
language-system = 跟随系统
language-english = English
language-chinese = 简体中文
recovery-config-title = 配置需要处理
recovery-startup-title = Gupi 无法启动
error-instance-startup = 请检查配置文件夹权限，或关闭其他正在运行的 Gupi 后重试。
recovery-pi-title = 配置 Pi 环境
recovery-confirm = 确定继续？重读成功后才会舍弃草稿；重置会先备份当前文件。
recovery-backup = 备份位置
error-config-read = 无法读取配置，请检查文件位置和权限后重读。
error-config-parse = 配置格式无效。可修正文件后重读，或备份并重置。
error-config-validation = 请填写单个可执行文件名或绝对可执行路径，不附带参数。
error-config-write = 保存失败，草稿与已应用设置已保留。请检查配置文件后重试。
error-pi-probe = 无法验证 Pi。请检查可执行文件路径和 Node 环境后重试。
action-check-pi = 重新检查 Pi
action-reset = 备份并重置
action-confirm = 确认
action-cancel = 取消
home-ready = Pi 已就绪
home-description = 当前版本提供环境设置与桌面偏好。对话功能将在下一阶段接入。
action-locate = 显示配置文件夹
error-pi-timeout = Pi 未在 15 秒内完成版本检查。
error-pi-output = Pi 返回的内容超出了版本检查的输出限制。
error-pi-version = Pi 未正常退出，或未返回有效版本号。
error-log = 无法写入日志文件，请检查日志目录与权限。

setup-tagline = 为 Pi 准备的原生桌面空间
setup-intro = 简单设置，即可开始。
setup-start = 开始设置
setup-language-title = 选择你的语言
setup-search-language = 搜索语言…
setup-system-language = 系统语言：{ $language }
setup-appearance-title = 选择喜欢的外观
setup-color-mode = 亮暗模式
setup-system-accent = 系统强调色
setup-pi-title = 连接本机 Pi
setup-pi-step = Pi 配置
setup-pi-help = 可在此配置并检测本机 Pi，也可以跳过此页，稍后再设置。
setup-check-pi = 检测 Pi
setup-back = 上一步
setup-next = 下一步
setup-finish = 完成设置
setup-saving = 正在保存…
light-themes = 浅色主题
dark-themes = 深色主题

# Conversation workspace
conversation-new = 新建会话
conversation-sidebar = 会话侧边栏
conversation-history = 会话历史
conversation-restoring = 正在恢复草稿…
conversation-discovering = 正在查找会话… 已发现 { $count } 个文件
conversation-read-progress = 已读取 { $completed } / { $total } 个文件
conversation-refresh-progress = 正在刷新 { $completed } / { $total } 个文件
conversation-project = 选择工作目录
conversation-project-search-placeholder = 搜索项目
conversation-project-search-empty = 没有匹配的项目
conversation-open-folder = 打开文件夹…
conversation-untitled = 未命名会话
conversation-idle = 空闲
conversation-loading = 正在打开
conversation-running = 正在运行
conversation-failed = 需要处理错误
conversation-waiting = 等待输入
conversation-search = 搜索会话
conversation-refresh = 刷新会话目录
conversation-refresh-current = 刷新当前会话
conversation-actions = 会话操作
conversation-show-sidebar = 展开会话侧栏
conversation-hide-sidebar = 收起会话侧栏
conversation-scan-failed = 会话目录加载失败
conversation-search-placeholder = 搜索名称、消息或项目路径
conversation-search-empty = 没有匹配的会话
conversation-rename = 重命名…
conversation-delete = 移到废纸篓
conversation-delete-failed = 无法删除会话。请查看详情后再试。
conversation-copy-path = 复制路径
conversation-clone = 复制会话
conversation-export = 导出会话为 HTML…
conversation-exported = 会话已导出至 { $path }
conversation-stop = 停止生成
conversation-close-run = 结束运行
conversation-show-less = 收起
conversation-show-more = 查看更多…
conversation-continue = 从此处继续
conversation-fork = 从这里另开会话
conversation-preview = 正在预览其他分支
conversation-return-current = 返回当前分支
conversation-welcome = 开始一段对话
conversation-welcome-project = 你想在 { $project } 中做些什么？
conversation-empty-history = 此会话还没有消息。
conversation-bottom = 返回底部
conversation-copy = 复制
conversation-compaction = 上下文压缩摘要
conversation-branch-summary = 分支摘要
conversation-working = 正在处理…
conversation-process = 查看处理过程
conversation-details = 展开或收起细节
tool-detail-offset = 起始行：
tool-detail-line-limit = 请求的行数上限：
tool-detail-timeout = 超时（秒）：
tool-detail-output = 输出
tool-detail-input = 输入
tool-detail-additional = 附加信息
tool-detail-error = 错误
tool-detail-glob = 文件匹配模式：
tool-detail-ignore-case = 忽略大小写：
tool-detail-literal = 按字面文本搜索：
tool-detail-context = 上下文行数：
tool-detail-result-limit = 请求的结果数量上限：
tool-detail-old-text = 请求匹配的原文
tool-detail-new-text = 请求替换的内容
tool-detail-truncated = Pi 已截断输出，当前显示的内容并非完整结果。
tool-detail-lines-truncated = Pi 已缩短部分结果行。
tool-detail-match-limit = 已达到匹配数量上限：
tool-detail-results-limited = 已达到结果数量上限：
tool-detail-entries-limited = 已达到目录条目数量上限：
tool-detail-full-output = 完整输出文件：
tool-detail-image-unavailable = 无法显示此图片。
conversation-role-user = 用户消息
conversation-role-assistant = 助手消息
conversation-role-tool = 工具结果
conversation-event = 会话事件
conversation-close-history = 收起会话历史
conversation-source = 查看来源会话文件
conversation-reconnect = 重新连接
conversation-interrupted = 本次运行已停止；已保留现有内容。
conversation-no = 否
conversation-submit = 提交
conversation-input = 输入消息
conversation-model = 模型
conversation-load-options = 连接 Pi 以加载可用模型
conversation-thinking = 思考等级
conversation-unknown = 暂不可用
conversation-context = 当前执行分支上下文
conversation-auto-compaction = 自动压缩
conversation-on = 已开启
conversation-off = 已关闭
conversation-tokens = 累计输入 / 输出
conversation-cache = 累计缓存读取 / 写入
conversation-cache-hit = 最近缓存命中率
conversation-cost = Pi 统计费用
conversation-statistics = 用量统计
conversation-send = 发送（Enter）；Alt+Enter 等待本轮结束后发送
conversation-sending = 正在发送…
conversation-graph-current = 当前执行
conversation-graph-preview = 预览
conversation-graph-left = 查看左侧轨道（也可横向滚动）
conversation-graph-right = 查看右侧轨道（也可横向滚动）
conversation-graph-reveal = 定位选中
conversation-graph-column = 分支
conversation-graph-message = 消息
conversation-catalog-empty = 暂无会话

conversation-thinking-content = 思考过程
conversation-thinking-running = 正在思考
conversation-tool-group = 工具调用（{ $count }）
conversation-tool-group-read = 读取文件（{ $count }）
conversation-tool-group-bash = 运行命令（{ $count }）
conversation-tool-group-search = 查找内容（{ $count }）
conversation-tool-group-edit = 修改文件（{ $count }）
conversation-tool-line = { $action } { $summary }
conversation-tool-action-read =
    { $state ->
        [running] 正在读取
        [complete] 已读取
        [failed] 读取失败
        *[unfinished] 未完成读取
    }
conversation-tool-action-write =
    { $state ->
        [running] 正在写入
        [complete] 已写入
        [failed] 写入失败
        *[unfinished] 未完成写入
    }
conversation-tool-action-edit =
    { $state ->
        [running] 正在编辑
        [complete] 已编辑
        [failed] 编辑失败
        *[unfinished] 未完成编辑
    }
conversation-tool-action-bash =
    { $state ->
        [running] 正在运行
        [complete] 已运行
        [failed] 运行失败
        *[unfinished] 未完成运行
    }
conversation-tool-action-search =
    { $state ->
        [running] 正在查找
        [complete] 已查找
        [failed] 查找失败
        *[unfinished] 未完成查找
    }
conversation-tool-action-other =
    { $state ->
        [running] 正在调用 { $name }
        [complete] 已调用 { $name }
        [failed] 调用失败 { $name }
        *[unfinished] 未完成调用 { $name }
    }

conversation-processed = 已处理 { $duration }
conversation-processed-failed = 处理失败 { $duration }
conversation-processed-stopped = 已停止 { $duration }
conversation-copied = 已复制
conversation-copy-failed = 复制失败，请重试
conversation-usage-title = 本次请求用量
conversation-usage-model = 模型
conversation-usage-provider = 服务商
conversation-usage-input = 输入 Token
conversation-usage-output = 输出 Token
conversation-usage-cache-read = 缓存读取 Token
conversation-usage-cache-write = 缓存写入 Token
conversation-usage-total = 总 Token
conversation-usage-cost = 费用

composer-context-used = 已占用 Token
composer-context-limit = 上下文容量
composer-context-percent = 占用比例
composer-token-input = 累计输入 Token
composer-token-output = 累计输出 Token
composer-token-cache-read = 累计缓存读取 Token
composer-token-cache-write = 累计缓存写入 Token
composer-token-usage = 会话 Token 用量

conversation-model-search = 搜索模型…
conversation-model-empty = 没有匹配的模型
conversation-thinking-off = 关闭
conversation-thinking-minimal = 最小
conversation-thinking-low = 低
conversation-thinking-medium = 中
conversation-thinking-high = 高
conversation-thinking-xhigh = 超高
conversation-thinking-max = 最大

conversation-model-reasoning = 推理
conversation-model-vision = 视觉

conversation-history-reply = 助手回复
conversation-history-brief = 简略：用户消息与最终回答
conversation-history-detailed = 详细：消息、工具和摘要
conversation-history-progress = 助手过程
conversation-history-thinking = 思考
conversation-history-calls = 工具调用
conversation-history-failed = 执行失败
conversation-history-stopped = 已中断
conversation-history-empty = 无正文回复
composer-model-thinking = 模型与思考等级
composer-model-refresh = 重新读取模型与思考等级
composer-thinking-unavailable = 不支持思考调节
composer-model-loading = 正在读取模型列表…
composer-thinking-loading = 正在读取思考等级…
composer-model-confirming = 正在确认模型设置…
composer-stats-loading = 正在读取用量…
composer-stats-retry = 重新读取用量
conversation-checking-file = 正在检查会话文件…
conversation-connecting = 正在连接 Pi…
conversation-history-loading = 正在读取会话历史…
conversation-history-refreshing = 正在刷新会话历史…
conversation-fork-options-loading = 正在读取另开会话选项…
conversation-fork-options-retry = 重新读取另开会话选项
history-canvas-zoom-in = 放大
history-canvas-zoom-out = 缩小
history-canvas-fit = 查看全图（0）
history-canvas-current = 定位当前执行节点
history-canvas-expand = 展开 { $count } 个节点
history-canvas-collapse = 收起 { $count } 个过程节点，保留当前定位
history-canvas-empty = 暂无可显示的历史节点
history-canvas-help = 会话树画布：滚动或拖动平移，捏合或加减号缩放，方向键选择，Enter 预览，E 展开下一段

history-view-tree = 树
history-view-list = 列表
history-level-brief = 简略
history-level-detailed = 详细
history-level-all = 全部
history-level-all-description = 全部：包括设置变化、标签和自定义记录
history-scope-all = 全部分支
history-scope-branch = 仅此分支
history-model-change = 模型变更
history-thinking-change = 思考等级变更
history-session-info = 会话信息变更
history-label-change = 标签变更
history-custom-record = 自定义记录


history-content = 内容
history-range = 范围

command-palette = 命令面板
command-focus-input = 聚焦对话输入
command-model = 选择模型与思考等级…
command-copy-last-answer = 复制最后回答
command-compact = 压缩上下文
conversation-compacting = 正在压缩上下文…
command-history-description = 查看历史，或选择用户消息分叉
command-show-history = 显示历史
command-hide-history = 隐藏历史
command-current-session = 当前会话：
command-unavailable = 当前状态不可用
command-target-changed = 此会话已不可用，请选择其他会话。
command-scanning = 正在加载会话…
command-loading = 正在加载 Pi 命令…
command-no-matches = 没有匹配的 Pi 命令
conversation-reconnect-unconfirmed = 无法确认 Pi 进程已退出，请重启 Gupi 后再连接。
action-retry = 重试

command-search-placeholder = 搜索操作或输入命令
command-empty = 当前会话没有 Pi 命令

command-group-app = 应用
command-group-extensions = 插件命令
command-group-skills = 技能
command-group-prompts = 提示词模板
command-scope-user = 个人
command-scope-project = 项目
command-scope-temporary = 临时
command-connection-unavailable = Pi 尚未就绪
command-dismiss = 退出
command-complete = 补全
command-send-text = 发送
command-execute = 执行

command-scope-current = 当前会话

conversation-working-duration = 正在处理 { $duration }
conversation-tool-group-skill = 读取技能（{ $count }）
conversation-shell-line = { $shell } · { $action } { $summary }
conversation-tool-action-skill =
    { $state ->
        [running] 正在读取技能
        [complete] 已读取技能
        [failed] 技能读取失败
       *[unfinished] 技能读取未完成
    }

# Unified settings
settings-page-general = 通用
settings-page-pi = Pi
settings-page-keys = 快捷键
settings-page-skills = Skill
settings-page-prompts = 提示词
settings-page-about = 关于
settings-key-reset = 恢复默认
settings-key-reset-all = 全部恢复默认
settings-key-reset-all-confirm = 恢复所有快捷键的默认绑定？所有自定义快捷键将被清除。
settings-key-clear = 清除快捷键
settings-key-cancel = 取消本次修改
settings-key-unbound = 未设置快捷键
settings-key-recording = 按下快捷键，Esc 取消
settings-key-conflict = 快捷键冲突或无效
settings-key-invalid = 请输入有效的快捷键；留空表示取消绑定
settings-resource-refresh = 重新读取
settings-resource-reload-help = 仅管理个人资源。修改后，请手动刷新需要更新的会话。
settings-resource-working = 正在处理…
settings-resource-pi-required = 请先在 Pi 设置中保存并检查可用的 Pi 命令。
settings-resource-invalid-name = 请输入合法文件名，不可包含路径分隔符。
settings-resource-register = 添加本地
settings-resource-create = 创建
settings-resource-empty = 没有匹配的资源。
settings-resource-delete = 移到废纸篓
settings-resource-save = 保存正文
shortcut-save-task = 保存
settings-resource-loading = 正在读取正文…
settings-resource-confirm-remove = 仅将所选文件移到废纸篓，保留所在目录。
settings-package-install = 安装包
settings-package-update = 更新
settings-package-remove = 移除
settings-package-source-help = 包来源：npm:包名、Git 地址或本地绝对路径。
settings-package-extensions = 扩展 { $count }
settings-package-skills = Skill { $count }
settings-package-prompts = 模板 { $count }
settings-package-themes = 主题 { $count }
settings-skill-search = 搜索 Skill 名称、说明、路径或来源
settings-skill-collapse = 收起
settings-resource-edit = 编辑…
settings-about-gupi = 版本
settings-about-pi = 版本
settings-about-path = 实际路径
settings-about-status = 检查状态
settings-about-unavailable = 尚未取得
settings-editor-discard = 放弃未保存的正文修改吗？
settings-package-confirm-remove = 从所选 Pi 作用域中移除；本地来源目录会保留。
settings-package-heading = 已安装包
settings-extension-heading = 独立扩展
settings-template-heading = 命令模板
settings-system-heading = 系统提示词
settings-config-heading = 配置文件
settings-config-open = 打开配置文件
settings-mode-help = 跟随系统时，自动使用对应的浅色或深色主题。
settings-light-help = 浅色模式下使用此主题。
settings-dark-help = 深色模式下使用此主题。
settings-pi-unsaved = 路径尚未保存；检查仅验证当前输入，不会保存修改。
settings-pi-saved = 当前路径已保存。检查可重新确认 Pi 是否可用。
settings-resource-location = 显示位置
settings-package-source = 包来源
settings-package-empty = 尚未安装个人 Pi 包。
settings-package-resources-empty = 未发现此包声明的资源。
settings-resource-name = 名称
settings-resource-none = 尚无此类个人资源。
settings-resource-view = 查看
settings-resource-readonly-badge = 只读
settings-source-personal = 个人
settings-source-external = 外部来源
settings-system-replace = 替换默认系统提示词
settings-system-replace-help = 完整替换 Pi 的默认系统提示词。
settings-system-append = 追加个人指令
settings-system-append-help = 保留默认系统提示词，并在末尾追加这些指令。
settings-resource-success = 已完成对“{ $target }”的操作。已有会话需手动刷新。
settings-resource-failed = 对“{ $target }”的操作失败。
settings-about-help = 显示已保存路径最近一次检查的结果；如需重新检查，请进入 Pi 设置。
settings-key-reveal-session = 显示会话文件

settings-key-group-app = 应用
settings-key-group-conversation = 会话
settings-key-group-files = 会话文件

settings-package-kind-extensions = 扩展
settings-package-kind-skills = Skill
settings-package-kind-themes = 主题
settings-package-kind-prompts = 模板
settings-package-expand = 展开包内容
settings-package-collapse = 收起包内容

settings-template-search = 搜索模板名称、说明或来源
settings-template-collapse = 收起预览

temporary-title = 临时对话
attachment-add = 添加附件
attachment-remove = 移除附件
attachment-file = 文件
attachment-clipboard = 剪贴板图片
image-preview-close = 关闭预览
image-preview-open = 查看图片
image-preview-zoom-in = 放大
image-preview-zoom-out = 缩小
shortcut-global = 全局快捷键
shortcut-launcher = 临时窗口
shortcut-add = 添加模板任务…
shortcut-edit = 编辑模板任务…
shortcut-delete = 删除模板任务
shortcut-name = 名称
shortcut-template = 命令模板
shortcut-source = 输入来源
shortcut-selection = 选中文字
shortcut-clipboard = 剪贴板
shortcut-fallback = 优先选中文字，否则剪贴板
shortcut-model = 模型
shortcut-thinking = 思考等级
shortcut-default-model = Pi 默认模型
shortcut-default-thinking = Pi 默认思考等级
shortcut-personal = 个人
shortcut-template-required = 请选择命令模板。
shortcut-reload-options = 重新读取选项
temporary-clean-released = 已释放的工作目录
temporary-clean = 清理
temporary-clean-help = 将已释放的临时工作目录移到废纸篓，保留仍在使用的会话。
temporary-cleaned = 已移到废纸篓的工作目录
shortcut-tasks = 模板快捷任务

shortcut-select-template = 选择命令模板

temporary-search-placeholder = 搜索临时会话
temporary-search-empty = 没有匹配的临时会话
temporary-toggle-input = 切换输入
temporary-hide = 隐藏
session-search-open = 打开会话

temporary-actions = 操作
temporary-paste-answer = 粘贴最后回答
temporary-reveal-workspace = 定位工作目录
temporary-search-actions = 搜索操作…
temporary-switch-session = 切换临时会话
temporary-paste-failed = 回答已复制，自动粘贴未完成。请检查辅助功能权限和目标应用，或手动粘贴。
temporary-session-1 = 切换到第 1 个临时会话
temporary-session-2 = 切换到第 2 个临时会话
temporary-session-3 = 切换到第 3 个临时会话
temporary-session-4 = 切换到第 4 个临时会话
temporary-session-5 = 切换到第 5 个临时会话
temporary-session-6 = 切换到第 6 个临时会话
temporary-session-7 = 切换到第 7 个临时会话
temporary-session-8 = 切换到第 8 个临时会话
temporary-session-9 = 切换到最后一个临时会话
temporary-send = 发送
temporary-trash = 移到废纸篓（临时会话）
settings-key-stop-or-hide = 停止生成 / 隐藏临时窗口

conversation-queue-title = 待处理（{ $count }）
conversation-queue-steer = 本轮补充
conversation-queue-follow-up = 后续任务
conversation-queue-restore = 全部文字取回草稿
conversation-queue-clear = 清空全部排队消息
conversation-queue-no-text = 无文字消息
conversation-queue-unavailable = 暂未获取队列内容
conversation-queue-text-only = 取回仅恢复文字，排队图片无法恢复。

conversation-retry-countdown = 第 { $attempt }/{ $total } 次重试，约 { $seconds } 秒后重试
conversation-retry-waiting = 第 { $attempt }/{ $total } 次重试，等待 Pi
conversation-summary-retry-countdown = 摘要第 { $attempt }/{ $total } 次重试，约 { $seconds } 秒后重试
conversation-summary-retry-waiting = 摘要第 { $attempt }/{ $total } 次重试，等待 Pi

message-details-open = 查看详情…
message-details-copy-all = 复制全部
tool-detail-command = 命令
tool-detail-content = 内容
tool-detail-changes = 修改

settings-notifications = 通知
settings-notification-help = 系统通知只显示通用描述，点击返回来源会话。在前台打开会话后清除未读标记。
settings-notification-waiting = 后台需要输入时发送系统通知
settings-notification-failures = 后台任务失败时发送系统通知
settings-notification-plugins = 将插件提醒作为后台系统通知发送
settings-notification-attention = 后台需要输入时请求系统注意力
settings-notification-completion = 回答完成通知
notification-off = 关闭
notification-background = 仅后台
notification-always = 始终
notification-waiting = 等待你的输入
notification-completed = 回答已完成
notification-failed = 任务失败，需要查看
notification-plugin = 插件提醒
notification-unread = 未读会话
notification-running = 运行中
notification-clear = 清除提醒
notification-session-notices = 会话提醒

settings-icon-theme = 应用图标
settings-icon-theme-help = 更改应用内标记和运行时的 macOS Dock 图标；Finder 与 Windows 任务栏保留打包图标。
icon-theme-classic = 经典
icon-theme-classic-gradient = 经典渐变
icon-theme-color = Pi 彩色
icon-theme-color-gradient = Pi 渐变
icon-theme-pride = 彩虹
icon-theme-ukraine = 乌克兰
icon-theme-ukraine-gradient = 乌克兰渐变

language-traditional-chinese = 繁體中文
language-japanese = 日本語
language-korean = 한국어
language-german = Deutsch
language-french = Français
language-spanish = Español
language-portuguese-brazil = Português (Brasil)
menu-about = 关于 Gupi
menu-conversation = 会话
menu-edit = 编辑
menu-view = 显示
menu-window = 窗口
menu-help = 帮助
menu-services = 服务
menu-hide = 隐藏 Gupi
menu-hide-others = 隐藏其他
menu-show-all = 显示全部
menu-undo = 撤销
menu-redo = 重做
menu-cut = 剪切
menu-copy = 复制
menu-paste = 粘贴
menu-select-all = 全选
menu-minimize = 最小化
menu-zoom = 缩放
menu-fullscreen = 切换全屏
menu-docs = Gupi 使用指南
menu-pi-docs = Pi 文档
menu-report-issue = 报告问题
menu-logs = 显示日志
menu-copy-diagnostics = 复制诊断信息
settings-permissions = 系统权限
settings-accessibility-help = 读取选中文字和向其他应用粘贴可能需要辅助功能权限。在系统设置中允许 Gupi 后，重新执行操作即可。粘贴失败时仍可复制回答。
settings-accessibility-open = 辅助功能设置
settings-notification-permission-help = 若没有通知或 Dock 角标，请在系统设置中允许 Gupi 的通知和角标。权限会在首次使用相关功能时请求。
settings-notification-permission-open = 系统通知设置
settings-native-language-help = 应用语言即时切换。macOS 系统对话框需重启 Gupi 后生效；Windows 系统对话框跟随 Windows 显示语言。
settings-diagnostics-help = 包含应用版本、平台、Pi 命令及配置和日志位置，不包含会话正文、凭据或环境变量。

setup-preferences-title = 语言与外观
setup-preferences-step = 语言与外观
setup-desktop-title = 快捷键与通知
setup-desktop-step = 快捷键与通知
setup-desktop-help = 稍后可以在设置中更改这些选项。
setup-page-skip = 跳过此页
setup-skip-all = 使用默认设置
setup-pi-later = 稍后配置 Pi
action-open = 打开

setup-appearance-options = 选择主题与图标
setup-pi-install-guide = 安装 Pi
setup-pi-model-guide = 配置模型访问

conversation-plugin-message = 插件消息
conversation-session-info-command = 会话信息…
conversation-session-info-title = 会话信息
conversation-session-info-identity = 身份与位置
conversation-session-info-name = 会话
conversation-session-info-id = Pi 会话 ID
conversation-session-info-directory = 工作目录
conversation-session-info-file = 会话文件
conversation-session-info-not-saved = 尚未保存
conversation-session-info-runtime = 运行配置
conversation-session-info-model = 模型
conversation-session-info-thinking = 思考等级
conversation-session-info-usage = 用量
conversation-session-info-input = 输入 Token
conversation-session-info-output = 输出 Token
conversation-session-info-cache-read = 缓存读取
conversation-session-info-cache-write = 缓存写入
conversation-session-info-cost = Pi 报告的费用
conversation-session-info-context = 当前上下文（Token / 窗口 / 占用率）
conversation-session-info-context-value = 已用 { $tokens } / 窗口 { $window } Token（{ $percent }）
conversation-session-info-counts = 会话统计
conversation-session-info-user-messages = 用户消息
conversation-session-info-assistant-messages = 助手消息
conversation-session-info-tool-calls = 工具调用
conversation-session-info-tool-results = 工具结果
conversation-session-info-total-messages = 消息总数

conversation-find = 查找会话正文
conversation-find-placeholder = 查找会话正文
conversation-find-count = { $current } / { $total }
conversation-find-previous = 上一处（Shift+Enter）
conversation-find-next = 下一处（Enter）
conversation-find-close = 关闭查找（Esc）

# Composer resources
command-select-resource = 填入消息
command-group-files = 文件和文件夹
resource-files-loading = 正在搜索项目文件和文件夹…
resource-open-file = 打开文件
resource-skill-content = Skill 指令
resource-template-unreadable = 无法读取提示词模板。请重新加载资源后重试。
resource-template-unused-files = 此模板未使用全部文件引用参数。请补充相应参数（例如 $ARGUMENTS），或移除这些引用后再发送。
resource-search-files = 搜索项目文件和文件夹…
resource-choose-files = 选择文件或文件夹…

resource-edit-in-composer = 在这里选择资源，参数回消息输入框编辑。
resource-template-unclosed-quote = 模板参数中有未闭合的引号，请补全后再发送文件引用。

resource-answer-required = 请选择一个答案后提交。

menu-check-updates = 检查更新…
updates-title = 更新
updates-check = 检查更新
updates-automatic = 自动检查更新
updates-automatic-help = 每小时通过 GitHub 检查新版本，由你选择何时下载。
updates-idle = 检查是否有新版 Gupi。
updates-checking = 正在检查更新
updates-current = 已是最新版本
updates-unpublished = 尚未发布正式版本。
updates-available = Gupi { $version } 已发布
updates-release = 更新说明与下载
updates-install-help = 请使用原来的安装器或包管理器更新 Gupi。
updates-error-network = 无法检查更新，请检查网络连接后重试。
updates-error-rate-limit = GitHub 请求次数已达上限，请稍后重试。
updates-error-response = 无法读取发行信息，请稍后重试。

updates-install = 下载并安装…
updates-native-progress = 请在更新窗口中完成更新。
updates-install-failed = 无法完成更新。请重试，或重新打开 Gupi 并查看日志。
updates-restart-help = 安装时会保存草稿、关闭 Pi 连接并重启 Gupi。
updates-show-progress = 查看更新进度…

shortcut-screenshot = 截图
screenshot-failed = 无法截取屏幕，请重试。
screenshot-permission-required = 截图需要屏幕录制权限。请在“系统设置 → 隐私与安全性 → 屏幕录制”中允许 Gupi，然后重试。如系统提示重启 Gupi，请先重启。

# Project files and source preview
files-title = 文件
files-panel = 工作区
files-close = 关闭面板
files-refresh = 重新读取
files-preview = 预览
files-readonly = 只读
files-wrap = 自动换行
files-reload = 重新读取
files-loading = 正在读取…
files-expanded = 已展开
files-collapsed = 已收起
files-incomplete = 部分项目无法读取，点击重试
files-read-failed = 无法读取文件或目录，请重试
files-unsupported = 此文件暂不支持文本预览
files-too-large = 文件超过预览上限（1 MiB 或 20,000 行）
files-long-line = 存在超长行，已关闭语法高亮
files-no-session = 选择会话以浏览项目文件

files-actions = 文件操作
files-copy-path = 复制路径
files-reveal = 在文件中定位
files-close-source = 关闭文件
files-focus-source = 聚焦源码
files-conversation = 会话
files-conversation-busy = 会话，正在回复
files-view = 视图
files-source = 源码

files-link-label = { $name } — 符号链接
files-link-directory-label = { $name } — 目录链接，{ $state }
files-link-unavailable-label = { $name } — 链接目标不可用
files-link-unavailable = 链接目标不可用

# Settings and shortcuts redesign
settings-page-packages = 包
settings-app-language = 界面语言
settings-config-location = 位置
settings-temporary-storage = 临时对话存储
settings-accessibility = 辅助功能
settings-open-system-settings = 打开系统设置
settings-color-mode-label = 模式
settings-light-help-active = 浅色模式下使用此主题。当前正在使用。
settings-dark-help-active = 深色模式下使用此主题。当前正在使用。
settings-icon-theme-help-other = 更改应用内标记；任务栏和文件管理器保留打包图标。
settings-notification-background-heading = 后台活动
settings-notification-completion-heading = 回答完成
settings-notification-permission-help-windows = 如果收不到提醒，请在 Windows 设置中允许 Gupi 通知。首次使用该功能时会请求权限。
settings-pi-status-ready = 可用 · { $version } · { $path }
settings-pi-draft = 未保存
settings-pi-saved-tag = 已保存
settings-diagnostics = 诊断
settings-open-pi = 打开 Pi 设置
settings-resource-actions = 更多操作
settings-resource-delete-ellipsis = 移到废纸篓…
settings-resource-from-package = 来自 { $package }
settings-resource-open-package = 显示所属包
settings-key-customized = 已自定义
settings-key-customized-only = 仅已自定义
settings-key-group-system = 系统级
settings-key-system-help = Gupi 运行时，这些快捷键在其他应用中也可使用。
settings-key-group-workspace = 工作区与文件
settings-key-reset-all-ellipsis = 全部恢复默认…
settings-key-confirm = 确定
settings-key-unsaved = 未保存
settings-key-scope-any-window = 任意 Gupi 窗口
settings-key-scope-palette = 主窗口、临时窗口与启动窗口
settings-key-scope-conversation = 主窗口与临时窗口中的会话
settings-key-scope-messages = 消息列表获得焦点时
settings-key-scope-stop = 主窗口：停止生成 · 临时窗口：隐藏
settings-key-scope-temporary = 临时窗口
settings-key-scope-temporary-paste = 临时窗口；纯 Enter 在输入框内不触发
settings-key-search = 搜索快捷键
settings-key-search-keystroke = 按按键搜索
settings-key-search-clear = 清除按键搜索
settings-key-sessions = 切换到临时会话 1–9
settings-key-sessions-expand = 展开
settings-key-sessions-collapse = 收起
settings-key-show-conflict = 显示
settings-key-used-by = 已被“{ $command }”使用（{ $scope }）
settings-key-task-actions = 任务操作
settings-key-task-description = /{ $template } · { $source }
settings-key-task-enabled = 启用任务
shortcut-delete-ellipsis = 删除模板任务…

# Package catalog
packages-installed-tab = 已安装
packages-browse-tab = 浏览
packages-search = 搜索包
packages-source-npm = 结果来自 npm 上带 pi-package 关键字的包。
packages-directory = Pi 包目录
packages-loading = 正在加载…
packages-retry = 重试
packages-installed = 已安装
packages-previous = 上一页
packages-next = 下一页
packages-back = 返回
packages-latest = 最新发布 { $version }
packages-publisher = 发布者 { $publisher }
packages-repository = 仓库
packages-npm = npm 页面
packages-install-source = 安装来源
packages-declared = 声明的内容
packages-undeclared = 包未声明内容，安装后列出。
packages-installed-version = 已安装 { $version }
packages-files-missing = 文件缺失
packages-trust = 包以你的权限运行代码。
packages-done = 已完成。重新加载现有会话或新建会话后生效。
packages-failed = Pi 报告了错误：
packages-install = 安装
packages-copy-source = 复制安装来源
packages-open-repository = 打开仓库
packages-open-npm = 打开 npm 页面
packages-problem-network = 无法连接 npm 注册表，请检查网络后重试。
packages-problem-rate-limited = npm 注册表限制了请求，请稍后重试。
packages-problem-not-found = 未找到这个包。
packages-problem-invalid = npm 注册表返回了无法识别的内容。
settings-package-install-source = 从来源安装…
settings-package-remove-ellipsis = 移除…

# Package browser layout
packages-source-short = npm · pi-package
packages-page = 第 { $page } 页
packages-page-empty = 本页无符合条件的包。
packages-close = 关闭详情
packages-local-unavailable = 本地包信息不可用，请刷新资源后再操作。
packages-refresh-details = 重新读取

## Pi settings
settings-domain-label = 设置区域
settings-domain-gupi = Gupi
settings-domain-pi = Pi
settings-page-connection = 连接
settings-page-pi-conversation = 对话
settings-page-pi-network = 网络
pi-group-model = 默认模型
pi-group-context = 上下文
pi-group-queue = 消息队列
pi-group-images = 图片
pi-group-proxy = 代理
pi-group-retry = 重试
pi-field-model = 默认模型
pi-field-model-help = 新对话的默认模型；模型范围可能改变初始模型。
pi-field-model-auto = 自动
pi-field-model-unavailable = 此模型当前不可用。
pi-field-thinking = 默认思考等级
pi-thinking-unsupported = { $level }（所选模型不支持）
pi-field-compaction = 自动压缩
pi-field-reserve = 压缩预留 token
pi-field-reserve-help = 压缩时为模型回复保留的 token。
pi-field-keep = 保留近期 token
pi-field-keep-help = 压缩时不被摘要的近期上下文。
pi-field-steering = 插话消息
pi-field-follow-up = 排队消息
pi-delivery-one = 逐条
pi-delivery-all = 一次全部
pi-field-auto-resize = 自动缩小图片
pi-field-auto-resize-help = 发送前将图片缩放到 2000×2000 以内。
pi-field-block-images = 不向模型发送图片
pi-field-proxy = HTTP 代理
pi-field-proxy-help = 只影响 Pi 发出的请求，不影响 Gupi。
pi-edit-in-global = 在全局中编辑
pi-value-unset = 未设置
pi-field-retry = 自动重试
pi-field-max-retries = 最大重试次数
pi-field-base-delay = 初始延迟（毫秒）
pi-field-max-delay = 最大延迟（毫秒）
pi-source-default = 默认
pi-source-global = 全局
pi-source-project = 项目
pi-unsaved = 未保存
pi-changed-on-disk = 读取后文件中的值已被修改。
pi-invalid-count = 请输入 0 或更大的整数。
pi-menu-revert = 撤销更改
pi-menu-pin-project = 设为项目值
pi-menu-pin-global = 设为全局值
pi-menu-inherit = 使用全局值
pi-menu-default = 使用默认值
pi-field-actions = 字段操作
pi-scope-global = 全局
pi-scope-label = 设置作用域
pi-trust-untrusted = 未信任
pi-trust-untrusted-help = 信任项目前，Pi 不会使用这些项目设置。
pi-trust-action = 信任项目…
pi-trust-title = 信任“{ $name }”？
pi-trust-body = Pi 将加载此文件夹的项目设置和资源、安装项目包并执行项目扩展。上级文件夹不会被信任。
pi-trust-confirm = 信任项目
pi-settings-save = 保存
pi-settings-discard = 放弃
pi-settings-file-actions = 更多操作
pi-settings-open-file = 打开 settings.json
pi-settings-reload = 重新读取
pi-settings-invalid = 请先修正标记的字段再保存。
pi-settings-conflict = 部分设置在读取后被其他程序修改。请重新读取后再保存。
pi-settings-locked = 设置文件正被其他进程使用，请稍后重试。
pi-settings-unsaved-title = 保存 Pi 设置？
pi-settings-unsaved-body = Pi 设置有未保存的更改。
pi-settings-loading = 正在读取…
pi-settings-unsupported = 在此编辑 Pi 设置需要 Pi 1.1.0 或更高版本。当前版本：{ $version }。
pi-settings-unavailable = Pi 不可用，请检查“连接”页。
pi-models-loading = 正在读取模型…
pi-models-retry = 重试
pi-project-settings = 项目 Pi 设置…

pi-scope-choose-folder = 选择项目文件夹…
pi-scope-choose-folder-action = 选择
settings-resource-group-project = 本项目
settings-resource-group-inherited = 继承自全局
settings-source-project = 本项目
settings-resource-same-global = 与全局同名 · 项目优先
settings-resource-same-project = 与项目同名 · 项目优先
settings-resource-not-loaded = 未加载（项目未信任）
settings-resource-edit-global = 在全局中编辑
settings-resource-none-project = 此项目的 .pi 中尚无此类资源。
settings-system-project-file = 项目文件
settings-system-uses-global = 使用全局
settings-system-use-global = 使用全局
settings-system-use-global-title = 改用全局 { $file }？
settings-system-use-global-body = 项目文件将移入废纸篓，Pi 改为回退到全局文件。
settings-editor-changed = 文件在打开后已被修改，未保存。可以重新读取（放弃此处修改）或取消。
settings-editor-reload = 重新读取
settings-editor-unsaved-title = 保存对 { $file } 的修改？
settings-editor-unsaved-body = 正文修改尚未保存。

pi-project-settings-folder = 项目 Pi 设置 — { $folder }

packages-install-project = 安装到项目 { $folder }
packages-project-delta = 全局包过滤设置 · 只读
packages-project-filtered = 在此项目中被过滤
packages-project-replaced = 被项目替换
packages-project-replaces = 替换全局包

settings-resource-disabled = 已停用
settings-theme-heading = 独立主题
settings-project-package-empty = 此项目没有包。

pi-model-scope = 模型范围
pi-model-scope-help = 新对话从此范围选择初始模型：默认模型在范围内时使用默认模型，否则使用第一个匹配的模型。不限制会话中可选的模型。
pi-model-scope-add-model = 添加模型…
pi-model-scope-add-pattern = 添加模式
pi-model-scope-up = 上移
pi-model-scope-down = 下移
pi-model-scope-delete = 删除模式
pi-model-scope-unrestricted = 不限制
pi-model-scope-unrestricted-project = 不限制（覆盖全局）
pi-model-scope-blank = 模式不能为空。
pi-model-scope-malformed = 此值必须是字符串数组，请在设置文件中修复。

conversation-edited-files = 已编辑 { $count } 个文件
conversation-edit-write = 写入
conversation-edit-error = 错误
conversation-written-content = 写入内容
conversation-edit-operation = 操作 { $index }
conversation-edit-more-files = 再显示 { $count } 个文件
conversation-edit-less-files = 收起文件
conversation-edit-more-operations = 再显示 { $count } 次操作
conversation-edit-previous-operations = 上一批操作
conversation-edit-stats-tooltip = 记录补丁合成后的净增删行数。不含写入和无法合并的文件，不代表工作区完整差异。
conversation-edit-file-label = { $path } — { $state }
conversation-edit-operation-stats-tooltip = 这次记录补丁的新增与删除行数。
conversation-edit-uncombined = 无法合并，按操作显示

# Advanced Pi settings
settings-page-pi-tools = 工具与环境
pi-group-tools = 工具
pi-group-shell = Shell
pi-group-package-command = 包管理
pi-group-privacy-trust = 隐私与信任
pi-group-advanced = 高级
pi-group-provider = Provider 层重试
pi-field-default-tools = 默认工具
pi-field-codemode-mode = Codemode 模式
pi-field-codemode-budget = 声明预算
pi-field-shell-path = Shell 路径
pi-field-shell-prefix = 命令前缀
pi-field-npm-command = npm 命令
pi-field-telemetry = 安装遥测
pi-field-project-trust = 默认项目信任
pi-field-model-thinking = 按模型思考等级
pi-field-model-overrides = 按模型压缩参数
pi-field-budget-minimal = 极低
pi-field-budget-low = 低
pi-field-budget-medium = 中
pi-field-budget-high = 高
pi-field-cache-warming = 缓存预热
pi-field-transport = 传输方式
pi-field-http-timeout = HTTP 空闲超时（毫秒）
pi-field-websocket-timeout = WebSocket 连接超时（毫秒）
pi-field-provider-timeout = 请求超时（毫秒）
pi-field-provider-retries = 重试次数
pi-field-provider-delay = 最大重试等待（毫秒）
pi-timeout-disabled = 关闭
pi-tools-startup-help = 扩展和 MCP 会另外启用工具；这不是安全限制，也不是完整的实时工具列表。
pi-map-add-model = 添加模型
pi-map-remove-local = 删除本作用域覆盖
pi-model-thinking-help = 启动时显式等级和会话记录优先；切换模型时使用此值；模型范围中的思考后缀优先。
pi-model-overrides-help = 两个 token 参数分别继承，只保存改过的叶字段。
pi-trust-always-title = 默认信任项目？
pi-trust-always-body = Pi 将信任没有适用决定的项目，加载项目设置并执行项目扩展。已有明确决定和信任扩展仍然优先。
pi-telemetry-env-on = 环境变量 PI_TELEMETRY 生效：开启
pi-telemetry-env-off = 环境变量 PI_TELEMETRY 生效：关闭
pi-codemode-help = Codemode 设置需要已启用的内置 codemode 扩展，且 codemode 工具处于启用状态。
pi-thinking-budget-help = 只对支持 token 预算的供应商和模型生效。
pi-cache-warming-help = 预热请求会产生用量。
pi-telemetry-help = 控制安装遥测，不影响更新检查，也不是所有分析数据的总开关。
pi-provider-retry-help = Pi 建议保持 0。
pi-trust-always-confirm = 默认信任
pi-choice-ask = 询问
pi-choice-always = 始终信任
pi-choice-never = 从不信任
pi-choice-codemode-on = 直接工具与 Codemode
pi-choice-codemode-only = 仅 Codemode
pi-choice-streaming = 流式输出时
pi-choice-idle = 也在空闲时
pi-choice-auto = 自动

pi-thinking-budgets = 思考预算（token）
pi-model-unavailable-short = 不可用
pi-models-unavailable = 无法读取模型列表
pi-map-model = 模型
pi-map-reserve = 预留 token
pi-map-keep = 保留近期 token
pi-map-token-action = { $field }：{ $action }
pi-tools-add = 添加
pi-tools-custom = 自定义…
pi-tool-remove = 移除 { $tool }
pi-tools-preview = 启动时：{ $tools }
pi-npm-invalid = 请输入引号配对且首个参数非空的命令。
pi-timeout-off = 已关闭
pi-npm-help = 用 shell 引号保留空格和空参数。
pi-budget-input = { $level }思考预算（token）

pi-group-resource-paths = 资源路径
pi-field-resource-extensions = 扩展
pi-field-resource-skills = Skill
pi-field-resource-prompts = 提示词
pi-field-resource-themes = 主题
pi-resource-paths-help = 当前作用域文件中的原始条目，按顺序保存。
pi-resource-unsupported = 格式不支持 · 只读
pi-resource-absent = 未写入此键
pi-resource-empty = 空数组 []
pi-resource-up = 上移
pi-resource-down = 下移
pi-resource-remove = 删除条目
pi-resource-add = 添加
pi-resource-clear = 清空数组 []
pi-resource-remove-key = 移除此键
pi-resource-entry-actions = 条目操作

# Pi configuration files
pi-files-models = 模型与供应商
pi-files-mcp = MCP
pi-files-startup-only = Pi 在新会话启动时读取此文件，已有会话不会重新加载。
pi-files-global-only = 此文件仅全局可用，请切换到全局作用域编辑。
pi-files-comments = 保存会移除文件中的注释。
pi-files-add = 添加…
pi-files-edit = 编辑…
pi-files-delete = 删除
pi-files-override = 覆盖
pi-files-definition = 完整定义
pi-files-disabled = 已关闭
pi-files-duplicate = 名称与已有条目冲突。
pi-files-invalid = 无法读取或保存此条目，请检查字段与配置文件。
pi-files-secret-set = 已设置 · 保持
pi-files-secret-keep = 保持
pi-files-secret-replace = 替换为新值
pi-files-secret-remove = 移除
pi-files-name = 名称
pi-files-key = 键
pi-files-model = 模型
pi-files-add-model = 添加模型
pi-files-on = 开
pi-files-off = 关

# Structured configuration editor
pi-files-label-name = 显示名称
pi-files-label-id = 模型 ID
pi-files-label-baseUrl = 基础 URL
pi-files-label-api = API
pi-files-label-apiKey = API 密钥
pi-files-label-authHeader = 认证头
pi-files-label-oauth = OAuth
pi-files-label-compat = 兼容性
pi-files-label-models = 模型
pi-files-label-modelOverrides = 模型覆盖
pi-files-label-enabled = 启用
pi-files-label-exposure = 暴露方式
pi-files-label-type = 传输方式
pi-files-label-description = 描述
pi-files-label-command = 命令
pi-files-label-url = URL
pi-files-label-cwd = 工作目录
pi-files-label-timeout = 超时
pi-files-label-toolExposure = 工具暴露方式
pi-files-label-args = 参数
pi-files-label-env = 环境变量
pi-files-label-headers = 请求头
pi-files-label-clientId = 客户端 ID
pi-files-label-clientSecret = 客户端密钥
pi-files-label-callbackPort = 回调端口
pi-files-label-callbackUrl = 回调 URL
pi-files-label-scope = 权限范围
pi-files-label-clientName = 客户端名称
pi-files-label-clientRegistration = 客户端注册方式
pi-files-label-authServerMetadataUrl = 认证元数据 URL
pi-files-label-provider = 供应商
pi-files-label-reasoning = 推理
pi-files-label-contextWindow = 上下文窗口
pi-files-label-maxTokens = 最大输出 token
pi-files-label-input = 输入类型
pi-files-label-cost = 费用
pi-files-label-thinkingLevelMap = 思考等级映射
pi-files-label-inputLimits = 输入限制
pi-files-label-promptCache = 提示缓存
pi-files-label-samplingParams = 采样参数
pi-files-label-samplingParamsByThinkingLevel = 按思考等级采样
pi-files-label-autoEnableCodemode = 自动启用 Codemode
pi-files-providers = 供应商
pi-files-servers = MCP 服务
pi-files-automation = Codemode
pi-files-empty = 暂无条目
pi-files-basic = 基本信息
pi-files-connection = 连接
pi-files-exposure-section = 暴露方式
pi-files-authentication = 认证
pi-files-advanced = 高级
pi-files-back = 返回
pi-files-stage = 暂存更改
pi-files-secret-present = 已设置
pi-files-add-value = 新值
pi-files-confirm-add = 添加
pi-files-definition-kind = 定义方式
pi-files-provider-summary = { $models } 个模型 · { $overrides } 项覆盖

# Pi provider authentication
pi-auth-title = 认证
pi-auth-help = 凭据保存在 Pi 的 auth.json 中。更改在新会话中完整生效。Gupi 不会自动重连或同步已有会话的模型列表。
pi-auth-add = 添加 API Key…
pi-auth-replace = 替换 API Key…
pi-auth-api-key = API Key
pi-auth-oauth = OAuth
pi-auth-unknown = 无法识别
pi-auth-missing = 未存储凭据
pi-auth-set = 已设置
pi-auth-empty = 尚无供应商凭据。添加 API Key 以开始。
pi-auth-provider = 供应商 ID
pi-auth-key-placeholder = 输入新的 API Key
pi-auth-saving = 正在保存…
pi-auth-candidates = 选择供应商
pi-auth-check = 检查配置
pi-auth-checking = 正在检查配置
pi-auth-ready = Pi 找到了可用的凭据配置。这不代表令牌有效。
pi-auth-not-ready = Pi 未找到可用的凭据配置。
pi-auth-invalid = Pi 报告配置无效。
pi-auth-check-failed = 无法检查配置。
pi-auth-provider-not-found = 未找到供应商。
pi-auth-not-configured = 尚未配置凭据。
pi-auth-not-available = 凭据不可用。
pi-auth-invalid-state = Pi 无法读取认证状态。
pi-auth-copy-login = 复制登录命令
pi-auth-login-help = 已复制登录命令。请在 Pi 终端中执行以重新登录。
pi-auth-oauth-help = 仅显示本地记录；Pi 使用凭据时可能尝试刷新，必要时请重新登录。
pi-auth-expired = 已超过记录的到期时间
pi-auth-expires = 记录的到期时间：{ $time }
pi-auth-remove = 移除凭据…
pi-auth-logout = 注销…
pi-auth-remove-title = 移除“{ $provider }”的凭据？
pi-auth-logout-title = 注销“{ $provider }”？
pi-auth-remove-help = 只删除本机凭据，不会吊销服务端令牌；环境变量或 models.json 中的 apiKey 仍可能生效。
pi-auth-remove-env = 供应商的环境与配置值也会一并移除。
pi-auth-remove-confirm = 移除
pi-auth-logout-confirm = 注销
pi-auth-replace-title = 替换“{ $provider }”的凭据？
pi-auth-replace-help = 将用 API Key 替换整条凭据，包括令牌与未知字段。
pi-auth-replace-confirm = 替换
pi-auth-read-error = 无法读取 auth.json。
pi-auth-parse-error = auth.json 必须是有效的 JSON 对象，当前禁止编辑。
pi-auth-busy = auth.json 正忙，请稍后重试。
pi-auth-conflict = 此供应商的凭据已在磁盘上更改，请重新读取后重试。
pi-auth-write-error = 无法写入 auth.json，请重新读取以检查当前状态。
pi-auth-input-error = 请选择供应商或输入非空的单行 ID，并填写 API Key。
pi-auth-login-select-help = 已复制登录命令。请在 Pi 终端中执行 /login 并选择供应商。

# Authentication and model provider settings
pi-files-auth-stored = 认证页已存凭据
pi-files-manage-auth = 在认证页管理
pi-files-api-key-auth-help = 认证页存在已存凭据时，Pi 不回退使用此处的 apiKey。已存凭据不代表认证有效。
pi-auth-models-help = 已存凭据会影响 Pi 对 models.json 中 apiKey 的使用；实际认证由 Pi 按供应商规则处理。
pi-auth-edit-models = 在模型与供应商中编辑

pi-provider-empty-id = 〈空 ID〉
pi-auth-check-unavailable = Pi 无法检查此供应商 ID。

settings-skill-name-help = 目录名。Pi 建议使用小写字母、数字和单个连字符，最长 64 字符。

settings-skill-name-warning = 名称不符合 Pi 推荐规则，仍可创建。此处不校验 SKILL.md 中的 name。

settings-prompt-name-help = 用于 /名称 调用；不可包含空白。

settings-prompt-invalid-name = 请输入合法文件名，不可包含空白或路径分隔符。

shortcut-add-title = 添加模板任务

shortcut-edit-title = 编辑模板任务

shortcut-options-failed = 无法读取模板或模型选项，请检查 Pi 连接后重试。

pi-auth-row-action = { $provider }: { $action }

pi-auth-add-title = 添加 API Key

pi-auth-replace-editor-title = 替换 API Key

temporary-clean-title = 清理已释放的工作目录？

settings-package-remove-title = 移除“{ $name }”？

settings-resource-delete-title = 将“{ $name }”移到废纸篓？

shortcut-delete-title = 删除模板任务“{ $name }”？

settings-key-control-label = { $command }: { $binding }
settings-key-control-recording = { $command }: { $binding } — { $state }

# MCP CLI login and last-check results
pi-mcp-help = 这是 Pi 内置 MCP 的检查与登录；替代内置 MCP 的扩展可能有不同的会话状态。使用 auth.provider 的服务在检查中可能需要登录，但在会话中可用。
pi-mcp-reconnect = 要让已运行的会话完整加载保存的 MCP 配置，请在会话菜单中选择“重新连接”；新会话自动加载。
pi-mcp-global-cwd = 全局检查在空临时目录运行，使用相对路径的服务结果可能与会话中不同。
pi-mcp-save-first = 请先保存或放弃更改
pi-mcp-check = 检查连接
pi-mcp-running = 正在执行…
pi-mcp-last-check = 上次检查 { $time }
pi-mcp-config-error = 配置有错误
pi-mcp-untrusted = 项目未受信任，Pi 只使用全局配置
pi-mcp-resources = 有资源
pi-mcp-connected = 已连接（{ $count } 个工具）
pi-mcp-needs-auth = 需要登录
pi-mcp-connection-failed = 连接失败
pi-mcp-disconnected = 未连接
pi-mcp-not-checked = 未检查
pi-mcp-unsupported-name = Pi 无法通过命令行对该名称登录或登出
pi-mcp-login = 登录
pi-mcp-logout = 登出
pi-mcp-login-title = 登录 MCP 服务“{ $name }”
pi-mcp-logout-title = 登出 MCP 服务“{ $name }”？
pi-mcp-login-help = 如需登录，Pi 会打开浏览器，请在浏览器中完成。
pi-mcp-logout-help = 删除本机保存的凭据；不会吊销服务端授权，也不会立即断开已运行的会话。重新连接这些会话以刷新连接。
pi-mcp-login-terminal = 未出现浏览器时，取消后在终端运行 pi mcp login -- { $name }。
pi-mcp-completed = 命令已完成
pi-mcp-cancelled = 已取消
pi-mcp-timeout = 操作超时，结果不完整
pi-mcp-login-failed = 登录失败；凭据可能已保存，但连接失败。请检查连接。
pi-mcp-failed = 操作失败
pi-mcp-close = 关闭
pi-mcp-scope-project = 项目
pi-mcp-disabled = 已停用
pi-mcp-check-failed = 检查失败
pi-mcp-check-timeout = 检查超时，结果不完整

workspace-changes = 改动
workspace-close-tab = 关闭标签
workspace-close-others = 关闭其他标签
workspace-close-right = 关闭右侧标签
workspace-toggle-file-tree = 显示或隐藏文件树
workspace-toggle-change-list = 显示或隐藏改动列表
workspace-out-of-scope = 此回复不在当前范围内。
workspace-net-zero = 本轮改动已全部抵消。
workspace-open-file = 打开文件
workspace-next-operations = 下一批操作
workspace-open-change = 在工作区中打开 { $path }

history-panel-title = 历史
history-panel-options = 历史选项
workspace-view-changes = 查看变更

workspace-new-tab = 新标签页
workspace-all-tabs = 所有标签
workspace-tools = 工具
workspace-recent = 最近
workspace-no-changes = 此对话还没有改动。
workspace-recent-unavailable = 当前范围内无法打开这些改动。
workspace-just-now = 刚刚
workspace-minutes-ago = { $count } 分钟前
workspace-maximize = 最大化工作区
workspace-restore = 还原工作区

# Direct Shell command output
shell-title = Shell
shell-running = 命令运行中
shell-stopping = 正在停止命令
shell-stop = 停止命令
shell-cancelled = 已取消
shell-exit-code = 退出码 { $code }
shell-completed = 已完成 · 未提供退出码
shell-excluded = 不加入模型上下文
shell-show-all = 显示全部输出
shell-show-less = 收起输出
shell-open-full-output = 查看完整输出
shell-display-truncated = 仅显示最近的输出。

titlebar-git-branch = Git 分支：{ $name }
titlebar-git-detached = 游离 HEAD：{ $name }

# Diff reader
diff-worktree = Git
diff-unstaged = 未暂存
diff-staged = 已暂存
diff-untracked = 未跟踪
diff-filter = 过滤路径…
diff-previous-file = 上一个文件
diff-next-file = 下一个文件
diff-unified = 统一
diff-split = 并排
diff-wrap = 自动换行
diff-actions = 文件操作
diff-word = 字内差异
diff-context = 全部未变更行
diff-expand-file = 展开此文件
diff-copy-path = 复制文件路径
diff-copy-patch = 复制 patch
diff-copy-content = 复制文件全文
diff-external = 在外部编辑器打开
diff-refresh = 刷新改动
diff-refresh-failed = 无法刷新改动，现有结果可能已过期。
diff-loading = 正在加载改动…
diff-not-repository = 此文件夹不在 Git 工作树中
diff-empty = 没有改动
diff-no-results = 没有匹配的文件
diff-retry = 重试
diff-recorded-context = 本轮改动 · 仅提供记录中的上下文
diff-added = 新增
diff-deleted = 删除
diff-modified = 修改
diff-conflict = 冲突 · 只读
diff-special = 特殊文件 · 只读
diff-binary = 二进制文件
diff-too-large = 此文件超出文本阅读限制
diff-unavailable-file = 当前文件不存在或不可用，无法在外部打开。
diff-git-unavailable = Git 不可用，请安装 Git 后刷新。
diff-files = 文件
diff-conflicts = 冲突
diff-direction-staged = HEAD → 暂存区
diff-direction-unstaged = 暂存区 → 工作树
diff-direction-untracked = 空 → 工作树
diff-updating = 正在更新旧版本…
diff-collapse-file = 折叠本文件
diff-update-failed = 此版本尚未更新
diff-reading-options = 阅读选项
diff-reading-options-narrow = 阅读选项：正文区域过窄，无法并排显示；扩大后恢复并排偏好
diff-view = 视图
diff-display = 显示
diff-list-view = 列表
diff-tree-view = 树
diff-stats-incomplete = 统计不完整

workspace-back-conversation = 返回对话
workspace-back-content = 返回正文
workspace-open = 打开工作区
workspace-close = 关闭工作区

diff-invalid-path = 文件名不是有效 UTF-8，无法读取
diff-no-text-changes = 没有文本改动

diff-external-failed = 无法打开文件。文件可能已不存在，或没有可用的应用。

diff-conflict-label = 冲突
diff-read-failed = 读取失败，请重试。

# Conversation navigation
conversation-navigation-jump = 跳到第 { $number } 条用户消息：{ $question }
conversation-navigation-current = 当前区间。跳到第 { $number } 条用户消息：{ $question }
conversation-navigation-no-text = 无文本内容

error-technical-details = 技术详情
conversation-load-failed = 无法加载对话内容。
conversation-refresh-failed = 无法刷新对话内容，当前显示的内容可能不是最新的。
history-load-failed = 无法加载会话历史。
history-refresh-failed = 无法刷新会话历史，当前显示的历史可能不是最新的。
conversation-runtime-failed = Pi 运行出现问题。
conversation-storage-failed = 无法访问本地会话数据。请查看详情，检查相关文件的位置和权限。
composer-model-load-failed = 无法加载模型列表。
composer-thinking-load-failed = 无法加载思考强度选项。
composer-model-change-failed = 无法确认模型或思考强度的切换结果。
command-load-failed = 无法加载命令列表。
resource-files-failed = 无法查找文件。请检查路径后重新搜索。
