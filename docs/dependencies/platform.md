# 系统与平台文档

[依赖文档入口](README.md)

系统调用、动态库、CPU、文件系统与系统信息。本页包含 9 个当前实现归属、9 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## cpu-feature-detect

[当前源码 / API](../../src/foundation/platform/cpu-feature-detect/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/cpu-feature-detect/Cargo.toml) · [当前维护说明](../../docs/components/cpu-feature-detect.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| cpufeatures@0.3.1 | [README.md](../../docs/upstream/cpufeatures-0.3.1/README.md) | [许可与通知](../../compliance/licenses/upstream/cpufeatures-0.3.1)；[来源](../../compliance/provenance/packages/cpufeatures-0.3.1/Cargo.toml.orig.txt) |

## errno

[当前源码 / API](../../src/foundation/platform/errno/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/errno/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| errno@0.3.14 | [README.md](../../docs/upstream/errno-0.3.14/README.md) | [许可与通知](../../compliance/licenses/upstream/errno-0.3.14)；[来源](../../compliance/provenance/packages/errno-0.3.14/Cargo.toml.orig.txt) |

## glob

[当前源码 / API](../../src/foundation/platform/glob/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/glob/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| glob@0.3.4 | [README.md](../../docs/upstream/glob-0.3.4/README.md) | [许可与通知](../../compliance/licenses/upstream/glob-0.3.4)；[来源](../../compliance/provenance/packages/glob-0.3.4/Cargo.toml.orig.txt) |

## libc

[当前源码 / API](../../src/foundation/platform/libc/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/libc/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| libc@0.2.191 | [README.md](../../docs/upstream/libc-0.2.191/README.md) | [许可与通知](../../compliance/licenses/upstream/libc-0.2.191)；[来源](../../compliance/provenance/packages/libc-0.2.191/Cargo.toml.orig.txt) |

## libloading

[当前源码 / API](../../src/foundation/platform/libloading/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/libloading/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| libloading@0.8.9 | [README.mkd](../../docs/upstream/libloading-0.8.9/README.mkd) | [许可与通知](../../compliance/licenses/upstream/libloading-0.8.9)；[来源](../../compliance/provenance/packages/libloading-0.8.9/Cargo.toml.orig.txt) |

## linux-raw-sys

[当前源码 / API](../../src/foundation/platform/linux-raw-sys/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/linux-raw-sys/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| linux-raw-sys@0.12.1 | [README.md](../../docs/upstream/linux-raw-sys-0.12.1/README.md) | [许可与通知](../../compliance/licenses/upstream/linux-raw-sys-0.12.1)；[来源](../../compliance/provenance/packages/linux-raw-sys-0.12.1/Cargo.toml.orig.txt) |

## rustix

[当前源码 / API](../../src/foundation/platform/rustix/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/rustix/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustix@1.1.5 | [README.md](../../docs/upstream/rustix-1.1.5/README.md) | [许可与通知](../../compliance/licenses/upstream/rustix-1.1.5)；[来源](../../compliance/provenance/packages/rustix-1.1.5/Cargo.toml.orig.txt) |

## sysinfo

[当前源码 / API](../../src/foundation/platform/sysinfo/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/sysinfo/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| sysinfo@0.39.6 | [README.md](../../docs/upstream/sysinfo-0.39.6/README.md)；[md_doc/minimum_cpu_update_interval.md](../../docs/upstream/sysinfo-0.39.6/md_doc/minimum_cpu_update_interval.md)；[md_doc/pid.md](../../docs/upstream/sysinfo-0.39.6/md_doc/pid.md)；[md_doc/serde.md](../../docs/upstream/sysinfo-0.39.6/md_doc/serde.md) | [许可与通知](../../compliance/licenses/upstream/sysinfo-0.39.6)；[来源](../../compliance/provenance/packages/sysinfo-0.39.6/Cargo.toml.orig.txt) |

## tempfile

[当前源码 / API](../../src/foundation/platform/tempfile/src/lib.rs) · [Cargo 清单](../../src/foundation/platform/tempfile/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| tempfile@3.27.0 | [README.md](../../docs/upstream/tempfile-3.27.0/README.md) | [许可与通知](../../compliance/licenses/upstream/tempfile-3.27.0)；[来源](../../compliance/provenance/packages/tempfile-3.27.0/Cargo.toml.orig.txt) |
