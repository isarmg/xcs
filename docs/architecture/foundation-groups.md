# Foundation 职能目录

Foundation 的 219 个 crate 按主要职能放入 12 个父目录。此迁移只改变物理目录和必要的路径引用，不合并或减少 crate，不修改 Cargo 包名、版本、功能开关、对外 API、依赖身份或原始许可证。

## 目录与边界

| 目录 | 职能 | crate 数 |
| --- | --- | --- |
| `src/foundation/concurrency/` | 并发与异步执行 | 25 |
| `src/foundation/collections/` | 容器与所有权 | 36 |
| `src/foundation/cryptography/` | 密码学与安全基础 | 18 |
| `src/foundation/data-storage/` | 数据格式与存储 | 14 |
| `src/foundation/networking/` | 网络协议与服务 | 25 |
| `src/foundation/platform/` | 系统与平台接口 | 9 |
| `src/foundation/text-unicode/` | 文本与 Unicode | 22 |
| `src/foundation/code-generation/` | 通用代码生成 | 14 |
| `src/foundation/tooling/` | 构建与命令行工具 | 14 |
| `src/foundation/diagnostics/` | 错误与诊断 | 11 |
| `src/foundation/core-types/` | 基础值类型 | 15 |
| `src/foundation/web-runtime/` | 浏览器与 WebAssembly | 16 |

每个 crate 恰好归入一个目录，归属以它提供的能力为准，不以调用它的业务模块为准。领域专用的宏与其运行时放在同组，例如 Tokio、Serde、HTTP 与 WASM 的配套宏；可复用的通用宏工具放在 `code-generation`。

`src/interface`、`src/naming`、`src/macros` 与 `tools/web-bindings` 保持原位。它们的上游整合身份继续单独记录，不因引用 Foundation 而一起搬动。`test/foundation/<crate>` 保持原路径，源代码中的外部测试引用相应调整。

## 各组包清单

### concurrency

执行器、任务调度、线程同步、通道、线程本地状态及 Future 支持。

`async-notification`, `async-trait`, `concurrent-collections`, `flume`, `futures`, `futures-intrusive`, `futures-macro`, `guardian`, `lazy_static`, `mio`, `once_cell`, `or_poisoned`, `parking`, `pin-project-lite`, `rayon`, `rayon-core`, `send_wrapper`, `signal-hook-registry`, `spin`, `sync_wrapper`, `synchronization`, `task-dispatch`, `thread_local`, `tokio`, `tokio-macros`。

### collections

分配器、集合、迭代器、智能指针、借用与零拷贝数据结构；不按使用这些容器的业务分类。

`allocator-api2`, `arrayvec`, `bit-set`, `bit-vec`, `borrow-or-share`, `bumpalo`, `bytes`, `equivalent`, `erased`, `fallible-iterator`, `hash-table`, `hashlink`, `id-arena`, `indexmap`, `iterator-toolkit`, `litemap`, `micromap`, `next_tuple`, `ref-cast`, `ref-cast-impl`, `scopeguard`, `sharded-slab`, `shared-value`, `slab`, `slotmap`, `smallvec`, `stable_deref_trait`, `tinyvec`, `yoke`, `yoke-derive`, `zerocopy`, `zerofrom`, `zerofrom-derive`, `zerotrie`, `zerovec`, `zerovec-derive`。

### cryptography

哈希、认证、密码散列、安全熵、恒定时间工具与密码学格式。

`argon2`, `base64ct`, `blake2`, `blake3`, `cmov`, `const-oid`, `constant_time_eq`, `cryptographic-core`, `ctutils`, `hmac`, `password-hash`, `random-generator-core`, `ring`, `sha2`, `subtle`, `system-entropy`, `untrusted`, `zeroize`。

### data-storage

序列化、编码、压缩、Schema 验证及 SQLite 持久化。

`base64`, `compression`, `crc32fast`, `data-encoding`, `data-serialization`, `form-codec`, `json-codec`, `leb128`, `leb128fmt`, `libsqlite3-sys`, `schema-validation`, `serialization-derive`, `sqlite-access`, `toml`。

### networking

HTTP 请求/响应、路由、中间件、客户端、URI/地址格式、套接字、TLS 与证书。

`axum-macros`, `email_address`, `fluent-uri`, `http`, `http-body`, `http-range-header`, `http-routing`, `http-transport`, `httparse`, `httpdate`, `hyper-rustls`, `ipnet`, `matchit`, `mime`, `mime_guess`, `reqwest`, `rustls`, `rustls-pki-types`, `rustls-webpki`, `service-stack`, `socket2`, `tokio-rustls`, `tower-http`, `url-encoding`, `webpki-roots`。

