# 云端 Foundation 收窄：SQLite 与实际消费者（2026-10-21）

实施基线锁定 `a04b7815b1419bbce2fa90fe067201e452e52d8a`，云端克隆时远端 main 与此完全一致。只在云端 `/workspace/zxs` 工作，不读取或导入用户电脑的未提交文件，不改配置中其他仓库。本文接续 [前轮风险分析](foundation-customization-20261021.md)，其“本阶段保留 migrate”是历史阶段结论，本批已经实施完整移除。

## SQLite 专用 SQLx

- facade/core/sqlite 同步删除 Migrator、迁移数据库管理、校验和、迁移错误与转换，以及关联测试数据库管理和迁移材料。
- 删除 Any 全部驱动及连接/结果/Describe 转换、AnyDriverError；删除 TOML 配置解析、配置应用 trait/实现、配置错误、宏类型偏好和配置公开入口。
- 删除 SQLx 的网络/socket/TLS 实现及错误、所有运行时选择 feature 和 async-io/async-std/smol 分支。池直接使用 Tokio 调度与公平信号量，SQLite 的原生线程/通道连接工作不改；无需 Tokio 的同步直接连接契约继续由 platform-io 回归验证。
- facade 直接依赖 SQLite 驱动，不再经 optional driver 与 `_sqlite` 空标记选择。消费者去掉 runtime-tokio 声明；sqlite-bundled 仍实际转发到原有 libsqlite3-sys/bundled。
- 行诊断格式化独立为 value_debug，保留原有类型选择顺序、NULL 和解码错误行为，不再依赖 query 宏配置；查询计划日志归入 connection，仅由仍有真实实现的 offline Describe 能力编译。正常查询日志直接使用 core 的 QueryLogger。
- CRC、CRC-catalog、tokio-stream 的全部活动/休眠 path 边消失，显式退役并保存最后逐文件摘要、字节与原因。原始 provenance、许可证与上游身份不改。

保留 Pool/Transaction/Query、SQLite URL/连接选项、参数绑定、缓存、SQLite 工作线程、查询日志、SqlStr 来源安全约束和平台可信 DDL。SQLite 3 的 C 实现、文件格式、平台安全 PRAGMA/defensive mode、项目初始化和恢复代码没有改动。保留仍有实现的 SQLite 类型/扩展选项与 offline Describe，不把移除通用配置等同于更换数据库协议。JSON/UUID 等未被产品选择的特性不宣称为运行时依赖。

## 验证策略

`.github/scripts/check-sqlite-scope.py` 使用临时离线实际消费者：先正向编译保留 SQLite 表面，再分别验证 8 组已删 API 与 15 个已删 feature 被拒绝，最后编译保留的 SQLite 类型/扩展/offline 表面。临时消费者不是活动来源入口，不改变五个登记入口、require_reachable_sources、来源/feature 门禁或根锁职责。

新增 platform-io 集成回归覆盖池容量和超时、事务 drop 回滚、嵌套 savepoint 回滚、Unicode 文本、语句缓存逐出/清空、关闭池拒绝获取，以及行诊断文本/NULL/blob/浮点及错误解码。保留原有真实连接安全、可信 DDL、初始化、WAL、恢复与存储图回归。

本批本地通过：platform-io/storage-engine 全套 348 项（含 2 项新增集成测试）、SQLx 两库 6 项单测、platform-io/storage-engine 全目标 Clippy、正/负向表面构建、来源 verify --metadata、静态 Python 148 + 87 + 5 项及 Node 5 项、生产 release WASM 分发构建。

首次并行验证时修改了构建输入，Web 构建正确拒绝了不稳定来源快照；冻结源码后重新构建通过。首次存储文档检查曾遇到并行修改期间的中间依赖产物，稳定源码完整重跑通过。保留特性扩展编译发现计划日志缺少 parse_query_summary 导入，补齐后正向编译通过。云端 libclang 缺资源头文件，使用现有 GCC 14 的 stdarg.h include 路径完成 preupdate-hook 编译；未改写系统或放松编译检查。

| 指标 | a04b781 基线 | SQLite 批次 |
| --- | ---: | ---: |
| Cargo 完整根 metadata 包数 | 353 | 350 |
| 活动来源树 | 335 | 332 |
| retired 身份 | 67 | 70 |
| foundation 文件数 | 11009 | 10754 |
| foundation 字节 | 150449202 | 149479343 |
| root-native / web / smb / build-tools / validation-tools 来源数 | 205 / 102 / 43 / 85 / 181 | 201 / 102 / 43 / 83 / 178 |

以上体积差额只表示维护输入减少，不作为 WASM 大小、运行内存或性能收益。原始来源、许可证、G4/字体/几何不改。云端真实 Chromium 154/ChromeDriver 154 浏览器套件通过：G4、字体、六种响应式视口、传输记录、快照 DOM/details/focus 保持、真实预检错误和流式下载会话隔离。首次执行隔离上下文内 Chrome 因 SUID 沙箱不可用退出；在同一云端环境的常规执行上下文保留浏览器沙箱运行后通过。截图位于云端 `/workspace/zxs-evidence/sqlx-browser.png` 与 `sqlx-viewports/`。GitHub 13 项 CI 最终结果按精确提交 SHA 回报。
