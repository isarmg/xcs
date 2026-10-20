# 基础实现的行为兼容复核（2026-10-20）

[报告目录](README.md) · [源码管理](../tools/rust-sources.md)

本次在固定 Rust stable 工具链和既定 Linux／WASM 产品范围内处理基础源码警告。常量改用类型关联常量、显式补充返回值生命周期，以及删除没有消费者的私有代码，不改变现有业务协议。原始许可证、上游文档和 provenance 不参与修改。本报告专门记录容易因“修复弃用警告”而引入行为变化的两项适配。

## IDNA 属性掩码

`src/foundation/idna_adapter/src/lib.rs` 原先通过已弃用的 `to_icu4c_value()` 生成 JoiningType 和 BidiClass 位掩码；当前直接匹配语义属性常量。生产路径不再依赖该数值转换接口。新增回归仍以原表达式为比较基准，对 **1,112,064 个 Unicode 标量**逐一核对两类掩码。此处的 `#[expect(deprecated)]` 只约束测试函数，若弃用诊断消失会产生未满足的 expectation。

```sh
cargo test --frozen --offline -p idna_adapter -p storage-client \
  --lib property_masks_preserve_all_unicode_scalars
```

`storage-client` 提供实际消费闭包以激活 `compiled_data`；测试名称过滤器只执行上述标量回归。基础包不属于 workspace 成员，不能直接对其指定 `--features`。

另用完全相同的当前 ICU／IDNA 依赖分别编译原适配器与修改后适配器，运行保留的 `idna/tests/uts46.rs` 收集器及 Unicode 13 `IdnaTestV2.txt`。原适配器来自基线提交 `faf11044cb7e999ae1c99f141691da3ac1fee802`，SHA-256 为 `b80b2f763904dcf818000bf3fd146da4188cd848e46f4981cc984e2f3990cf7c`。向量文件 SHA-256 为 `ffcf59f6e97d765caa5ac8315e9511ba3b42955a554cbc431e4c016f20d25ca9`。

两份实现均为 **6,386 通过、3 项旧期望不匹配**，通过／失败身份完全一致：

| 向量行 | 旧向量期望 | 原版与改版的共同实际结果 |
| --- | --- | --- |
| 4219 | 包含 U+32931 的 Unicode 域名返回 V7 | 返回该有效 Unicode 域名 |
| 4220 | 对应 ASCII Punycode 输入返回 V7 | 返回与上一行相同的 Unicode 域名 |
| 4405 | 包含 U+32B9A 的 Punycode 输入返回 V7 | 返回解码后的有效 Unicode 域名 |

这三项说明保留的 Unicode 13 语料与当前 ICU 数据存在代际差异，不能把整份旧语料宣称为全部通过。相同依赖下的基线对照及所有标量掩码比较均未发现本次适配导致的差异。没有修改历史向量或为迎合旧期望回退现代 IDNA 行为。

比较方法为：先完成当前 `storage-client` 构建；从上述提交提取原适配器，使用当前构建产物中的 `icu_normalizer`、`icu_properties` 分别编译原 `idna_adapter` 与链接它的 `idna`；将收集器作为普通 Rust 模块，逐个执行 `TestFn::DynTestFn` 并以 `catch_unwind` 收集失败身份。当前适配器重复同一步骤后比较完整结果。原适配器编译时产生的两条弃用诊断是对照基线的一部分，不属于当前产品构建。

## 历史 Crypt 编码

`password-hash` 的 `Encoding::Crypt` 既有语义是历史大端位序的 `Base64Crypt`。虽然上游已弃用该类型，把它直接替换为 `Base64ShaCrypt` 会改变已存储字节。因此当前用一个局部类型别名明确保留旧编码；生产中的这一条 `#[expect(deprecated)]` 有兼容理由，不向整个 crate 放宽弃用检查。

新增固定向量覆盖空输入、零字节、普通 ASCII、非整组长度和高位字节，分别校验编码、长度及解码。代表性结果为 `*** → 8Wce`、`01 02 03 04 → .E61/.`，并验证带填充或换行的非法输入被拒绝。

```sh
cargo test --frozen --offline -p password-hash --lib
```

本轮该包 **36 项通过**。这证明保留编码兼容，不能据此推导任何密码算法已获额外安全认证。

## 数值与私有辅助实现

`num-traits`、`num-integer`、`num-iter`、`num-rational` 的替换保留类型和边界值，没有把泛型 trait 调用改成不同精度的运算。复用原单元测试覆盖有符号最小值、溢出、浮点特殊值、舍入和迭代边界。本轮分别 **51／136／10／52 项通过**；文档示例共 **209 项通过、1 项原有 ignored**。

```sh
cargo test --frozen --offline -p num-traits -p num-integer \
  -p num-iter -p num-rational --lib
cargo test --frozen --offline -p num-traits -p num-integer \
  -p num-iter -p num-rational --doc
```

Blake2 只删除未调用的私有可变字节转换。aHash 保留稳定默认实现与 x86／x86_64 AES 后端，删除固定 stable 工具链无法启用的 specialization／ARM nightly 实验和没有依赖实现的 compile-time-rng；运行时随机源与显式种子接口继续保留。原私有 CallHasher 分派层合并到 `RandomState::hash_one`，保留的两项输入／引用回归针对实际生成的库执行通过。AES 专用读片段函数与真正的 AES 模块使用一致条件，避免将只在硬件后端使用的实现误删为无用代码；`RUSTFLAGS='-C target-feature=+aes' cargo check --frozen --offline -p ahash` 验证硬件后端可编译。

位向量、位集合、arena 与符号解码只明确借用关系和清理语法，复用原单元测试分别 **52／23／1／58 项通过**。当前产品 Web 的 `cargo check --frozen --offline --release -p web-app --target wasm32-unknown-unknown` 也通过，JSON 诊断中警告、错误均为 0。
