# Foundation 按职能整合方案

本方案是分阶段架构计划，不是已完成的合并清单。数量依据 2026-10-21 的依赖盘点；最终包数服从清晰边界与验证结果。

## 优化目标

以本项目的正确性、性能和结构简洁为目标，不以方便同步上游为目标。来源边界用于追溯，不要求活跃实现继续保留上游的包划分、重复源码或转发层。可以合并重复实现、重新划分模块和删除无用兼容，但必须保留许可证、来源版本与变更证明。

包数不是单独的验收指标。每项优化应说明解决的实际问题，并以相应的行为测试、依赖隔离检查或测量验证；涉及运行速度、内存、构建时间和产物体积时，应分别提供可复现证据，不能用“合包”代替性能结论。独立接口若能避免循环依赖或不必要的运行时依赖，应保留，而不是为减少一个包强行合并。

## 基线与目标

- 单一 GitHub 仓库、根 Cargo workspace 和根 Cargo.lock，不新增并行源码仓库。
- 盘点时 src/foundation 有 314 个 Cargo 包；整个根解析图有 334 个包，另外 20 个是业务与工具包。
- 来源目录登记的 316 项包含上述 314 项，以及位于其他目录的 view-macros 和 web-bindings。来源记录数量不能直接当作 foundation 包数。
- 第一阶段：下列 28 组同族候选暂定减少 68 个包，规划值为 246；另统一两套系统熵源后为 245。后续验证发现必须独立保留 log 门面，因此比最初图模型的估算多保留一个包。
- 深度整合规划目标：约 150–200 个 foundation 包。约 120–150 个属于更激进研究范围，不作交付保证。
- 职能目录可以归纳为二十来类，但每类可能包含多个 Cargo 包。不要将职能分类数等同于编译单元数。

## 第一阶段候选清单

以下使用盘点时包名标识来源，不预先固定最终功能名称。合并组之外的过程宏、原生绑定和跨层公共类型不自动并入。每组仍须通过功能、构建和来源审计，不能仅凭本清单宣布完成。

1. `digest` + `block-buffer` + `crypto-common`：3 → 1。
2. `clap` + `clap_builder` + `clap_lex`：3 → 1。
3. `tokio` + `tokio-util`：2 → 1。
4. `futures` + `futures-core` + `futures-channel` + `futures-executor` + `futures-io` + `futures-sink` + `futures-task` + `futures-util`：8 → 1。
5. `crossbeam-deque` + `crossbeam-epoch` + `crossbeam-queue` + `crossbeam-utils`：4 → 1。
6. `parking_lot` + `parking_lot_core` + `lock_api`：3 → 1。
7. `http-body` + `http-body-util`：2 → 1。
8. `hyper` + `hyper-util`：2 → 1。
9. `axum` + `axum-core`：2 → 1。
10. `tower` + `tower-layer` + `tower-service`：3 → 1。
11. `url` + `idna` + `idna_adapter` + `percent-encoding` + `form_urlencoded`：5 → 1。
12. `data-serialization` + `serialization-core` + `serialization-error-path`：3 → 1。
13. `toml` + `toml_datetime` + `toml_edit` + `toml_write` + `source-spanned-value`：5 → 1。
14. `jsonschema` + `jsonschema-regex` + `jsonschema-value` + `referencing`：4 → 1。
15. `sqlx` + `sqlx-core` + `sqlx-sqlite`：3 → 1。
16. `anstream` + `anstyle` + `anstyle-parse` + `anstyle-query` + `colorchoice`：5 → 1。
17. `log` + `env_logger` + `env_filter`：3 → 2。保留轻量 `log` 门面，只合并日志后端与过滤器。
18. `tracing` + `tracing-core` + `tracing-log` + `tracing-subscriber` + `matchers`：5 → 1。
19. `proc-macro-utils` + `proc-macro2-diagnostics`：2 → 1。
20. `convert_case` + `convert_case_extras`：2 → 1。
21. `icu_normalizer` + `icu_normalizer_data`：2 → 1。
22. `icu_properties` + `icu_properties_data`：2 → 1。
23. `num` + `num-bigint` + `num-complex` + `num-integer` + `num-iter` + `num-rational` + `num-traits`：7 → 1。
24. `flate2` + `miniz_oxide` + `adler2` + `simd-adler32`：4 → 1。
25. `regex` + `regex-automata` + `regex-syntax` + `aho-corasick`：4 → 1。
26. `time` + `time-core` + `deranged` + `num-conv` + `powerfmt`：5 → 1。
27. `walkdir` + `same-file`：原计划 2 → 1；2026-10-22 实际 2 → 0，唯一消费者改用 std::fs 文档扫描，见 [专项验收](../reports/document-scanner-20261022.md)。
28. `wasm-bindgen-macro` + `wasm-bindgen-macro-support`：2 → 1。

