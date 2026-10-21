# Foundation 项目定制与架构优化（2026-10-21）

本轮基线为 main `c6421fb5cd07d7c7d67bee087d59050d8d83a269`。已有工作区干净，已通过 fast-forward 拉取与 Git push dry-run。此前 Linux amd64 裁剪、后端矩阵、ring 测试材料恢复及 13 项 CI 结果属于先前工作，不计入本轮成果。

## 分析方法与范围

先保存完整根 Cargo metadata（357 包、21 个工作区成员），检查全部 manifest 的普通、构建、开发、可选与目标依赖，再与来源清单的五个入口闭包交叉核对。完整逐来源消费者、特性及目录图见 [可再生成架构索引](../architecture/foundation-consumers.md)。该索引覆盖休眠声明；来源清单的 resolutions 是全工作区特性统一后的保守超集，不能称为单入口运行时精确闭包。

原生只验收 Linux amd64；浏览器 WASM 必需代码继续保留。SQLite 版本、数据库格式、加密正确性路径、G4、字体与几何均不在本轮改造范围。没有部署、发布产品、破坏性物理盘测试，也没有创建凭据。GitHub 提交仅用于已授权的源码版本管理。

## 实际目录与职责

```text
Cargo.toml / Cargo.lock             唯一产品与工具工作区
src/{auth,contract,platform,engine} Linux 认证、协议、系统 I/O、存储
src/{server,client,admin,smb*}      服务与访问入口、Samba SDK 边界
src/{web,interface,macros}         浏览器、CSR 内核、宿主过程宏
src/naming                        项目命名规范与合并的 Unicode 实现
src/foundation/<upstream crate>    可识别、可审计的上游源码单元
tools/{web-build,web-bindings,...}  构建宿主和验收工具
tools/rust-sources/manage.py       来源、路径、功能边、摘要与闭包治理
tools/rust-sources/architecture.py 消费者架构索引生成与漂移检查
compliance/provenance              不可重写原始来源及历史收据
```

按 network/crypto/web 重搬数百个 crate 没有清晰收益：共享库跨域，且路径边和 source_directories 以直接子目录为边界。本轮选择合并真实微型职责，并让维护入口按消费者可查；不通过搬目录制造架构优化。ring 仍是显式验证成员，不因不在 default-members 中被删除。

## 第一批：宏所需纯函数归并

实际引用不止最初候选的两个函数：component.rs 使用 value_to_string；lib.rs 使用 span_to_stable_id；view/mod.rs 还使用 value_to_string 与 is_component_node。全部合并到 `src/macros/src/view_source.rs`，组件标签判定保持最后路径段、ASCII 大写的规则。

移除 leptos_hot_reload 的文件遍历、文件解析状态、视图 diff、patch.js 热补丁基础设施。camino 只有该库一个消费者，因此一并显式退役。serde、indexmap、walkdir 等还有实际消费者，继续保留，不能把取消一条编译边等同于删除整个库。

debug ID 继续逐字符替换正反斜杠，不规范化路径；非 UTF-8 路径继续回退空文件名；行号完全不变。字面量仍只转换字符串、字符、整数和浮点数，布尔值、负数表达式、代码块及字节串不转。边界测试覆盖 Unicode/转义/数字进制、非 UTF-8、空路径、重复分隔符、行号极值与组件标签大小写、命名空间及动态标签。

保留 leptos_hot_reload 的 MIT 身份、法律材料和原始逐文件收据；当前实现作为 src/macros 的额外 merged 来源，另存合并前最后摘要。现有 src/interface、src/naming 的 merged 当前摘要由 refresh 正常更新，原始历史没有改写。

## 后续候选的取舍与依赖证据

