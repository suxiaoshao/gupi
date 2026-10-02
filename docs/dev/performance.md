# 性能采集

`performance` 是显式启用的开发 feature。默认构建不包含 `tracing-chrome`、追踪初始化或应用埋点；GPUI 的 `profiling` 后端仅在此 feature 下启用。`release` / `release-fast` 不允许启用该 feature，发行任务使用 `--no-default-features --features bundled`，不会启用性能采集。

性能入口使用独立的 `target/profiling` 和优化的 `performance` profile，保留函数符号。该 profile 开启 debug assertions，以便发行 profile 拒绝性能 feature；前后对比必须使用同一 profile，绝对耗时不能直接视为发行版本耗时。

```sh
cargo build -p gupi --bin gupi --locked --profile performance --features performance --target-dir target/profiling
GUPI_PERFORMANCE_TRACE=/tmp/gupi-before.json target/profiling/performance/gupi
```

使用日常配置、真实 Pi 和模型。前后对比记录会话历史规模；继续原会话时，后一次会包含前一次新增的历史，分析时需要考虑这一差异。

启动即记录，使用 Cmd+Q 完全退出后保存完整时间线。关闭窗口在 macOS 上可能只是隐藏，不代表退出。每次采集使用新的输出文件，已有 trace 不会被覆盖。进程被强杀时不能保证 trace 完整。

记录 RPC 事件类型、消息投影、历史快照、实时合并、行差异检测、页面/消息行 render 和 GPUI 已有性能区间。只添加计数和事件类型，不记录消息正文、工具参数和认证内容；普通日志保持原有文件输出。性能 feature 也必须提供 `GUPI_PERFORMANCE_TRACE` 才会安装时间线 subscriber。

`trace.json` 可在 Perfetto 中查看，无需 Chrome 浏览器。时间线区间反映操作的墙钟耗时；CPU 采样用于补充未埋点函数、分配和系统调用，二者不能等同。

前后对比尽量保持模型、请求、窗口尺寸和交互一致，分别保存到不同 trace 文件。记录事件数量、历史规模、消息同步每次耗时及总耗时、长帧和用户感受；历史规模或模型输出有差异时，按事件/更新数量解释结果。

消息展示保留已有行，以消息 ID 索引定位流式更新。仅处理一个连续版本的 `message_update` 时替换对应消息并重新测量该行；新增或结束消息、历史刷新、跳过版本、分支预览与强制同步仍重新投影。共享行避免复制未变化的历史内容。查找栏打开时仍同步完整查找来源。

## 真实运行测量（2026-10-02）

用户在 macOS 上使用日常配置、真实 Pi 和模型运行优化前后版本，均使用上述 `performance` profile。优化前记录 318.5 秒、两段模型运行；优化后记录 385.8 秒、三段模型运行，中间均包含空闲时间。

| 指标 | 优化前 | 优化后 |
| --- | ---: | ---: |
| 流式更新次数 | 6,382 | 10,449 |
| 历史条目最大数量 | 1,615 | 1,632 |
| 消息同步总耗时 | 38.01 秒 | 0.70 秒 |
| 单次消息同步中位数 | 5.4335 ms | 0.0059 ms |
| 单次消息同步 P95 | 7.9763 ms | 0.0212 ms |
| 完整消息投影次数 | 6,449 | 70 |

优化后 10,449 次流式更新均命中增量路径。按相邻同步间隔不超过 1 ms 合并区间，连续同步且没有绘制的最长区间从 15.04 秒降到 44.18 ms。

这些是埋点墙钟耗时，包含采集开销，嵌套区间不能相加为 CPU 时间。两轮模型输出、交互、运行次数和历史规模不同，不能把总耗时变化直接等同于整机 CPU 降幅；更新数量增加时，单次同步耗时与完整投影次数仍明显下降。

优化后运行阶段约每秒绘制 96–119 次，单次绘制中位数约 4.3 ms，绘制成为当前记录中的主要耗时区间。消息接收与组装总耗时约 46 ms；状态层文本追加和 Markdown 控件的 `push_str` 仍保留。展示同步仍复制当前消息快照，下一阶段需要分别测量动画调度、绘制范围和消息复制成本。

原始时间线保留在本机 `/tmp/gupi-real-before.json` 和 `/tmp/gupi-real-after.json`，不随源码提交。