这些候选在当前已解析依赖图上收缩后，没有新增循环依赖；这不代表全部可选 feature、构建脚本、类型一致性和所有目标平台都已验证。现有开发/测试依赖关系必须单独判定，不能把 dev-only 环误当作生产依赖环或忽略新产生的生产环。

实际清单审查已确认日志组的可选依赖回边：过滤器依赖 regex，而 regex 引擎及 memchr 又可依赖 log。把门面与过滤器合并会形成 Cargo 包循环，关闭 feature 也不能修复这种清单级循环。因此保留独立门面，而不通过删除日志诊断或伪装依赖来追求更少包数。

## 必须保留或重新设计的边界

- 过程宏与普通运行时库的导出限制不同，需要宿主机编译；不能把宏直接塞入运行时库。多个过程宏是否同包，要检查其支持库的依赖方向。
- build.rs 的宿主依赖、目标库、原生链接标识和生成文件分别审查。多个构建脚本不能通过直接拼接视为完成整合。
- Linux amd64 服务端与浏览器 WASM 分别验证；保留项目仍需的 no_std、alloc 和可选功能隔离。
- 日志接口不因整合默认引入完整日志后端；Future/Stream 等基础接口不默认引入执行器；序列化基础类型不默认引入所有格式。
- TLS 证书信任、密码学实现、原生 ABI、公共基础类型等边界按风险和依赖方向保留，而不是因为目录相邻就合并。
- 若 A 依赖 B、B 依赖 C，把 A 与 C 合包可能形成新环。应重新划分包或提取接口，不能隐藏依赖来绕过检查。
- unsafe 代码按原有模块边界隔离；合包不能整体放宽原本禁止 unsafe 的模块。
- 具有固定历史证明的主包应最后执行终态合并：先完成其日志、锁和其他依赖的重命名，再绑定异步运行时等主包的最终来源映射，避免后续普通改名破坏已封存证明。

## 来源、命名和目录规范

- 活跃包、目录与内部接口按职责命名，不在名称中附带上游版本号；Cargo 必需的版本字段与精确耦合约束仍保留。
- 合包后每个上游来源、版本、许可证、原始文件与修改记录继续独立可追溯；保留的源码必须登记为合入来源，而不是伪装成已删除来源。
- 测试移入统一 test 目录时保留原始来源归属，维护逐文件映射和可校验清单。旧证明文件与许可证不批量改写。
- 不保留无必要的旧名称别名或旧版本兼容层；协议、算法、系统 ABI、标准术语和历史来源名称不机械替换。
- 业务与库实现放 src，测试放 test；公共模块名按真实功能选择，不把所有符号统一加项目名前缀。

## 每批完成条件

1. 完成真实源码与模块整合，删除冗余 manifest 和 feature 转发，不只移动目录。
2. 核对所有消费者、可选 feature、构建脚本、宏生成路径、公共类型和原生链接行为。
3. 运行对应回归测试、必要的编译失败测试与 Linux/WASM 检查，明确不能在当前环境完成的项目。
4. 来源与测试归属校验通过；根 workspace、锁文件、依赖结构和文档保持一致。
5. 进行质量、静态和文档契约检查，分批提交 GitHub。提交后继续下一项，不等待整个 CI；发现失败及时修复。
6. 最终验收核对最新提交的 CI 状态，不能用较旧提交的通过结果替代。

## 执行顺序与收益口径

先处理密码学公共层、命令行、异步运行时及熵源等边界已审查的组合，并补齐多来源测试迁移支持。随后逐批评审解析、配置、数学和其他同族组合；存在构建、依赖或安全问题时调整分组。

