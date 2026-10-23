# 业务组件内整合的依赖文档

[依赖文档入口](README.md)

命名、共享界面、视图宏与 Web 绑定工具。本页包含 4 个当前实现归属、5 个上游来源身份。

先读当前维护说明和本地 API/Rustdoc，再按需查阅与来源身份对应、随源码保留的说明。这些资料可能描述已移除的平台、特性、独立包名或开发流程，不代表 ZXS 当前支持范围；合并来源不再提供独立 crate。

由 `python3 tools/rust-sources/documentation.py refresh` 生成；`verify` 检查覆盖、原件和链接目标。这里只选择使用入口，不列上游变更日志、贡献流程或发布记录。

## web-interface

[当前源码 / API](../../src/interface/src/lib.rs) · [Cargo 清单](../../src/interface/Cargo.toml) · [当前维护说明](../../docs/components/interface.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| leptos@0.8.19（合并来源） | [README.md](../../docs/dependencies/originals/leptos-0.8.19/README.md)（精确发布包原件） | [许可与通知](../../compliance/licenses/upstream/zxs-web-core-0.8.19)；[来源](../../compliance/provenance/origins.json) |

## view-macros

[当前源码 / API](../../src/macros/src/lib.rs) · [Cargo 清单](../../src/macros/Cargo.toml) · [当前维护说明](../../docs/components/interface.md) · [相关说明](../../docs/architecture/view-prop-parser.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| leptos_macro@0.8.19 | [README.md](../../docs/upstream/tachys-0.2.19/README.md)（精确发布包原件） | [许可与通知](../../compliance/licenses/upstream/zxs-web-macro-0.8.19)；[来源](../../compliance/provenance/origins.json) |
| leptos_hot_reload@0.8.7（合并来源） | [README.md](../../docs/upstream/leptos_hot_reload-0.8.7/README.md) | [许可与通知](../../compliance/licenses/shared/vendor/web/dependencies)；[来源](../../compliance/provenance/packages/leptos_hot_reload-0.8.7/Cargo.toml.orig.txt) |

## name-policy

[当前源码 / API](../../src/naming/src/lib.rs) · [Cargo 清单](../../src/naming/Cargo.toml) · [当前维护说明](../../docs/components/naming.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| unicode-normalization@0.1.24（合并来源） | [README.md](../../docs/dependencies/originals/unicode-normalization-0.1.24/README.md)（精确发布包原件） | [许可与通知](../../compliance/licenses/upstream/unicode-normalization-0.1.24)；[来源](../../compliance/provenance/packages/unicode-normalization-0.1.24/Cargo.toml.orig.txt) |

## web-bindings

[当前源码 / API](../../tools/web-bindings/src/main.rs) · [Cargo 清单](../../tools/web-bindings/Cargo.toml) · [当前维护说明](../../docs/tools/web-bindings.md)

| 上游来源身份 | 使用文档 | 许可与来源 |
| --- | --- | --- |
| wasm-bindgen-cli@0.2.129 | [README.md](../../docs/upstream/zxs-bindgen-0.2.129/README.md)（导入时保留的本地说明，非上游发布包文档） | [许可与通知](../../compliance/licenses/upstream/zxs-bindgen-0.2.129)；[来源](../../compliance/provenance/packages/zxs-bindgen-0.2.129/PROVENANCE.json) |