| 候选 | 基线 Cargo 反向依赖与决策 |
| --- | --- |
| env_logger → jiff → jiff-core | 仅 web-bindings 的 env_logger::init 消费；时间戳属于构建日志，可收窄 API 后退役，不影响服务日志或存储格式。RUST_LOG 正则和颜色保留。 |
| SQLx migrate → CRC / SHA-2 | sqlx 对 sqlx-core 强制开启 migrate；产品声明 default-features=false，项目无 Migrator/Migrate 消费。必须完整收窄迁移 API 和休眠边，不能留下空 migrate 特性或仅删目录。 |
| 自有工具 sha2 0.10 → 0.11 | web-build、unicode-build 使用摘要 API；需逐项结果验证。基线 sqlx-core 仍消费 0.10，因此仅迁移工具不足以退役旧版。 |
| tracing/test-macros | 属回归材料；删除不是运行时优化，保留。 |

## 重复版本与保留依据

| 库 | 第一批后版本及实际直接消费者 | 保留/收窄依据 |
| --- | --- | --- |
| syn | 2.0.119: axum-macros@0.5.1, bindgen@0.72.1, prettyplease@0.2.37, strum_macros@0.28.0, tracing-attributes@0.1.31, typed-builder-macro@0.23.2, walrus-macro@0.26.0; 3.0.9: async-trait@0.1.92, attribute-derive-macro@0.11.0, attribute-derive@0.11.0, clap_derive@4.6.7, derive-where@1.7.0, displaydoc@0.2.7, futures-macro@0.3.34, manyhow@0.14.1, prettyplease@0.3.0, proc-macro2-diagnostics@0.11.0, quote-use-macros@0.9.0, ref-cast-impl@1.0.27, rstml@0.13.1, serde_derive@1.0.229, synstructure@0.14.0, thiserror-impl@2.0.21, tokio-macros@2.7.2, view-macros@0.8.19, wasm-bindgen-macro-support@0.2.129, yoke-derive@0.8.4, zerofrom-derive@0.1.8, zerovec-derive@0.11.6 | 过程宏 API 代际不同；保持 syn 2/3 的真实消费者边界。 |
| prettyplease | 0.2.37: bindgen@0.72.1; 0.3.0: view-macros@0.8.19 | 分别配套 syn 2/3 AST，不能机械统一。 |
| getrandom | 0.2.17: rand_core@0.6.4, ring@0.17.14; 0.3.4: ahash@0.8.12, jsonschema-value@0.58.6, jsonschema@0.58.6, referencing@0.58.6; 0.4.3: authentication@0.1.0, platform-io@0.1.0, tempfile@3.27.0, uuid@1.28.0, web-app@0.1.0 | 0.2/0.3/0.4 调用 API 与 rand 生态代际不同；保留 Linux 和浏览器必须实现。全工作区 wasm_js 不证明原生产物含 JS。 |
| base64 | 0.22.1: browser-smoke@0.1.0, reqwest@0.12.28, sqlx-core@0.9.0, wasm-bindgen-cli-support@0.2.129; 0.23.1: authentication@0.1.0, hyper-util@0.1.21 | 0.22/0.23 的跨库 API 代际消费者仍存在，不机械替换。 |
| digest | 0.10.7: blake2@0.10.6, sha2@0.10.9; 0.11.3: hmac@0.13.0, sha2@0.11.1 | 0.10 由 Argon2 → blake2 等实际消费；保留密码正确性链。 |
| sha2 | 0.10.9: sqlx-core@0.9.0, unicode-build@0.1.0, web-build@0.1.0; 0.11.1: authentication@0.1.0, platform-io@0.1.0 | 旧版仍有 SQLx 迁移和自有工具消费者；按实际职责评估，不能直接删除。 |

## 本轮量化结果

| 指标 | 基线 c6421fb5 | 当前第一批 |
| --- | ---: | ---: |
| 活动来源树 | 339 | 337 |
| 合并来源身份 | 2 | 3 |
| 已退役身份 | 64 | 65 |
| foundation 文件数 | 11142 | 11129 |
| foundation 文件字节 | 154448935 | 154257981 |

以上只计活动 foundation 目录的源码/测试/材料体积，不宣称相同数值的运行内存、产物大小或编译耗时收益。新 helper、测试、架构文档和保留来源收据另有维护成本；Git diff 及下表分别记录，避免只展示删除行数。

