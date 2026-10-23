<a id="http-routing"></a>

# HTTP 路由

`http-routing` 提供路由、提取器、请求扩展、响应转换及 HTTP 正文类型。内核是内部模块，公共 trait 不重复定义。

无默认特性时，路由与内核可在 Linux amd64 和浏览器 WASM 编译，不依赖 Tokio、Hyper 或网络套接字。表单/查询拒绝构造器随各自提取器特性启用。`axum-macros` 是独立宿主过程宏，其生成路径使用 `http_routing`。内核禁止 unsafe。

<a id="transport-integration-boundary"></a>

## 传输集成边界

`http1` 只启用协议引擎，`tokio` 启用运行时/服务端适配器。仅 HTTP/1 配置不引入 Tokio 网络、运行时或时间特性。应用路由与协议接收保持各自正文类型；就绪感知中间件通过显式服务适配器接入。

服务端安装 `TokioTimer` 并提供 30 秒请求头期限；协议探测把连接交给 HTTP/1 解析器时不重启计时。正文与整个操作的期限由应用层负责，见[HTTP 传输](http-transport.md)。

## 不支持的能力

不提供 WebSocket、multipart 提取器或 HTTP/2。Cargo 拒绝 `ws`、`multipart`、`__private`、`__private_docs` 和 `http2`，不接受空兼容特性。

HTTP/1.1 通用协议升级由 http-transport 提供，包括 101 交换、已缓冲字节及双向 I/O；这不代表 WebSocket 握手/帧实现。

## 检查与来源

运行 `python3 .github/scripts/check-http-routing.py`，检查原生全部声明特性、WASM 无运行时子集及不支持能力的拒绝，并测试路径/查询路由、404/405、正文/状态、提取器拒绝和 derive/debug-handler 宏。

来源为 Axum 0.8.9、axum-core 0.5.6；原始版本、许可证、提交、不可变源码和测试归属保存在合规材料中。`src/extract/ws.rs`、`src/extract/multipart.rs`、`src/test_helpers/mod.rs` 和 `src/test_helpers/test_client.rs` 是受哈希检查保护的未启用源码，不是受支持 API。`test/foundation/http-routing` 中未接入的上游测试也不计为执行。
