# 并发与同步文档

[依赖文档入口](README.md)

执行器、任务调度、线程同步、通道与 Future。本页包含 25 个当前实现归属、40 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## async-notification

[当前源码 / API](../../src/foundation/concurrency/async-notification/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/async-notification/Cargo.toml) · [当前维护说明](../../docs/components/async-notification.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| async-lock@3.4.2 | [README.md](../../docs/upstream/async-lock-3.4.2/README.md) | [许可与通知](../../compliance/licenses/upstream/async-lock-3.4.2)；[来源](../../compliance/provenance/packages/async-lock-3.4.2/Cargo.toml.orig.txt) |
| event-listener@5.4.2（合并来源） | [README.md](../../docs/upstream/event-listener-5.4.2/README.md) | [许可与通知](../../compliance/licenses/upstream/event-listener-5.4.2)；[来源](../../compliance/provenance/packages/event-listener-5.4.2/Cargo.toml.orig.txt) |
| event-listener-strategy@0.5.4（合并来源） | [README.md](../../docs/upstream/event-listener-strategy-0.5.4/README.md) | [许可与通知](../../compliance/licenses/upstream/event-listener-strategy-0.5.4)；[来源](../../compliance/provenance/packages/event-listener-strategy-0.5.4/Cargo.toml.orig.txt) |

## async-trait

[当前源码 / API](../../src/foundation/concurrency/async-trait/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/async-trait/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| async-trait@0.1.92 | [README.md](../../docs/upstream/async-trait-0.1.92/README.md) | [许可与通知](../../compliance/licenses/upstream/async-trait-0.1.92)；[来源](../../compliance/provenance/packages/async-trait-0.1.92/Cargo.toml.orig.txt) |

## concurrent-collections

[当前源码 / API](../../src/foundation/concurrency/concurrent-collections/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/concurrent-collections/Cargo.toml) · [当前维护说明](../../docs/architecture/concurrent-collections.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| crossbeam-utils@0.8.23 | [README.md](../../docs/upstream/crossbeam-utils-0.8.23/README.md) | [许可与通知](../../compliance/licenses/upstream/crossbeam-utils-0.8.23)；[来源](../../compliance/provenance/packages/crossbeam-utils-0.8.23/Cargo.toml.orig.txt) |
| crossbeam-deque@0.8.8（合并来源） | [README.md](../../docs/upstream/crossbeam-deque-0.8.8/README.md) | [许可与通知](../../compliance/licenses/upstream/crossbeam-deque-0.8.8)；[来源](../../compliance/provenance/packages/crossbeam-deque-0.8.8/Cargo.toml.orig.txt) |
| crossbeam-epoch@0.9.21（合并来源） | [README.md](../../docs/upstream/crossbeam-epoch-0.9.21/README.md) | [许可与通知](../../compliance/licenses/upstream/crossbeam-epoch-0.9.21)；[来源](../../compliance/provenance/packages/crossbeam-epoch-0.9.21/Cargo.toml.orig.txt) |
| crossbeam-queue@0.3.14（合并来源） | [README.md](../../docs/upstream/crossbeam-queue-0.3.14/README.md) | [许可与通知](../../compliance/licenses/upstream/crossbeam-queue-0.3.14)；[来源](../../compliance/provenance/packages/crossbeam-queue-0.3.14/Cargo.toml.orig.txt) |

## flume

[当前源码 / API](../../src/foundation/concurrency/flume/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/flume/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| flume@0.12.0 | [README.md](../../docs/upstream/flume-0.12.0/README.md) | [许可与通知](../../compliance/licenses/upstream/flume-0.12.0)；[来源](../../compliance/provenance/packages/flume-0.12.0/Cargo.toml.orig.txt) |

## futures

[当前源码 / API](../../src/foundation/concurrency/futures/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/futures/Cargo.toml) · [当前维护说明](../../docs/components/futures.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| futures@0.3.34 | [README.md](../../docs/upstream/futures-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-0.3.34)；[来源](../../compliance/provenance/packages/futures-0.3.34/Cargo.toml.orig.txt) |
| futures-channel@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-channel-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-channel-0.3.34)；[来源](../../compliance/provenance/packages/futures-channel-0.3.34/Cargo.toml.orig.txt) |
| futures-core@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-core-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-core-0.3.34)；[来源](../../compliance/provenance/packages/futures-core-0.3.34/Cargo.toml.orig.txt) |
| futures-executor@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-executor-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-executor-0.3.34)；[来源](../../compliance/provenance/packages/futures-executor-0.3.34/Cargo.toml.orig.txt) |
| futures-io@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-io-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-io-0.3.34)；[来源](../../compliance/provenance/packages/futures-io-0.3.34/Cargo.toml.orig.txt) |
| futures-sink@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-sink-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-sink-0.3.34)；[来源](../../compliance/provenance/packages/futures-sink-0.3.34/Cargo.toml.orig.txt) |
| futures-task@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-task-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-task-0.3.34)；[来源](../../compliance/provenance/packages/futures-task-0.3.34/Cargo.toml.orig.txt) |
| futures-util@0.3.34（合并来源） | [README.md](../../docs/upstream/futures-util-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/futures-util-0.3.34)；[来源](../../compliance/provenance/packages/futures-util-0.3.34/Cargo.toml.orig.txt) |

