<a id="cryptographic-core"></a>

# 密码学核心

一个 no_std crate 包含公共类型、哈希/MAC trait 及可选块缓冲。各算法仍保留在独立组件中。

来源为 RustCrypto digest 0.11.3（主来源）、block-buffer 0.12.1 和 crypto-common 0.2.2（合入的实现，而非停用代码）。原始版本、源码快照、文档和 MIT/Apache-2.0 许可证均保留。

缓冲模块包含原有且未改动的 unsafe 操作；哈希/公共模块禁止 unsafe 代码。特性保留 alloc、block-api、mac、oid 和 zeroize。统一的 zeroize 特性启用原有缓冲清零能力及 hybrid-array 的可选清零支持。

<a id="maintained-boundaries"></a>

## 维护边界

生产门禁在 Linux amd64 和 WebAssembly 上检查无特性配置、alloc、block-api、mac、oid、zeroize、dev 及其受支持并集，并检查真正的 --all-features。它还运行原始 block-buffer 测试和保留的哈希序列化向量，包括导出的序列化宏，不改变归档的上游数据。zeroize 特性还提供 hybrid-array 清零实现，但不会添加数组析构时的自动清零或 std 依赖。

新包不再宣称支持未使用的 rand_core/getrandom 特性，也不保留悬空的 rand_core 再导出，因为仓库中没有对应上游提供方。dev 特性现可在没有外部测试数据解析器的情况下运行，并保留固定哈希、XOF、MAC 和序列化辅助能力。未使用的 new_test/new_mac_test 二进制加载宏及 blobby 再导出已移除；过时的 hash_rt_outsize_serialization_test 宏也已移除，因为 digest 0.11 中没有其所需的 VariableOutput trait。它们的精确原始定义仍保存在经过核验的源码归档中。未移除任何算法、运行时可变输出的 BLAKE2 API、重置行为或 I/O 能力。维护探针拒绝已移除的特性名，并验证受支持的 dev 导出。

三个原始源码及测试清单在 compliance/cryptographic-core-source-archive.json.gz 中经过核验。原始 MIT 和 Apache-2.0 许可材料仍保存在 compliance/licenses/upstream。更早适配到 BLAKE2 中的 digest 0.10.7 实现仍是独立的合入来源；本次整合不将其停用或重新归类。