包数量减少表示 manifest、依赖边和维护入口减少，不等于删除同等比例源码，也不保证编译时间、二进制体积或运行速度改善。这些收益需要分别测量。

## Namespace migration boundaries

- Match Rust identifiers and the owning namespace, not arbitrary substrings.
  `lock_api` must never match the suffix of `block_api`; a dependency rename
  must not rewrite `std::time` or another crate's private nested module.
- Audit every changed consumer against actual dependency and module ownership.
  Exact input/output hashes and provenance validation do not prove semantic
  correctness. Family-only tests are not a substitute for real product consumers.
- Compile exported macro paths and representative native/WASM consumers before
  publishing a namespace migration. The synchronization gate also compiles an
  unrelated cryptographic boundary fixture to detect this regression class; it
  does not add cryptography to the synchronization package's dependency graph.
- Review documentation separately: executable doctests need the public crate
  path, while implementation modules may correctly use `crate::`. Preserve
  canonical feature names, protocol identifiers, schema URLs and origin notices.

## Batch authentication of consumed rename history

A single source-naming validation now collects consumed rename lookups and
validates the complete consolidation history once for that call. Each requested
rename still requires its exact embedded history binding, unique successor and
unchanged origin/name identity. Unqueried malformed consolidation records remain
errors; batching does not limit validation to requested rows.

No cache survives the call, and filesystem checks are not cached. A subsequent
operation rechecks manifests, paths, ownership and immutable receipts, even when
the same pathname or Python catalog object is reused. Live source-inventory
verification remains a separate required check. Error ordering may differ when
several records are invalid; the acceptance predicates are unchanged.

Five maintained regressions cover one-pass batching and subsequent manifest,
path and receipt changes, plus malformed unqueried history. The receiving-tree
run passed 182 focused history/naming/consolidation/move tests, independent
baseline/candidate corruption comparisons, full metadata verification and the
consumer-index check. This removes redundant verification work; it is not a
claim of a measured application-runtime or end-to-end build speedup.

## Consumer-led case normalization and dependency removal

The view macro's slot-name normalization now performs one Snake-case conversion.
Previously its `is_case` predicate converted the entire string, then the helper
converted it again when the input was not already canonical. Canonical inputs
still require one conversion. The component-name predicate keeps its exact
conditional Pascal-case semantics; it is not the same double-conversion case.

The only product consumer of `convert_case_extras` needed that equality predicate,
not its broader case-detection API. The macro now uses the existing converter
directly. The unused wrapper is retired with its exact source and external test
bytes preserved by the pinned historical archive, alongside unchanged upstream
origin and license records. Regression cases include Unicode, acronym/digit
boundaries, raw identifiers, and 1,331 boundary combinations against the former
normalization behavior. This establishes one fewer conversion on noncanonical
slot names, not an unmeasured claim about application runtime or total build time.

At that earlier normalization stage, the server's unused direct `tokio-util`
dependency was removed while client `ReaderStream` and transitive consumers
still needed the package. The subsequent runtime integration below replaces
those remaining edges and removes the standalone adapter package.


## Runtime and adapter ownership (2026-10-22)

`tokio` now owns the runtime and its actual adapters under `tokio::adapters`.
`tokio-util` is no longer a package or dependency; client transfer primitives,
reqwest streams, service-stack buffer/concurrency and tower-http use the runtime
implementation directly. The proc-macro `tokio-macros` remains separate because
it executes in the host compiler, not the runtime target.

Core `rt`, `sync`, `time`, and `io-util` features retain their original meanings.
Optional `adapters-*` features deliberately distinguish codec, compatibility,
I/O streams and blocking bridges, task management, timers and keyed tasks from
the similarly named core features. `full` covers the runtime;
`adapters-full` adds the adapter capability set. Minimal runtime configurations
have no futures, bytes or hash-table dependency. Plain synchronization adapters
also avoid bytes; keyed tasks alone opt into hash-table. Browser-WASM checks use
only supported single-threaded runtime features, never native networking,
processes or signals.

The regression gates check minimal and independent feature combinations on
Linux amd64 and browser WASM, ReaderStream buffering and EOF/error fusion,
semaphore fairness/cancellation/permit release, sender reservation cancellation
and close semantics, and keyed-task lifecycle behavior. Original upstream tests
remain preserved as provenance evidence; unavailable upstream test-only helper
packages are not replaced by unsupported claims that every upstream test runs.
The retained transfer primitives are internal native-client functionality; this
does not reintroduce removed Web upload, download or preview interfaces.

