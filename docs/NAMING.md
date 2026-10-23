# 内部组件命名规范

[文档目录](README.md) · [单仓库结构](monorepo.md) · [命名迁移与来源架构](architecture/source-naming.md)

## 目标与状态

内部组件按功能和职责命名，不再把上游仓库品牌或发布版本嵌入活动目录、Cargo 包名及内部代码标识。来源身份、原始版本、许可证、版权和修改记录完整保留；改名不是重新许可，也不是抹去来源。

本文制定规范，并登记分批实施状态。**下文标为“计划”的映射不代表已经实施或验收**；实际活动路径及身份以 [当前来源清单](../compliance/rust-sources.json)、Cargo manifest 和对应验证记录为准。现有第一方组件已使用 authentication、storage-engine、web-interface、view-macros 等职责名称，不为统一外观重复改名。

这里的“合并”指源码、依赖图、文档、构建及治理入口纳入本仓库，不涉及删除上游托管仓库、修改上游账号或移除许可证。

## 命名规则

1. 目录及 Cargo 包采用小写 kebab-case；Rust crate、依赖别名及模块采用对应 snake_case。类型采用 Rust 的 UpperCamelCase，函数和变量采用 snake_case。
2. 名称表达“是什么”或“承担什么职责”，优先使用领域词及稳定职责。不添加产品名、上游品牌、发布日期或发布版本后缀。
3. 同一组件族按职责使用 `-core`、`-macros`、`-derive`、`-bindings`、`-runtime`、`-parser`、`-format` 等后缀；不为了凑齐后缀拆分现有组件。
4. 已有功能性名称可以保留。base64、sha2、SQLite、HTTP、TLS、Unicode、WASM 等表示编码、算法、数据库、协议或标准；不能因为上游项目同名，就把它们替换成含糊的自造词。`sha2` 的数字表示 SHA-2 算法族，不是软件发布版本。
5. 多个不兼容实现不能仅删除版本后缀后挤入同一目录。先核查 API、类型、特性、宏展开和消费者，再决定合并；必须并存时按实际职责或实现差异命名，不使用 `old/new`、`v1/v2` 或任意序号。
6. 改名应覆盖内部 Cargo 包、库目标、依赖别名、模块及宏生成路径，不能只改文件夹、长期保留一整套上游品牌别名后宣称完成。迁移中的临时别名必须明确标为未完成工作。
7. 正常语义版本继续保存在 Cargo 的 `version` 元数据中；“无版本后缀”不是删除依赖解析所需的版本，也不允许改写上游原始版本记录。

## 领域分层

当前 Foundation 按主要职责采用 `src/foundation/<职能组>/<包名>` 两层布局。219 个包各归入一个父目录，目录分类不等于包合并，也不修改 Cargo 包名、版本或原始来源。完整分类与维护规则见 [Foundation 职能目录](architecture/foundation-groups.md)。

职能组为 concurrency、collections、cryptography、data-storage、networking、platform、text-unicode、code-generation、tooling、diagnostics、core-types、web-runtime。领域专用宏与其运行时同组；通用代码生成工具独立分类。

以下分批迁移表记录当时的命名变更，表中包名不隐含当前父目录；实际路径以当前来源清单和职能目录为准。历史原始路径、来源收据及许可证不随分类改写。

## 首批实施：六个单版本组件

Base64 旧实现经差异验证合并后，这批不存在同名多版本冲突，已实施协调命名迁移。四个组件同时更改包和内部命名；SHA-2 和合并后的 Base64 组件只去掉目录中的发布版本。

| 迁移前目录（相对 src/foundation） | 当前目录 / Cargo 包 | 当前 Rust crate 标识 | 原始身份 |
| --- | --- | --- | --- |
| bitflags-2.13.2 | flag-set | flag_set | bitflags 2.13.2 |
| itertools-0.14.0 | iterator-toolkit | iterator_toolkit | itertools 0.14.0 |
| rand_core-0.6.4 | random-generator-core | random_generator_core | rand_core 0.6.4 |
| sha2-0.11.1 | sha2 | sha2 | sha2 0.11.1 |
| shlex-2.0.1 | shell-tokenizer | shell_tokenizer | shlex 2.0.1 |
| base64-0.23.1 | base64 | base64 | base64 0.23.1 |

这六项均有已分离的测试来源映射；源目录、测试 owner、活动测试路径、manifest 及内部引用应协调迁移。不能修改旧的不可变测试搬迁收据以迁就新名称。

### 精确来源与法律材料

下列归档地址、版本和来源键来自现有 [原始来源收据](../compliance/provenance/origins.json)。归档 SHA-256 与原始逐文件摘要继续由该收据维护；不因命名迁移重新接受未知来源。

