<a id="logging"></a>

# 日志

一个内部能力统一维护环境变量驱动的配置、日志指令解析与匹配、格式化，以及同步写入 stdout/stderr/自定义管道。web-bindings 构建工具初始化此日志器。共享 terminal-style 能力仍负责 ANSI 样式和输出适配；本包不复制或替代它。

```rust
use logging::Builder;
let logger = Builder::new().filter_level(log::LevelFilter::Warn).build();
assert_eq!(logger.filter(), log::LevelFilter::Warn);
```

默认特性启用自动颜色和正则表达式消息过滤。禁用 regex 时，消息过滤仍按字面子串匹配。日志能力始终依赖 std，包括解析错误和容错指令解析。目标前缀优先级、默认错误级别、环境变量名和输出格式保持不变。`filter` 暴露底层过滤器构建器，供自定义日志器组合使用，无需另设 Cargo 包。

原始 env_logger 0.11.11 和 env_filter 2.0.0 身份、MIT/Apache-2.0 许可、原始文档及源码/测试证据仍保留在合规来源材料中。
