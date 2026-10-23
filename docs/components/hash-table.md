<!-- 面向本地 crate 的维护文档，改编自 hashbrown 0.17.1。原件：docs/upstream/hashbrown-0.17.1/README.md。 -->
<a id="hash-table"></a>

# 哈希表

[![Build Status](https://github.com/rust-lang/hashbrown/actions/workflows/rust.yml/badge.svg)](https://github.com/rust-lang/hashbrown/actions)
[![crates.io](https://img.shields.io/crates/v/hashbrown.svg)](https://crates.io/crates/hashbrown)
[![Documentation](https://docs.rs/hashbrown/badge.svg)](https://docs.rs/hashbrown)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/rust-lang/hashbrown)

此 crate 是 Google 高性能 [SwissTable] 哈希映射的 Rust 移植，经过适配，可直接替代 Rust 标准库的 `HashMap` 和 `HashSet` 类型。

SwissTable 的原始 C++ 版本见[此处][here]；这段 [CppCon 演讲][CppCon talk]介绍算法原理。

自 Rust 1.36 起，Rust 标准库已将此实现用于 `HashMap`，同时使用标准库自己的默认哈希器（见 [`std::hash::DefaultHasher`](https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html)）。此 crate 还可在不含 `std` 的环境使用，例如嵌入式系统和内核；这些上游适用场景不扩大本项目仅支持 Linux amd64 服务端和浏览器 WASM 的产品范围。

[SwissTable]: https://abseil.io/blog/20180927-swisstables
[here]: https://github.com/abseil/abseil-cpp/blob/master/absl/container/internal/raw_hash_set.h
[CppCon talk]: https://www.youtube.com/watch?v=ncHmEUmJZf4

<a id="change-log"></a>

## [变更记录](../upstream/hashbrown-0.17.1/CHANGELOG.md)

<a id="features"></a>

## 特点

以下性能描述来自上游说明，不是本项目的独立测量结论：

- 可直接替代标准库 `HashMap` 和 `HashSet` 类型。
- 默认使用 [foldhash](https://github.com/orlp/foldhash) 哈希器，上游描述其速度显著快于 SipHash。但 foldhash 的 HashDoS 抵抗能力不及 SipHash；若重视此能力，应考虑其他哈希器。
- 上游描述其速度约为此前标准库 `HashMap` 的两倍。
- 内存开销更低：每条目开销为 1 字节，而非 8 字节。
- 兼容 `#[no_std]`，但需要全局分配器及 `alloc` crate。
- 空哈希映射不分配内存。
- SIMD 查找可并行扫描多个哈希条目。

<a id="usage"></a>

## 用法

在 `Cargo.toml` 中添加：

```toml
[dependencies]
hash-table = { version = "0.17.1", path = "../hash-table" }
```

然后使用：

```rust
use hash_table::HashMap;

let mut map = HashMap::new();
map.insert(1, "one");
```

<a id="flags"></a>

## 特性开关

此 crate 提供以下 Cargo 特性：

- `nightly`：启用仅 nightly 可用的能力，包括 `#[may_dangle]`。
- `serde`：启用 serde 序列化支持。
- `rayon`：启用 rayon 并行迭代器支持。
- `equivalent`：允许通过 `Equivalent` trait 自定义比较，默认启用。
- `raw-entry`：允许访问已弃用的 `RawEntry` API。
- `inline-more`：为多数函数增加内联提示，以编译时间换取运行性能，默认启用；实际收益仍需具体场景测量。
- `default-hasher`：编译时使用 foldhash 作为默认哈希器，默认启用。
- `allocator-api2`：支持兼容 `allocator-api2` 的分配器，默认启用。

<a id="license"></a>

## 许可（保留上游原文）

Licensed under either of:

 * Apache License, Version 2.0, ([LICENSE-APACHE](../../compliance/licenses/upstream/hashbrown-0.17.1/LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../../compliance/licenses/upstream/hashbrown-0.17.1/LICENSE-MIT) or
   https://opensource.org/license/mit)

at your option.

<a id="contribution"></a>

### 贡献许可声明（保留上游原文）

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
