# 密码学文档

[依赖文档入口](README.md)

哈希、认证、安全熵与密码学基础。本页包含 18 个当前实现归属、21 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## argon2

[当前源码 / API](../../src/foundation/cryptography/argon2/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/argon2/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| argon2@0.5.3 | [README.md](../../docs/upstream/argon2-0.5.3/README.md) | [许可与通知](../../compliance/licenses/upstream/argon2-0.5.3)；[来源](../../compliance/provenance/packages/argon2-0.5.3/Cargo.toml.orig.txt) |

## base64ct

[当前源码 / API](../../src/foundation/cryptography/base64ct/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/base64ct/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| base64ct@1.8.3 | [README.md](../../docs/upstream/base64ct-1.8.3/README.md) | [许可与通知](../../compliance/licenses/upstream/base64ct-1.8.3)；[来源](../../compliance/provenance/packages/base64ct-1.8.3/Cargo.toml.orig.txt) |

## blake2

[当前源码 / API](../../src/foundation/cryptography/blake2/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/blake2/Cargo.toml) · [当前维护说明](../../docs/components/cryptographic-core.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| blake2@0.10.6 | [README.md](../../docs/upstream/blake2-0.10.6/README.md) | [许可与通知](../../compliance/licenses/upstream/blake2-0.10.6)；[来源](../../compliance/provenance/packages/blake2-0.10.6/Cargo.toml.orig.txt) |
| digest@0.10.7（合并来源） | [README.md](../../docs/upstream/digest-0.10.7/README.md) | [许可与通知](../../compliance/licenses/upstream/digest-0.10.7)；[来源](../../compliance/provenance/packages/digest-0.10.7/Cargo.toml.orig.txt) |

## blake3

[当前源码 / API](../../src/foundation/cryptography/blake3/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/blake3/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| blake3@1.8.7 | [README.md](../../docs/upstream/blake3-1.8.7/README.md) | [许可与通知](../../compliance/licenses/upstream/blake3-1.8.7)；[来源](../../compliance/provenance/packages/blake3-1.8.7/Cargo.toml.orig.txt) |

## cmov

[当前源码 / API](../../src/foundation/cryptography/cmov/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/cmov/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| cmov@0.5.4 | [README.md](../../docs/upstream/cmov-0.5.4/README.md) | [许可与通知](../../compliance/licenses/upstream/cmov-0.5.4)；[来源](../../compliance/provenance/packages/cmov-0.5.4/Cargo.toml.orig.txt) |

## const-oid

[当前源码 / API](../../src/foundation/cryptography/const-oid/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/const-oid/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| const-oid@0.10.2 | [README.md](../../docs/upstream/const-oid-0.10.2/README.md) | [许可与通知](../../compliance/licenses/upstream/const-oid-0.10.2)；[来源](../../compliance/provenance/packages/const-oid-0.10.2/Cargo.toml.orig.txt) |

## constant_time_eq

[当前源码 / API](../../src/foundation/cryptography/constant_time_eq/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/constant_time_eq/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| constant_time_eq@0.4.2 | [README](../../docs/upstream/constant_time_eq-0.4.2/README) | [许可与通知](../../compliance/licenses/upstream/constant_time_eq-0.4.2)；[来源](../../compliance/provenance/packages/constant_time_eq-0.4.2/Cargo.toml.orig.txt) |

## cryptographic-core

[当前源码 / API](../../src/foundation/cryptography/cryptographic-core/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/cryptographic-core/Cargo.toml) · [当前维护说明](../../docs/components/cryptographic-core.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| digest@0.11.3 | [README.md](../../docs/upstream/digest-0.11.3/README.md) | [许可与通知](../../compliance/licenses/upstream/digest-0.11.3)；[来源](../../compliance/provenance/packages/digest-0.11.3/Cargo.toml.orig.txt) |
| block-buffer@0.12.1（合并来源） | [README.md](../../docs/upstream/block-buffer-0.12.1/README.md) | [许可与通知](../../compliance/licenses/upstream/block-buffer-0.12.1)；[来源](../../compliance/provenance/packages/block-buffer-0.12.1/Cargo.toml.orig.txt) |
| crypto-common@0.2.2（合并来源） | [README.md](../../docs/upstream/crypto-common-0.2.2/README.md) | [许可与通知](../../compliance/licenses/upstream/crypto-common-0.2.2)；[来源](../../compliance/provenance/packages/crypto-common-0.2.2/Cargo.toml.orig.txt) |

## ctutils

[当前源码 / API](../../src/foundation/cryptography/ctutils/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/ctutils/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| ctutils@0.4.4 | [README.md](../../docs/upstream/ctutils-0.4.4/README.md) | [许可与通知](../../compliance/licenses/upstream/ctutils-0.4.4)；[来源](../../compliance/provenance/packages/ctutils-0.4.4/Cargo.toml.orig.txt) |

## hmac

[当前源码 / API](../../src/foundation/cryptography/hmac/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/hmac/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| hmac@0.13.0 | [README.md](../../docs/upstream/hmac-0.13.0/README.md) | [许可与通知](../../compliance/licenses/upstream/hmac-0.13.0)；[来源](../../compliance/provenance/packages/hmac-0.13.0/Cargo.toml.orig.txt) |

## password-hash

[当前源码 / API](../../src/foundation/cryptography/password-hash/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/password-hash/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| password-hash@0.5.0 | [README.md](../../docs/upstream/password-hash-0.5.0/README.md) | [许可与通知](../../compliance/licenses/upstream/password-hash-0.5.0)；[来源](../../compliance/provenance/packages/password-hash-0.5.0/Cargo.toml.orig.txt) |

## random-generator-core

[当前源码 / API](../../src/foundation/cryptography/random-generator-core/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/random-generator-core/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rand_core@0.6.4 | [README.md](../../docs/upstream/rand_core-0.6.4/README.md) | [许可与通知](../../compliance/licenses/upstream/rand_core-0.6.4)；[来源](../../compliance/provenance/packages/rand_core-0.6.4/Cargo.toml.orig.txt) |

## ring

[当前源码 / API](../../src/foundation/cryptography/ring/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/ring/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| ring@0.17.14 | [README.md](../../docs/upstream/ring-0.17.14/README.md) | [许可与通知](../../compliance/licenses/upstream/ring-0.17.14)、[其他许可](../../compliance/licenses/upstream/ring-0.17.14/src/polyfill/once_cell)、[其他许可](../../compliance/licenses/upstream/ring-0.17.14/third_party/fiat)；[来源](../../compliance/provenance/packages/ring-0.17.14/Cargo.toml.orig.txt) |

## sha2

[当前源码 / API](../../src/foundation/cryptography/sha2/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/sha2/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| sha2@0.11.1 | [README.md](../../docs/upstream/sha2-0.11.1/README.md) | [许可与通知](../../compliance/licenses/upstream/sha2-0.11.1)；[来源](../../compliance/provenance/packages/sha2-0.11.1/Cargo.toml.orig.txt) |

## subtle

[当前源码 / API](../../src/foundation/cryptography/subtle/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/subtle/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| subtle@2.6.1 | [README.md](../../docs/upstream/subtle-2.6.1/README.md) | [许可与通知](../../compliance/licenses/upstream/subtle-2.6.1)；[来源](../../compliance/provenance/packages/subtle-2.6.1/Cargo.toml.orig.txt) |

## system-entropy

[当前源码 / API](../../src/foundation/cryptography/system-entropy/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/system-entropy/Cargo.toml) · [当前维护说明](../../docs/components/system-entropy.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| getrandom@0.4.3 | [README.md](../../docs/upstream/getrandom-0.4.3/README.md) | [许可与通知](../../compliance/licenses/upstream/getrandom-0.4.3)；[来源](../../compliance/provenance/packages/getrandom-0.4.3/Cargo.toml.orig.txt) |

## untrusted

[当前源码 / API](../../src/foundation/cryptography/untrusted/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/untrusted/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| untrusted@0.9.0 | [README.md](../../docs/upstream/untrusted-0.9.0/README.md) | [许可与通知](../../compliance/licenses/upstream/untrusted-0.9.0)；[来源](../../compliance/provenance/packages/untrusted-0.9.0/Cargo.toml.orig.txt) |

## zeroize

[当前源码 / API](../../src/foundation/cryptography/zeroize/src/lib.rs) · [Cargo 清单](../../src/foundation/cryptography/zeroize/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| zeroize@1.9.1 | [README.md](../../docs/upstream/zeroize-1.9.1/README.md) | [许可与通知](../../compliance/licenses/upstream/zeroize-1.9.1)；[来源](../../compliance/provenance/packages/zeroize-1.9.1/Cargo.toml.orig.txt) |
