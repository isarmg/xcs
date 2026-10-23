# 基础值类型文档

[依赖文档入口](README.md)

数值、日期时间、标识符、标志位与基础值类型。本页包含 15 个当前实现归属、25 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## ahash

[当前源码 / API](../../src/foundation/core-types/ahash/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/ahash/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| ahash@0.8.12 | [README.md](../../docs/upstream/ahash-0.8.12/README.md)；[FAQ.md](../../docs/upstream/ahash-0.8.12/FAQ.md) | [许可与通知](../../compliance/licenses/upstream/ahash-0.8.12)；[来源](../../compliance/provenance/packages/ahash-0.8.12/Cargo.toml.orig.txt) |

## date-time

[当前源码 / API](../../src/foundation/core-types/date-time/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/date-time/Cargo.toml) · [当前维护说明](../../docs/architecture/date-time.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| time@0.3.55 | [README.md](../../docs/upstream/time-0.3.55/README.md) | [许可与通知](../../compliance/licenses/upstream/time-0.3.55)；[来源](../../compliance/provenance/packages/time-0.3.55/Cargo.toml.orig.txt) |
| deranged@0.5.9（合并来源） | [README.md](../../docs/upstream/deranged-0.5.9/README.md) | [许可与通知](../../compliance/licenses/upstream/deranged-0.5.9)；[来源](../../compliance/provenance/packages/deranged-0.5.9/Cargo.toml.orig.txt) |
| num-conv@0.2.2（合并来源） | [README.md](../../docs/upstream/num-conv-0.2.2/README.md) | [许可与通知](../../compliance/licenses/upstream/num-conv-0.2.2)；[来源](../../compliance/provenance/packages/num-conv-0.2.2/Cargo.toml.orig.txt) |
| powerfmt@0.2.1（合并来源） | [README.md](../../docs/upstream/powerfmt-0.2.1/README.md) | [许可与通知](../../compliance/licenses/upstream/powerfmt-0.2.1)；[来源](../../compliance/provenance/packages/powerfmt-0.2.1/Cargo.toml.orig.txt) |
| time-core@0.1.9（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/core-types/date-time/src/calendar_core/mod.rs) | [许可与通知](../../compliance/licenses/upstream/time-core-0.1.9)；[来源](../../compliance/provenance/packages/time-core-0.1.9/Cargo.toml.orig.txt) |

## either

[当前源码 / API](../../src/foundation/core-types/either/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/either/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| either@1.19.0 | [README.md](../../docs/upstream/either-1.19.0/README.md) | [许可与通知](../../compliance/licenses/upstream/either-1.19.0)；[来源](../../compliance/provenance/packages/either-1.19.0/Cargo.toml.orig.txt) |

## fastrand

[当前源码 / API](../../src/foundation/core-types/fastrand/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/fastrand/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| fastrand@2.5.0 | [README.md](../../docs/upstream/fastrand-2.5.0/README.md) | [许可与通知](../../compliance/licenses/upstream/fastrand-2.5.0)；[来源](../../compliance/provenance/packages/fastrand-2.5.0/Cargo.toml.orig.txt) |

## flag-set

[当前源码 / API](../../src/foundation/core-types/flag-set/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/flag-set/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| bitflags@2.13.2 | [README.md](../../docs/upstream/bitflags-2.13.2/README.md)；[spec.md](../../docs/upstream/bitflags-2.13.2/spec.md) | [许可与通知](../../compliance/licenses/upstream/bitflags-2.13.2)；[来源](../../compliance/provenance/packages/bitflags-2.13.2/Cargo.toml.orig.txt) |

## foldhash

[当前源码 / API](../../src/foundation/core-types/foldhash/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/foldhash/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| foldhash@0.2.0 | [README.md](../../docs/upstream/foldhash-0.2.0/README.md) | [许可与通知](../../compliance/licenses/upstream/foldhash-0.2.0)；[来源](../../compliance/provenance/packages/foldhash-0.2.0/Cargo.toml.orig.txt) |

## fraction

[当前源码 / API](../../src/foundation/core-types/fraction/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/fraction/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| fraction@0.17.0 | [README.md](../../docs/upstream/fraction-0.17.0/README.md) | [许可与通知](../../compliance/licenses/upstream/fraction-0.17.0)；[来源](../../compliance/provenance/packages/fraction-0.17.0/Cargo.toml.orig.txt) |

## hybrid-array

[当前源码 / API](../../src/foundation/core-types/hybrid-array/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/hybrid-array/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| hybrid-array@0.4.15 | [README.md](../../docs/upstream/hybrid-array-0.4.15/README.md) | [许可与通知](../../compliance/licenses/upstream/hybrid-array-0.4.15)；[来源](../../compliance/provenance/packages/hybrid-array-0.4.15/Cargo.toml.orig.txt) |

## itoa

[当前源码 / API](../../src/foundation/core-types/itoa/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/itoa/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| itoa@1.0.18 | [README.md](../../docs/upstream/itoa-1.0.18/README.md) | [许可与通知](../../compliance/licenses/upstream/itoa-1.0.18)；[来源](../../compliance/provenance/packages/itoa-1.0.18/Cargo.toml.orig.txt) |

## num-cmp

[当前源码 / API](../../src/foundation/core-types/num-cmp/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/num-cmp/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| num-cmp@0.1.0 | [README.md](../../docs/upstream/num-cmp-0.1.0/README.md) | [许可与通知](../../compliance/licenses/upstream/num-cmp-0.1.0)；[来源](../../compliance/provenance/packages/num-cmp-0.1.0/cargo-checksum.json.txt) |

## numeric

[当前源码 / API](../../src/foundation/core-types/numeric/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/numeric/Cargo.toml) · [当前维护说明](../../docs/architecture/numeric.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| num-traits@0.2.19 | [README.md](../../docs/upstream/num-traits-0.2.19/README.md) | [许可与通知](../../compliance/licenses/upstream/num-traits-0.2.19)；[来源](../../compliance/provenance/packages/num-traits-0.2.19/Cargo.toml.orig.txt) |
| num@0.4.3（合并来源） | [README.md](../../docs/upstream/num-0.4.3/README.md) | [许可与通知](../../compliance/licenses/upstream/num-0.4.3)；[来源](../../compliance/provenance/packages/num-0.4.3/Cargo.toml.orig.txt) |
| num-bigint@0.4.8（合并来源） | [README.md](../../docs/upstream/num-bigint-0.4.8/README.md) | [许可与通知](../../compliance/licenses/upstream/num-bigint-0.4.8)；[来源](../../compliance/provenance/packages/num-bigint-0.4.8/Cargo.toml.orig.txt) |
| num-complex@0.4.6（合并来源） | [README.md](../../docs/upstream/num-complex-0.4.6/README.md) | [许可与通知](../../compliance/licenses/upstream/num-complex-0.4.6)；[来源](../../compliance/provenance/packages/num-complex-0.4.6/Cargo.toml.orig.txt) |
| num-integer@0.1.47（合并来源） | [README.md](../../docs/upstream/num-integer-0.1.47/README.md) | [许可与通知](../../compliance/licenses/upstream/num-integer-0.1.47)；[来源](../../compliance/provenance/packages/num-integer-0.1.47/Cargo.toml.orig.txt) |
| num-iter@0.1.46（合并来源） | [README.md](../../docs/upstream/num-iter-0.1.46/README.md) | [许可与通知](../../compliance/licenses/upstream/num-iter-0.1.46)；[来源](../../compliance/provenance/packages/num-iter-0.1.46/Cargo.toml.orig.txt) |
| num-rational@0.4.2（合并来源） | [README.md](../../docs/upstream/num-rational-0.4.2/README.md) | [许可与通知](../../compliance/licenses/upstream/num-rational-0.4.2)；[来源](../../compliance/provenance/packages/num-rational-0.4.2/Cargo.toml.orig.txt) |

## rustc-hash

[当前源码 / API](../../src/foundation/core-types/rustc-hash/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/rustc-hash/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustc-hash@2.1.3 | [README.md](../../docs/upstream/rustc-hash-2.1.3/README.md) | [许可与通知](../../compliance/licenses/upstream/rustc-hash-2.1.3)；[来源](../../compliance/provenance/packages/rustc-hash-2.1.3/Cargo.toml.orig.txt) |

## typenum

[当前源码 / API](../../src/foundation/core-types/typenum/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/typenum/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| typenum@1.20.1 | [README.md](../../docs/upstream/typenum-1.20.1/README.md) | [许可与通知](../../compliance/licenses/upstream/typenum-1.20.1)；[来源](../../compliance/provenance/packages/typenum-1.20.1/Cargo.toml.orig.txt) |

## uuid

[当前源码 / API](../../src/foundation/core-types/uuid/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/uuid/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| uuid@1.28.0 | [README.md](../../docs/upstream/uuid-1.28.0/README.md) | [许可与通知](../../compliance/licenses/upstream/uuid-1.28.0)；[来源](../../compliance/provenance/packages/uuid-1.28.0/Cargo.toml.orig.txt) |

## zmij

[当前源码 / API](../../src/foundation/core-types/zmij/src/lib.rs) · [Cargo 清单](../../src/foundation/core-types/zmij/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| zmij@1.0.23 | [README.md](../../docs/upstream/zmij-1.0.23/README.md) | [许可与通知](../../compliance/licenses/upstream/zmij-1.0.23)；[来源](../../compliance/provenance/packages/zmij-1.0.23/Cargo.toml.orig.txt) |
