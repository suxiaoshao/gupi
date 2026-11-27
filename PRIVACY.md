# Gupi Privacy Policy

[简体中文](PRIVACY.zh-CN.md)

Gupi is a desktop interface for Pi. Gupi has no account service, analytics, automatic crash reporting, or cloud sync of its own. It keeps its diagnostics local and does not send your conversations, attachments, project files, or provider credentials to the Gupi maintainers. You can choose what to share if you file a support request or a bug report.

## When Gupi connects to the internet

| Feature | Destination and data |
| --- | --- |
| Check for Gupi updates | Gupi requests `https://api.github.com/repos/suxiaoshao/gupi/releases/latest` when you check manually. Packaged builds also check shortly after startup and hourly by default. You can turn off automatic checks in **Settings → About**. The request includes the Gupi version in the `User-Agent` and may include the cached GitHub `ETag`. |
| Download a Gupi update | When you choose the in-app update on a supported macOS or Windows build, Sparkle or WinSparkle requests the update feed and downloads the selected ZIP or MSI from `github.com/suxiaoshao/gupi/releases/download/` and GitHub's redirected download hosts. The feed and asset paths reveal the target release and Mac architecture or Windows installer language. The updater sends its usual HTTP headers. Gupi disables Sparkle's system profiling. Updates through a package manager follow that package manager's own behavior. |
| Browse Pi packages | Opening **Browse** in the package settings, searching, or changing pages sends an npm search to `https://registry.npmjs.org/-/v1/search`. It includes the `pi-package` keyword, your search text, and pagination parameters. Viewing uncached package details requests `https://registry.npmjs.org/<package>/latest`, including the selected package name. Both requests include the Gupi version in the `User-Agent`. |
| Display remote Markdown images | When a Markdown view renders an image with a remote URL, Gupi can request that URL without a click. This can happen in conversations and Markdown previews. The image host receives the requested URL, including any query parameters in it. |
| Open a link | When you open a web link, Gupi passes the URL to the system browser. Examples include Gupi help, release notes and issue reporting on `github.com`, Pi documentation and the directory at `https://pi.dev/packages`, and links from packages or conversation content. The destination receives the browser's request and may receive its cookies. |

These services can see the IP address used for the connection and standard request metadata. Gupi also runs your system Git for repository status and diffs. If a partial clone needs a missing object, Git can contact the configured remote and request it by object ID, using your Git authentication if needed. Repository-configured filters can also run as part of Git's inspection and follow their own behavior.

## Pi, model providers, and extensions

You install and update the Pi executable separately. For example, the documented npm installation downloads Pi and its dependencies using your npm registry settings. Gupi can check the local executable's version and ask Pi to install or update **Pi packages**. For those package operations, Pi and its subprocesses contact the configured registries or Git hosts as needed, using the package names, versions, or repository references involved.

Gupi passes prompts and selected attachments to the local Pi process. Pi may send prompts, attachments, conversation history, project context, tool output, and authentication data to your chosen model provider, depending on your configuration and the operation. Pi, extensions, tools, and MCP servers can make their own requests. For example, Pi may fetch provider model catalogs, and MCP operations can contact your configured servers or their authorization services. Pi controls its own telemetry and has its own settings. Consult the relevant provider or service's privacy policy and the [Pi telemetry documentation](https://docs.pi.dev/latest/telemetry), [provider documentation](https://docs.pi.dev/latest/providers), and [MCP documentation](https://docs.pi.dev/latest/mcp).

## Local data

| Data | Default location |
| --- | --- |
| Gupi preferences, window layout, and drafts | `config.toml`, `state.toml`, and `conversations.toml` in `~/Library/Application Support/gupi` on macOS, `%APPDATA%\gupi` on Windows, or `$XDG_CONFIG_HOME/gupi` (normally `~/.config/gupi`) on Linux. |
| Gupi logs | `~/Library/Logs/gupi/gupi.log` on macOS, `%LOCALAPPDATA%\gupi\logs\gupi.log` on Windows, or `$XDG_DATA_HOME/gupi/logs/gupi.log` (normally `~/.local/share/gupi/logs/gupi.log`) on Linux. |
| Temporary workspaces and image snapshots | Temporary conversation workspaces use `temporary-workspaces` under Gupi's local application data directory. Image attachments can be snapshotted in local temporary files for preview and sending. |
| Pi sessions, settings, and credentials | Pi manages these files on the device, normally under `~/.pi/agent` or the configured Pi directories. Some project settings are in the project's `.pi` directory. |

`GUPI_CONFIG_DIR`, `GUPI_LOG_DIR`, and `GUPI_DATA_DIR` override Gupi's configuration, log, and application data directories. Gupi captures and crops screenshots on your device. When you send a screenshot as an attachment or run a screenshot shortcut configured to submit it, Gupi passes it to Pi like any other image attachment.

For questions about this policy, contact the maintainers through [Gupi's GitHub Issues](https://github.com/suxiaoshao/gupi/issues).
