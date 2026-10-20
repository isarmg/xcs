# Linux amd64 基础源码收敛：第二批（2026-10-20）

[报告目录](README.md) · [首批报告](linux-amd64-foundation-20261020.md) · [机器证据](../validation/evidence/foundation/linux-amd64-second-20261020.json)

本批基于首批正式提交 `9eb217008b35720ec423c7ed3284301bac67bdd5`，其树为 `40045ab95f505af1233f5c622a3594ab3504c772`，与验证使用的本地快照 `006ed85ba907c683cd88d8e00c11d99ff70abe06` 完全相同。后续 HTTP/传输业务改动不属于本批；最终提交与 CI 状态需以对应提交记录确认。

## 范围与实际体积

原生边界仍为 Linux、x86_64、64 位指针，不按 libc vendor 额外缩窄；当前实际验收目标仍只有 `x86_64-unknown-linux-gnu`。Linux amd64 的 musl/ohos/uclibc 共享代码保留，但没有把源码保留或 build-script 接收解释成这些 ABI 的构建、链接、运行通过。浏览器的 `wasm32-unknown-unknown` 路径保留，不扩展原生范围。

| 源码树 | 删除文件 | 删除文件字节 | 整树净减少字节 |
| --- | ---: | ---: | ---: |
| rustix | 12 | 134,481 | 141,946 |
| libc | 306 | 3,244,961 | 3,256,339 |
| 合计 | 318 | 3,379,442 | 3,398,285 |

foundation 当前源码由 11,643 文件、155,802,064 字节降为 11,325 文件、152,403,779 字节，净减约 2.18%。整树净值包含分派代码的简化；这不是二进制大小或运行性能承诺。活动来源仍为 339 个、退役身份仍为 64 个、目标闭包仍为 5 个，没有把仍被使用的整个 crate 强行删除。

## rustix：仅收敛架构分派，不改系统调用实现

- 删除 aarch64、arm/thumb、mips 家族、powerpc 家族、riscv64、s390x、x86 的 12 个汇编文件，同时收敛 `arch/mod.rs`、build.rs 和 manifest 条件依赖，不留下指向已删除文件的分派。
- x86_64 汇编字节仍与原始来源哈希一致。完整保留 API features、Linux raw 后端、libc 后端以及 `use-libc`/Miri 选择能力；不按本项目当前启用的少量 API 删除将来可用的 Linux 能力。
- 对实际 `all-apis` 消费者，raw/libc 后端的 rustc 文件集合分别仍为 201/194 个。除 2/1 个入口或分派文件外，199/193 个选中文件逐字节不变。
- 两个后端的完整宏展开代码与基线相同，仅在比较时忽略空行和整行注释。GNU/musl/uclibc 环境值与 `use-libc`、显式 cfg、Miri、`no_linux_raw` 组合的 48 个 build-script stdout 也逐字相同；环境值检查不冒充非 GNU 的真实编译。
- 新增目标边界测试和通过真实 `platform-io` 消费者启用的双后端回归。测试使用根 manifest/锁文件，不恢复上游已移除的开发依赖，不新建未登记 Cargo workspace。

这些证据证明本轮选中源码、分派和被测 ABI/行为未变，不表示每一个系统调用、内核版本、Miri 或硬件情形都已运行覆盖。

## libc：从模块选择链证明删除范围

同步修改顶层、new、Unix、Linux-family、GNU b64/x86_64 和 arch 的模块分派，移除明确不在保留范围中的 OS、GNU 非 amd64 架构及 x32 文件。没有通过目录名推断 `linux_l4re_shared` 可以删除；该共享文件、generic 架构、new/common、glibc、Linux UAPI、musl/ohos/uclibc 路径均保留。

GNU 实际选中的 65 个源文件集合不变，57 个字节不变，8 个只调整入口分派。GNU default/no_std/extra_traits 三种模式及 wasm32 空 libc 表面的完整展开代码/数据均与基线一致。48 个保留目标/配置的 build-script stdout 与原版相同。

新增 [ABI 检查器](../../tools/rust-sources/check_libc_abi.py)，每种 GNU feature 模式对照本机 C 头文件的 345 个布局、字段偏移、常量及 helper 值，覆盖文件/时间/网络/线程/信号/IPC 等 ABI；Rust 编译拒绝 warnings，C 编译使用 `-Werror`。6 个允许配置与 11 个非支持平台的目标 guard 检查通过，并编译 wasm32 空表面。

可在具备相同工具链的 GNU amd64 宿主重跑：

```sh
python3 tools/rust-sources/check_libc_abi.py \
  --compare-revision 9eb217008b35720ec423c7ed3284301bac67bdd5
python3 .github/scripts/check-linux-syscalls.py
```

展开对照仅使用编译器的检查输出，不改变产品构建选项。它不替代运行测试，也不是任意 Linux libc 的 ABI 兼容证明。

## ring：本批只读，完整上游回归仍有基线缺口

