<a id="retained-native-c-build-toolchain"></a>

# 保留的原生 C 构建工具链

内部 `cc` 组件在 Linux amd64 宿主机运行，接受 Linux amd64（不含 x32 ABI）及 `wasm32-unknown-unknown` 浏览器编译目标。这是本项目的支持边界，不表示上游 cc 不支持其他平台。

<a id="exact-consumers-and-retained-behavior"></a>

## 实际消费者与保留行为

- `ring` 使用 C 和 GNU 汇编编译原生密码实现。
- `blake3` 使用原生 x86 汇编及编译器能力探测。浏览器 SIMD 实现为 Rust，不由 C/Clang 构建。
- `libsqlite3-sys` 在选择内置 SQLite 时使用 `cc`。
- `bindgen` 与 `cc` 都使用保留的 `shell-tokenizer`。Samba SDK 桥接需要 bindgen 及其附加 Clang 参数解析，不能将任一消费者替换为按空白切分。

GNU/Clang 发现、显式编译器/归档器覆盖、编译器包装器、编译器族探测、支持参数探测、汇编参数、C++ 处理、带引号参数分词和浏览器 Clang 目标选择均保留。不能因为某个上游分支提到其他平台，就删除共享的编译器族识别或参数构造逻辑。

<a id="removed-capability"></a>

## 已移除能力

`find-msvc-tools` 在工作区内除 `cc` 外没有消费者。Windows registry、COM、Visual Studio / Windows SDK 发现、MASM 分派与 MSVC 归档器发现已移除。不支持的编译目标明确失败，不回退为 Linux 宿主机编译器。

没有保留的消费者启用 cc 的 `parallel` 特性。原声明本就不可用，因为实现引用已不存在的 jobserver 包。未使用的 parallel/jobserver 特性、可选 libc 依赖边、模块声明及活动实现分支已移除，而非重新引入依赖。八份原始 `src/parallel/` 文件仍逐字节原样保留，不能到达，也不参与编译。物理退役这些历史源码仍等待单独审阅的通用来源机制：现有工具允许整个包退役，不支持这些局部删除。不得伪造合并或放松源码删除检查。C 编译仍串行，与变更前所有保留消费者一致；Cargo 仍可正常调度独立包。

<a id="preservation-and-regression"></a>

## 保留与回归

最后版本的完整 `find-msvc-tools` 源码和外置测试保存在固定退役归档中，由历史来源验证器检查。原始上游身份、元数据、文档与许可证不变。专用化前完整 cc 源码和外置测试位于 `compliance/cc-1.8.0-specialization.json.gz`；原生工具链检查验证固定摘要、逐文件哈希和仍在仓库中的许可证字节。

`python3 .github/scripts/check-native-c-toolchain.py` 属于 `foundation-platform` 分片。它验证真实原生 BLAKE3 已知答案，使用带空格的参数及路径编译 C/C++/GNU 汇编并链接执行，检查支持/不支持的编译器参数，拒绝不支持的目标与已删除 Cargo 特性。Rust 构建使用共享严格诊断检查器。它还运行 cc 保留的单元测试和文档测试，检查显式 CC/CFLAGS 覆盖、长编译器 stderr 行的完整转发，并固定不可达 parallel 源码字节。这不是 Samba SDK 或 WASM C 编译器测试；后两者必须使用实际配置的 SDK 或 Clang 工具链。