| 原始身份与归档 | 不可变 origin_directory | 原始许可材料 |
| --- | --- | --- |
| [bitflags 2.13.2](https://static.crates.io/crates/bitflags/bitflags-2.13.2.crate) | vendor/rust/bitflags-2.13.2 | [Apache-2.0](../compliance/licenses/upstream/bitflags-2.13.2/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/bitflags-2.13.2/LICENSE-MIT) |
| [itertools 0.14.0](https://static.crates.io/crates/itertools/itertools-0.14.0.crate) | vendor/rust/itertools-0.14.0 | [Apache-2.0](../compliance/licenses/upstream/itertools-0.14.0/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/itertools-0.14.0/LICENSE-MIT) |
| [rand_core 0.6.4](https://static.crates.io/crates/rand_core/rand_core-0.6.4.crate) | vendor/rust/rand_core-0.6.4 | [Apache-2.0](../compliance/licenses/upstream/rand_core-0.6.4/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/rand_core-0.6.4/LICENSE-MIT)、[版权](../compliance/licenses/upstream/rand_core-0.6.4/COPYRIGHT) |
| [sha2 0.11.1](https://static.crates.io/crates/sha2/sha2-0.11.1.crate) | vendor/rust/sha2-0.11.1 | [Apache-2.0](../compliance/licenses/upstream/sha2-0.11.1/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/sha2-0.11.1/LICENSE-MIT) |
| [shlex 2.0.1](https://static.crates.io/crates/shlex/shlex-2.0.1.crate) | vendor/rust/shlex-2.0.1 | [Apache-2.0](../compliance/licenses/upstream/shlex-2.0.1/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/shlex-2.0.1/LICENSE-MIT) |
| [base64 0.23.1](https://static.crates.io/crates/base64/base64-0.23.1.crate) | vendor/rust/base64-0.23.1 | [Apache-2.0](../compliance/licenses/upstream/base64-0.23.1/LICENSE-APACHE)、[MIT](../compliance/licenses/upstream/base64-0.23.1/LICENSE-MIT) |

## 第二批实施：界面组件族

DOM 辅助、视图渲染、响应式状态、任务调度、视图分支和共享值六个组件已完成内部协调改名，具体映射和验证见[界面迁移记录](reports/interface-naming-20261021.md)。渲染器内部模块与 SharedValue 类型同步更新，不保留上游品牌兼容别名。

## 第三批实施：共享哈希表

两套 hashbrown 实现完成合并后，存续实现的活动目录和包名改为 `hash-table`，crate 标识为 `hash_table`；原始身份 hashbrown 0.17.1 保留。HashMap/HashSet 等标准功能类型名称不变，hashlink/indexmap 的顺序语义包装不合并进无序表。消费者内部 `from_hashbrown` 辅助名称及所有活跃依赖和特性路径同步更改。当前使用文档见 [hash-table](components/hash-table.md)，上游原始文档保留原样。

## 第四批实施：序列化组件族

七个组件及内部代码、辅助属性和 97 个内部文件路径已协调迁移，详细映射与格式保护见[序列化迁移记录](reports/serialization-naming-20261021.md)。私有支持模块不含版本后缀，但三方版本一致性与精确依赖固定由门禁保证；Cargo 版本元数据不被删除。漏洞审计继续使用原始上游名称和版本，改名不形成审计豁免。

## 第五批实施：语法、排版与 CPU 检测

重复实现合并后，存续组件采用 `rust-syntax`、`rust-source-format`、`cpu-feature-detect` 功能名；依赖、生成代码、内部模块、外置测试和维护门禁同步迁移。来源及契约例外见[迁移记录](reports/syntax-cpu-naming-20261021.md)。

## 第六批实施：错误定义组件族

`error-definition` 与 `error-definition-macros` 协调替换运行时、派生实现及消费者路径。隐藏支持模块统一为 `__derive_support`，由完整版本一致性与精确依赖门禁保护；错误文本、Display、source 和 backtrace 语义不变。原始 thiserror / thiserror-impl 2.0.21 来源保留，验证范围见[组件记录](architecture/error-definition-naming.md)。

## 后续组件族：仅为计划

以下是需要联合评审的候选，不是已落地清单，也不要求把所有有明确领域含义的名称强行替换。

| 现有组件族 | 计划方向 | 必须协调的边界 |
| --- | --- | --- |
| tokio / tokio-macros | runtime package with tokio::adapters / separate proc-macro | tokio-util 已合入运行时；保留属性宏编译边界，adapters-* 特性独立于核心运行时特性 |
| ring | native-cryptography | 原生符号前缀、预生成汇编、构建脚本和 links；不能仅改 Cargo 名称 |

`src/interface` 和 `src/macros` 自身已按职责命名；本批也已协调其重导出和生成的 `web_interface::view_renderer` 路径。内部化同时修改宏生产者和消费者，而不只修改依赖文件夹。

## 必须保留与逐项审查的例外

- 许可证、版权归属、上游 URL、原始版本、归档哈希、原始文档、不可变来源及历史记录保留原样。
- 标准算法、协议、数据库、平台 ABI、系统 API 和格式的名称不因仓库品牌匹配而误改。
- 对外符号、持久化标记、序列化字段、WASM/JavaScript 绑定契约和原生链接命名不可进行无差别替换；必须记录具体契约和保留原因。若确需变更，应按独立兼容性迁移验证。
- 例外必须是确切的来源、法律、标准或互操作边界，不能笼统地以“兼容”为由保留所有内部上游品牌。

完成标准是：活动实现采用一致职责命名，残留标识都有明确归类，来源和行为可验证；不是在整个仓库中把所有上游字符串删到零。

## 合并后的密码学核心

`cryptographic-core` 统一原 digest、block-buffer、crypto-common 的共享接口与缓冲层，内部保留职责模块，不再维护三个独立包。原始来源、版本和许可证分别保留；算法组件及 hybrid-array 不因此合并。详见[密码学合并记录](components/crypto-consolidation.md)。

## Command-line runtime and procedural derives

- `command-line`: one runtime owner for parsing, command construction, help/errors,
  and a private OS-string tokenizer. Builder and lexer are modules, not packages.
- `command-derive`: host-side procedural macros. Rust requires this compiler target
  to remain separate from the normal runtime library.

The four historical upstream origins remain recorded under these two owners.
There are no old-name aliases. See the [consolidation report](reports/command-line-consolidation-20261021.md)
for preserved behavior, removed unsupported capabilities, and archival evidence.