| 保守入口 | 基线来源数 | 第一批来源数 |
| --- | ---: | ---: |
| root-native@x86_64-unknown-linux-gnu | 205 | 205 |
| web@wasm32-unknown-unknown | 107 | 102 |
| smb@x86_64-unknown-linux-gnu | 43 | 43 |
| build-tools@x86_64-unknown-linux-gnu | 85 | 85 |
| validation-tools@x86_64-unknown-linux-gnu | 181 | 181 |

## 验证与边界

- Git：本地已有改动检查、main 精确确认、fast-forward、push dry-run 均已完成。
- 来源与架构：refresh 后执行 verify --metadata；架构索引有声明 alias、optional、target、dev 边的语义回归。
- 首次编译暴露 view/mod.rs 的额外节点判定消费者，已补全，不能把该失败记为通过。后续测试结果随本批提交记录更新。
- 浏览器/WASM、G4 与真实 Samba SDK 的 CI 结果按本轮提交跟踪，上一轮通过不替代本轮验证。
- 本机没有发现 Chrome/ChromeDriver 命令；不会声称本地浏览器测试通过。物理盘故障和产品部署未执行。

### 第一批本地验证结果

- `cargo test --locked --offline -p view-macros -p web-interface -p web-app`：通过，包括 helper 边界、宏文档和真实界面集成测试。
- `manage.py verify --metadata` 与 `architecture.py verify`：通过，原始身份、原始材料及休眠边不变量保留。
- 静态 Python 回归 148 + 86 + 5 项全部通过；mock 归档/发布安全测试及 shell 语法检查通过。
- Node 证据探针 5 项通过。最初 shell PATH 未包含 Node，静态脚本在最后一步退出；使用机器已有 Node 26.7.0 补跑该项通过，没有安装新工具或跳过检查。


### 构建与测试职责地图

| 入口 | 实际对象 / 依赖边界 |
| --- | --- |
| static / rust-sources | 来源摘要、休眠 feature/path、根锁和原始来源不变量、生成索引、脚本安全回归；metadata 阶段使用空 Cargo home 离线重解析 |
| quality | 自有包 fmt/Clippy、全部宿主目标依赖诊断、release WASM 依赖诊断；Samba FFI 留给真实 SDK |
| workspace | Linux 产品 default-members 与大型历史图回归；不执行破坏性盘故障注入 |
| foundation | UAPI std/no_std、libc ABI、rustix raw/libc 双后端、系统/socket/poll、parking、搜索/进程/IPC、熵路径、ring、密码与 Unicode/数值基础回归 |
| web | web-app、web-interface、view-macros 的单元、集成与文档测试 |
| contracts / build-tools | 契约、Unicode、Web 绑定/构建及 mock 发布归档校验；不是实际发布 |
| security | 扫描全部活动与 merged 原始身份；外部 cargo-audit 安装使用独立固定审计锁 |
| web-production / browser-smoke | 真实 release WASM + Web 绑定 → 同一运行资产 → Chrome/WebDriver、G4、字体和响应式视口回归 |
| smb-protocol / smb-sdk | 协议单测与固定 Samba 4.23.0 的真实 SDK ABI/FFI 验证 |

各 `.github/scripts/check-linux-*.py` 在临时消费者里验证保留后端，不是新的活动产品工作区；临时锁不替代根锁。
`backend_environment.py` 的 CARGO_ENCODED_RUSTFLAGS/非空 RUSTFLAGS 拒绝逻辑保持原样，不通过继承 flags 让矩阵失去覆盖。
本轮第一批宏包的全部目标 Clippy 与生产 WASM 分发构建均通过。G4/字体/视口及真实 Samba SDK 仍须由本轮 GitHub CI 给出结果。


## 第二批：构建日志职责收窄

`tools/web-bindings` 是 env_logger 唯一直接消费者，入口只调用 init。当前只需要构建诊断的级别、目标、RUST_LOG 正则及颜色。
删除时间戳 feature、TimestampPrecision/Formatter/Builder 时间戳 API、默认格式字段与写出分支、humantime 实现，以及 optional jiff path 边。
没有留下空 humantime feature 或不再工作的时间戳 setter。消费方显式选择 auto-color 与 regex；服务端日志、SQLite 与认证链没有改变。