The reviewed consolidation records 398 production-file routes and preserves
338 external upstream test files with exact unchanged hashes and explicit
per-origin attribution. This is preservation evidence, not an assertion that
338 upstream tests executed. After this runtime stage the active source count
was 244 (242 foundation packages), with 90 retired and 2 superseded origins; all original licenses,
versions, archive hashes and historical source inventories remain unchanged.

## C binding parser ownership (2026-10-22)

The subsequent [binding expression parser consolidation](binding-expression-parser.md)
folds cexpr into its actual bindgen consumer and replaces the unused generic
nom/minimal-lexical dependency chain with a slice-only implementation. The
clang-sys ABI boundary remains separate. Exact origin snapshots and reversible
external-test transfers retain cexpr attribution; the removed nom and
minimal-lexical production/test bytes are authenticated in complete retirement
archives. SDK-independent and actual-libclang checks have distinct foundation
shards; the existing system-SQLite and Samba SDK gates are not weakened.

## Shared form float formatting (2026-10-22)

[Form float formatting](form-float-formatting.md) now uses the existing zmij
implementation directly. The sole ryu consumer is removed, with all former
source/test bytes archived and authenticated. Positive exponents intentionally
use a plus sign, percent-encoded as `%2B` in forms; this unreleased format has no
legacy-spelling compatibility adapter. Numerical bit round-trips and actual
HTTP form/query consumers are checked independently of archival preservation.

## Consumer-led UUID syntax specialization (2026-10-22)

[UUID format validation](uuid-format-validation.md) replaces decoding whose
output was discarded with a safe exact syntax predicate. The uuid-simd, vsimd
and outref implementations remain available only in authenticated historical
archives. Correctness, real schema consumers and native/WASM boundaries are
validated separately; the measured small predicate cost is disclosed, not
presented as a speed improvement.


## Repeated consolidation provenance

A physical owner that already contains several upstream origins may be an input
to a later reviewed consolidation, as either the primary or a secondary input.
The schema-1 plan keeps one exact current package/external-test snapshot per
physical root. Every current production file still needs an explicit route; each
external test still needs its own bijective, reversible transfer. Inherited origins
are not additional physical copies and must not be classified as retired.

For each already consolidated input, add `input_consolidations[directory]` with:

- `receipt_id`: the content-addressed ID of that owner's immediately preceding,
  unconsumed consolidation receipt.
- `merged_packages`: every exact current catalog merge row for that directory,
  in catalog order, including the full shared current inventory.

The validator checks a bounded acyclic receipt graph, unique consumption of each
predecessor, complete and non-duplicated origin coverage, stable upstream/legal
identities, and exact current input snapshots. Earlier receipts, archives, IDs,
licenses, and rename/move bindings remain unchanged. Current inventory changes
after an earlier receipt are not confused with its historical output snapshot.
Only terminal owners must exist now; all historical consumed roots remain absent.
Every final inherited origin shares the terminal physical inventory.

External-test ownership is inherited through exact `(origin, source, original
source hash)` grants, not broad prefixes. Multiplicity and historical attribution
are retained through later stages; new native tests still require normal source
ownership. Altered source/hash/origin labels, omitted tests, stale before snapshots,
branching/replayed predecessor links, and changed current manifest identities fail.
Ordinary refresh may update current bytes but cannot rewrite the old receipts.

Unsupported transitions remain errors: legacy multi-origin owners without a
consolidation receipt, upgraded inputs, pinned proof-routing successors, the fixed
date-time/schema-validation benchmark relocation, and nested build-script
retirement proofs. Removing/restoring inherited external fragments also needs a
separate authenticated transition; the nested receipt retains their labels and
multiplicity. These cases require their own narrowly authenticated adapters, not
an exemption from provenance checks.

Targeted Python regressions are in `test_nested_consolidation.py`, including
three-stage mixed-origin external transfers and primary/secondary manager
refresh/verify/replay. These prove accounting and receipt integrity, not semantic
equivalence or Rust build correctness.
