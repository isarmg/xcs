<a id="http-routing"></a>

# HTTP 路由

`http-routing` 提供路由、提取器、请求扩展、响应转换及 HTTP 正文类型。内核是内部模块，公共 trait 不重复定义。

无默认特性时，路由与内核可在 Linux amd64 和浏览器 WASM 编译，不依赖 Tokio、Hyper 或网络套接字。表单/查询拒绝构造器随各自提取器特性启用。`axum-macros` 是独立宿主过程宏，其生成路径使用 `http_routing`。内核禁止 unsafe。

## 路由与处理器

当前包名为 `http-routing`，Rust 路径为 `http_routing`。默认特性包含原生 HTTP/1 服务与常用提取器；要使用无运行时的最小子集，应显式设置 `default-features = false`。基本路由、`State`、`Path` 和字符串响应不需要可选的 JSON/查询能力：

```rust
use http_routing::{extract::{Path, State}, routing::get, Router};

async fn item(State(prefix): State<String>, Path(id): Path<u64>) -> String {
    format!("{prefix}:{id}")
}

fn routes() -> Router {
    Router::new()
        .route("/items/{id}", get(item))
        .with_state(String::from("storage"))
}
```

路径参数使用 `/{id}`，捕获剩余路径使用 `/{*path}`，不是旧式 `/:id` 或 `/*path`。`Router<S>` 中 `S` 表示仍待提供的 state，`.with_state(value)` 后才能作为完整请求服务使用。状态需要廉价共享时用 `Arc`，不要在异步处理器里持有跨 `.await` 的阻塞锁。

处理器参数从左到右提取，消耗正文的 `Json<T>`、`String`、`Bytes` 或 `Request` 必须放在最后，且正文只能消费一次。`Path`、`State`、`Query`、请求头等 parts 提取器放在它之前；查询、表单与 JSON 分别需要 `query`、`form`、`json` 特性。数据类型使用 [data-serialization](data-serialization.md) 的派生与 `#[serialization(...)]` 属性。

监听服务需要 `tokio,http1`，在活动 Tokio 运行时中调用 `http_routing::serve(listener, routes()).await`。优雅停机使用 `.with_graceful_shutdown(future)`；需要对端地址时，将 router 转成 `.into_make_service_with_connect_info::<SocketAddr>()`，再在处理器中提取 `ConnectInfo<SocketAddr>`。真实入口见 [storage-server](../../src/server/src/lib.rs)。

## 正文限制与错误边界

默认的 2 MiB 限制作用于 `Bytes` 及基于它的 `String`、`Json`、`Form` 提取器，不是对任意 `Body` 消费方式的全局限制。使用 `DefaultBodyLimit::max(bytes)` 配置路由/请求；直接读取流时仍须自行限量，且 `Content-Length` 不能替代对实际数据的计数。

本产品 router 关闭通用默认值，然后由 [HTTP 边界实现](../../src/server/src/http_boundary.rs)按操作显式设置大小与正文期限，并将拒绝转成 `ApiError`。新增处理器应复用该边界，不应只复制 `.layer(DefaultBodyLimit::disable())`。`IntoResponse` 可返回状态、头和正文；原生提取拒绝默认不保证符合产品 JSON 错误格式。正文开始发送后的流错误也不能再改写成 JSON 错误信封。

中间件组合见 [service-stack](service-stack.md)。`Router::layer` 只作用于调用时已有路由，后加路由不会自动继承；授权常使用 `route_layer` 包住已匹配路由，以免把正常 404 改成鉴权错误。

<a id="transport-integration-boundary"></a>

## 传输集成边界

`http1` 只启用协议引擎，`tokio` 启用运行时/服务端适配器。仅 HTTP/1 配置不引入 Tokio 网络、运行时或时间特性。应用路由与协议接收保持各自正文类型；就绪感知中间件通过显式服务适配器接入。

服务端安装 `TokioTimer` 并提供 30 秒请求头期限；协议探测把连接交给 HTTP/1 解析器时不重启计时。正文与整个操作的期限由应用层负责，见[HTTP 传输](http-transport.md)。

## 不支持的能力

不提供 WebSocket、multipart 提取器或 HTTP/2。Cargo 拒绝 `ws`、`multipart`、`__private`、`__private_docs` 和 `http2`，不接受空兼容特性。

HTTP/1.1 通用协议升级由 http-transport 提供，包括 101 交换、已缓冲字节及双向 I/O；这不代表 WebSocket 握手/帧实现。

## 检查与来源

运行 `python3 .github/scripts/check-http-routing.py`，检查原生全部声明特性、WASM 无运行时子集及不支持能力的拒绝，并测试路径/查询路由、404/405、正文/状态、提取器拒绝和 derive/debug-handler 宏。

来源为 [Axum 0.8.9](../upstream/axum-0.8.9/README.md)、[axum-core 0.5.6](../upstream/axum-core-0.5.6/README.md)；原始包名和可选能力不等同于当前公共 API。原始版本、许可证、提交、不可变源码和测试归属保存在合规材料中。`src/extract/ws.rs`、`src/extract/multipart.rs`、`src/test_helpers/mod.rs` 和 `src/test_helpers/test_client.rs` 是受哈希检查保护的未启用源码，不是受支持 API。`test/foundation/http-routing` 中未接入的上游测试也不计为执行。
