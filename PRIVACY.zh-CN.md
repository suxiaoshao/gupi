# Gupi 隐私政策

[English](PRIVACY.md)

Gupi 是 Pi 的桌面界面。Gupi 没有自己的账户服务、使用情况统计、自动崩溃报告或云同步。Gupi 将诊断信息保存在本机，不会将你的会话、附件、项目文件或模型提供方凭据发送给 Gupi 维护者。寻求支持或提交问题报告时，由你决定分享哪些内容。

## Gupi 何时联网

| 功能 | 访问地址和发送的数据 |
| --- | --- |
| 检查 Gupi 更新 | 手动检查时，Gupi 请求 `https://api.github.com/repos/suxiaoshao/gupi/releases/latest`。发行包默认在启动后不久检查一次，之后每小时检查一次。可在**设置 → 关于**中关闭自动检查。请求的 `User-Agent` 包含 Gupi 版本，且可能附带缓存的 GitHub `ETag`。 |
| 下载 Gupi 更新 | 在支持应用内更新的 macOS 或 Windows 发行包中选择更新后，Sparkle 或 WinSparkle 从 `github.com/suxiaoshao/gupi/releases/download/` 及 GitHub 重定向的下载主机获取更新清单和选定的 ZIP 或 MSI。清单和安装包的路径会体现目标版本、Mac 架构或 Windows 安装语言。更新组件会发送其常规 HTTP 请求头。Gupi 关闭了 Sparkle 的系统信息上报。通过包管理器更新时，联网行为由包管理器决定。 |
| 浏览 Pi 扩展包 | 在包管理设置中打开**浏览**、搜索或翻页时，Gupi 请求 `https://registry.npmjs.org/-/v1/search`，发送 `pi-package` 关键字、你输入的搜索词和分页参数。查看未缓存的包详情时，Gupi 请求 `https://registry.npmjs.org/<package>/latest`，路径包含所选包名。两种请求的 `User-Agent` 均包含 Gupi 版本。 |
| 显示 Markdown 远程图片 | Markdown 界面渲染含远程 URL 的图片时，Gupi 可能直接请求该 URL，无需点击。会话与 Markdown 预览均可能触发此类请求。图片服务器会收到请求的 URL 及其中的查询参数。 |
| 打开链接 | 打开网页链接时，Gupi 将 URL 交给系统浏览器。常见链接包括 `github.com` 上的 Gupi 帮助、更新说明和问题反馈，Pi 文档和 `https://pi.dev/packages` 包目录，以及扩展包或会话中的链接。目标网站会收到浏览器请求，也可能收到浏览器附带的 Cookie。 |

这些服务可看到连接使用的 IP 地址和常规请求元数据。Gupi 还会运行系统 Git 来查看仓库状态与差异。如果部分克隆缺少所需对象，Git 可以访问已配置的远程仓库，按对象 ID 请求数据，并在需要时使用你的 Git 凭据。Git 查看仓库时也可能运行仓库配置的过滤器，具体行为由过滤器决定。

## Pi、模型提供方与扩展

Pi 可执行文件由你单独安装和升级。例如，按官方文档通过 npm 安装时，npm 会按你的注册表设置下载 Pi 及其依赖。Gupi 可以检查本机 Pi 可执行文件的版本，也可以调用 Pi 安装或更新 **Pi 扩展包**。在进行扩展包操作时，Pi 及其子进程会按需要访问已配置的包注册表或 Git 主机，并使用相应的包名、版本或仓库引用。

Gupi 将提示词和所选附件交给本机的 Pi 进程。根据配置和操作，Pi 可能将提示词、附件、会话历史、项目上下文、工具输出和认证数据发送给你选择的模型提供方。Pi、扩展、工具和 MCP 服务器都可以各自发起请求。例如，Pi 可能获取提供方的模型目录，MCP 操作可能连接你配置的服务器或其授权服务。Pi 自行管理遥测及其设置。详情见所用提供方或服务的隐私政策，以及 Pi 的[遥测文档](https://docs.pi.dev/latest/telemetry)、[提供方文档](https://docs.pi.dev/latest/providers)和 [MCP 文档](https://docs.pi.dev/latest/mcp)。

## 本地数据

| 数据 | 默认位置 |
| --- | --- |
| Gupi 偏好设置、窗口布局和草稿 | `config.toml`、`state.toml` 和 `conversations.toml`。macOS 位于 `~/Library/Application Support/gupi`，Windows 位于 `%APPDATA%\gupi`，Linux 位于 `$XDG_CONFIG_HOME/gupi`（通常是 `~/.config/gupi`）。 |
| Gupi 日志 | macOS 位于 `~/Library/Logs/gupi/gupi.log`，Windows 位于 `%LOCALAPPDATA%\gupi\logs\gupi.log`，Linux 位于 `$XDG_DATA_HOME/gupi/logs/gupi.log`（通常是 `~/.local/share/gupi/logs/gupi.log`）。 |
| 临时工作区和图片快照 | 临时会话工作区保存在 Gupi 本地应用数据目录下的 `temporary-workspaces` 中。用于预览和发送的图片附件可能作为快照保存在本机临时文件中。 |
| Pi 会话、设置与凭据 | Pi 在本机管理这些文件，通常位于 `~/.pi/agent` 或你配置的 Pi 目录中。部分项目设置保存在项目的 `.pi` 目录中。 |

`GUPI_CONFIG_DIR`、`GUPI_LOG_DIR` 和 `GUPI_DATA_DIR` 可分别覆盖 Gupi 的配置、日志和应用数据目录。Gupi 在本机截取和裁剪屏幕截图。将截图作为附件发送，或运行配置为提交截图的快捷任务时，Gupi 会像处理其他图片附件一样将截图交给 Pi。

如对本政策有疑问，可通过 [Gupi 的 GitHub Issues](https://github.com/suxiaoshao/gupi/issues) 联系维护者。
