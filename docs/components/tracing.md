<a id="project-owned-tracing"></a>

# 项目维护的追踪能力

`tracing` 包维护插桩宏、`primitives` 分派和元数据、`collector` 注册表/层/过滤/格式化、`log_bridge` 适配器及 `pattern` 增量字段匹配。宿主 `tracing-attributes` 过程宏仍独立。门面保留其功能名称和生成宏身份；不再保留旧包兼容 crate 或重复的提供方身份。

<a id="capabilities"></a>

## 能力

- 不启用默认特性：no_std + alloc 插桩，仅依赖 pin-project-lite。不引入 collector、regex、log、time、线程本地注册表或序列化提供方。
- `collector`：可在 alloc 下使用的层抽象；`registry` 增加 std 和 span 存储。
- `fmt`、`ansi`、`env-filter`、`json`、`time` 按需启用各自实际提供方。
- `pattern` 按需启用共享正则 DFA 语法/搜索；`unicode` 增加 Unicode 支持。
- `log` 是 trace-to-log 回退；`log-tracer` 启用 log-to-trace。两者均不同于有意同时输出二者、可能产生重复事件的 `log-always`。
- `collector-default` 为管理可执行程序选择格式化、ANSI、注册表、桥接和小向量支持。库默认仍为 std+attributes。

JSON 事件字段通过 SerializeMap 流式输出，不构建中间事件树；保留类型值、转义、重复键顺序和首个错误传播。Span 宏保持调用点及字段求值语义。全局最大级别采用显式整数编码，意外编码在所有构建配置下均闭锁处理。

<a id="retained-limits"></a>

## 保留的限制

维护 Linux amd64 和浏览器 WASM 编译；WASM 检查不证明浏览器运行时、文件系统或终端行为。不支持 interest-cache、系统本地时间查询或 valuable 集成；UTC/固定偏移格式化可用。

运行 `python3 .github/scripts/check-tracing-family.py`。依赖不可用 tracing-mock、futures 测试提供方或 wasm-bindgen-test 的用例，以及忽略测试和基准，仅保留来源，不计为执行。

<a id="sources"></a>

## 来源

- tracing 0.1.44、tracing-core 0.1.36、tracing-subscriber 0.3.23、tracing-log 0.2.0：https://github.com/tokio-rs/tracing
- matchers 0.2.0：https://github.com/hawkw/matchers

原始清单、版本、许可证和源码凭据保留在合规来源材料中。保留它们是为了归属与审计，并不要求维护代码继续保留上游布局、兼容名称或不可用特性。
