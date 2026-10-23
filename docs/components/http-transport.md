<a id="http-transport"></a>

# HTTP 传输

`http-transport` 提供 Hyper 协议引擎和共用协议类型的 `adapters`。客户端/服务端、运行时、连接池、代理及服务适配器均由显式特性控制，基础协议配置不启用 Tokio 网络或运行时。

<a id="maintained-scope"></a>

## 维护范围

维护 HTTP/1，不提供 HTTP/2；Cargo 拒绝 `http2`。`full` 是受支持 HTTP/1 与适配器能力的并集，不提供 `adapters-full` 别名。C ABI 的 HTTP/2 选项入口返回 `HYPERE_FEATURE_NOT_ENABLED` 且不消耗选项。

HTTP/1.1 通用升级支持 101 响应、已缓冲字节保留、后续双向 I/O 及正常 EOF，不包含 WebSocket 握手或帧。解析器须在应用服务执行前拒绝不支持的 HTTP/2.0 线协议请求。就绪感知适配器不得在服务就绪前调用它。

<a id="header-deadlines-and-protocol-detection"></a>

## 请求头期限与协议探测

自动服务端检测并拒绝分片 HTTP/2 前言，且不发送 HTTP/1 响应。首次轮询开始首个请求头期限，协议探测与 HTTP/1 解析共用同一个计时器；keepalive 后续请求头取得新期限。已缓冲升级字节、协议错误和优雅关闭不改变此边界。

路由服务端安装 Tokio 计时器并默认启用 30 秒请求头期限。通用消费者未提供计时器时没有默认期限；显式配置超时必须提供计时器，也可显式禁用超时。请求头期限不替代应用正文或操作期限。

<a id="private-dispatch-and-wakeup-ownership"></a>

## 私有分派与唤醒

`src/common/{demand,try_lock,task_waker}.rs` 分别维护 want 0.3.2、try-lock 0.2.5 和 atomic-waker 1.1.2 的实现。`demand` 及锁仅随 `client` 编译，`task_waker` 仅随 `http1` 编译；均非公共 API。demand 禁止 unsafe；锁和唤醒器各自隔离其 unsafe 实现及原子顺序。

<a id="validation"></a>

## 检查

运行 `python3 .github/scripts/check-http-transport.py`。门禁检查原生/WASM 特性配置、显式 tracing/FFI、已停用能力拒绝及编译警告；内存双工测试覆盖：

- HTTP/1 握手、正文/状态/请求头、畸形请求、服务错误、路由与连接池复用。
- 服务身份及就绪状态、通用升级、不支持版本/ALPN 拒绝、自持有连接和优雅关闭。
- FFI 选项分配、设置、释放及不支持的 HTTP/2 入口。
- 分片协议探测、首次计时、期限交接和 keepalive；正文传输不被请求头期限截断。
- 私有分派、取消、发送/接收方丢弃、锁互斥与展开释放、唤醒器替换及并发注册/通知。

上述测试不等于真实监听套接字、浏览器运行时或吞吐量验收。未启用的上游测试、示例和基准不计为执行。

<a id="provenance"></a>

## 来源

主来源为 Hyper 1.12.0 和 HyperUtil 0.1.21；分派/唤醒来源分别登记。原始版本、输入哈希、许可证、源码、基准和测试路径由不可变来源目录及整合凭据维护，包括未启用 HTTP/2 实现的逐字节归档。产品原生支持范围仍仅为 Linux amd64。
