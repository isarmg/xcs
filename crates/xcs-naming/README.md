# XCS 命名策略

`PortableNameV2` 与 `PortablePathV1` 在服务器、WASM 和 CLI 使用同一实现。Unicode 数据固定为 16.0.0；不得随宿主机更新。`src/unicode16.rs` 是数据表，`data/` 保存对应原始输入。

表由本项目的 [Rust 生成器](../../tools/unicode-build/README.md)重现；默认检查输入 SHA-256 与输出逐字节相等，修改必须显式执行 `--write`。

Unicode 数据来自 Unicode Consortium 的 16.0.0 UCD（https://www.unicode.org/Public/16.0.0/ucd/），适用 Unicode 数据许可：https://www.unicode.org/license.txt 。NFC 使用固定为 Unicode 16.0 的第三方 `unicode-normalization` 0.1.24 包。其原包名属于依赖标识。

源文件 SHA-256：

- case-folding: `6f1f9c588eb4a5c718d9e8f93b782685e5c7fec872cf05e8e6878053599e09bb`
- unicode-data: `ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f`
- derived-core-properties: `39d35161f2954497f69e08bdb9e701493f476a3d30222de20028feda36c1dabd`
