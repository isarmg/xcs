# ZXS 单仓库整合与集中溯源记录

> 后续范围说明：独立分支演示及其图示、构建和素材采集已于 2026-10-20 移除。本文涉及这些内容的路径、命令、计数及产物仅作历史记录；业务分支规则与存储验收分别保留。见[演示清理记录](demo-removal-20261020.md)。

[文档目录](../README.md) · [当前整体结构](../monorepo.md) · [统一许可与溯源](../../compliance/README.md)

记录日期：2026-10-20。此次改造将业务源码、Web、SMB、示例、构建工具和所需 Rust 基础实现整合到根工作区及其本地依赖图，保留一个根活动锁文件。项目面向用户的入口为 Web 与 SMB，运维命令继续提供服务启动和管理能力。

## 结构和实际合并

- 第一方源码按职责放入 `src/{auth,contract,naming,platform,engine,server,client,admin,web,interface,macros,smb,smb-vfs}`；基础实现统一放入 `src/foundation/`，同名多版本保留必要版本后缀。
- 原独立 CSR 内核包的实现直接合入 `src/interface`，不再通过原内核包依赖；过程宏保留编译器要求的独立目标，统一归根工作区管理。
- Unicode 规范化算法直接合入 `src/naming/src/unicode/`，与固定映射和命名策略在同一组件内部维护；原来的独立规范化包被消除。项目扩展入口按职责命名为 `casefold.rs`。
- SMB 协议与 VFS 分别位于 `src/smb/`、`src/smb-vfs/`，与 Linux 服务、Web 和工具使用同一根锁文件；VFS 的真实 Samba SDK 要求继续保留。
- 示例位于 `examples/branch-system/`，工具位于 `tools/`。正式 Web 和示例生成物分别位于 `build/web/`、`build/branch-demo/`。

这些目录表示模块及编译职责，不是嵌套 Git 仓库或独立工作区。来源归档中的原清单、子锁和 CI 元数据以文本证据保存，文件名为 `*.txt`，原 `.github`／`.cargo` 目录名称改为 `upstream-github`／`upstream-cargo`，避免形成可执行的嵌套工程。原始上游 `AGENTS.md` 归档为 `AGENTS.md.txt`。

## 职责命名调整

按补充要求移除自有目录与内部包的项目名前缀：认证、命名、平台、引擎、服务端与管理入口分别使用 `authentication`、`name-policy`、`platform-io`、`storage-engine`、`storage-server`、`storage-admin`；Web 分为 `web-app`、`web-interface` 和 `view-macros`，SMB 使用 `smb-protocol`、`smb-vfs`。绑定生成工具目录和包名统一为 `tools/web-bindings`／`web-bindings`，其他工具按构建或验收职责命名。

