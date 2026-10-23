<a id="pattern-matching-source-family"></a>

# 模式匹配源码族

<a id="package-boundary"></a>

## 包边界

一个 `regex` 包负责四个实现层，依赖方向无环：

- `regex::syntax`：AST、HIR、Unicode 属性、UTF-8 范围和字面提取，需要 `alloc`，不需要匹配器。
- `regex::literal`：Aho–Corasick 和字面预过滤，需要 `alloc`；`literal-perf` 显式启用 memchr 加速。
- `regex::automata`：DFA/NFA 构造与搜索、混合执行、捕获及 meta 引擎，只通过明确能力使用 syntax/literal。
- 根 `Regex`、`RegexSet`、`bytes`：由 `matcher` 启用，复用相同自动机与语法类型，不使用包装或重复 crate。

仅语法消费者使用 `default-features = false, features = ["syntax"]`，Unicode 和标准库能力独立选择。schema 模式转换（原 `jsonschema-regex`，现位于 `schema-validation`）使用语法/Unicode 能力而不启用匹配器。`matchers` 选择 DFA 构建/搜索；`fancy-regex` 选择实际需要的自动机能力；tracing 采集层使用相同自动机类型身份。禁用默认特性的高层消费者必须显式选择 `matcher`。

仅 `dfa-search` 为无需分配的 `no_std`，可搜索预编译 DFA 而不编译语法；`syntax`、`literal` 和构建能力需要分配，不需要 `std`。支持 Linux amd64 和浏览器 WASM。四个来源都无构建脚本，没有新增包环或过程宏依赖。

特性统一是加法合并：同一二进制组合解析器与匹配器会启用二者。解析器隔离只针对独立特性闭包，不承诺所有统一依赖图都隔离。`engine-perf` 保留仅自动机的加速策略，高层 `perf` 保留面向匹配器的引擎选择。减少包数不代表运行加速。

<a id="unsupported-compatibility-removed"></a>

## 已移除兼容接口

不保留 nightly `Pattern` 集成，也不保留 `pattern`、`unstable`、`use_std`、`perf-cache` 兼容特性名。移除的 Pattern 实现在整合收据中逐字保存。`arbitrary` 曾是不可解析的可选提供者，其 derive 属性和专用 impl 已移除，不再暴露坏特性。稳定解析/匹配 API 继续可用。原上游文档和许可证据不可变。

<a id="source-and-license-attribution"></a>

## 来源与许可证

| 层 | 原始包 | 版本 | 来源 | 许可证 |
| --- | --- | --- | --- | --- |
| 根 API | regex | 1.13.1 | https://github.com/rust-lang/regex | MIT OR Apache-2.0 |
| 自动机 | regex-automata | 0.4.18 | https://github.com/rust-lang/regex | MIT OR Apache-2.0 |
| 语法 | regex-syntax | 0.8.11 | https://github.com/rust-lang/regex | MIT OR Apache-2.0；内置 Unicode 数据许可 |
| 字面匹配 | aho-corasick | 1.1.5 | https://github.com/BurntSushi/aho-corasick | Unlicense OR MIT |

manifest 合并实现所需许可义务，各来源记录仍保留独立许可证和版本。原始归档、哈希、上游 manifest、Unicode 通知在 `compliance/provenance`、`compliance/licenses/upstream`。`compliance/rust-sources.json` 中按内容寻址的收据记录每份生产文件及全部 107 份原始外置测试，包括可逆导入修改；没有丢弃测试，各自保留 `source_sha256_before` 与来源。

<a id="verification-and-limits"></a>

## 验证与限制

可运行的原始语法/字面单元套件仍接入实际生产模块。项目 `test/regex-family/retained_regressions.rs` 适配器运行既有高层回归、替换及 fuzz 回归，不改变断言或提供假辅助实现。

完整自动机单元框架仍需要缺失的真实 `quickcheck`、`bstr` 及声明中的 `env_logger` 测试依赖；上游 TOML 汇总套件需要缺失的 `regex-test`。这些是不可运行套件，不是通过的测试，源码继续保留。上游昂贵 fuzz 用例的默认忽略状态保持明确，门禁可显式执行相关回归；Rust API 示例直接供 rustdoc 使用。

<a id="maintained-verification"></a>

## 持续验证

基础 CI 运行 `python3 .github/scripts/check-pattern-matching.py`，构建每项保留原生能力和上述 WASM 边界矩阵，拒绝 regex 包警告，执行保留的语法/字面/高层回归（包含显式运行忽略 fuzz 回归）、文档与真实消费者行为。

临时 manifest/锁文件位于维护源码之外，不用 mock 替代缺失上游提供者。完整自动机属性测试与 regex-test TOML 汇总套件仍因真实提供者缺失而未执行。七份上游说明文档测试保持忽略；不宣称加速或穷尽输入。
