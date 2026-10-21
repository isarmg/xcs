# Source-spanned values

The toml::span module records a value together with its byte range in a source document. Its origin is serde_spanned 0.6.9. The original [upstream README](../../upstream/serde_spanned-0.6.9/README.md), source provenance, and MIT/Apache-2.0 license materials are preserved unchanged.

The value wrapper does not require a serialization engine to construct or inspect:

```rust
use toml::span::Spanned;

let value = Spanned::new(4..9, String::from("hello"));
assert_eq!(value.span(), 4..9);
assert_eq!(value.get_ref(), "hello");
assert_eq!(value.into_inner(), "hello");
```

The serialization feature implements the internal data-serialization traits. Its four special serialized marker strings remain the original format contract; Rust component renaming does not change document bytes or span semantics.
