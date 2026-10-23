<a id="source-spanned-values"></a>

# 带源码区间的值

`toml::span` 模块（原 source-spanned-value 组件）将值与其在源文档中的字节区间一并记录。其来源为 serde_spanned 0.6.9。原始[上游 README](../upstream/serde_spanned-0.6.9/README.md)、源码来源及 MIT/Apache-2.0 许可材料均原样保留。

构造或查看此值包装器无需序列化引擎：

```rust
use toml::span::Spanned;

let value = Spanned::new(4..9, String::from("hello"));
assert_eq!(value.span(), 4..9);
assert_eq!(value.get_ref(), "hello");
assert_eq!(value.into_inner(), "hello");
```

serialization 特性实现内部 data-serialization trait。四个特殊序列化标记字符串保持原有格式契约；Rust 组件重命名不改变文档字节或区间语义。
