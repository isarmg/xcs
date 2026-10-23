<a id="http-routing"></a>

# HTTP 路由

`http-routing` 统一维护路由、提取器、请求扩展、响应转换和 HTTP 正文类型；这些能力此前分属 Axum 与 axum-core。内核只是内部组织边界，不是第二个 Cargo 包或重复的 trait 实现。

默认特性保留现有应用配置。禁用默认特性时，路由与内核逻辑可在 Linux amd64 和浏览器 WASM 上编译，不依赖 Tokio、Hyper 或网络套接字。HTTP 服务端/运行时集成仍通过 `tokio` 和 `http1` 显式启用。表单和查询拒绝结果的构造函数现随各自提取器特性启用，不再在合并 crate 中留下对应特性未启用时无用的构造函数。

`axum-macros` 仍是独立的宿主过程宏包。其生成路径现指向 `http_routing`，不保留旧 crate 别名。内核原有的 unsafe 禁令被显式保留。未重写 HTTP 解析器、TLS、路由语义或响应状态/正文行为。

`.github/scripts/check-http-routing.py` 执行十一项原生/WASM 检查，包括全部剩余声明特性的组合及完整的不依赖运行时的 WASM 子集，并执行五项维护中的内存行为用例：路径/查询路由、404/405 区分、请求/响应正文与状态、提取器拒绝以及真实的 derive/debug-handler 宏展开。无需绑定套接字或伪造测试提供方。历史上游测试仍逐项保留来源，不会被误报为全部执行过。

原始来源为 Axum 0.8.9 和 axum-core 0.5.6。精确版本、许可证、上游提交、不可变原件及迁移后的测试身份仍保存在来源记录、目录及整合凭据中。上游宏包的原始身份也在那里保留。

<a id="transport-integration-boundary"></a>

## 传输集成边界

`http1` 特性仅启用协议引擎。运行时/服务端适配器由 `tokio` 启用，而不是由可选传输依赖本身启用。维护门禁验证仅含 HTTP/1 的原生及 WASM 依赖图，确保不含传输适配器或 Tokio 网络/运行时/时间特性。应用路由与协议接收保留各自的正文类型；感知就绪状态的中间件通过显式适配接入，不作为协议服务 trait 的别名。维护中的服务端安装 `TokioTimer`，启用传输层原有的 30 秒请求头期限，并涵盖协议探测。探测将连接交给 HTTP/1 解析器时不会重启超时计时。

<a id="capability-audit-and-inactive-source-evidence"></a>

## 能力审计与未启用源码证据

维护中的组件不提供 WebSocket 或 multipart 提取器。Cargo 会拒绝 `ws`、`multipart`、`__private`、`__private_docs` 和 `http2`，不会将其作为空兼容特性接受。产品清单没有启用这些不可用特性。应用默认特性集保持不变。

编译此前声明的可选特性暴露出缺失的提供方：`ws` 导入 `sha1`、`tokio_tungstenite` 和 `futures::sink::SinkExt`；`multipart` 需要 `multer`；旧 `__private` 测试客户端 API 需要不可用的 `reqwest::multipart` 接口；`__private_docs` 通过宽泛的 `tower-http` 特性启用不可用的 `async_compression`。因此移除不受支持的特性声明与活动导出，而不是新增替代引擎或宣称未执行的行为。仅关联这些能力的可选依赖声明也已移除。

现有四个文件 `src/extract/ws.rs`、`src/extract/multipart.rs`、`src/test_helpers/mod.rs` 和 `src/test_helpers/test_client.rs` 仍位于 `http-routing` 内的原位置，字节完全不变，包括内嵌声明。它们是未启用的历史源码证据，不是已编译模块或受支持 API。未删除生产源码，也未新增重复归档。门禁检查全部四个精确哈希，并拒绝重新出现的特性声明、仅用于未启用能力的依赖及模块导出。`test/foundation/http-routing/` 中现有历史测试仍是源码证据，不是一套已通过的测试。维护中的路由门禁现会在原生目标上编译所有声明特性、为浏览器 WASM 编译不依赖运行时的子集，并拒绝每项已移除能力。

HTTP/1.1 通用协议升级支持仍在 `http-transport` 中。其真实内存双工测试覆盖 101 交换、跨升级边界的已缓冲字节、双向数据、EOF，以及 HTTP/2.0 线协议请求到达服务前的拒绝。这些是传输测试，不是 WebSocket 握手/帧实现。不能据此推断 WebSocket 支持或性能提升；规范 HTTP 版本常量保持不变。
