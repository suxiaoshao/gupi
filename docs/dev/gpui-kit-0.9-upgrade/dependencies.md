# 全量依赖审阅

[返回升级计划](README.md)。2026-11-05 读取两个 workspace 根/成员 normal、dev、build、target-specific manifests：94 个现有唯一 registry package；本计划新增 `gpui-pre-reqwest-client =0.3.11`，目标共 95 项。目标为官方 sparse index 当日最高稳定非 yanked 版本；表格保留升级前约束用于核对升级覆盖。Gupi manifests 与 lock 已按目标更新，共享部分由固定 revision 提供。按 + 前的语义版本识别 prerelease；toml 的 +spec-1.1.0 是构建元数据，不是预发布。

直接依赖的 features 延续既有用途；普通 API 适配不改变产品/数据边界。transitive 兼容锁刷新并解释多版本；内部 path 包保持产品版本，不从 registry 升级。

| 库（官方版本源） | 仓库 | 升级前约束 | 升级目标 | 范围/核对要点 |
| --- | --- | --- | --- | --- |
| [async-channel](https://crates.io/crates/async-channel) | gpui | 2.5.0 | 2.5.0 | 已最新，复核后保留。 |
| [async-compat](https://crates.io/crates/async-compat) | gpui | 0.2.6 | 0.2.6 | 已最新，复核后保留。 |
| [async-compression](https://crates.io/crates/async-compression) | gpui | 0.4.51 | 0.4.52 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [async-stream](https://crates.io/crates/async-stream) | gpui | 0.3.6 | 0.3.6 | 已最新，复核后保留。 |
| [base64](https://crates.io/crates/base64) | gupi, gpui | 0.23.1 | 0.23.1 | 已最新，复核后保留。 |
| [block2](https://crates.io/crates/block2) | gpui | 0.6.2 | 0.6.2 | 已最新，复核后保留。 |
| [bytes](https://crates.io/crates/bytes) | gpui | 1.12.1 | 1.12.1 | 已最新，复核后保留。 |
| [cc](https://crates.io/crates/cc) | gupi | 1.9.0 | 1.11.0 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [chrono](https://crates.io/crates/chrono) | gupi | 0.4.45 | 0.4.45 | 已最新，复核后保留。 |
| [clap](https://crates.io/crates/clap) | gupi, gpui | 4.6.7 | 4.6.7 | 已最新，复核后保留。 |
| [dirs-next](https://crates.io/crates/dirs-next) | gupi, gpui | 2.0.0 | 2.0.0 | 已最新，复核后保留。 |
| [duckdb](https://crates.io/crates/duckdb) | gpui | 1.10506.1 | 1.20000.1 | 引擎版本跳变；只用临时数据库验证 Feiwen 查询/迁移，不操作用户数据。 |
| [ed25519-dalek](https://crates.io/crates/ed25519-dalek) | gupi | 3.0.0 | 3.0.0 | 已最新，复核后保留。 |
| [encoding_rs](https://crates.io/crates/encoding_rs) | gpui | 0.8.42 | 0.8.44 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [flate2](https://crates.io/crates/flate2) | gupi | 1.1.10 | 1.1.10 | 已最新，复核后保留。 |
| [fluent-bundle](https://crates.io/crates/fluent-bundle) | gupi, gpui | 0.16.0 | 0.16.0 | 已最新，复核后保留。 |
| [futures](https://crates.io/crates/futures) | gpui | 0.3.34 | 0.3.34 | 已最新，复核后保留。 |
| [futures-util](https://crates.io/crates/futures-util) | gpui | 0.3.34 | 0.3.34 | 已最新，复核后保留。 |
| [garde](https://crates.io/crates/garde) | gpui | 0.23.0 | 0.23.0 | 已最新，复核后保留。 |
| [get-selected-text](https://crates.io/crates/get-selected-text) | gupi | 0.1.6 | 0.1.6 | 已最新，复核后保留。 |
| [getrandom](https://crates.io/crates/getrandom) | gupi | 0.4.3 | 0.4.3 | 已最新，复核后保留。 |
| [global-hotkey](https://crates.io/crates/global-hotkey) | gupi | 0.9.0 | 0.9.0 | 已最新，复核后保留。 |
| [globset](https://crates.io/crates/globset) | gupi | 0.4.20 | 0.4.21 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [gpui-component](https://crates.io/crates/gpui-component) | gpui | =0.8.0 | 0.9.0 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [gpui-kit](https://crates.io/crates/gpui-kit) | gupi, gpui | =0.8.0 | 0.9.0 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [gpui-kit-assets](https://crates.io/crates/gpui-kit-assets) | gpui | =0.8.0 | 0.9.0 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [gpui-pre](https://crates.io/crates/gpui-pre) | gupi, gpui | =0.3.9 | 0.3.11 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [gpui-pre-reqwest-client](https://crates.io/crates/gpui-pre-reqwest-client) | gupi | 未接入 | =0.3.11 | 官方 HTTP client，经 App::set_http_client 安装；Files Markdown 远程图片走 Kit 默认加载。 |
| [hayro](https://crates.io/crates/hayro) | gpui | 0.8.0 | 0.9.0 | 0.x 次版本：PDF 解析/渲染 API 与真实样本验证，不新增 PDF 能力。 |
| [http](https://crates.io/crates/http) | gupi, gpui | 1.5.0 | 1.5.0 | 已最新，复核后保留。 |
| [http-body-util](https://crates.io/crates/http-body-util) | gpui | 0.1.5 | 0.1.5 | 已最新，复核后保留。 |
| [hyper](https://crates.io/crates/hyper) | gpui | 1.12.0 | 1.12.0 | 已最新，复核后保留。 |
| [hyper-util](https://crates.io/crates/hyper-util) | gpui | 0.1.21 | 0.1.21 | 已最新，复核后保留。 |
| [ignore](https://crates.io/crates/ignore) | gupi | 0.4.33 | 0.4.34 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [image](https://crates.io/crates/image) | gupi, gpui | 0.25.10 | 0.25.10 | 已最新，复核后保留。 |
| [libc](https://crates.io/crates/libc) | gupi | 0.2.191 | 0.2.191 | 已最新，复核后保留。 |
| [libloading](https://crates.io/crates/libloading) | gupi | 0.9.0 | 0.9.0 | 已最新，复核后保留。 |
| [markdown](https://crates.io/crates/markdown) | gupi | 1.0.0 | 1.0.0 | 已最新，复核后保留。 |
| [material-color-utils](https://crates.io/crates/material-color-utils) | gpui | 0.1.3 | 0.1.3 | 已最新，复核后保留。 |
| [mime](https://crates.io/crates/mime) | gpui | 0.3.17 | 0.3.17 | 已最新，复核后保留。 |
| [mime_guess](https://crates.io/crates/mime_guess) | gpui | 2.0.5 | 2.0.5 | 已最新，复核后保留。 |
| [msi](https://crates.io/crates/msi) | gupi | 0.10.0 | 0.11.1 | 0.x 次版本：xtask MSI 元数据与安装包读取接点核对，保留更新校验。 |
| [nom](https://crates.io/crates/nom) | gpui | 8.0.0 | 8.0.0 | 已最新，复核后保留。 |
| [objc2](https://crates.io/crates/objc2) | gupi, gpui | 0.6.5 | 0.6.5 | 已最新，复核后保留。 |
| [objc2-app-kit](https://crates.io/crates/objc2-app-kit) | gupi, gpui | 0.3.2 | 0.3.2 | 已最新，复核后保留。 |
| [objc2-core-graphics](https://crates.io/crates/objc2-core-graphics) | gupi, gpui | 0.3.2 | 0.3.2 | 已最新，复核后保留。 |
| [objc2-foundation](https://crates.io/crates/objc2-foundation) | gupi, gpui | 0.3.2 | 0.3.2 | 已最新，复核后保留。 |
| [objc2-user-notifications](https://crates.io/crates/objc2-user-notifications) | gpui | 0.3.2 | 0.3.2 | 已最新，复核后保留。 |
| [pinyin](https://crates.io/crates/pinyin) | gpui | 0.11.0 | 0.11.0 | 已最新，复核后保留。 |
| [plist](https://crates.io/crates/plist) | gupi, gpui | 1.10.2 | 1.10.3 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [proc-macro2](https://crates.io/crates/proc-macro2) | gpui | 1.0.107 | 1.0.107 | 已最新，复核后保留。 |
| [profiling](https://crates.io/crates/profiling) | gupi | 1.0.18 | 1.0.18 | 已最新，复核后保留。 |
| [quote](https://crates.io/crates/quote) | gpui | 1.0.47 | 1.0.47 | 已最新，复核后保留。 |
| [r2d2](https://crates.io/crates/r2d2) | gpui | 0.8.10 | 0.8.10 | 已最新，复核后保留。 |
| [raw-window-handle](https://crates.io/crates/raw-window-handle) | gupi | 0.6.2 | 0.6.2 | 已最新，复核后保留。 |
| [regex](https://crates.io/crates/regex) | gupi | 1.13.1 | 1.13.1 | 已最新，复核后保留。 |
| [reqwest](https://crates.io/crates/reqwest) | gupi, gpui | 0.13.5 | 0.13.5 | 已最新，复核后保留。 |
| [rodio](https://crates.io/crates/rodio) | gpui | =0.23.0 | 0.23.0 | 已最新，复核后保留。 |
| [scraper](https://crates.io/crates/scraper) | gpui | 0.27.0 | 0.29.0 | 0.x 次版本：Feiwen/novel-download selector 与 HTML 提取回归。 |
| [semver](https://crates.io/crates/semver) | gupi | 1.0.28 | 1.0.28 | 已最新，复核后保留。 |
| [serde](https://crates.io/crates/serde) | gupi, gpui | 1.0.229 | 1.0.229 | 已最新，复核后保留。 |
| [serde_json](https://crates.io/crates/serde_json) | gupi, gpui | 1.0.154 | 1.0.154 | 已最新，复核后保留。 |
| [serde_yaml_ng](https://crates.io/crates/serde_yaml_ng) | gupi | 0.10.0 | 0.10.0 | 已最新，复核后保留。 |
| [sha2](https://crates.io/crates/sha2) | gupi | 0.11.1 | 0.11.1 | 已最新，复核后保留。 |
| [shell-words](https://crates.io/crates/shell-words) | gupi | =1.1.1 | 1.1.1 | 已最新，复核后保留。 |
| [similar](https://crates.io/crates/similar) | gupi | 2.7.0 | 3.2.1 | 2 → 3 主版本：验证 Git text-to-patch 换行、空文件、侧别契约，不扩 Diff 字符引用。 |
| [smol](https://crates.io/crates/smol) | gupi, gpui | 2.0.2 | 2.0.2 | 已最新，复核后保留。 |
| [sonic-rs](https://crates.io/crates/sonic-rs) | gupi | 0.5.10 | 0.5.10 | 已最新，复核后保留。 |
| [syn](https://crates.io/crates/syn) | gpui | 3.0.9 | 3.0.9 | 已最新，复核后保留。 |
| [sys-locale](https://crates.io/crates/sys-locale) | gupi, gpui | 0.3.2 | 0.3.2 | 已最新，复核后保留。 |
| [sysinfo](https://crates.io/crates/sysinfo) | gupi | 0.39.6 | 0.40.1 | 0.x 次版本：进程清理/会话内存指标 API 与权限核对。 |
| [tar](https://crates.io/crates/tar) | gupi | 0.4.46 | 0.4.46 | 已最新，复核后保留。 |
| [tauri-bundler](https://crates.io/crates/tauri-bundler) | gupi, gpui | 2.10.3 | 2.10.4 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [tauri-utils](https://crates.io/crates/tauri-utils) | gupi, gpui | 2.10.2 | 2.10.2 | 已最新，复核后保留。 |
| [tempfile](https://crates.io/crates/tempfile) | gupi, gpui | 3.27.0 | 3.27.0 | 已最新，复核后保留。 |
| [thiserror](https://crates.io/crates/thiserror) | gupi, gpui | 2.0.21 | 2.0.21 | 已最新，复核后保留。 |
| [time](https://crates.io/crates/time) | gupi, gpui | 0.3.55 | 0.3.55 | 已最新，复核后保留。 |
| [tokio](https://crates.io/crates/tokio) | gupi, gpui | 1.54.0 | 1.54.1 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [tokio-util](https://crates.io/crates/tokio-util) | gpui | 0.7.20 | 0.7.20 | 已最新，复核后保留。 |
| [toml](https://crates.io/crates/toml) | gupi, gpui | 1.1.8 | 1.2.0+spec-1.1.0 | 1.2.0+spec-1.1.0 是稳定版（+ 后为构建元数据）；核对解析/序列化，不改配置语义。 |
| [tracing](https://crates.io/crates/tracing) | gupi, gpui | 0.1.44 | 0.1.44 | 已最新，复核后保留。 |
| [tracing-chrome](https://crates.io/crates/tracing-chrome) | gupi | 0.7.2 | 0.7.2 | 已最新，复核后保留。 |
| [tracing-subscriber](https://crates.io/crates/tracing-subscriber) | gupi, gpui | 0.3.23 | 0.3.23 | 已最新，复核后保留。 |
| [trash](https://crates.io/crates/trash) | gupi | 5.2.9 | 5.2.9 | 已最新，复核后保留。 |
| [tray-icon](https://crates.io/crates/tray-icon) | gupi | 0.26.1 | 0.26.1 | 已最新，复核后保留。 |
| [trybuild](https://crates.io/crates/trybuild) | gpui | 1.0.122 | 1.0.122 | 已最新，复核后保留。 |
| [two-face](https://crates.io/crates/two-face) | gupi | 0.4.5 | 0.5.2+bat-0.26.1 | 0.x 次版本：与 Kit syntect 特性/类型一起检查，保留语法覆盖。 |
| [unic-langid](https://crates.io/crates/unic-langid) | gpui | 0.9.6 | 0.9.6 | 已最新，复核后保留。 |
| [url](https://crates.io/crates/url) | gupi, gpui | 2.5.8 | 2.5.8 | 已最新，复核后保留。 |
| [uuid](https://crates.io/crates/uuid) | gupi | 1.28.0 | 1.29.0 | 升级并核对 changelog/feature/MSRV，延续现有职责。 |
| [walkdir](https://crates.io/crates/walkdir) | gupi, gpui | 2.5.0 | 2.5.0 | 已最新，复核后保留。 |
| [which](https://crates.io/crates/which) | gupi, gpui | 8.0.6 | 8.0.6 | 已最新，复核后保留。 |
| [windows](https://crates.io/crates/windows) | gupi, gpui | 0.62.2 | 0.62.2 | 保持 windows::core 同一拥有者，不单独引入不匹配的 windows-core。 |
| [windows-sys](https://crates.io/crates/windows-sys) | gupi | 0.61.2 | 0.61.2 | 已最新，复核后保留。 |
| [xcap](https://crates.io/crates/xcap) | gupi | 0.9.8 | 0.9.8 | 已最新，复核后保留。 |

共享 Git crate 不在这 95 项中：app-theme、gpui-form、gpui-form-gpui-component、gpui-operation、gpui-store、platform-ext、gpui-lucide、gpui-tokio 一起固定到共享修复 SHA `c3edcc047d42dd299a089655f0e2ea28b89384d7`（gpui#272，包含 0.9 升级）；Form macros 等传递 workspace crate 必须跟随同一来源。

## Gupi 实施核对

- `similar 3.2.1`（MSRV 1.85）：保留 TextDiff 与现有 Git patch 生成/严格解析、超限 fallback；采用上游改进后的 Myers，不启用 merge 或新的宽松解析。
- `sysinfo 0.40.1`（MSRV 1.95）：`System::new` 的失败显式处理，无法检查进程时禁止清理 gallery；仍只启用 system feature。
- `msi 0.11.1`（MSRV 1.85）：LanguageId/id API 适配，不改变安装包语言号。
- `two-face 0.5.2`（MSRV 1.79）：保持 syntect-fancy、关闭默认 onig；bat 0.26.1 的语法资源随依赖更新，不新增主题入口。
- `toml 1.2.0`（MSRV 1.85）：使用稳定版；manifest 不写 Cargo 会忽略的构建元数据，lock 保留完整发行版本。配置读写由现有产品测试覆盖。
- 官方 HTTP 客户端锁到 0.3.11；其 gpui-pre-reqwest 分支与既有 reqwest 0.13 并存，但共用 rustls 0.23 / platform-verifier 0.7，无第二套 TLS 后端。
- `cargo update` 刷新兼容的传递依赖；内部 path 包版本未变。Rust 1.99.1 满足上述 MSRV，官方 stable manifest 仍为 1.99.1。
- Nix 的 nixpkgs 更新为 `7dcdd87373d72e7a2173e4db02e20d3d7f495cfd`，rust-overlay 更新为 `7fcafdf6f0761462d68012aba6a627be3f008d68`。Actions checkout v7.0.1、upload-artifact v7.0.2、download-artifact v8.0.2、rust-cache v2.9.3 已与各官方 latest release 核对，保留现有固定 SHA。
