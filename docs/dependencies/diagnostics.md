# 诊断与追踪文档

[依赖文档入口](README.md)

错误、日志、调试信息与追踪。本页包含 11 个当前实现归属、16 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## anyhow

[当前源码 / API](../../src/foundation/diagnostics/anyhow/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/anyhow/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| anyhow@1.0.104 | [README.md](../../docs/upstream/anyhow-1.0.104/README.md) | [许可与通知](../../compliance/licenses/upstream/anyhow-1.0.104)；[来源](../../compliance/provenance/packages/anyhow-1.0.104/Cargo.toml.orig.txt) |

## console_error_panic_hook

[当前源码 / API](../../src/foundation/diagnostics/console_error_panic_hook/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/console_error_panic_hook/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| console_error_panic_hook@0.1.7 | [README.md](../../docs/upstream/console_error_panic_hook-0.1.7/README.md)；[README.tpl](../../docs/upstream/console_error_panic_hook-0.1.7/README.tpl) | [许可与通知](../../compliance/licenses/upstream/console_error_panic_hook-0.1.7)；[来源](../../compliance/provenance/packages/console_error_panic_hook-0.1.7/Cargo.toml.orig.txt) |

## error-definition

[当前源码 / API](../../src/foundation/diagnostics/error-definition/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/error-definition/Cargo.toml) · [当前维护说明](../../docs/architecture/error-definition-naming.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| thiserror@2.0.21 | [README.md](../../docs/upstream/thiserror-2.0.21/README.md) | [许可与通知](../../compliance/licenses/upstream/thiserror-2.0.21)；[来源](../../compliance/provenance/packages/thiserror-2.0.21/Cargo.toml.orig.txt) |

## error-definition-macros

[当前源码 / API](../../src/foundation/diagnostics/error-definition-macros/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/error-definition-macros/Cargo.toml) · [当前维护说明](../../docs/architecture/error-definition-naming.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| thiserror-impl@2.0.21 | 未附独立文档或 Rustdoc；[内部实现源码](../../src/foundation/diagnostics/error-definition-macros/src/lib.rs)，按配套 [thiserror 使用文档](../../docs/upstream/thiserror-2.0.21/README.md) 阅读 | [许可与通知](../../compliance/licenses/upstream/thiserror-impl-2.0.21)；[来源](../../compliance/provenance/packages/thiserror-impl-2.0.21/Cargo.toml.orig.txt) |

## gimli

[当前源码 / API](../../src/foundation/diagnostics/gimli/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/gimli/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| gimli@0.32.3 | [README.md](../../docs/upstream/gimli-0.32.3/README.md) | [许可与通知](../../compliance/licenses/upstream/gimli-0.32.3)；[来源](../../compliance/provenance/packages/gimli-0.32.3/Cargo.toml.orig.txt) |

## log

[当前源码 / API](../../src/foundation/diagnostics/log/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/log/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| log@0.4.34 | [README.md](../../docs/upstream/log-0.4.34/README.md) | [许可与通知](../../compliance/licenses/upstream/log-0.4.34)；[来源](../../compliance/provenance/packages/log-0.4.34/Cargo.toml.orig.txt) |

## logging

[当前源码 / API](../../src/foundation/diagnostics/logging/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/logging/Cargo.toml) · [当前维护说明](../../docs/components/logging.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| env_logger@0.11.11 | [README.md](../../docs/upstream/env_logger-0.11.11/README.md) | [许可与通知](../../compliance/licenses/upstream/env_logger-0.11.11)；[来源](../../compliance/provenance/packages/env_logger-0.11.11/Cargo.toml.orig.txt) |
| env_filter@2.0.0（合并来源） | [README.md](../../docs/upstream/env_filter-2.0.0/README.md) | [许可与通知](../../compliance/licenses/upstream/env_filter-2.0.0)；[来源](../../compliance/provenance/packages/env_filter-2.0.0/Cargo.toml.orig.txt) |

## rustc-demangle

[当前源码 / API](../../src/foundation/diagnostics/rustc-demangle/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/rustc-demangle/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustc-demangle@0.1.28 | [README.md](../../docs/upstream/rustc-demangle-0.1.28/README.md) | [许可与通知](../../compliance/licenses/upstream/rustc-demangle-0.1.28)；[来源](../../compliance/provenance/packages/rustc-demangle-0.1.28/Cargo.toml.orig.txt) |

## throw_error

[当前源码 / API](../../src/foundation/diagnostics/throw_error/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/throw_error/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| throw_error@0.3.1 | [README.md](../../docs/upstream/throw_error-0.3.1/README.md) | [许可与通知](../../compliance/licenses/shared/vendor/web/dependencies)；[来源](../../compliance/provenance/packages/throw_error-0.3.1/Cargo.toml.orig.txt) |

## tracing

[当前源码 / API](../../src/foundation/diagnostics/tracing/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/tracing/Cargo.toml) · [当前维护说明](../../docs/components/tracing.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tracing@0.1.44 | [README.md](../../docs/upstream/tracing-0.1.44/README.md) | [许可与通知](../../compliance/licenses/upstream/tracing-0.1.44)；[来源](../../compliance/provenance/packages/tracing-0.1.44/Cargo.toml.orig.txt) |
| matchers@0.2.0（合并来源） | [README.md](../../docs/upstream/matchers-0.2.0/README.md) | [许可与通知](../../compliance/licenses/upstream/matchers-0.2.0)；[来源](../../compliance/provenance/packages/matchers-0.2.0/Cargo.toml.orig.txt) |
| tracing-core@0.1.36（合并来源） | [README.md](../../docs/upstream/tracing-core-0.1.36/README.md) | [许可与通知](../../compliance/licenses/upstream/tracing-core-0.1.36)、[其他许可](../../compliance/licenses/upstream/tracing-core-0.1.36/src/spin)；[来源](../../compliance/provenance/packages/tracing-core-0.1.36/Cargo.toml.orig.txt) |
| tracing-log@0.2.0（合并来源） | [README.md](../../docs/upstream/tracing-log-0.2.0/README.md) | [许可与通知](../../compliance/licenses/upstream/tracing-log-0.2.0)；[来源](../../compliance/provenance/packages/tracing-log-0.2.0/Cargo.toml.orig.txt) |
| tracing-subscriber@0.3.23（合并来源） | [README.md](../../docs/upstream/tracing-subscriber-0.3.23/README.md) | [许可与通知](../../compliance/licenses/upstream/tracing-subscriber-0.3.23)；[来源](../../compliance/provenance/packages/tracing-subscriber-0.3.23/Cargo.toml.orig.txt) |

## tracing-attributes

[当前源码 / API](../../src/foundation/diagnostics/tracing-attributes/src/lib.rs) · [Cargo 清单](../../src/foundation/diagnostics/tracing-attributes/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tracing-attributes@0.1.31 | [README.md](../../docs/upstream/tracing-attributes-0.1.31/README.md) | [许可与通知](../../compliance/licenses/upstream/tracing-attributes-0.1.31)；[来源](../../compliance/provenance/packages/tracing-attributes-0.1.31/Cargo.toml.orig.txt) |