ring 的 381 个文件、manifest、算法和生成产物没有修改。机器证据记录实际选中的 28 个输入及其 SHA-256，包括 15 个 C 文件、2 个 Fiat 汇编文件和 11 个 Perl 生成器。

- 原始 build.rs helper 与真实开发生成路径分别重现 12 个 x86_64 ELF 汇编输出和 3 个前缀头文件，两次均 15/15 字节完全相同。GNU/musl/uclibc 的环境值不改变原始 ELF 选择。
- SHA-256 输出由 `sha512-x86_64.pl` 根据输出文件名再次生成。不能因为没有独立 SHA-256 Perl 文件或某个名字含其他平台就盲删。
- 实际 C 预处理闭包观察到 30 个编译输入、54 个本地依赖文件，但它不是删除允许清单。共享 C/头文件、无硬件加速后备实现、测试源码和 x86_64 translator 必须保留。
- `RING_PREGENERATE_ASM=1` 是遍历全部 10 个 ASM_TARGETS 的打包流程，不是当前目标生成器。未来任何裁剪必须同时处理 RING_SRCS、ASM_TARGETS、SHA-256 合成、打包 include 和完整生成流程。
- 在 GNU 当前 CPU 上，308 组独立对照向量、9 项 crypto smoke 和 5 项现有可运行集成测试，在预生成与本地生成两模式通过；no-default-features 库构建通过。这只是 smoke，不是完整上游回归。

完整 ring 库测试在基线即因 34 个不同 vector 文件缺失产生 42 个 include 错误，默认 features 和 `std` 两种尝试都没有执行完整单元测试。这些文件也不在导入的上游发布包逐文件 checksum 清单中，因此没有把缺口误记为本项目裁剪造成。准确路径已列入机器证据；不能用少量外部向量替代它们后宣称全面覆盖。musl 运行、CPU 特征降级矩阵和完整上游套件均未执行。

后续恢复测试材料应作为独立批次：

1. 只从原始 manifest 指向的官方 ring 仓库获取已记录 commit `2723abbca9e83347d82b056d5b239c6604f786df`，核验 Git 对象及获取材料的哈希。原 provenance 的 `dirty: true` 意味着不能假设该 commit 与发布包全字节一致。
2. 对所有重合的测试、算法、C/汇编与生成器文件逐一核对已保存的原始哈希；差异未解释前停止。为恢复的 fixtures 单独登记来源、commit/blob、长度、SHA-256 和关联测试源码哈希，不改写原始 receipt。重新下载同一 crate 不能找回原本未打包的 fixtures。
3. 在隔离副本中恢复精确缺失材料和必要的真实上游测试入口，完整运行默认/标准、debug/release、预生成/本地生成、慢测试及目标/CPU 矩阵，再讨论 ring 源码裁剪。本批没有下载或恢复这些材料。

## 验证与来源记录

本批验证使用 Rust 1.99.0。`manage.py verify --metadata` 已通过，仍为 339 个活动树、64 个退役身份、5 个闭包，逐目标空 Cargo home 离线解析。

原始 origin/provenance/逐文件哈希、中央许可证、upstream 身份、合并记录及全部既有退役记录均保持不变。根 Cargo.lock、policy 和五个 resolution 记录字节不变。当前收据仅更新 libc/rustix 两个源码树，manifest 摘要只有 rustix 改变；生成的来源文档同步说明本批差异。

libc 库单测 17/17 通过；platform-io 的 52 项定向回归通过。本机两个已知需 `UnixListener::bind` 的特殊文件测试受执行沙箱限制，在这次定向运行中明确过滤，不能记成 54 项完整通过；CI 的原套件没有添加 skip，也没有绕过沙箱。

| 收尾检查 | 结果 |
| --- | --- |
| `check.sh static` | 来源工具 81 项、CI 脚本 50 项、性能脚本 5 项通过；shell/发布 mock/Node 静态门禁通过 |
| `check.sh foundation` | 435 项 Rust 测试通过，另有 libc 三模式各 345 项 C-header 对照、17 个目标 guard 配置及 wasm 空表面编译 |
| `cargo clippy --locked --offline -p platform-io -p storage-client --all-targets -- -D warnings` | 通过，没有编译警告 |
| `cargo check --locked --offline --release -p web-app --target wasm32-unknown-unknown` | 通过，没有编译警告 |
| `cargo test --locked --offline -p storage-client -p storage-contract --lib` | 客户端 20 项、契约库 18 项通过；客户端启动的单个子进程测试不重复计数 |
| `cargo run --locked --offline -p contract-check -- --root .` | 文档/契约检查通过，不作为平台运行验收 |
| `git diff --check` | 通过 |

foundation 套件仍输出未改动的 rustc-demangle 测试中两条既有 `ambiguous_panic_imports` 警告；58 项对应测试通过。没有以全局 lint 放宽掩盖它，也没有把整个上游测试集合声称为零警告。本批新 ABI/目标测试与消费者编译使用独立的严格门禁。

以上检查不代替浏览器、真实 Samba SDK、CPU 矩阵或硬件故障验收。
