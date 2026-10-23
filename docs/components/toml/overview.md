<!-- 维护中的示例适配文档。不可变来源与版本记录见 ../toml.md。 -->
<a id="toml"></a>

# TOML 编解码

[![Latest Version](https://img.shields.io/crates/v/toml.svg)](https://crates.io/crates/toml)
[![Documentation](https://docs.rs/toml/badge.svg)](https://docs.rs/toml)

面向 Rust、兼容 [serde] 的 [TOML][toml] 解码器和编码器。当前统一包的模块、能力和验证边界见[组件总览](../toml.md)。

若需保留格式的编辑或更细的输出控制，可参阅上游 [toml_edit] 说明；当前项目入口为 `toml::editor`。

[serde]: https://serde.rs/
[toml]: https://github.com/toml-lang/toml
[toml_edit]: https://docs.rs/toml_edit

<a id="license"></a>

## 许可（保留上游原文）

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../../../compliance/licenses/upstream/toml-0.8.23/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](../../../compliance/licenses/upstream/toml-0.8.23/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

<a id="contribution"></a>

### 贡献许可声明（保留上游原文）

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.
