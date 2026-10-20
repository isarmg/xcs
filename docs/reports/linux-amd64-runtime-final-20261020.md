# Linux amd64 runtime 实现最终收敛

## 范围与结果

基于正式提交 `5815ea78569719d80e25ab89484ede73bdae1880`（与先前 ring style 锁定树等价），本批退役 60 个明确非目标 OS/架构实现文件，合计 223,354 字节；不是将这些原已由 cfg 排除的代码宣称为 Linux 运行性能提升。保留 Linux amd64 原生，不新增 libc/vendor 限制；浏览器 `wasm32-unknown-unknown` 保留。

| 组件 | 退役文件 | 实现字节 | 保留边界 |
| --- | ---: | ---: | --- |
| mio | 5 | 65,369 | epoll、force poll、eventfd、force pipe、shell、Unix pipe/net 全 API |
| parking_lot_core | 6 | 24,366 | Linux futex、Unix/generic、wasm/wasm_atomic、TSan 检测 |
| memchr | 5 | 49,871 | generic/all、x86_64、wasm SIMD/非 SIMD |
| Tokio | 2 | 16,601 | Unix process/signal/net、全部未来 Linux API、Windows docsrs 壳 |
| getrandom 三版本 | 42 | 67,147 | 历史内核 fallback、Linux raw/libc、custom、rdrand、Web、extern_impl |

逐文件退役原因、删除前摘要、保留叶文件摘要及展开证据见[机器证据](../validation/evidence/foundation/linux-amd64-runtime-final-20261020.json)。不删除任何第三方身份、原始许可证/来源；不触碰 ring 的根工作区验证成员及 69 份材料及其可逆 style 来源证明，不改变 G4 或 SMB 远程名称语义。cc 整体保留。

额外仅对 rustc-demangle 的测试模块增加显式 `use std::panic;`，消除 Rust 1.99 两个 ambiguous_panic_imports 警告；算法与断言不变。原有 foundation helpers 多包命令保持不变，实际通过 58 项 demangle 单测。单独选择工具依赖包不等同于该原始多包消费者图，未因此新增成员或依赖。

## 关键安全边界

- getrandom 0.3/0.4 显式 `rndr`、`efi_rng`、`windows_legacy` 选择现在给出明确退役错误；不能删分派后悄悄换用 Linux 默认熵源。六个负例拒绝意外额外错误及任何 warning。
- Linux `target_env=""` 优先选 linux_raw 的顺序不变。0.2 linux_disable_fallback；0.3/0.4 linux_getrandom、linux_raw、历史内核 fallback、custom 和 unsupported 均实际执行。0.2 没有强制历史回退钩子，不宣称其旧内核回退被强制执行。
- 保留所有 Android 命名的 Linux 共享文件。0.2 的其他 OS getrandom.rs 退役，但 0.3/0.4 的同名 Linux 强制后端保留。
- Tokio net/windows 两文件共 99,449 字节暂保留：Linux docsrs 真正使用其中 public API/文档混合代码，直接删除会破坏文档表面。process 内 docsrs Windows 扩展和 signal stub 保留。
- memchr 内联说明同步修正为维护中的 SIMD 后端；上游原始文档材料不改。无全仓平台字符串机械删除。

## 源码等价与严格离线回归

mio 四模式（epoll/poll/pipe/shell）选中源码和归一化展开不变；parking 默认 Linux 展开不变。parking 原基线在独立 manifest 捕获，并从基准 Git 源再次重建核对。

memchr native std/no-default、Web SIMD/非 SIMD 四份展开仅有两处明确授权的文档文字变化；精确反向替换两段后与原展开逐字相等。no-default 的 x86_64 仍选择 x86 实现，不冒称 generic 在 native 已运行。Tokio full 展开逐字不变。getrandom 19 份 native/Web 展开等价，仅 0.2 归一化两行随退役分派移除的普通注释。

新增三个 portable 检查器接入 foundation：check-linux-parking.py、check-linux-search-process.py、check-linux-random-backends.py。它们先离线生成独立临时锁，再以 `--locked --offline` 构建，使用共享 CargoDiagnostics 拒绝 warning/error，构建成功后执行唯一测试产物。不得将临时 consumer 锁当成根锁/生产闭包证明；Cargo JSON 诊断没有被丢弃。没有生成正式子 crate Cargo.lock。

- parking：2 项 public API consumer，验证 timeout/invalid/最后等待者回调与 wake-before-sleep token；根上游单测 12/12。
- memchr/Tokio：2 项 consumer，标量 oracle、非对齐/边界/空串/正反搜索；真实进程 spawn/output/kill/wait 和 UnixStream pair。
- getrandom：16 native 模式、3 Web 编译、6 显式退役负例全部通过。覆盖空缓冲、多种长度、canary、MaybeUninit 长度、custom 精确填充和错误传播，不以随机值“不相等”作完整路径证明。
- 前批 system/socket/mio 检查器原样保留并重跑五模式，15 项通过；force poll+force pipe 组合也实际执行。
- 根 socket2/mio/parking_lot_core 库测试共 25 项通过；authentication 13 项、storage-contract 18 项、platform payload 专项 3 项通过。原 foundation helpers 完整多包单测通过，rustc-demangle 无歧义警告。

## 已知限制与未声称完成的范围

- platform-io 全套为 52 通过、2 环境失败：两个原有 fs_safety 测试在标准库 UnixListener::bind 临时路径处返回 EPERM。与之前本地基线相同，测试和断言未修改；未跳过、不绕过沙箱，不宣称全套通过。
- parking_lot_core deadlock_detection 原已不支持：当前空 feature 缺 backtrace/petgraph 集成，原源码仍引用它们。五 profile 没有启用消费者，仅 parking_lot 保留转发选项。本批不扩大依赖、不添加假兼容、不宣称 all-features 可用。
- Tokio docsrs 正常构建通过，20 项既存 warning 与基线同序；严格 broken-link 模式仍有原有 RuntimeMetrics 与 AtomicBool 两项错误。它不是严格 consumer 零警告矩阵的一部分。
- 不宣称已执行浏览器、非 GNU Linux ABI、nightly wasm atomics、nightly getrandom extern_impl 或硬件熵源故障注入。保留源码不等于这些环境已验收。
- 本批不重复宣称完整 ring 矩阵已重跑；此前独立 ring 提交的 CI 证据保持独立。

## 治理

当前来源收据刷新并经 `manage.py verify --metadata` 验证：339 活动源码树、64 退役身份、5 真实根闭包。原始身份/来源/法律材料和五 profile 解析 feature/role 保持不变；文件级退役不是删除整个身份。静态回归核对 60 个文件不存在、保留文件摘要不变、未来 API/文档壳存在及熵源显式拒绝规则。

收尾补充：`check.sh static` 全套通过；真实根 `cargo check --locked --offline -p storage-server` 通过。真实根 `cargo check --locked --offline -p web-app --target wasm32-unknown-unknown` 也通过；这仍不是浏览器运行验收。
