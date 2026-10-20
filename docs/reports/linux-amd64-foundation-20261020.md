# Linux amd64 基础源码范围收敛（2026-10-20）

[报告目录](README.md) · [源码管理](../tools/rust-sources.md)

原生产品及构建宿主仅维护 Linux amd64，当前登记及验收目标为 `x86_64-unknown-linux-gnu`；其他 Linux amd64 ABI 的必要共享源码仍保留，但不承诺未经执行的 ABI 验收。浏览器继续生成 `wasm32-unknown-unknown`，不把 Web 构建目标解释为额外原生平台。当前五个入口闭包为 native、Web、SMB、构建工具、验证工具；移除原先两个 aarch64 元数据闭包。

## 删除依据与保留范围

收敛前 338 个 foundation 包均被至少一个 amd64、Web 或维护工具入口使用；仅删除 ARM 元数据目标不会让任何整包自然不可达。因此没有按目录名或包数量批量删库。首批退役后剩余 337 个 foundation 包；包含两个已整合来源树在内，当前来源清单登记 339 个活动树、64 个退役身份。

本批从 foundation 删除 1,255 个文件；当前源码从 174,264,249 字节降到 155,802,064 字节，净减 18,462,185 字节（约 10.59%）。这些是源文件实际字节，不是二进制大小、运行时内存或性能提升承诺。

- `vcpkg`：SQLite 的 MSVC 库发现器。移除 `libsqlite3-sys` 的构建依赖与两个 feature 转发、SQLx 的 feature 请求，以及 build.rs 中的 MSVC 调用；保留 bundled SQLite、显式 Linux 库路径和 pkg-config。实际移除 818 个源码/测试输入文件，2,450,563 字节。最后源码收据、原始 archive 身份、中央许可证和通知索引继续保存，退役不表示删除版权来源。
- `linux-raw-sys`：逐一解析完整 feature/cfg/path 模块声明，移除 437 个不能由 Linux amd64 LP64 选择的生成绑定文件，15,964,431 字节；同时收敛模块入口和 C 类型 ABI 分派。完整保留 23 个 x86_64 API 文件和共享 elf.rs，字节逐一匹配不可变的原始文件哈希。保留全部 x86_64 API features，包括当前尚未启用的 io_uring、网络、进程、挂载相关底层能力；没有按本轮默认 features 过度删减未来 Linux 能力。
- 原生入口在 platform-io 中明确拒绝其他 OS、架构和非 64 位指针 ABI；Samba VFS 在 SDK 发现前检查 OS、架构和指针宽度。公共 contract/client 和 Web 代码不加入这类原生 guard。
- ring 共享汇编/生成器、rustix/libc 共用 ABI、SQLite 实现与算法安全分支不在这一批盲目裁剪。当前清单、保留来源、当前源码摘要与原始来源摘要分别维护。

## 验证边界

原始来源与中央法律材料保持原始字节。本报告测试结果仅针对本轮本地源码，不表示所有 Samba SDK、浏览器客户端、硬件故障或正式发布已经验收。

本轮使用 Rust 1.99.0，以业务快照 `f3fb93db42d01b4db0cc6f97fbb7e94bb6986d6c` 为基线验证本批变更；最终提交和远端 CI 状态由后续提交记录确认。

| 检查 | 本轮结果 |
| --- | --- |
| `manage.py verify --metadata` | 339 个活动树、64 个退役身份、5 个入口闭包通过；每个目标使用空 Cargo home 离线解析 |
| `check.sh static` | 来源工具 76 项、CI 脚本 46 项、性能检查脚本 5 项通过；shell 语法、发布打包与 8 种模拟发布、Node 5 项及 26 个历史模块静态检查通过 |
| `check.sh foundation` | 426 项通过，包括全部保留 UAPI 的 std/no_std 各 3 项；新 UAPI 编译严格拒绝 warnings |
| `cargo clippy --locked --offline -p platform-io -p storage-client --all-targets -- -D warnings` | 通过，未输出编译警告 |
| `cargo check --locked --offline --release -p web-app --target wasm32-unknown-unknown` | 通过，未输出编译警告；不是浏览器运行验收 |
| SQLite / UAPI 默认库回归 | `libsqlite3-sys` 1/1、`linux-raw-sys` 2/2 通过 |
| `platform-io` 库回归 | 52/54 通过；2 项现有特殊文件测试在临时 Unix socket 的 `UnixListener::bind` 处因执行沙箱 EPERM 被阻塞，未改写或跳过测试 |
| Samba 目标前置检查 | Linux amd64 64 位指针接受；aarch64 在 SDK 发现前拒绝。该检查不代替真实 SDK 构建 |

本轮基础套件仍报告未改动的 `rustc-demangle` 测试代码中两条既有 `ambiguous_panic_imports` 警告；58 项测试通过。这不属于本批生成绑定或产品消费者的编译警告，也没有用全局 lint 放宽隐藏它。

当前来源收据只刷新活动实现和已验证退役状态；逐字段复核确认原始身份、原始文件哈希、中央资料/许可证、已合并来源及之前 63 个退役记录未变。中央 NOTICE 索引继续保留 vcpkg 的原始法律材料，当前活动/退役状态由来源清单及生成文档说明。根锁的变更仅移除 vcpkg 包和指向它的构建依赖边。