### platform

系统调用、动态库加载、CPU 能力、文件系统工具与系统信息。

`cpu-feature-detect`, `errno`, `glob`, `libc`, `libloading`, `linux-raw-sys`, `rustix`, `sysinfo`, `tempfile`。

### text-unicode

字符串检索、正则、字符分类、Unicode 规范化、区域数据及文本格式化。

`bytecount`, `const_str_slice_concat`, `convert_case`, `fancy-regex`, `heck`, `html-escape`, `icu_collections`, `icu_locale_core`, `icu_provider`, `memchr`, `potential_utf`, `regex`, `tinystr`, `unicase`, `unicode-general-category`, `unicode-ident`, `unicode-normalizer`, `unicode-properties`, `unicode-segmentation`, `utf8_iter`, `winnow`, `writeable`。

### code-generation

Rust 语法与 Token、通用派生宏、类型安全构建器以及 Rust/XML 模板解析。

`derive-where`, `displaydoc`, `pastey`, `proc-macro2`, `proc-macro2-diagnostics`, `quote`, `rstml`, `rust-source-format`, `rust-syntax`, `strum`, `strum_macros`, `synstructure`, `typed-builder`, `typed-builder-macro`。

### tooling

编译器能力探测、C 绑定生成、原生构建、命令行参数、Shell 分词和终端样式。

`autocfg`, `bindgen`, `cc`, `cfg-if`, `clang-sys`, `command-derive`, `command-line`, `nu-ansi-term`, `pkg-config`, `rustversion`, `semver`, `shell-tokenizer`, `terminal-style`, `version_check`。

### diagnostics

错误类型、日志、Panic 报告、符号/DWARF 调试信息及跟踪插桩。

`anyhow`, `console_error_panic_hook`, `error-definition`, `error-definition-macros`, `gimli`, `log`, `logging`, `rustc-demangle`, `throw_error`, `tracing`, `tracing-attributes`。

### core-types

数值、日期时间、标识符、标志位、通用求和类型、非安全伪随机数及非密码学哈希。

`ahash`, `date-time`, `either`, `fastrand`, `flag-set`, `foldhash`, `fraction`, `hybrid-array`, `itoa`, `num-cmp`, `numeric`, `rustc-hash`, `typenum`, `uuid`, `zmij`。

### web-runtime

JS/DOM 互操作、响应式界面、视图树、WASM 绑定及二进制转换。

`dom-helpers`, `gloo-events`, `gloo-timers`, `js-sys`, `reactive-state`, `view-alternatives`, `view-renderer`, `walrus`, `walrus-macro`, `wasm-bindgen`, `wasm-bindgen-cli-support`, `wasm-bindgen-shared`, `wasm-binding-macros`, `wasm-encoder`, `wasmparser`, `web-sys`。

## 来源与验证

当前目录与历史来源目录是两种不同的标识。当前源码清单登记分组后的路径；所有历史来源名称、来源散列、旧迁移收据和原始许可证保持不变。新的显式路径迁移收据逐文件记录一一对应关系，并对必要的路径修改进行可逆校验。

完整性检查不依赖旧目录的兼容副本或软链接。历史测试证明从真实存在的新路径读取源码，只反向恢复已登记的路径编辑后校验原始散列；文件丢失、多出或实现字节漂移仍然失败。

相关文档：[消费者与功能索引](foundation-consumers.md)、[源码命名](source-naming.md)、[源码清单](../THIRD_PARTY_RUST_SOURCES.md)、[CI 分组](foundation-ci.md)。

## 分类后的维护边界

常规源码修改仍通过 `manage.py refresh` 更新当前摘要，再由 `verify --metadata` 验证。
独立、未重命名、未合并、未裁剪且不受固定平台/密码学证明保护的包，继续支持显式
`--retire-plan` 退役和 `--move-plan` 包内源码文件移动；多次移动和移动后退役也须逐次
保留旧收据。涉及本次分类已登记路径编辑的文件移动（作为起点或终点）仍须专门的
后继反向校验适配器，当前明确拒绝。退役外部测试仍要求精确的最后清单和明确移除处置。分类安装时根锁文件
必须保持不变，但后续合法维护可以在常规清单约束下更新它。

这不是对全部旧生命周期工作流的无条件兼容承诺。分类后的重命名、合并、上游升级、
源码裁剪、外部测试文件移动，以及共享来源或固定证明保护包的上述生命周期变更，
目前须先实现并评审独立的后继链适配器。命令会明确拒绝这些未支持的计划，不会通过
改写旧身份、重算历史摘要、跳过验证或留下兼容目录来继续执行。
