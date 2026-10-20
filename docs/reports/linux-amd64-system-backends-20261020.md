# Linux amd64 系统与网络后端精简

日期：2026-10-20。基线：`e24776ef6a7f92164d1d8ef7069ee2672c308a01`（独立 ring 测试恢复批次）。

## 范围与实际收益

本批仅退役与 Linux 互斥的三个基础库平台后端，共 **82 文件、601,766 字节**：

| 路径（均位于 src/foundation） | 文件 | 字节 |
| --- | ---: | ---: |
| sysinfo/src/windows | 15 | 150,456 |
| sysinfo/src/unix/apple | 27 | 153,720 |
| sysinfo/src/unix/bsd | 26 | 159,234 |
| socket2/src/sys/windows.rs | 1 | 36,475 |
| mio/src/sys/windows | 13 | 101,881 |

这是源码审查与维护面积的减少，不代表 Linux 运行速度或二进制体积改善。ring、memchr、tokio、cc、所有第三方 manifest、原始名称、许可证、provenance 与 upstream 全文均未修改；没有移除现有 feature。根 Cargo.toml、Cargo.lock 和 workspace policy 保持不变。

## 删除选择链与保留契约

- sysinfo：lib.rs 的 Linux 分支选择 unix，unix/mod.rs 选择 linux；Windows、Apple、BSD 分支互斥。同步移除三类模块入口和 rustfmt 辅助声明。保留 Linux 全部组件、磁盘、网络、用户/组、进程与系统 API，以及 serde、C interface、multithread、unknown-ci、unknown fallback。没有按当前 server 仅启用 system 的事实删减未来 API。
- socket2：Linux 的 cfg_attr 选择 sys/unix.rs；删除 Windows 路径属性和后端。保留完整 Unix 实现、all 功能与公共 socket/sockaddr/sockref API。
- mio：sys/mod.rs 的 Unix os-poll 分支选择 Linux 实现；删除 Windows os-poll 分支和文件。保留默认 epoll/eventfd、Linux 强制 poll 与 pipe 两个 cfg、两种 cfg 同时打开、os-ext 公共 pipe、shell、TCP/UDP/UDS。没有扩大到 kqueue、event_ports 或其他本批之外的后端。

三个库的原生入口统一限定 Linux、x86_64、64 位，不对 Linux libc 环境或 vendor 新加限制。仅 GNU amd64 已运行验证；共享 musl/其他 ABI 代码仍保留，不能据此宣称验收。sysinfo 另保留 wasm32-unknown-unknown 的未知平台表面；mio/socket2 不在产品浏览器闭包，不声称支持 browser 原生 socket。

## 实际编译选择与展开对照

删除前已保存编译输出、dep-info、完整宏展开和全部文件摘要。删除后逐配置对照实际选中路径；除入口分派文件外，选中的实现全部字节不变。展开对照只忽略整行注释、空行和行首尾空白，不移除代码、字符串或文档属性。

| 配置 | 选中源码 | 入口文件 | 字节不变实现 | 展开一致 |
| --- | ---: | ---: | ---: | --- |
| sysinfo system | 17 | 2 | 15 | 是 |
| sysinfo 所有 Linux 功能（不混入 unknown-ci） | 30 | 2 | 28 | 是 |
| sysinfo default + unknown-ci | 22 | 1 | 21 | 是 |
| socket2 all | 5 | 1 | 4 | 是 |
| mio net + os-ext + os-poll | 34 | 2 | 32 | 是 |
| mio force poll | 34 | 2 | 32 | 是 |
| mio force pipe | 34 | 2 | 32 | 是 |
| mio 无 os-poll / shell | 14 | 2 | 12 | 是 |

具体退役路径、原始摘要、每配置的选中路径和保留实现摘要见[机器证据](../validation/evidence/foundation/linux-amd64-system-backends-20261020.json)。来源管理只刷新当前源码收据；所有历史来源记录及已退役身份不变。

## 回归与限制

新增 `.github/scripts/check-linux-system-backends.py`，接入 `check.sh foundation`：

- 直接计算实际生产 guard 的 27 组目标/位宽条件，拒绝 x32、非 amd64、Windows、Apple、BSD、WASI，保留约定的 Linux/unknown 表面。
- 在临时目录构造独立消费者，不向仓库添加第二份 Cargo workspace 或锁文件，不恢复上游 dev 依赖。先离线生成临时锁，再以 `--locked --offline` 构建五模式；这不是根锁覆盖的测试。复用 Cargo JSON 诊断检查器，拒绝任何 Rust warning/error，确认构建成功后执行唯一消费者测试程序；目标 guard 的 rustc 检查另加 `--deny=warnings`。真实根 socket2/mio 单测与 server check 提供生产依赖图补充。
- epoll、force poll、force pipe、两种 force cfg 同开和 shell 五模式，各三项测试，共 15 项通过。
- 验证当前 PID/进程、CPU/内存，以及磁盘、温度、网络、用户和组 API；启用 serde、C interface、multithread、linux-netdevs、linux-tmpfs 和 debug 的编译路径。
- 验证 SockAddrStorage 的 128 字节/8 字节对齐，真实 loopback TCP、UDP、Unix socket pair；验证 mio 网络注册、事件、waker 与 os-ext pipe。没有执行物理设备破坏性操作或绕过沙箱。
- shell 模式验证实际存在的 Events/Interest 公共 API，不虚构不存在的 Poll::new。

额外上游库单测前后结果相同：socket2 10/10、mio 3/3，通过并接入 foundation；sysinfo 69 通过、1 项环境失败。失败为 `common::network::tests::check_ip_networks` 的“至少一个 IP network”断言，删除前后均出现。sysinfo 单测仅在临时 manifest 引入现有 tempfile/serde_json 两项测试依赖，未修改任何仓库 manifest，也未跳过或放宽失败断言。不能把此结果写成 sysinfo 全量通过。

未覆盖：非 GNU ABI 运行、真实浏览器运行、真实硬件/设备故障、上游库完整集成套件。native 消费者通过不替代这些验收。

## 收尾检查

- `manage.py refresh` 与 `manage.py verify --metadata` 通过：339 活动树、64 已退役身份、5 个闭包；所有五闭包与基线相同。
- `check.sh static` 通过；foundation 调度单测确认新消费者与 socket2/mio 单元测试已接入。
- 根工作区 `cargo test --locked --offline --lib -p socket2 -p mio` 通过，13 项。
- `cargo check --locked --offline -p storage-server` 通过，覆盖实际产品依赖链。
- `git diff --check` 通过。未把本次未重跑的整个 foundation/ring 套件记为本批完成。
