# window-ext

Gupi 内部的原生窗口辅助接口，提供窗口显示、隐藏、层级、位置及光标区域操作。

`WindowExt` 从 GPUI 窗口取得原生句柄；macOS 使用 AppKit，Windows 使用 Win32。能力按平台实现，其他平台中的空实现不代表系统支持对应操作。应用负责窗口生命周期，句柄不延长窗口寿命。

依赖版本由根 `Cargo.toml` 固定，此 crate 随 Gupi 维护，不单独发布。
