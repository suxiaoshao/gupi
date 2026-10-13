# 退出收尾、原生窗口句柄与 Windows 显示器标识（[Gupi #58](https://github.com/suxiaoshao/gupi/issues/58)）

状态：计划已确认，实施中。

## 目标与范围

- 退出时，已经开始的资源写入、删除、注册技能和包命令执行完毕后才结束进程；资源目录读取仍随退出取消。
- `window-ext` 的安全方法在窗口关闭后不再解引用失效指针；对 GPUI 已关闭的窗口返回错误，而不是重新显示或调整它。
- Windows 按 GPUI `DisplayId`（原生 `HMONITOR` 值）解析目标屏幕缩放比例，找不到目标屏幕时返回错误，不再静默使用窗口当前缩放比例；截图在定位失败时取消。

Windows 专用代码在本机一并实现，运行验证交给用户在 Windows 上进行。

不改资源页交互、写入格式、退出界面和窗口焦点行为。

## 依据

- 资源变更在 [`ResourceController::change`](../../../crates/gupi-resources/src/resources.rs) 中以 `Tokio::spawn` 运行：文件操作在其中 `spawn_blocking`，包命令为 `kill_on_drop(true)` 子进程。`gpui-tokio` 的任务在丢弃时中止外层 Tokio 任务（`AbortOnDropHandle`）。`stop()` 把 `mutation` 置为 `Idle`，因此丢弃任务：阻塞写入不受控制地继续，包命令被杀死，退出等待也不再看见它们。
- [`StartupView::begin_quit`](../../../src/features/startup.rs) 的等待循环只检查配置操作和两个 Pi 探测。`stop_resources` 只由退出调用，测试另有直接调用。
- `NativeWindowHandle` 是 `Copy`，只保存 `RawWindowHandle`（macOS 为 `NSView` 指针）；`hide`/`show`/`set_window_level`/`move_and_resize` 都是安全方法。调用方：`src/app.rs`（主窗口隐藏/显示，`cx.defer` 中执行）、`src/app/temporary.rs`（临时窗口显示/隐藏，在 `cx.spawn` 中且不持有 App 借用执行，带可见代次检查）、`src/app/shortcuts.rs`（截图前隐藏临时窗口）、`gupi-screen-capture` 的覆盖窗口（在 `open_window` 构建闭包中调整位置）。
- 以 GPUI 上下文确认存活不能作为安全依据：`App::windows()` 包含正被更新（暂时取出）的窗口；`read_window` 对此类窗口会 panic；`update_window` 返回前会刷新待处理效果，期间排队的关闭可能在确认成功后才生效。
- GPUI 移除窗口时（`update_window_erased` 收尾）先删除窗口表中已为空的槽位，同步调用 `App::on_window_closed` 观察者，收尾返回时才释放被取出的 `Window`（及平台窗口）；`WindowId` 来自带版本的 slotmap，不会复用。
- 应用关闭（`App::shutdown`）不走这条路径：先调用 `on_app_quit` 回调，再直接清空窗口表并刷新效果，不触发 `on_window_closed`。
- 平台窗口的实际销毁晚于移除：`MacWindow::drop` 在前台任务中 `close`（窗口以 `releasedWhenClosed: NO` 创建）；`WindowsWindow::drop` 先异步隐藏，再在前台任务中等待对话框空闲后才 `DestroyWindow`。此间隔内原生句柄仍然有效，销毁后 `HWND` 数值还可能被本进程本线程的新窗口复用（见 [`IsWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindow)），因此原生查询不能证明仍是原来那个 GPUI 窗口。
- `gpui-pre-windows` 的 `display_id_for_monitor` 为 `DisplayId::new(monitor.0 as u64)`；临时窗口选屏已按完整 `HMONITOR` 匹配（[Gupi #221 设计](../issue-221/README.md)）。`window-ext` 的 `scale_factor_for_display` 却把它当作 `EnumDisplayMonitors` 序号，查找失败时回退到创建句柄时记录的窗口缩放比例。

## 设计

### 资源变更在退出时完成

`ResourceController` 区分两种停止：

- 读取：目录刷新仍立即取消。
- 变更：新增退出状态。`shutdown()` 取消目录刷新、拒绝新的 `change`/`refresh`，但保留正在运行的 `Mutation` 及其任务；任务完成后变回 `Idle`，不再触发完成后的目录重读。删除现有 `stop()`，没有退出以外的调用方。

包命令按同一规则等待完成，不在退出时杀死子进程：包安装和移除会改动包目录与设置文件，中途终止的残留比等待更难恢复。等待时长由命令本身决定，退出界面保持“正在退出”。命令卡住（网络、git、扩展脚本均无超时）时退出会一直停留，只能由系统强制退出；本 Issue 不新增超时或取消界面。意外的任务或运行时销毁仍由 `kill_on_drop` 终止子进程。

`SettingsView` 提供资源变更是否仍在运行的查询（全局与所有已打开的项目控制器）；`begin_quit` 的等待循环把它加入忙碌条件。等待仍沿用现有轮询方式，不引入新的退出协调机制。

### 原生窗口句柄绑定 GPUI 窗口生命周期

句柄携带一个与原 GPUI 窗口生命周期绑定的存活标记，在调用原生 API 前检查；安全性不依赖调用时查询 GPUI，也不改变任何调用点的执行时机与借用边界：

- 存活标记：`window-ext` 为每个 GPUI 窗口（按 `WindowId`）维护一个共享标记，由一次性注册的 `on_window_closed` 观察者在 GPUI 移除该窗口时同步失效，早于平台窗口的释放和实际销毁。应用关闭时，`on_app_quit` 回调在返回其 future 之前同步让所有标记失效，早于窗口表被清空；注册表随 App 存在，注册表销毁时剩余标记同样失效。`native_window_handle` 改为接收 `App` 以确保观察者已注册，同一窗口的句柄共享同一标记。标记读取只是本地状态，不借用 App 或窗口。
- 四个操作先检查标记，失效时返回新的 `WindowExtError::WindowClosed`，不触碰原生句柄。因为 `WindowId` 不复用，`HWND` 数值复用或销毁前的间隔都不会让旧句柄作用到其他窗口或重新显示已移除的窗口。
- macOS 句柄另外持有 `Retained<NSWindow>`（创建时取自 `&Window`），即使标记逻辑出错也不会解引用已释放的对象；句柄因此不再是 `Copy`，只在主线程使用。Windows 仍用 `HWND`，标记检查之后再以 `IsWindow` 作防御。
- 调用点不变：主窗口仍在 `cx.defer` 中调用（已离开窗口借用，但仍处于 App 效果处理中）；临时窗口和截图前隐藏仍在 `cx.spawn` 中、所有 `cx.update` 之外调用，代次检查保持原样。两种边界不相同，各自沿用现状，不合并。

已持有 `&Window` 的截图覆盖窗口构建闭包不经过句柄，改用 `WindowExt` 上的立即操作，只用于现有的层级和位置设置，不扩展到显示/隐藏。

Windows 的句柄不再记录创建时的缩放比例；未指定目标屏幕时按窗口当前所在屏幕实时读取。

### 目标屏幕与缩放比例

`DisplayId` 在 Windows 上还原为 `HMONITOR`，必须出现在 `EnumDisplayMonitors` 的当前列表中，再用 `GetDpiForMonitor` 读取缩放比例。选择逻辑以纯函数实现（输入标识、当前显示器列表、缩放查询），可在任意平台测试。

指定了 `DisplayId` 却找不到时返回 `DisplayNotFound`，缩放查询失败时返回单独的错误；macOS 找不到对应 `NSScreen` 时同样返回 `DisplayNotFound`。未指定 `DisplayId` 仍表示窗口当前所在屏幕。

调用方处理：

- 截图：覆盖窗口定位失败时，请求以现有的 `Error::DisplayChanged` 结束并关闭已创建的覆盖窗口，不留下错位的截图层。层级设置失败仍只记录警告。
- 临时窗口：定位失败只记录警告，仍然显示（保持在原位置）；用户召唤窗口优先于定位精度，与现状一致。

## 实施

1. `gupi-resources`：`ResourceController` 退出状态与 `shutdown()`；删除 `stop()`，更新测试。
2. 根包：`SettingsView`/资源页暴露变更运行查询，`begin_quit` 改调 `shutdown` 并加入等待条件。
3. `window-ext`：窗口存活标记与 `on_window_closed`/`on_app_quit` 观察者、句柄检查标记并持有原生窗口、新增错误；Windows `HMONITOR` 选择纯函数与显式失败；macOS 目标屏幕显式失败；删除 `resolve_target_scale_factor` 及其回退测试。
4. 调用方迁移：`src/app.rs`、`src/app/temporary.rs`、`src/app/shortcuts.rs`；`gupi-screen-capture` 定位失败时结束请求并关闭覆盖窗口。
5. 更新[能力架构](../capability-architecture/README.md)中与此相关的描述（如有），在[开发索引](../README.md)登记本文。

## 验证

- 自动测试：
  - 包命令退出：使用测试内生成的假 Pi 可执行文件（不联网、不安装真实包），经真实的 `Change::Package` 路径启动；假命令通知已开始后调用 `shutdown()`，变更保持运行、子进程未被杀死；放行后变更结束、不触发目录重读，之后的变更和刷新被拒绝。失败退出的命令同样正常结束。文件写入与包命令共用同一变更任务所有权，不另做阻塞写入测试。
  - 退出等待：资源变更运行时等待不结束，包括当前未选中的项目控制器；变更结束后退出完成。
  - 显示器选择：非序号的不透明标识、显示器列表重排、目标缺失、缩放查询失败。
  - 截图：定位失败时请求以 `DisplayChanged` 结束且覆盖窗口被关闭（在可构造该失败的范围内）。
  - 窗口生命周期：在 GPUI 测试平台打开窗口取得存活标记，经真实的移除路径关闭窗口后标记失效，失效标记使操作在原生调用前返回 `WindowClosed`（测试平台无法提供原生句柄时，直接测试标记与操作的判定入口）；同一窗口隐藏后标记仍有效；新开的窗口不共享旧标记；经真实的 `App::shutdown` 路径关闭应用后，未释放的句柄同样返回 `WindowClosed`。移除、观察者和标记都是平台无关代码，失效判定发生在任何原生调用之前，因此平台延迟销毁和 `HWND` 复用不影响该结论。
- 真实平台只验证普通隐藏/重新显示仍然成功（原生试用）；移除后的原生拒绝由上述测试覆盖，不另做原生集成场景。
- 检查：`cargo fmt --all -- --check`；`cargo check`、`cargo test`、`cargo clippy --all-targets -- -D warnings` 覆盖 `gupi`、`gupi-resources`、`window-ext`、`gupi-screen-capture`（`--locked`）。本机尝试 `window-ext` 的 Windows 目标检查；Windows 编译以 CI（`workflow_dispatch` 或 PR）或用户的 Windows 构建结果为准，分支推送本身不触发 CI。
- macOS 原生快速试用（协调者使用 Computer Use，`script/gupi-ui-gallery` 隔离环境）：主窗口关闭/重新打开、临时窗口显示/隐藏/粘贴、截图快捷键（含临时窗口可见时）、保存资源文本后立即退出且改动写入。
- Windows：由用户检查主窗口与临时窗口显示/隐藏、截图；有不同缩放比例的多屏时检查临时窗口与截图覆盖窗口在副屏上的位置和大小。
