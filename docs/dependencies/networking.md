# 网络与协议文档

[依赖文档入口](README.md)

HTTP、路由、中间件、URI、套接字与 TLS。本页包含 25 个当前实现归属、37 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## axum-macros

[当前源码 / API](../../src/foundation/networking/axum-macros/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/axum-macros/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| axum-macros@0.5.1 | [README.md](../../docs/upstream/axum-macros-0.5.1/README.md) | [许可与通知](../../compliance/licenses/upstream/axum-macros-0.5.1)；[来源](../../compliance/provenance/packages/axum-macros-0.5.1/Cargo.toml.orig.txt) |

## email_address

[当前源码 / API](../../src/foundation/networking/email_address/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/email_address/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| email_address@0.2.9 | [README.md](../../docs/upstream/email_address-0.2.9/README.md) | [许可与通知](../../compliance/licenses/upstream/email_address-0.2.9)；[来源](../../compliance/provenance/packages/email_address-0.2.9/Cargo.toml.orig.txt) |

## fluent-uri

[当前源码 / API](../../src/foundation/networking/fluent-uri/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/fluent-uri/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| fluent-uri@0.4.1 | [README.md](../../docs/upstream/fluent-uri-0.4.1/README.md) | [许可与通知](../../compliance/licenses/upstream/fluent-uri-0.4.1)；[来源](../../compliance/provenance/packages/fluent-uri-0.4.1/Cargo.toml.orig.txt) |

## http

[当前源码 / API](../../src/foundation/networking/http/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/http/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| http@1.5.0 | [README.md](../../docs/upstream/http-1.5.0/README.md) | [许可与通知](../../compliance/licenses/upstream/http-1.5.0)；[来源](../../compliance/provenance/packages/http-1.5.0/Cargo.toml.orig.txt) |

## http-body

[当前源码 / API](../../src/foundation/networking/http-body/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/http-body/Cargo.toml) · [当前维护说明](../../docs/components/http-body.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| http-body@1.1.0 | [README.md](../../docs/upstream/http-body-1.1.0/README.md) | [许可与通知](../../compliance/licenses/upstream/http-body-1.1.0)；[来源](../../compliance/provenance/packages/http-body-1.1.0/Cargo.toml.orig.txt) |
| http-body-util@0.1.5（合并来源） | [README.md](../../docs/upstream/http-body-util-0.1.5/README.md) | [许可与通知](../../compliance/licenses/upstream/http-body-util-0.1.5)；[来源](../../compliance/provenance/packages/http-body-util-0.1.5/Cargo.toml.orig.txt) |

## http-range-header

[当前源码 / API](../../src/foundation/networking/http-range-header/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/http-range-header/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| http-range-header@0.4.2 | [README.md](../../docs/upstream/http-range-header-0.4.2/README.md) | [许可与通知](../../compliance/licenses/upstream/http-range-header-0.4.2)；[来源](../../compliance/provenance/packages/http-range-header-0.4.2/Cargo.toml.orig.txt) |

## http-routing

[当前源码 / API](../../src/foundation/networking/http-routing/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/http-routing/Cargo.toml) · [当前维护说明](../../docs/components/http-routing.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| axum@0.8.9 | [README.md](../../docs/upstream/axum-0.8.9/README.md)；[src/docs/extract.md](../../docs/upstream/axum-0.8.9/src/docs/extract.md)；[src/docs/response.md](../../docs/upstream/axum-0.8.9/src/docs/response.md)；[src/docs/middleware.md](../../docs/upstream/axum-0.8.9/src/docs/middleware.md)；[src/docs/error_handling.md](../../docs/upstream/axum-0.8.9/src/docs/error_handling.md) | [许可与通知](../../compliance/licenses/upstream/axum-0.8.9)；[来源](../../compliance/provenance/packages/axum-0.8.9/Cargo.toml.orig.txt) |
| axum-core@0.5.6（合并来源） | [README.md](../../docs/upstream/axum-core-0.5.6/README.md) | [许可与通知](../../compliance/licenses/upstream/axum-core-0.5.6)；[来源](../../compliance/provenance/packages/axum-core-0.5.6/Cargo.toml.orig.txt) |

## http-transport

[当前源码 / API](../../src/foundation/networking/http-transport/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/http-transport/Cargo.toml) · [当前维护说明](../../docs/components/http-transport.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| hyper@1.12.0 | [README.md](../../docs/upstream/hyper-1.12.0/README.md) | [许可与通知](../../compliance/licenses/upstream/hyper-1.12.0)；[来源](../../compliance/provenance/packages/hyper-1.12.0/Cargo.toml.orig.txt) |
| atomic-waker@1.1.2（合并来源） | [README.md](../../docs/upstream/atomic-waker-1.1.2/README.md) | [许可与通知](../../compliance/licenses/upstream/atomic-waker-1.1.2)；[来源](../../compliance/provenance/packages/atomic-waker-1.1.2/Cargo.toml.orig.txt) |
| hyper-util@0.1.21（合并来源） | [README.md](../../docs/upstream/hyper-util-0.1.21/README.md) | [许可与通知](../../compliance/licenses/upstream/hyper-util-0.1.21)；[来源](../../compliance/provenance/packages/hyper-util-0.1.21/Cargo.toml.orig.txt) |
| try-lock@0.2.5（合并来源） | [README.md](../../docs/upstream/try-lock-0.2.5/README.md) | [许可与通知](../../compliance/licenses/upstream/try-lock-0.2.5)；[来源](../../compliance/provenance/packages/try-lock-0.2.5/Cargo.toml.orig.txt) |
| want@0.3.2（合并来源） | [README.md](../../docs/upstream/want-0.3.2/README.md) | [许可与通知](../../compliance/licenses/upstream/want-0.3.2)；[来源](../../compliance/provenance/packages/want-0.3.2/Cargo.toml.orig.txt) |

## httparse

[当前源码 / API](../../src/foundation/networking/httparse/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/httparse/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| httparse@1.10.1 | [README.md](../../docs/upstream/httparse-1.10.1/README.md) | [许可与通知](../../compliance/licenses/upstream/httparse-1.10.1)；[来源](../../compliance/provenance/packages/httparse-1.10.1/Cargo.toml.orig.txt) |

## httpdate

[当前源码 / API](../../src/foundation/networking/httpdate/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/httpdate/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| httpdate@1.0.3 | [README.md](../../docs/upstream/httpdate-1.0.3/README.md) | [许可与通知](../../compliance/licenses/upstream/httpdate-1.0.3)；[来源](../../compliance/provenance/packages/httpdate-1.0.3/Cargo.toml.orig.txt) |

## hyper-rustls

[当前源码 / API](../../src/foundation/networking/hyper-rustls/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/hyper-rustls/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| hyper-rustls@0.27.10 | [README.md](../../docs/upstream/hyper-rustls-0.27.10/README.md) | [许可与通知](../../compliance/licenses/upstream/hyper-rustls-0.27.10)；[来源](../../compliance/provenance/packages/hyper-rustls-0.27.10/Cargo.toml.orig.txt) |

## ipnet

[当前源码 / API](../../src/foundation/networking/ipnet/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/ipnet/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| ipnet@2.12.2 | [README.md](../../docs/upstream/ipnet-2.12.2/README.md) | [许可与通知](../../compliance/licenses/upstream/ipnet-2.12.2)；[来源](../../compliance/provenance/packages/ipnet-2.12.2/Cargo.toml.orig.txt) |

## matchit

[当前源码 / API](../../src/foundation/networking/matchit/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/matchit/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| matchit@0.8.4 | [README.md](../../docs/upstream/matchit-0.8.4/README.md) | [许可与通知](../../compliance/licenses/upstream/matchit-0.8.4)；[来源](../../compliance/provenance/packages/matchit-0.8.4/Cargo.toml.orig.txt) |

## mime

[当前源码 / API](../../src/foundation/networking/mime/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/mime/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| mime@0.3.17 | [README.md](../../docs/upstream/mime-0.3.17/README.md) | [许可与通知](../../compliance/licenses/upstream/mime-0.3.17)；[来源](../../compliance/provenance/packages/mime-0.3.17/Cargo.toml.orig.txt) |

## mime_guess

[当前源码 / API](../../src/foundation/networking/mime_guess/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/mime_guess/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| mime_guess@2.0.5 | [README.md](../../docs/upstream/mime_guess-2.0.5/README.md) | [许可与通知](../../compliance/licenses/upstream/mime_guess-2.0.5)；[来源](../../compliance/provenance/packages/mime_guess-2.0.5/Cargo.toml.orig.txt) |

## reqwest

[当前源码 / API](../../src/foundation/networking/reqwest/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/reqwest/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| reqwest@0.12.28 | [README.md](../../docs/upstream/reqwest-0.12.28/README.md) | [许可与通知](../../compliance/licenses/upstream/reqwest-0.12.28)；[来源](../../compliance/provenance/packages/reqwest-0.12.28/Cargo.toml.orig.txt) |

## rustls

[当前源码 / API](../../src/foundation/networking/rustls/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/rustls/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustls@0.23.46 | [README.md](../../docs/upstream/rustls-0.23.46/README.md) | [许可与通知](../../compliance/licenses/upstream/rustls-0.23.46)；[来源](../../compliance/provenance/packages/rustls-0.23.46/Cargo.toml.orig.txt) |

## rustls-pki-types

[当前源码 / API](../../src/foundation/networking/rustls-pki-types/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/rustls-pki-types/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustls-pki-types@1.15.1 | [README.md](../../docs/upstream/rustls-pki-types-1.15.1/README.md) | [许可与通知](../../compliance/licenses/upstream/rustls-pki-types-1.15.1)；[来源](../../compliance/provenance/packages/rustls-pki-types-1.15.1/Cargo.toml.orig.txt) |

## rustls-webpki

[当前源码 / API](../../src/foundation/networking/rustls-webpki/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/rustls-webpki/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustls-webpki@0.103.16 | [README.md](../../docs/upstream/rustls-webpki-0.103.16/README.md) | [许可与通知](../../compliance/licenses/upstream/rustls-webpki-0.103.16)；[来源](../../compliance/provenance/packages/rustls-webpki-0.103.16/Cargo.toml.orig.txt) |

## service-stack

[当前源码 / API](../../src/foundation/networking/service-stack/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/service-stack/Cargo.toml) · [当前维护说明](../../docs/components/service-stack.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tower@0.5.3 | [README.md](../../docs/upstream/tower-0.5.3/README.md) | [许可与通知](../../compliance/licenses/upstream/tower-0.5.3)；[来源](../../compliance/provenance/packages/tower-0.5.3/Cargo.toml.orig.txt) |
| tower-layer@0.3.3（合并来源） | [README.md](../../docs/upstream/tower-layer-0.3.3/README.md) | [许可与通知](../../compliance/licenses/upstream/tower-layer-0.3.3)；[来源](../../compliance/provenance/packages/tower-layer-0.3.3/Cargo.toml.orig.txt) |
| tower-service@0.3.3（合并来源） | [README.md](../../docs/upstream/tower-service-0.3.3/README.md) | [许可与通知](../../compliance/licenses/upstream/tower-service-0.3.3)；[来源](../../compliance/provenance/packages/tower-service-0.3.3/Cargo.toml.orig.txt) |

## socket2

[当前源码 / API](../../src/foundation/networking/socket2/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/socket2/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| socket2@0.6.5 | [README.md](../../docs/upstream/socket2-0.6.5/README.md) | [许可与通知](../../compliance/licenses/upstream/socket2-0.6.5)；[来源](../../compliance/provenance/packages/socket2-0.6.5/Cargo.toml.orig.txt) |

## tokio-rustls

[当前源码 / API](../../src/foundation/networking/tokio-rustls/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/tokio-rustls/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tokio-rustls@0.26.6 | [README.md](../../docs/upstream/tokio-rustls-0.26.6/README.md) | [许可与通知](../../compliance/licenses/upstream/tokio-rustls-0.26.6)；[来源](../../compliance/provenance/packages/tokio-rustls-0.26.6/Cargo.toml.orig.txt) |

## tower-http

[当前源码 / API](../../src/foundation/networking/tower-http/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/tower-http/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tower-http@0.6.11 | [README.md](../../docs/upstream/tower-http-0.6.11/README.md) | [许可与通知](../../compliance/licenses/upstream/tower-http-0.6.11)；[来源](../../compliance/provenance/packages/tower-http-0.6.11/Cargo.toml.orig.txt) |

## url-encoding

[当前源码 / API](../../src/foundation/networking/url-encoding/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/url-encoding/Cargo.toml) · [当前维护说明](../../docs/architecture/url-encoding.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| url@2.5.8 | [README.md](../../docs/upstream/url-2.5.8/README.md) | [许可与通知](../../compliance/licenses/upstream/url-2.5.8)；[来源](../../compliance/provenance/packages/url-2.5.8/Cargo.toml.orig.txt) |
| form_urlencoded@1.2.2（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/networking/url-encoding/src/form/mod.rs) | [许可与通知](../../compliance/licenses/upstream/form_urlencoded-1.2.2)；[来源](../../compliance/provenance/packages/form_urlencoded-1.2.2/Cargo.toml.orig.txt) |
| idna@1.1.0（合并来源） | [README.md](../../docs/upstream/idna-1.1.0/README.md) | [许可与通知](../../compliance/licenses/upstream/idna-1.1.0)；[来源](../../compliance/provenance/packages/idna-1.1.0/Cargo.toml.orig.txt) |
| idna_adapter@1.2.2（合并来源） | [README.md](../../docs/upstream/idna_adapter-1.2.2/README.md) | [许可与通知](../../compliance/licenses/upstream/idna_adapter-1.2.2)；[来源](../../compliance/provenance/packages/idna_adapter-1.2.2/Cargo.toml.orig.txt) |
| percent-encoding@2.3.2（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/networking/url-encoding/src/percent/mod.rs) | [许可与通知](../../compliance/licenses/upstream/percent-encoding-2.3.2)；[来源](../../compliance/provenance/packages/percent-encoding-2.3.2/Cargo.toml.orig.txt) |

## webpki-roots

[当前源码 / API](../../src/foundation/networking/webpki-roots/src/lib.rs) · [Cargo 清单](../../src/foundation/networking/webpki-roots/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| webpki-roots@1.0.10 | [README.md](../../docs/upstream/webpki-roots-1.0.10/README.md) | [许可与通知](../../compliance/licenses/upstream/webpki-roots-1.0.10)；[来源](../../compliance/provenance/packages/webpki-roots-1.0.10/Cargo.toml.orig.txt) |