根工作区内 jiff、jiff-core 的唯一消费链消失，两者按最后文件摘要及字节数显式退役。原始身份、许可证与 docs/upstream 原始文档继续保留。
外部 cargo-audit 安装锁与原始发布材料中的锁完全没有修改；“退役”只描述产品/工具活动源码，不宣称外部审计工具也没有同名依赖。

新增两个绑定工具可观测行为回归：级别/目标/消息正则共同过滤，输出不含时间戳；显式颜色仍产生 ANSI 样式。
env_filter 的现有测试原先因缺 snapbox 无法编译；审查确认它只用于 15 个错误字符串比较。
保留全部测试用例及全部预期文本，将这 15 处改为标准 assert_eq，不恢复测试框架依赖。
格式器与过滤器 48 + 18 项单测通过，绑定工具 4 项测试通过；两库回归已加入 build-tools CI。

## 阶段累计结果

| 指标 | c6421fb5 基线 | 两批及文档收尾 | 差额 |
| --- | ---: | ---: | ---: |
| Cargo 完整根 metadata 包数 | 357 | 353 | -4 |
| 活动来源树 | 339 | 335 | -4 |
| merged 来源身份 | 2 | 3 | 1 |
| retired 历史身份 | 64 | 67 | 3 |
| foundation 文件数 | 11142 | 11009 | -133 |
| foundation 字节 | 154448935 | 150449202 | -3999733 |

本轮合并一个来源身份，退役 camino、jiff、jiff-core 三个来源；未引入新的第三方实现、凭据或兼容层。
上表是活动源码目录统计，包含随功能退役的上游测试材料，不能当作运行时内存/二进制/耗时收益。
密码、安全、ring、G4、字体、几何和既有存储回归均保留。
对 c6421fb5 的程序化比较确认 origins 记录摘要、原始 provenance/license 全部字节摘要、ring 当前源码及 src/naming/src/interface 既有 merged 当前摘要未变。

### 已分析但本阶段保留的部分

**SQLx 结论：本阶段保留完整 migrate 能力及当前启用方式，不实施只关 feature 的局部裁剪。它不是当前产品运行必需能力；保留是本阶段的验证边界决定，不是兼容旧数据库版本的要求。**

证据：根 workspace 与 src/platform 的 SQLx 声明均为 default-features=false，仅选 runtime-tokio/sqlite-bundled；src/engine 继承根声明。
产品/工具 Rust 源码（排除 foundation）未发现 sqlx::migrate、Migrator、MigrateDatabase 或 migrate! 调用。
`src/platform/src/sqlite/mod.rs` 明确不拥有产品 DDL/migrations；项目 SQLx 访问与可信 DDL 校验并不调用 SQLx Migrator。
当前 Cargo 图仍在 sqlx-core 启用 migrate、crc、sha2：crc 的唯一直接消费者是 sqlx-core，crc-catalog 的唯一消费者是 crc；sha2 0.10.9 还有 web-build 与 unicode-build 两个消费者。
这是实际当前图的证据，不把文本零匹配单独作为删除证明。

| 可量化候选成本 | 当前文件数 | 当前字节 | 判定 |
| --- | ---: | ---: | --- |
| core/migrate + core/any/migrate.rs + sqlite/migrate.rs | 9 | 47732 | 1450 行；不含配置、错误和测试辅助的关联修改，不能当作完整删除清单 |
| crc | 11 | 64354 | 完整移除迁移功能后有条件退役 |
| crc-catalog | 6 | 97138 | 随 CRC 唯一消费链一并评估 |
| sha2 0.10.9 | 24 | 89290 | 移除 SQLx 迁移仍不能退役，两个工具必须另行迁移并核验摘要结果 |

