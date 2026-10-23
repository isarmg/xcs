<a id="unified-cryptographic-traits-and-blake2-integration"></a>

# BLAKE2 适配器来源映射

本页是来源清单引用的当前代码对应关系。公共接口、参数限制和检查命令见[密码学核心](cryptographic-core.md)。

<a id="imported-implementation-attribution"></a>

## 导入实现的来源归属

BLAKE2 缓冲适配器包含 RustCrypto digest 0.10.7 的实现片段；该版本是合入来源，不是完全移除的源码，其安全公告身份与 MIT/Apache-2.0 许可继续有效。

| 原始源码 | 当前 BLAKE2 源码 | 保留能力 |
| --- | --- | --- |
| `src/core_api/wrapper.rs` | `src/buffered.rs` | 固定输出缓冲、条件式 Reset、带密钥 trait、`std::io::Write` |
| `src/core_api/rt_variable.rs` | `src/variable.rs` | 运行时输出长度包装器、条件式重置/Write |
| `src/lib.rs` | `src/variable.rs` | `VariableOutput`、`VariableOutputReset`、`InvalidBufferSize` |

当前文件位于 `src/foundation/cryptography/blake2`。适配器使用 cryptographic-core 的 block_api/common/array trait，不引入第二套 trait；不包含 BLAKE2 未使用的 XOF/OID 包装器，也未导入密码学压缩算法。

原始源码及外部测试清单逐字节保存在 `compliance/crypto-consolidation-source-archive.json.gz`；`test/ci/test_crypto_consolidation.py` 核验压缩及逐文件哈希。历史来源凭据、原始测试源码标签和许可证由合规目录保管。

<a id="blake2-parameter-limits"></a>

## 参数约束

见[当前 BLAKE2 参数限制](cryptographic-core.md#blake2-parameter-limits)。
