<a id="datetime-capability"></a>

# 日期与时间能力

`date-time` 负责日历运算、时间戳、UTC 与显式固定偏移、格式描述、解析、格式化及可选序列化。它将 `time`、`time-core`、`deranged`、`powerfmt`、`num-conv` 五个包合为一个职责包，而非转发包装或兼容 Cargo 别名。Rust 导入名为 `date_time`；标准库的 `std::time` / `core::time` 及 tracing 采集器私有 `fmt::time` 命名空间与它不同。

日历单位、有界整数不变量、宽度感知显示与整数转换共用编译/特性边界。除有意公开的日历单位与工具函数外，支持模块均私有。数值辅助层只保留日期运算实际使用的扩宽与低位截断契约，不保留废弃转换别名或未用的饱和/检查扩展 API。未用的独立格式缓冲及扩展已移除。有界运算在各整数宽度上使用统一生成实现；宽度/特性选择所需的 dead-code 例外范围受限，不降低警告门禁。原 time manifest 的 68 个连字符 lint 名改为下划线拼写，级别和优先级不变；`check-cfg` 仍是 Cargo 的连字符配置键。

<a id="supported-consumers-and-features"></a>

## 消费者与支持特性

- storage-engine 解析和格式化持久化容器时间戳。
- storage-server 格式化线上时间戳及遥测时间。
- tracing 的 `time` 特性为真实 `UtcTime`、`OffsetTime` 采集器启用 date-time；固定偏移由调用者显式提供。
- 序列化门禁使用真实 data-serialization/json-codec，包含 date-time JSON 往返。
- 默认 `std` 启用 `alloc`。禁用默认特性的日历/时长运算不需要分配器或标准库；解析也不需要二者，动态分配的格式描述需要 `alloc`。
- `formatting` 需要 `std`；`large-dates` 在包内扩展年份范围，无跨包特性转发。
- `serialization` 直接使用 data-serialization，包括私有有界表示。可读及知名格式适配器另启用格式化/解析；紧凑序列化不需要 `std`。
- `wasm-bindgen` 保留真实 js-sys Date 互操作和浏览器时钟路径。原生目标为 Linux amd64；浏览器 WASM 服务于 Web 客户端，不是另一种原生平台。

<a id="explicit-unsupported-boundary"></a>

## 明确不支持的能力

不提供操作系统本地时区发现或时区刷新 API。原 `local-offset` 特性引用缺失的 `num_threads` 后端，也没有产品消费者；特性、本地偏移错误变体、本地时钟及 TZ 刷新方法和平台实现已移除并归档。请求系统偏移时不能静默以 UTC 代替。调用者使用 UTC 或显式固定偏移；命名时区、夏令时及变化中的本地时区解析需要单独设计。

未提供实际实现的随机、属性测试及过程宏特性已删除。没有引入或合并日期过程宏包；应使用构造器和运行时格式描述解析，而非不存在的宏。负向门禁验证不支持的特性、本地时间方法及私有实现模块无法被消费。

<a id="evidence-and-limits"></a>

## 证据与限制

`.github/scripts/check-date-time-family.py` 检查原生/WASM 隔离特性闭包、日历和时间戳边界、真实序列化，以及 tracing 的 UTC / +05:30 格式化。WASM 检查是编译证据，不是浏览器执行。tracing 与序列化门禁保留各自消费者，现解析到 date-time 包。

原始来源保持不可变。历史外置测试通过精确可逆的导入/配置编辑迁移，不默默改写为新测试。项目能力测试位于 `test/date-time`，不在 `src`。17 份历史基准源码仍原样、不执行，等待单独审阅的迁移；不能由包合并推断速度、产物大小或内存改善，也不能把历史上游集成/文档测试材料等同于项目门禁。

特性隔离的继承默认单元目标执行 64 项测试。ISO 夹具将不可用的 `rstest` 改为两项普通表驱动测试：23 组配置各保留六项断言，另独立检查七种非法编码。接收工作区启用 formatting 后，`cargo test -p date-time --lib` 通过 66 项测试。原始来源归属不变；[限定夹具逆向证明](../reports/date-time-iso-fixture-proof.json)将此已审阅转换与冻结整合历史分开记录。历史全特性单元目标仍引用不可用的 `serde_test2` 和旧 JSON 测试框架；独立能力门禁使用真实序列化、格式化与解析实现，不代表完整上游全特性套件通过。

新增字节计数回归发现并修复既有 RFC3339 缺陷：`format_into` 写入小数秒数字却未计入返回值。现在返回值包含这些字节。测试比较 UTC 和 +05:30 中 1、3、9 位小数秒的报告/实际字节数，包含纪元前时刻。这是正确性修复，不是性能声明。
