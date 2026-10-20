# unicode-normalization

## ZXS maintained runtime extension

This local fork adds `src/zxs.rs` and the exported `nfc_casefold` iterator. It
integrates the existing canonical normalization/recomposition implementation
with the caller's fixed case-fold mapping, allowing ZXS naming keys to stream
NFC → full Unicode 16 case fold → NFC without two materialized intermediate
strings. The original normalization algorithms and Unicode 16.0 tables are
retained. `crates/zxs-naming/src/unicode16.rs` supplies the versioned fold data;
no host Unicode tables or compatibility normalization are substituted.

The upstream source is `unicode-rs/unicode-normalization` 0.1.24, imported from
`https://static.crates.io/crates/unicode-normalization/unicode-normalization-0.1.24.crate`
with SHA-256
`5033c97c4262335cded6d6fc3e5c18ab755e1a3dc96376350f3d8e9f009ad956`.
The existing MIT / Apache-2.0 licenses and copyright notices remain applicable.
This extension is ZXS-owned maintenance, not an upstream release or security
fix. The exact original identity remains in `../upstream-audit-identities.json`.

ZXS regression coverage lives in `crates/zxs-naming/src/key_tests.rs`: every
Unicode scalar and every folding-table entry in multiple combining/Hangul
contexts are compared with the former implementation. The repeatable
`key_profile` release example measures the old and new pipelines on the same
corpus. Both pipelines retain required combining-sequence buffers; the change
does not promise constant memory for unbounded input.

[![Build Status](https://travis-ci.org/unicode-rs/unicode-normalization.svg)](https://travis-ci.org/unicode-rs/unicode-normalization)
[![Docs](https://docs.rs/unicode-normalization/badge.svg)](https://docs.rs/unicode-normalization/)

Unicode character composition and decomposition utilities
as described in
[Unicode Standard Annex #15](http://www.unicode.org/reports/tr15/).

This crate requires Rust 1.36+.

```rust
extern crate unicode_normalization;

use unicode_normalization::char::compose;
use unicode_normalization::UnicodeNormalization;

fn main() {
    assert_eq!(compose('A','\u{30a}'), Some('Å'));

    let s = "ÅΩ";
    let c = s.nfc().collect::<String>();
    assert_eq!(c, "ÅΩ");
}
```

## crates.io

You can use this package in your project by adding the following
to your `Cargo.toml`:

```toml
[dependencies]
unicode-normalization = "0.1.23"
```

## `no_std` + `alloc` support

This crate is completely `no_std` + `alloc` compatible. This can be enabled by disabling the `std` feature, i.e. specifying `default-features = false` for this crate on your `Cargo.toml`.
