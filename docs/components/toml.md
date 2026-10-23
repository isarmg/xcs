<a id="toml-configuration"></a>

# TOML 配置

`toml` 按模块和特性提供配置能力：

- [`datetime`](toml/datetime.md)：日期、时间和偏移，始终可用。
- [`span`](toml/span.md)：附有原始字节区间的值，始终可用。
- [`writer`](toml/writer.md)：由 `write` 启用的底层编码；`alloc` 增加分配式输出辅助能力。
- [`editor`](toml/editor.md)：由 `edit` 启用的保留格式模型；`parse` 引入 winnow，`display` 引入 writer。
- 根级 `Value`、`Table`、序列化/反序列化和转换 API 由 `serialization` 启用；默认保留 serialization、parse 和 display。

禁用默认特性后，datetime/span 没有第三方生产依赖。两者均不需要 editor、indexmap、winnow 或序列化引擎。单独序列化不要求编辑能力。维护原生 Linux amd64 和浏览器 WASM。统一包是基于 std 的组件，不宣称保留旧独立 writer 的嵌入式/no_std 支持。`alloc` 和 `std` 选择 writer 辅助能力，不代表整个包保证 no_std。

受支持的 `preserve_order` 和 `unbounded` 选项保留原有语义。不支持 `perf` 特性。序列化的六个私有 TOML/span 标记字符串精确保留。错误区间仍为字节偏移。editor 原有行为会将 CRLF 规范化为 LF 并补充末尾换行；不保证逐字节保留 CRLF。

editor 保留原有、范围严格受限的 UTF-8 快速路径，其输入由解析器边界保证有效。所有其他实现模块禁止 unsafe 代码。公共解析器接受 `&str`，不接受任意字节缓冲。

<a id="sources"></a>

## 来源

toml 0.8.23、toml_datetime 0.6.11、toml_edit 0.22.27、toml_write 0.1.2 和 serde_spanned 0.6.9的不可变来源记录、原始清单、上游 README 和 MIT/Apache-2.0 许可证均保留。精确源码映射由合规凭据维护。

<a id="test-scope"></a>

## 测试范围

运行 `python3 .github/scripts/check-toml-consolidation.py`。门禁检查原生/WASM 最小特性及完整并集、能力反向编译和无依赖原语图，执行单元、边界和文档测试。专项覆盖源字节区间、格式化、畸形 TOML、Unicode、日期精度/偏移和六个序列化标记，消费者包括 release-tools 与 contract-check。

原始外部测试的路径、哈希及归属由来源凭据保留。缺少 proptest 的生成器、上游基准和不可用外部语料驱动不计为执行；确定性 Unicode 标量/边界测试不能替代随机组合及收缩过程。测试在临时源码副本中运行，不修改产品根锁文件或源码清单。
