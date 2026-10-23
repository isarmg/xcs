<a id="unified-cryptographic-traits-and-blake2-integration"></a>

# 统一密码学 trait 与 BLAKE2 集成

BLAKE2、Argon2、SHA-2、HMAC 及可选的 BLAKE3 trait 集成现共用一个 cryptographic-core 包。它将 digest 0.11.3、block-buffer 0.12.1 和 crypto-common 0.2.2 实现作为内部模块。共享存储表示为 hybrid-array 0.4.15。此前独立的 digest 0.10.7、block-buffer 0.10.4、crypto-common 0.1.7 和 generic-array 0.14.7 依赖栈已不可达，也不再作为包存在。

这不是算法替换。BLAKE2 压缩轮、常量、参数编码、计数器、最终块处理及 SIMD 源码均保留。Argon2 H' 所需的运行时选择输出长度能力仍可用。可选重置、分配输出及 std::io::Write 行为仍可用；过时的 digest::VariableOutput 导入改为 blake2::VariableOutput，不保留旧路径兼容别名。

<a id="imported-implementation-attribution"></a>

## 导入实现的来源归属

缓冲适配器保留 RustCrypto digest 0.10.7 的实现片段。此身份在 BLAKE2 目录中记录为合入来源，不被错误归类为完全移除的源码。其安全公告身份及 MIT/Apache-2.0 许可材料与 BLAKE2 自身来源一同保持有效。

- digest 0.10.7 src/core_api/wrapper.rs → BLAKE2 src/buffered.rs：固定输出缓冲、条件式 Reset、带密钥 trait 和 std::io::Write。不导入 BLAKE2 未使用的 XOF/OID 包装器。
- digest 0.10.7 src/core_api/rt_variable.rs → BLAKE2 src/variable.rs：运行时输出长度包装器及其条件式重置/Write 实现。
- digest 0.10.7 src/lib.rs → BLAKE2 src/variable.rs：VariableOutput、VariableOutputReset 和 InvalidBufferSize。

适配器使用保留 digest 的 block_api/common/array trait，不引入第二套 trait。BufferedHash 实现保留的 CoreProxy 接口。最后活动版本的原始源码及外部测试清单逐字节保存在 compliance/crypto-consolidation-source-archive.json.gz 中；test/ci/test_crypto_consolidation.py 固定其压缩哈希及解码后的逐文件哈希。历史来源记录、许可材料、原始测试源码标签及更早凭据保持不变。

<a id="reproducible-acceptance"></a>

## 可复现验收

运行 python3 .github/scripts/check-crypto-consolidation.py。它只创建仓库外临时 Cargo 工作区，使用本地路径依赖及离线/锁定解析，并拒绝重复的 digest/crypto-common/block-buffer 包或任何 generic-array 依赖。

维护中的 test/ci/crypto_vectors.py 生成器解码全部五份保留的 BLAKE2 二进制测试数据，使用 Python hashlib 独立核验全部 518 个已知向量，并生成有界的确定性 Rust 测试。它增加 1,152 个独立计算的无密钥用例，在十二个输入边界和六种分块模式下覆盖 BLAKE2b/BLAKE2s 的每种有效输出长度，另有 54 个带密钥/个性化/截断用例。它绝不修改原始测试数据。现有 Argon2 KAT 和 PHC 测试使用等价字面数组编译，替代不可用的 hex-literal 开发依赖；原始忽略用例仍保持忽略，不得计为通过。

能力测试覆盖固定/运行时输出、重置 API 遇到非法长度时不消耗状态、带密钥/个性化重置、常量时间 MAC 验证接口、装箱输出和 Write。运行器检查 default、reset、no-default、no-default+reset、no-default+alloc、size_opt 以及原生/WASM no_std 配置。现有 nightly SIMD 源码未改动；本次整合不声称其历史 platform_intrinsics API 可在当前稳定编译器上使用。

<a id="blake2-parameter-limits"></a>

<a id="separate-parameter-validation-fix"></a>

## 独立的参数校验修复

整合后，一项专项修复使可失败的 BLAKE2 new_with_salt_and_personal 构造函数对所有超长 key、salt 和 persona 输入返回 InvalidLength，避免中间长度进入核心断言。BLAKE2b 密钥上限为 64 字节，BLAKE2s 为 32 字节；salt/persona 字段上限分别为 16 和 8 字节。这些上限不随截断输出长度缩小。返回 Self 的底层 new_with_params 构造函数有意保留断言。

维护中的能力测试对每个参数、完整及截断输出的 BLAKE2b/BLAKE2s MAC，穷举从零到原块大小上限再加一的长度，包括各参数同时达到精确最大值及无效最大值的情况。现有独立向量和特性矩阵验证有效密码学输出、重置及 no_std 行为不变。此修复不重写原始快照和来源凭据。

<a id="one-maintained-cryptographic-core"></a>

## 唯一维护的密码学核心

共享接口与缓冲层是一个 `cryptographic-core` crate，而不是三个独立包。其 `common_types`、哈希/MAC 和可选 `buffer` 模块各自保留不同职责。算法仍在现有组件中，`hybrid-array` 仍独立共享。根级拒绝 unsafe 代码，哈希/公共模块禁止 unsafe，仅导入的缓冲模块允许其原有 unsafe 操作。

主来源为 RustCrypto digest 0.11.3。block-buffer 0.12.1 和 crypto-common 0.2.2 是合入的实现来源，不是停用代码。更早的 digest 0.10.7 适配器仍作为 BLAKE2 中独立的合入来源保留。原始版本、源码标签、许可证、清单及不可变快照全部保留。没有过时包别名。这取代了尚未发布的三个名称的中间布局，而非引入链式重命名凭据。

维护门禁测试原始密码学向量和 BLAKE2 错误边界、SHA2/HMAC/BLAKE3 消费者、原生/WASM no_std、十四种直接最小特性组合、启用和禁用 zeroize 时全部十一项原始缓冲测试、显式缓冲/数组清零，以及仅在临时副本中执行的编译器强制安全边界检查。`zeroize` 统一原先可选的缓冲和数组清零能力；`block-api`、`mac`、`oid` 和 `alloc` 仍为可选。未启用的大内存 Argon2 测试 cfg 值已登记，但不会启用这些测试。

测得的结构变化为：三个包和八条外发依赖声明减少为一个包和四条。这是可维护性改善，不是经过测量的运行性能或二进制体积结论。
