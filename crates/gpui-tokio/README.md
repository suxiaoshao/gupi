# gpui-tokio

Gupi 内部的 GPUI / Tokio 桥接。

`init(cx)` 创建并持有 runtime。`Tokio::spawn` 返回 `Task<Result<R, JoinError>>`，丢弃 GPUI task 会中止对应 Tokio task。全局状态销毁时使用后台关闭 runtime，业务资源必须由应用显式收尾。

依赖版本由根 `Cargo.toml` 固定，此 crate 随 Gupi 维护，不单独发布。
