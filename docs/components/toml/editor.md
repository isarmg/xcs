<!-- 维护中的示例适配文档。不可变来源与版本记录见 ../toml.md。 -->
<a id="toml_edit"></a>

# TOML 格式保留编辑器

[![Build Status](https://github.com/toml-rs/toml/workflows/Continuous%20integration/badge.svg)](https://github.com/toml-rs/toml/actions)
[![codecov](https://codecov.io/gh/toml-rs/toml/branch/master/graph/badge.svg)](https://codecov.io/gh/toml-rs/toml)
[![crates.io](https://img.shields.io/crates/v/toml_edit.svg)](https://crates.io/crates/toml_edit)
[![docs](https://docs.rs/toml_edit/badge.svg)](https://docs.rs/toml_edit)
[![Join the chat at https://gitter.im/toml_edit/Lobby](https://badges.gitter.im/a.svg)](https://gitter.im/toml_edit/Lobby)


编辑器允许解析和修改 TOML 文档，同时保留注释、空格和条目的相对顺序。当前入口为统一 `toml` 包的 `editor` 模块，能力及验证范围见[组件总览](../toml.md)。

上游 `toml_edit` 主要面向 [cargo-edit](https://github.com/killercup/cargo-edit/) 的需求设计。

<a id="example"></a>

## 示例

```rust
use toml::editor::{DocumentMut, value};

fn main() {
    let toml = r#"
"hello" = 'toml!' # comment
['a'.b]
    "#;
    let mut doc = toml.parse::<DocumentMut>().expect("invalid doc");
    assert_eq!(doc.to_string(), toml);
    // let's add a new key/value pair inside a.b: c = {d = "hello"}
    doc["a"]["b"]["c"]["d"] = value("hello");
    // autoformat inline table a.b.c: { d = "hello" }
    doc["a"]["b"]["c"].as_inline_table_mut().map(|t| t.fmt());
    let expected = r#"
"hello" = 'toml!' # comment
['a'.b]
c = { d = "hello" }
    "#;
    assert_eq!(doc.to_string(), expected);
}
```

<a id="limitations"></a>

## 限制

以下内容不会保留：

* 点分隔键的顺序，见[上游问题](https://github.com/toml-rs/toml/issues/163)。

<a id="license"></a>

## 许可（保留上游原文）

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../../../compliance/licenses/upstream/toml_edit-0.22.27/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](../../../compliance/licenses/upstream/toml_edit-0.22.27/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

<a id="contribution"></a>

### 贡献许可声明（保留上游原文）

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.
