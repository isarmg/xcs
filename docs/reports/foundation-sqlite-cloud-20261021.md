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


## 第二批：构建工具 SHA-256 统一

SQLx 移除迁移摘要后，旧 sha2 的最后两个消费者是 web-build 与 unicode-build。两工具迁至已有 sha2 0.11.1；该版输出 Array 不实现 LowerHex，逐字节使用 02x 生成固定 64 字符小写摘要，而不是只改 manifest 版本号。
web-build 在同一 helper 中统一文件与目录清单的摘要转换，并使用独立固定的空输入、abc、64 个零字节向量验证。既有复制/清单回归的预期值改为独立固定摘要，不让同一 helper 同时生成实际值和预期值。Unicode 的三个原始输入摘要和生成表全部通过 --check，未修改输入、生成表或密码协议。

sha2 0.10.9 已显式退役，原始材料及最后源码收据保留。digest 0.10.7、blake2、Argon2、旧 block-buffer/crypto-common 和 Argon2 使用的 cpufeatures 0.2 继续保留；它们仍是实际密码消费者，不因另一个消费者消失而删除。
本批后完整根 metadata 为 349 包，活动来源 331，retired 71，foundation 为 10730 文件 / 149390053 字节。构建工具全套（含日志格式/过滤 48 + 18 项）、Unicode 固定输入、静态门禁及来源/metadata 验证通过；生产 release WASM 分发构建及真实 Chrome 全套再次通过（含 G4、字体、六视口、失效/延迟/已释放字体传输及 owner 生命周期隔离）；截图为云端 `/workspace/zxs-evidence/sha2-browser.png` 和 `sha2-viewports/`。

## 第三批：消费者中的标准库整合和两组版本统一

- bindgen 的两个 peeking_take_while 调用迁至已有 itertools 0.14；split 调用迁至已有 shlex 2。保留 bindgen 身份、C 解析、语法树代际及调用方参数失败处理，没有机械改 syn/prettyplease。
- anstream 的五处终端判断直接使用 std::io::IsTerminal。检查全部 manifest 后还发现 winnow/debug 的休眠声明和调用，也同步迁移，之后才退役 is_terminal_polyfill。
- Tachys keyed 列表只用旧 drain_filter 移除 None，返回迭代器立即丢弃，其 Drop 会执行剩余过滤。改为立即执行的 children.retain(Option::is_some)，保留 Some 的相对顺序和活状态；None 不携带用户状态，删除它不会新增用户析构动作。没有使用未消费的 extract_if，也不改 clear/removal/unmount/移动索引等生命周期顺序。
- attribute-derive-macro 的七处 hash! 实际只展开为 HashMap::from 数组，直接在诊断构造处使用标准库。编译真实消费者时发现活跃说明把 conflicts 写成函数形式，而解析器要求 conflicts = [..]，一并修正文档；原始 docs/upstream 不改。
- 五个来源显式退役：itertools 0.13、shlex 1、is_terminal_polyfill、drain_filter_polyfill、collection_literals。无新大 crate，无物理目录搬迁计数，无来源身份混淆。

新增 check-foundation-consumers.py 并纳入 foundation CI，实际测试包含夹在普通字段之间的多组 C 位域（验证 peeking 边界）、带空格/引号的外部 clang 宏定义、Unicode/显式颜色策略/非终端，以及真实 FromAttr 派生的未知单字段/多字段、必需字段与帮助模板、重复和冲突诊断。三个回归全部通过。SQLite 正负表面和 preupdate 头文件生成重新通过；Web/接口/宏全套 236 项通过；构建工具全套、来源 verify --metadata 与静态门禁通过。

直接选择根工作区外、仅作为 build dependency 的 bindgen 运行单测触发 Cargo 1.99 特性解析 panic；anstream 上游库测试还依赖未登记的 proptest。没有声称这两个命令通过，也没有恢复旧测试框架、屏蔽测试或降低门禁；改用上述可重现实际消费者验收，并保留原有材料与日志。宏冲突参数和帮助文本的新测试初稿曾与实际语法/空格不符，按原始实现校正后固定预期，产品诊断没有变更。

| 指标 | a04b781 | 三批后 | 差额 |
| --- | ---: | ---: | ---: |
| 完整根 Cargo 包数 | 353 | 344 | -9 |
| 活动来源 | 335 | 326 | -9 |
| retired 身份 | 67 | 76 | +9 |
| foundation 文件 | 11009 | 10646 | -363 |
| foundation 字节 | 150449202 | 148696710 | -1752492 |

第三批后五个保守来源闭包依次为 root-native 200、web 100、smb 42、build-tools 83、validation-tools 170；与包数、最终链接输入和运行时内存不同，不能相互替代。

第三批真实生产 WASM、Web/接口/宏/绑定工具全目标 Clippy、Chrome/G4/字体/六视口和 owner/计时器/异步回调生命周期全套通过；截图为云端 `/workspace/zxs-evidence/stdlib-browser.png` 和 `stdlib-viewports/`。相对 a04 的程序化比较确认 origin_records_sha256 与全部 provenance/法律材料摘要完全相等。

根 metadata 中有多个版本的名称从 12 组减少为 9 组，额外版本实例从 13 个减少为 10 个：sha2、itertools、shlex 各统一一组。仍保留 base64、hashbrown、getrandom、syn/prettyplease 和密码相关 block-buffer/crypto-common/digest/cpufeatures 的实际消费者边界。base64 的新增 SIMD 默认策略、hashlink 的 HashTable/顺序/析构、getrandom 0.2 错误契约及 syn AST 类型配套均需调用方迁移和独立实证，不在本批只改版本号。

## 后续阶段边界

本次低风险阶段没有实施底层 wasm-bindgen-cli-support 输出模式大裁剪、CSR/SSR/模板职责分离或 web-sys HTML/事件 API 闭包收窄。它们仍是后续独立验收单元：浏览器 template 依赖的 RenderHtml/hydration 必须先分离，Safari/Firefox 真实回退继续保留。JSON Schema 子集仅为候选，没有弱化契约校验。未取消 SMB、gzip、多语言/主题、Unicode 文件名、HTTPS、密码和恢复检查；未改 G4/字体/几何，未部署或发布产品，未创建凭据或执行物理盘破坏测试。
