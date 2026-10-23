<a id="compression-foundation"></a>

# 压缩基础组件

`compression` 提供 Adler-32、DEFLATE、zlib 和 gzip 能力，消费者按需选择特性。

<a id="capabilities-and-boundaries"></a>

## 能力与边界

默认功能集为空。`compression::checksum` 提供安全的标量 Adler-32 实现，
不依赖分配器、标准库或其他依赖。标量校验和模块与 DEFLATE 编解码器均保留
模块级 `forbid(unsafe_code)`。

- `std` 启用基于标准库的校验和 I/O，不引入流格式。
- `codec` 启用无需分配内存的底层 DEFLATE inflate。
- `alloc` 与 `codec` 配合时启用需要内存分配的编解码 API，包括
  deflate 和限制输出大小的便捷解压 API。
- `streams` 启用 `std`、`alloc`、`codec` 和真实的 `crc32fast` 提供方。
  `streams::{read,bufread,write}` 模块支持原始 DEFLATE、zlib 和 gzip。
- `simd` 启用 `accelerated_checksum`，并在编解码器内部使用它。稳定版
  Linux amd64 分派会选择可用的 SSE2、SSSE3 或 AVX2；no-std 按编译期功能选择。
  浏览器 WASM 即使由启用 simd128 的编译器构建，也回退到标量实现。
  不安全的内建指令仍限于原有的局部内核。
- `runtime-detection` 启用 std 和可选的 CRC 运行时检测。发布构建器需将其
  与 `streams` 一起启用。Adler SIMD 分派使用 `std`。
- `block-boundary` 启用编解码器真实的块边界能力。
- `serialization` 通过真实的数据序列化提供方加入现有的编解码器状态适配器，
  并启用 alloc 和 derive；该功能仍为可选项。

`tools/web-build` 的 gzip 资源写入器显式启用 streams、SIMD 和运行时检测。Adler-32 不是密码学校验和。

<a id="streaming-contracts"></a>

## 流式处理约定

read 包装器可能读取超出压缩流末尾的字节；bufread 包装器会保留缓冲区中的剩余字节。
单成员 gzip 解码器在第一个成员结束后停止；MultiGzDecoder 处理串联的多个成员。
调用方仍须完成写入器的收尾，并遵守底层已消耗/已写入计数。
限制输出大小的编解码便捷 API 仍可用于不可信输入。不同 DEFLATE/zlib 流适配器不保证以相同方式诊断截断，应分别遵守其 API 与回归约定。

<a id="removed-unsupported-surfaces"></a>

## 不支持的接口

不提供原生 zlib/C 后端、旧后端选择开关、rustc-dep-of-std 或不稳定的 alloc_io。
Rust 后端不宣称支持仅适用于原生后端的窗口大小、字典和压缩级别调整 API。
Nightly AVX512、NEON 和 WASM-SIMD 模块不在保留的 Linux amd64/WASM 标量
支持范围内。

<a id="origins-and-immutable-attribution"></a>

## 来源

- flate2 1.1.10: https://github.com/rust-lang/flate2-rs
- miniz_oxide 0.9.1: https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide
- adler2 2.0.1: https://github.com/oyvindln/adler2
- simd-adler32 0.3.10: https://github.com/mcountryman/simd-adler32

原始清单、上游文档、许可证及实现归属由集中合规材料管理。

<a id="maintained-validation-and-explicit-gaps"></a>

## 持续维护的验证与明确缺口

`.github/scripts/check-compression-family.py` 在 foundation CI 套件中运行。
它检查 26 种隔离的原生/WASM 能力配置，验证仅校验和/仅编解码器的依赖闭包仍为空，
通过真实的编译器/Cargo 失败确认已禁用 API 和已移除后端功能不可用，运行保留的
单元测试、集成测试及文档测试，测试真实的解码器状态序列化检查点/恢复流程，
并测试实际的 web-build 消费方。
它仅将此包及其测试镜像到临时存储；依赖提供方仍使用真实维护的源码，
不编辑任何受管理的清单。

`test/compression/contracts.rs` 覆盖独立的 zlib/gzip 测试向量、压缩级别和空数据、
格式错误/截断/校验和损坏数据、有界输出、流式 flush 和短缓冲区、串联 gzip 与尾随字节，
以及对齐/分块边界处标量/SSE2/SSSE3/AVX2 结果的一致性。每个 SIMD 内核仅在其原始
运行时/编译期可用性检查允许时才会执行。WASM 仅做编译检查；不声称已执行 WASM
或取得跨机器基准结果。

三个随机化流单元测试模块和 SIMD 随机化后端模块需要尚未纳入仓库的上游
rand/quickcheck 提供方。这些外部测试文件按原始字节保留且未启用，没有替代实现
或伪造提供方。仅适用于原生后端的能力测试和两个仅原生的内存测试也未启用。
这些缺口不计为通过的测试。持续维护的确定性约定测试补充了缺失的上游覆盖，
但不冒充其替代品。

两种标量 Adler 实现有意分开：串行内核与四路内核适合不同的缓冲区范围。消费者无需强制共用同一内核。