预期收益是减少未使用迁移/checksum 分支的编译输入、源文件和依赖边。是否改善最终链接体积、内存或耗时没有实测证据，不能据上述源码数字作运行时承诺。
风险集中在保留特性的代码清理完整性：`sqlx-core/src/error.rs` 的 MigrateError 变体与转换、`src/lib.rs` 的 migration/testing 模块、
`any/driver.rs` 的驱动构造和数据库 create/drop 回调、Any 连接转换、SQLite Migrate 实现、配置解析都参与该 API 表面。
这不是修改当前 SQLite 文件格式的必要步骤，也不需要引入 v1/v2 或旧版本兼容层；但只删模块或只跑产品默认入口不能验证这些保留特性的完整性。

小幅取消 facade 对 core 的强制 migrate 会让 CRC 两个来源退出登记入口闭包，却仍留下 optional path/feature 边；直接删除这两个目录会违反休眠边检查。
为“保留以后可能用到”而新造一个没有真实用途的验证入口、放松 require_reachable_sources 或留下空 migrate feature，都会损坏本轮既定治理约束，本阶段不采用。
因此这里明确保留完整功能并记录成本，SQLx 的 facade/core/sqlite 同步 API 收窄属于一个独立的存储验收单元，
其准入条件是完整消除迁移 API 与所有休眠边、为 Any/config 负向构建建立证据、核验现有 SQLite/可信 DDL/存储全套，并将工具 SHA-2 迁移作为单独可验证决策。
这一决定保留了可运行的现状，没有制造半退役来源，也不声称整个旧加密栈可删除。
digest 0.10 另有 Argon2 → blake2 消费者，继续保留。syn、prettyplease、getrandom、base64 的代际并存依据见前表与生成索引。

### 验收边界

第二批发布前验证来源/metadata、生成索引、静态门禁、构建工具全套、绑定工具 Clippy 和生产 WASM。
最终 13 项 GitHub CI（含真实 Chrome、G4/字体/视口、Samba SDK、原生存储与后端矩阵）以本轮最后代码提交为准；最终状态与运行链接在交付回报中提供。
本机未运行物理盘故障注入或产品部署，未把上轮 CI 当作本轮结果。


第二批本地实际通过的命令：

```sh
python3 tools/rust-sources/manage.py verify --metadata
python3 tools/rust-sources/architecture.py verify
cargo test --locked --offline --lib -p env_logger -p env_filter
cargo test --locked --offline -p web-bindings
cargo test --locked --offline --doc -p env_logger -p env_filter
bash .github/scripts/check.sh build-tools
bash .github/scripts/check.sh static
cargo clippy --locked --offline -p web-bindings --all-targets -- -D warnings
bash .github/scripts/check.sh web-production
git diff --check
```

静态套件使用机器已有 Node 的 PATH；148 + 87 + 5 项 Python 回归及 5 项 Node 回归通过。
修正了原有 suite 测试对“只有一条 cargo test 命令”的假定：现在分别核对工具套件与日志基础库套件，原有 perf-check 解析覆盖和禁止执行工作负载的断言仍保留。
第一批独立提交为 `c68115eaa9b67797e7521dd93c68c6d1c5cc90d9`，已推送 GitHub；第二批单独提交。


### 活跃文档收尾复核

独立复核发现 env_logger 开头 14 个默认日志示例仍含旧墙上时钟时间戳，已同步为当前默认输出，并保留 INFO 级别的实际宽度补齐。
原始 docs/upstream 文档及许可证字节不变；更新的只是活跃库说明及当前来源摘要。上表累计字节数已包含本次文档收尾。
该收尾单独提交，重新验证格式器单元、文档示例、真实工具日志回归、来源闭包与静态门禁；最终 CI 跟踪收尾提交。


### 结构调整的实际边界

真实的职责归并发生在宏消费者：三个纯 helper 合并入 src/macros，原热重载 crate 被撤销；并非仅改名称或搬目录。
architecture.py 与生成的 foundation-consumers.md 提供按实际消费者查找的架构索引，属于维护工具与文档分组，没有把数百个 crate 物理重排为 network/crypto/web。
foundation 继续采用可识别的上游直接子目录，以保留精确来源边界和现有 source_directories/path 管理收益。
