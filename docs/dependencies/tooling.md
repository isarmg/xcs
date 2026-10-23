# 构建与命令行文档

[依赖文档入口](README.md)

C 绑定、原生构建、命令行、Shell 分词与终端样式。本页包含 14 个当前实现归属、23 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## autocfg

[当前源码 / API](../../src/foundation/tooling/autocfg/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/autocfg/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| autocfg@1.5.1 | [README.md](../../docs/upstream/autocfg-1.5.1/README.md) | [许可与通知](../../compliance/licenses/upstream/autocfg-1.5.1)；[来源](../../compliance/provenance/packages/autocfg-1.5.1/Cargo.toml.orig.txt) |

## bindgen

[当前源码 / API](../../src/foundation/tooling/bindgen/lib.rs) · [Cargo 清单](../../src/foundation/tooling/bindgen/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| bindgen@0.72.1 | [README.md](../../docs/upstream/bindgen-0.72.1/README.md) | [许可与通知](../../compliance/licenses/upstream/bindgen-0.72.1)；[来源](../../compliance/provenance/packages/bindgen-0.72.1/Cargo.toml.orig.txt) |
| cexpr@0.6.0（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/tooling/bindgen/src/cexpr/mod.rs) | [许可与通知](../../compliance/licenses/upstream/cexpr-0.6.0)；[来源](../../compliance/provenance/packages/cexpr-0.6.0/Cargo.toml.orig.txt) |

## cc

[当前源码 / API](../../src/foundation/tooling/cc/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/cc/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| cc@1.8.0 | [README.md](../../docs/upstream/cc-1.8.0/README.md) | [许可与通知](../../compliance/licenses/upstream/cc-1.8.0)；[来源](../../compliance/provenance/packages/cc-1.8.0/Cargo.toml.orig.txt) |

## cfg-if

[当前源码 / API](../../src/foundation/tooling/cfg-if/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/cfg-if/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| cfg-if@1.0.5 | [README.md](../../docs/upstream/cfg-if-1.0.5/README.md) | [许可与通知](../../compliance/licenses/upstream/cfg-if-1.0.5)；[来源](../../compliance/provenance/packages/cfg-if-1.0.5/Cargo.toml.orig.txt) |

## clang-sys

[当前源码 / API](../../src/foundation/tooling/clang-sys/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/clang-sys/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| clang-sys@1.9.1 | [README.md](../../docs/upstream/clang-sys-1.9.1/README.md) | [许可与通知](../../compliance/licenses/upstream/clang-sys-1.9.1)；[来源](../../compliance/provenance/packages/clang-sys-1.9.1/Cargo.toml.orig.txt) |

## command-derive

[当前源码 / API](../../src/foundation/tooling/command-derive/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/command-derive/Cargo.toml) · [当前维护说明](../../docs/architecture/command-line-reference/derive/README.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| clap_derive@4.6.7 | [README.md](../../docs/upstream/clap_derive-4.6.7/README.md) | [许可与通知](../../compliance/licenses/upstream/clap_derive-4.6.7)；[来源](../../compliance/provenance/packages/clap_derive-4.6.7/Cargo.toml.orig.txt) |

## command-line

[当前源码 / API](../../src/foundation/tooling/command-line/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/command-line/Cargo.toml) · [当前维护说明](../../docs/architecture/command-line-reference/runtime/README.md) · [相关说明](../../docs/architecture/command-line-reference/builder/README.md) · [相关说明](../../docs/architecture/command-line-reference/lexer/README.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| clap@4.6.7 | [README.md](../../docs/upstream/clap-4.6.7/README.md)；[examples/tutorial_builder/01_quick.md](../../docs/upstream/clap-4.6.7/examples/tutorial_builder/01_quick.md)；[examples/tutorial_derive/01_quick.md](../../docs/upstream/clap-4.6.7/examples/tutorial_derive/01_quick.md) | [许可与通知](../../compliance/licenses/upstream/clap-4.6.7)；[来源](../../compliance/provenance/packages/clap-4.6.7/Cargo.toml.orig.txt) |
| clap_builder@4.6.7（合并来源） | [README.md](../../docs/upstream/clap_builder-4.6.7/README.md) | [许可与通知](../../compliance/licenses/upstream/clap_builder-4.6.7)；[来源](../../compliance/provenance/packages/clap_builder-4.6.7/Cargo.toml.orig.txt) |
| clap_lex@1.1.1（合并来源） | [README.md](../../docs/upstream/clap_lex-1.1.1/README.md) | [许可与通知](../../compliance/licenses/upstream/clap_lex-1.1.1)；[来源](../../compliance/provenance/packages/clap_lex-1.1.1/Cargo.toml.orig.txt) |
| strsim@0.11.1（合并来源） | [README.md](../../docs/upstream/strsim-0.11.1/README.md) | [许可与通知](../../compliance/licenses/upstream/strsim-0.11.1)；[来源](../../compliance/provenance/packages/strsim-0.11.1/Cargo.toml.orig.txt) |

## nu-ansi-term

[当前源码 / API](../../src/foundation/tooling/nu-ansi-term/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/nu-ansi-term/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| nu-ansi-term@0.50.3 | [README.md](../../docs/upstream/nu-ansi-term-0.50.3/README.md) | [许可与通知](../../compliance/licenses/upstream/nu-ansi-term-0.50.3)；[来源](../../compliance/provenance/packages/nu-ansi-term-0.50.3/Cargo.toml.orig.txt) |

## pkg-config

[当前源码 / API](../../src/foundation/tooling/pkg-config/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/pkg-config/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| pkg-config@0.3.34 | [README.md](../../docs/upstream/pkg-config-0.3.34/README.md) | [许可与通知](../../compliance/licenses/upstream/pkg-config-0.3.34)；[来源](../../compliance/provenance/packages/pkg-config-0.3.34/Cargo.toml.orig.txt) |

## rustversion

[当前源码 / API](../../src/foundation/tooling/rustversion/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/rustversion/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| rustversion@1.0.23 | [README.md](../../docs/upstream/rustversion-1.0.23/README.md) | [许可与通知](../../compliance/licenses/upstream/rustversion-1.0.23)；[来源](../../compliance/provenance/packages/rustversion-1.0.23/Cargo.toml.orig.txt) |

## semver

[当前源码 / API](../../src/foundation/tooling/semver/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/semver/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| semver@1.0.28 | [README.md](../../docs/upstream/semver-1.0.28/README.md) | [许可与通知](../../compliance/licenses/upstream/semver-1.0.28)；[来源](../../compliance/provenance/packages/semver-1.0.28/Cargo.toml.orig.txt) |

## shell-tokenizer

[当前源码 / API](../../src/foundation/tooling/shell-tokenizer/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/shell-tokenizer/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| shlex@2.0.1 | [README.md](../../docs/upstream/shlex-2.0.1/README.md)；[src/quoting_warning.md](../../docs/upstream/shlex-2.0.1/src/quoting_warning.md) | [许可与通知](../../compliance/licenses/upstream/shlex-2.0.1)；[来源](../../compliance/provenance/packages/shlex-2.0.1/Cargo.toml.orig.txt) |

## terminal-style

[当前源码 / API](../../src/foundation/tooling/terminal-style/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/terminal-style/Cargo.toml) · [当前维护说明](../../docs/architecture/terminal-style.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| anstyle@1.0.14 | [README.md](../../docs/upstream/anstyle-1.0.14/README.md) | [许可与通知](../../compliance/licenses/upstream/anstyle-1.0.14)；[来源](../../compliance/provenance/packages/anstyle-1.0.14/Cargo.toml.orig.txt) |
| anstream@1.0.0（合并来源） | [README.md](../../docs/upstream/anstream-1.0.0/README.md) | [许可与通知](../../compliance/licenses/upstream/anstream-1.0.0)；[来源](../../compliance/provenance/packages/anstream-1.0.0/Cargo.toml.orig.txt) |
| anstyle-parse@1.0.0（合并来源） | [README.md](../../docs/upstream/anstyle-parse-1.0.0/README.md) | [许可与通知](../../compliance/licenses/upstream/anstyle-parse-1.0.0)；[来源](../../compliance/provenance/packages/anstyle-parse-1.0.0/Cargo.toml.orig.txt) |
| anstyle-query@1.1.5（合并来源） | [README.md](../../docs/upstream/anstyle-query-1.1.5/README.md) | [许可与通知](../../compliance/licenses/upstream/anstyle-query-1.1.5)；[来源](../../compliance/provenance/packages/anstyle-query-1.1.5/Cargo.toml.orig.txt) |
| colorchoice@1.0.5（合并来源） | [README.md](../../docs/upstream/colorchoice-1.0.5/README.md) | [许可与通知](../../compliance/licenses/upstream/colorchoice-1.0.5)；[来源](../../compliance/provenance/packages/colorchoice-1.0.5/Cargo.toml.orig.txt) |
| utf8parse@0.2.2（合并来源） | 未附独立使用文档；[当前源码内 Rustdoc](../../src/foundation/tooling/terminal-style/src/utf8/mod.rs) | [许可与通知](../../compliance/licenses/upstream/utf8parse-0.2.2)；[来源](../../compliance/provenance/packages/utf8parse-0.2.2/Cargo.toml.orig.txt) |

## version_check

[当前源码 / API](../../src/foundation/tooling/version_check/src/lib.rs) · [Cargo 清单](../../src/foundation/tooling/version_check/Cargo.toml)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| version_check@0.9.5 | [README.md](../../docs/upstream/version_check-0.9.5/README.md) | [许可与通知](../../compliance/licenses/upstream/version_check-0.9.5)；[来源](../../compliance/provenance/packages/version_check-0.9.5/Cargo.toml.orig.txt) |