迁移前的包名和目录名保留在不可变来源与许可归档中，用于追溯导入前身份。完整目录与当前包名对照见[单仓库结构](../monorepo.md#按职责命名)。产品可执行入口仍为 `zxs`；已有配置环境变量、公开协议及持久化标识继续保留兼容含义。

## 文档、许可和来源

所有活动说明统一在 `docs/`：自有组件、工具、示例、部署与验收说明分别集中到 `components/`、`tools/`、`examples/`、`operations/`、`validation/`。上游原始说明以字节不变的方式保存在 `docs/upstream/`，不将其历史命令和链接视为当前操作指南。

全部独立许可证、版权、作者通知、补充条款和字体 OFL 全文统一在 `compliance/licenses/`；保留源码文件中的原始版权头。第一方默认 Apache-2.0 与各第三方许可分开说明，合并和集中存放不重新许可第三方代码。

[origins.json](../../compliance/provenance/origins.json) 保留改造前 405 个来源包的上游包身份、版本、仓库／归档地址、原提交、归档 SHA-256、原始逐文件哈希及中央法律材料。原目录到新目录的映射在 [directory-migration.json](../../compliance/provenance/directory-migration.json)，原文档和元数据到中央位置的逐文件映射在 [document-migration.json](../../compliance/provenance/document-migration.json)。第一方文档在修复中央链接前另留不可变原文，映射的 `original_copy_path` 指向该证据。

追加静态审查将 `micromap`／`crc-catalog` 的 SPDX 命名许可树、`cfg_aliases` 的第三方 MIT 通知和 `micromap` 的 REUSE 版权元数据归入统一法律材料；另将包根 CI、编辑器、Nix、任务运行器和发布辅助配置集中归档，保留原字节及哈希。源码测试 fixture、构建脚本、原生构建文件与平台代码继续按实际编译用途保留。

本轮还同步记录了 `serde`／`serde_core` 的 WASM 导入条件修正，详细原始与当前文件身份见 [local-code-changes.json](../../compliance/provenance/local-code-changes.json)。

直接维护源码后保留编译检查，修正 `allocator-api2` 的隐式原始指针引用，以及 `try-lock`、`mime` 的生命周期写法；保留 `zerocopy` 构建脚本实际读取的版本配置。命名组件保留禁止不安全代码的约束，Hangul 转换使用检查后的字符构造，规范化代码通过严格 Clippy 检查。这些当前修改及摘要由活动来源清单记录，原始来源摘要另行保留。

## 验证范围

中央材料迁移检查已执行：3,321 条逐文件迁移记录的原始 SHA-256 全部一致，405 条来源记录引用的所有中央材料均存在。检查使用 `original_copy_path`（存在时）或 `central_path` 核对原字节，不把活动指南的路径修复误认为上游变化。此结果是文件完整性检查，不是许可证法律审查、运行验收或发布证据。

活动文档本地链接经过迁移检查；上游原始说明不改写，由来源哈希门禁检查。允许缺失的本地产生验收日志仍按既有规则单独统计。

本轮已重新执行以下验证：

| 验证 | 实际结果 |
| --- | --- |
| 原生默认成员回归 | 551 个通过结果、0 失败，包含隔离子进程结果；默认跳过手动延迟测量与大规模历史测试 |
| 大规模历史专项 | 超过真实 64 MiB 读取限制的历史编解码专项单独通过 |
| 命名与 Unicode | 20 项通过；全 Unicode 标量、上下文和持久化键兼容性检查通过，严格 Clippy 通过 |
| Web 原生逻辑 | 界面组件 53 项、应用 108 项、过程宏 7 项通过；Wasm 检查通过 |
| 示例 | 19 项通过，含构建收据、静态预览与路径安全检查 |
| Samba SDK | 原有真实 Samba 4.23.0 SDK 下 VFS 7 项通过；默认成员同时覆盖 SMB 协议测试 |
| 文档契约 | 1,304 条本地链接、71 个语义向量、30 条 OpenAPI 路径通过；3 条缺失记录为允许缺失的本地生成证据 |
| 原始源码复核 | 新导入入口从原始归档重新提取两项已修改依赖，校验和与原始逐文件收据一致，活动源码未被覆盖 |
| 来源门禁 | 403 个当前源码组件、2 个实际合并实现、405 个原始身份与 8 个根工作区入口／目标闭包校验通过；三个目标的元数据解析通过 |
| 构建与治理工具 | 工具 37 项、来源治理 Python 53 项、构建脚本 Python 8 项通过；发行归档的集中许可内容及 8 类模拟发布情况通过 |
| SDK 质量 | 真实 Samba SDK 下 VFS 全目标严格 Clippy 通过，生成共享库导出 `samba_init_module` |
| 正式原生构建 | 同一次 `cargo build --frozen --release -p storage-admin -p smb-vfs` 通过；使用无依赖源码缓存的 Cargo home 与既有真实 SDK，CLI 帮助和动态导出检查通过 |
| 全仓库质量 | 第一方格式检查、证据探针和全部选定包的全目标严格 Clippy 通过；Samba FFI 另按真实 SDK 验证 |
| 公告匹配扫描 | 统一锁文件 420 个身份、全部原始上游 405 个身份均无公告匹配发现；未关闭 informational 检查或忽略具体公告 |
| 源码入库规则 | 23,091 项预期源码／文档／法律文件均可进入源码导出；本地状态、缓存和凭据被排除，上游一份测试配置按原始哈希识别 |
| 正式 Web | 空 Cargo home 下锁定离线构建通过；747 个分发文件、21,900 项输入摘要全部与当前文件一致，分发许可与中央原文一致 |
| 真实 Chrome | 最终精简网页与本轮正式 release 服务端的 16 组检查全部通过，覆盖实际上传、传输快照、历史下载、会话隔离、生命周期和字体异常处理 |

原生文件所有者、特殊文件与 Unix socket 测试在沙箱内受限后，使用本机权限重新执行并通过，未放宽业务断言。RustSec 数据库提交为 `b19b2c83c2493961ccb12f9fc4699a00fb2c6c44`（2026-10-20），扫描结果仅表示这些原始身份在该公告库中的匹配结果。空 Cargo home 中仅产生缓存标记，没有 registry 源码、归档或索引参与正式构建。

正式 Web 的 WASM 为 2,204,637 字节，gzip 为 661,917 字节；WASM SHA-256 为 `57822d183ab7f30a0fbf216f217b2232203c6296976ca9604ab31d9c0f331086`。统一工作区后，网页构建显式采用体积优化、fat LTO 和符号剥离，避免继承原生服务的符号设置。真实产物检查确认名称段为 1,671 字节，已消除曾观察到的约 20 MB 符号段膨胀；子进程回归验证即使调用方传入原生 profile 也使用网页专用设置。

本轮正式原生产物如下，均为本地构建文件：

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `target/native/release/zxs` | 19,654,104 | `d89d911c286721815456677a3bab75806bbfc85e7c35b9fda4344d28cf180a4f` |
| `target/native/release/libsmb_vfs.so` | 927,688 | `ef2abaec630fd287c5d8fb9b7fe8498836df39131c12aef8ec2cf3b335266873` |

所有上述运行结论来自本轮目录调整后的执行。源码规则验证针对当前未提交树，未创建 Git 提交或远程发行。

## 尚未关闭的边界

`convert_case_extras@0.2.0`、`drain_filter_polyfill@0.1.3`、`erased@0.1.2` 的原始许可资料复核仍未关闭。固定上游提交树未提供独立原文的事实、来源和已有声明完整保留；本次改造没有补造通知或把这些事项标记为解决。完整说明见 [compliance](../../compliance/README.md)。

真实物理盘故障、全部浏览器、跨客户端 SMB 互操作和正式发布仍须各自提供证据。源码整合、静态来源核验和构建通过不能替代这些验收。
