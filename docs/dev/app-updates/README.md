# 应用更新

Gupi 统一负责检查新版和提醒；macOS 使用 Sparkle 2.9.6，Windows 使用 WinSparkle 0.9.4 下载并验证更新包，再经过 Gupi 的退出流程安装和重启。Linux 保留新版提示与发行页入口，安装使用包管理器。密钥、产物和发布操作见[发行指南](../../releasing.md#application-update-signing)。

## 产品行为

- 菜单「检查更新…」与设置「关于 → 更新」共用状态和操作。检查只接受公开的稳定版 GitHub Latest Release，草稿、预发布和单独推送的 tag 不会触发更新。
- 打包版默认自动检查，启动后延迟 10 秒，之后每小时一次；源码运行默认关闭，可以在设置中修改。关闭自动检查不会取消用户主动发起的检查。
- 后台检查失败不弹窗，同一新版在一次运行中最多提醒一次。原生窗口中的「跳过此版本」保存到 `skipped_update`，后续自动检查不再提醒该版本；手动检查和安装仍然可用。
- 下载由用户选择「下载并安装…」后开始。原生窗口负责进度、取消、签名错误和安装确认；Gupi 提供「查看更新进度」重新打开正在进行的更新。安装期间不启动另一轮检查。
- 配置操作与原生更新窗口互斥：配置忙时不能打开新的原生更新窗口；更新窗口存续期间禁止修改配置，关闭或失败后恢复。「跳过此版本」使用同一配置保存入口写入文件，保存成功后才更新内存中的跳过记录，不排队保存。
- 用户确认安装后，先保存配置、草稿和布局，关闭 Pi 连接，再交给平台安装器；后台发现新版不会中断 Pi。更新不升级 Pi、插件或用户会话文件。
- macOS 原生窗口提供「安装并重新启动」。已经下载并验证的更新也可能由 Sparkle 在正常退出时安装；这不改变退出前的保存流程。
- 没有配置更新公钥的构建，以及 macOS 开发包，保留检查与手动下载。macOS 仅在 Developer ID 正式包配置了更新公钥时嵌入 Sparkle。Windows 仅在识别到当前目录对应的 MSI 安装时提供应用内安装；复制出来的便携目录使用手动安装。

## 所有权与接口

| 位置 | 职责 |
| --- | --- |
| [foundation/releases](../../../src/foundation/releases.rs) | GitHub Latest HTTPS 请求、SemVer 比较与响应校验 |
| [state/updates](../../../src/state/updates.rs) | 检查调度、在途任务、安装状态、提示去重与跳过版本 |
| [features/updates](../../../src/features/updates.rs) | 菜单和设置共用的状态展示与操作 |
| [foundation/updater](../../../src/foundation/updater.rs) | 原生引擎、平台回调、安装文件与 Windows helper 准备 |
| [app/updater](../../../src/app/updater.rs) | 原生事件回到 GPUI 主线程，协调保存、退出、安装和重启 |
| [StartupView::quit_then](../../../src/features/startup.rs) | 普通退出与更新共用的持久化和 Pi 关闭流程 |
| [xtask/updater](../../../crates/xtask/src/updater.rs) | 固定 SDK 下载与校验、密钥生成、标准 appcast 签名 |

检查使用固定仓库 HTTPS 地址，连接超时 5 秒、整体超时 15 秒、响应上限 1 MiB，不发送 GitHub Token、配置或会话数据。ETag 与解析结果只在内存中缓存。404 表示尚无公开 Latest，限流、网络错误和格式错误分别反馈。SemVer 比较忽略 build metadata，过滤预发布版本。

检查和自动轮询只有 Gupi 一套。两个原生引擎均关闭自己的自动检查及自动下载；用户选择安装时，向引擎传入**当前展示版本**对应的 `/releases/download/<tag>/appcast-….xml`，避免检查结果和最终安装版本漂移。安装失败可以重试，取消后回到新版可用状态。

更新检查的请求头、ETag、状态码和响应大小回归直接构造内存请求与响应，不启动 HTTP 服务、不访问网络；下载与平台安装行为由隔离更新试验验证。

## macOS

`build.rs` 编译薄 Objective-C 桥，运行时从应用自己的 `Contents/Frameworks/Sparkle.framework` 加载公开 API。源码运行不依赖已安装的 Framework。xtask 嵌入 Framework、许可证、更新公钥和 feed 配置，然后走原有应用签名及公证流程。固定 Sparkle 2.9.6 保留 Gupi 的 macOS 11 基线。

Sparkle 验证更新 ZIP 的 Ed25519 签名并负责应用替换、必要的授权和重新启动。`shouldPostponeRelaunchForUpdate:untilInvokingBlock:` 保存安装回调，通过通道通知应用层；Gupi 完成 `quit_then` 后再恢复回调。打开原生窗口和恢复安装均调度到下一次主线程循环，避免原生窗口激活或退出同步回调重入正在借用的 GPUI `App`。

回调只发送事件；更新器在成功取得单实例所有权后初始化，并随应用存活。自动更新使用正式签名的 ZIP；DMG 继续用于用户首次安装。开发包不会进入公开 macOS appcast。

## Windows

仅从当前安装目录加载 WinSparkle DLL，并限制 DLL 的依赖搜索目录。通过 MSI 的稳定 UpgradeCode 枚举产品，确认 `InstallLocation` 与当前可执行文件目录相同，再根据已安装的 MSI 语言选择 feed。改变 Gupi 界面语言不会改变升级用的 MSI 语言或产品身份。

WinSparkle 完成下载与 Ed25519 校验后，通过 `user_run_installer_callback` 交出已验证 MSI。回调把它复制到用户临时目录并发送事件，不操作 GPUI，不直接启动 MSI。应用先将随包提供的 `gupi-update-helper.exe` 复制到该临时目录，再通过私有 stdin/stdout 管道发送安装计划并等待 `ready`。helper 准备失败时，当前应用保持可用并显示可重试状态。

Gupi 完成保存和 Pi 关闭后，通过同一管道提交 `install`，随后退出。helper 持有父进程句柄并等待真正退出，使用系统目录下的 `msiexec.exe` 安装原语言 MSI，传入原安装目录、`/passive /norestart` 和日志路径。Windows Installer 负责 UAC、升级和回滚；helper 保持普通用户身份，并在成功、取消或安装失败后重新启动安装目录中的 Gupi。MSI 失败会显示错误，日志位于 Gupi 日志目录的 `update-install.log`。父进程在提交前崩溃或关闭管道时，helper 不安装。

MSI 的原生运行、UAC 取消、回滚与重启仍需在 Windows 机器验证，跨目标类型检查不能替代这些证据。

## 发布与信任

- 使用标准 Sparkle appcast，不维护额外的自定义网络协议。macOS 按架构生成两份 feed，Windows 按 MSI 语言生成九份 feed。
- 更新签名使用一组长期 Ed25519 公私钥。应用仅包含公钥，私钥只用于发行收集步骤，签名覆盖最终 ZIP/MSI 的准确字节；不能替换已发布版本下的包或 feed。
- Ed25519 签名不代替 Developer ID、公证或 Authenticode。私钥保管和公钥迁移是发行责任，不能直接更换公钥让旧客户端失去信任链。
- Homebrew Cask 仅在正式 macOS 包且配置了更新公钥时声明 `auto_updates true`。WinGet 继续识别 MSI 注册的版本。包管理器发布步骤见发行指南。
- Gupi 的九种界面语言均覆盖更新入口、进度、失败和重启说明；原生引擎使用自带本地化。

## 已执行验证（2026-09-30）

- macOS 隔离应用、配置、数据和 Pi 目录，使用两个实际构建版本 `0.1.990 → 0.1.991`，本机 HTTP fixture 只替代发行源。有效包由 Sparkle 官方签名工具生成签名，经过实际下载、验证、应用替换及自动重启；新版设置界面显示 `0.1.991`，原配置保留，日志记录更新前 managed quit 完成。
- 将签名篡改后，Sparkle 显示签名验证错误，旧版仍可使用；换回有效签名后可重试。跳过版本写入配置，手动安装不受影响。重新打开进度窗口与正常退出时安装也已验证。
- fixture 使用本机临时密钥和 ad-hoc 签名，未发布到 GitHub。它验证应用交接和原生引擎链路，不替代正式 Developer ID、公证、跨架构和管理员授权验证。
- 检查服务的既有回归覆盖 SemVer、来源、ETag/304、404、限流和异常响应；新增状态回归覆盖跳过版本、安装期间排除重复检查、取消/失败后重试。xtask 回归验证签名、公私钥匹配与 feed 选择，九语言键一致性回归覆盖新增文案。
- Windows 桥接和 helper 已通过 `x86_64-pc-windows-msvc` 目标类型及 Clippy 检查；没有在本机声称 Windows MSI 实际升级通过。

验证按改动选择受影响路径。每次 tag 不重复全量 UI 或 MSI 安装/卸载；更改安装器和更新交接时，再执行对应平台的旧版到新版升级试验。

## 方案依据

Zed 的[自动更新](https://github.com/zed-industries/zed/blob/39b5329322f4d2c14dc8fd6db535a31ac5431770/crates/auto_update/src/auto_update.rs)与 [Windows helper](https://github.com/zed-industries/zed/blob/39b5329322f4d2c14dc8fd6db535a31ac5431770/crates/auto_update_helper/src/updater.rs)用于理解检查、退出和重启的职责；其内部实现与 EXE 安装方式不直接移植到 Gupi 的 MSI。采用原生引擎，使签名验证、下载和平台安装生命周期由现有维护库承担。

接口依据：[Sparkle 程序化接入](https://sparkle-project.org/documentation/programmatic-setup/)、[Sparkle updater delegate](https://sparkle-project.org/documentation/api-reference/Protocols/SPUUpdaterDelegate.html)、[WinSparkle 回调](https://winsparkle.org/c-api/callbacks/)、[WinSparkle 发布更新](https://winsparkle.org/guides/publishing-updates/)。
