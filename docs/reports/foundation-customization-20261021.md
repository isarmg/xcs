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