## futures-intrusive

[当前源码 / API](../../src/foundation/concurrency/futures-intrusive/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/futures-intrusive/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| futures-intrusive@0.5.0 | [readme.md](../../docs/upstream/futures-intrusive-0.5.0/readme.md) | [许可与通知](../../compliance/licenses/upstream/futures-intrusive-0.5.0)；[来源](../../compliance/provenance/packages/futures-intrusive-0.5.0/Cargo.toml.orig.txt) |

## futures-macro

[当前源码 / API](../../src/foundation/concurrency/futures-macro/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/futures-macro/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| futures-macro@0.3.34 | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/concurrency/futures-macro/src/lib.rs) | [许可与通知](../../compliance/licenses/upstream/futures-macro-0.3.34)；[来源](../../compliance/provenance/packages/futures-macro-0.3.34/Cargo.toml.orig.txt) |

## guardian

[当前源码 / API](../../src/foundation/concurrency/guardian/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/guardian/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| guardian@1.3.0 | [README.md](../../docs/upstream/guardian-1.3.0/README.md) | [许可与通知](../../compliance/licenses/upstream/guardian-1.3.0)；[来源](../../compliance/provenance/packages/guardian-1.3.0/Cargo.toml.orig.txt) |

## lazy_static

[当前源码 / API](../../src/foundation/concurrency/lazy_static/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/lazy_static/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| lazy_static@1.5.1 | [README.md](../../docs/upstream/lazy_static-1.5.1/README.md) | [许可与通知](../../compliance/licenses/upstream/lazy_static-1.5.1)；[来源](../../compliance/provenance/packages/lazy_static-1.5.1/Cargo.toml.orig.txt) |

## mio

[当前源码 / API](../../src/foundation/concurrency/mio/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/mio/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| mio@1.2.4 | [README.md](../../docs/upstream/mio-1.2.4/README.md) | [许可与通知](../../compliance/licenses/upstream/mio-1.2.4)；[来源](../../compliance/provenance/packages/mio-1.2.4/Cargo.toml.orig.txt) |

## once_cell

[当前源码 / API](../../src/foundation/concurrency/once_cell/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/once_cell/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| once_cell@1.21.4 | [README.md](../../docs/upstream/once_cell-1.21.4/README.md) | [许可与通知](../../compliance/licenses/upstream/once_cell-1.21.4)；[来源](../../compliance/provenance/packages/once_cell-1.21.4/Cargo.toml.orig.txt) |

## or_poisoned

[当前源码 / API](../../src/foundation/concurrency/or_poisoned/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/or_poisoned/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| or_poisoned@0.1.0 | [README.md](../../docs/upstream/or_poisoned-0.1.0/README.md) | [许可与通知](../../compliance/licenses/shared/vendor/web/dependencies)；[来源](../../compliance/provenance/packages/or_poisoned-0.1.0/Cargo.toml.orig.txt) |

## parking

[当前源码 / API](../../src/foundation/concurrency/parking/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/parking/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| parking@2.2.1 | [README.md](../../docs/upstream/parking-2.2.1/README.md) | [许可与通知](../../compliance/licenses/upstream/parking-2.2.1)；[来源](../../compliance/provenance/packages/parking-2.2.1/Cargo.toml.orig.txt) |

## pin-project-lite

[当前源码 / API](../../src/foundation/concurrency/pin-project-lite/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/pin-project-lite/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| pin-project-lite@0.2.17 | [README.md](../../docs/upstream/pin-project-lite-0.2.17/README.md) | [许可与通知](../../compliance/licenses/upstream/pin-project-lite-0.2.17)；[来源](../../compliance/provenance/packages/pin-project-lite-0.2.17/Cargo.toml.orig.txt) |

## rayon

[当前源码 / API](../../src/foundation/concurrency/rayon/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/rayon/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rayon@1.12.0 | [README.md](../../docs/upstream/rayon-1.12.0/README.md)；[FAQ.md](../../docs/upstream/rayon-1.12.0/FAQ.md) | [许可与通知](../../compliance/licenses/upstream/rayon-1.12.0)；[来源](../../compliance/provenance/packages/rayon-1.12.0/Cargo.toml.orig.txt) |

## rayon-core

[当前源码 / API](../../src/foundation/concurrency/rayon-core/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/rayon-core/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rayon-core@1.13.0 | [README.md](../../docs/upstream/rayon-core-1.13.0/README.md) | [许可与通知](../../compliance/licenses/upstream/rayon-core-1.13.0)；[来源](../../compliance/provenance/packages/rayon-core-1.13.0/Cargo.toml.orig.txt) |

## send_wrapper

[当前源码 / API](../../src/foundation/concurrency/send_wrapper/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/send_wrapper/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| send_wrapper@0.6.0 | [README.md](../../docs/upstream/send_wrapper-0.6.0/README.md) | [许可与通知](../../compliance/licenses/upstream/send_wrapper-0.6.0)；[来源](../../compliance/provenance/packages/send_wrapper-0.6.0/Cargo.toml.orig.txt) |

## signal-hook-registry

[当前源码 / API](../../src/foundation/concurrency/signal-hook-registry/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/signal-hook-registry/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| signal-hook-registry@1.4.8 | [README.md](../../docs/upstream/signal-hook-registry-1.4.8/README.md) | [许可与通知](../../compliance/licenses/upstream/signal-hook-registry-1.4.8)；[来源](../../compliance/provenance/packages/signal-hook-registry-1.4.8/Cargo.toml.orig.txt) |

## spin

[当前源码 / API](../../src/foundation/concurrency/spin/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/spin/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| spin@0.9.9 | [README.md](../../docs/upstream/spin-0.9.9/README.md) | [许可与通知](../../compliance/licenses/upstream/spin-0.9.9)；[来源](../../compliance/provenance/packages/spin-0.9.9/Cargo.toml.orig.txt) |

## sync_wrapper

[当前源码 / API](../../src/foundation/concurrency/sync_wrapper/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/sync_wrapper/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| sync_wrapper@1.0.2 | [README.md](../../docs/upstream/sync_wrapper-1.0.2/README.md) | [许可与通知](../../compliance/licenses/upstream/sync_wrapper-1.0.2)；[来源](../../compliance/provenance/packages/sync_wrapper-1.0.2/Cargo.toml.orig.txt) |

## synchronization

[当前源码 / API](../../src/foundation/concurrency/synchronization/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/synchronization/Cargo.toml) · [当前维护说明](../../docs/components/synchronization.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| parking_lot_core@0.9.12 | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/concurrency/synchronization/src/parking/mod.rs) | [许可与通知](../../compliance/licenses/upstream/parking_lot_core-0.9.12)；[来源](../../compliance/provenance/packages/parking_lot_core-0.9.12/Cargo.toml.orig.txt) |
| lock_api@0.4.14（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/concurrency/synchronization/src/api/mod.rs) | [许可与通知](../../compliance/licenses/upstream/lock_api-0.4.14)；[来源](../../compliance/provenance/packages/lock_api-0.4.14/Cargo.toml.orig.txt) |
| parking_lot@0.12.5（合并来源） | [README.md](../../docs/upstream/parking_lot-0.12.5/README.md) | [许可与通知](../../compliance/licenses/upstream/parking_lot-0.12.5)；[来源](../../compliance/provenance/packages/parking_lot-0.12.5/Cargo.toml.orig.txt) |

## task-dispatch

[当前源码 / API](../../src/foundation/concurrency/task-dispatch/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/task-dispatch/Cargo.toml) · [当前维护说明](../../docs/architecture/task-dispatch.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| any_spawner@0.3.0 | [README.md](../../docs/upstream/any_spawner-0.3.0/README.md) | [许可与通知](../../compliance/licenses/shared/vendor/web/dependencies)；[来源](../../compliance/provenance/packages/any_spawner-0.3.0/Cargo.toml.orig.txt) |

## thread_local

[当前源码 / API](../../src/foundation/concurrency/thread_local/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/thread_local/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| thread_local@1.1.10 | [README.md](../../docs/upstream/thread_local-1.1.10/README.md) | [许可与通知](../../compliance/licenses/upstream/thread_local-1.1.10)；[来源](../../compliance/provenance/packages/thread_local-1.1.10/Cargo.toml.orig.txt) |

## tokio

[当前源码 / API](../../src/foundation/concurrency/tokio/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/tokio/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tokio@1.53.3 | [README.md](../../docs/upstream/tokio-1.53.3/README.md) | [许可与通知](../../compliance/licenses/upstream/tokio-1.53.3)；[来源](../../compliance/provenance/packages/tokio-1.53.3/Cargo.toml.orig.txt) |
| tokio-util@0.7.20（合并来源） | [README.md](../../docs/upstream/tokio-util-0.7.20/README.md) | [许可与通知](../../compliance/licenses/upstream/tokio-util-0.7.20)；[来源](../../compliance/provenance/packages/tokio-util-0.7.20/Cargo.toml.orig.txt) |

## tokio-macros

[当前源码 / API](../../src/foundation/concurrency/tokio-macros/src/lib.rs) · [Cargo 清单](../../src/foundation/concurrency/tokio-macros/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tokio-macros@2.7.2 | [README.md](../../docs/upstream/tokio-macros-2.7.2/README.md) | [许可与通知](../../compliance/licenses/upstream/tokio-macros-2.7.2)；[来源](../../compliance/provenance/packages/tokio-macros-2.7.2/Cargo.toml.orig.txt) |
