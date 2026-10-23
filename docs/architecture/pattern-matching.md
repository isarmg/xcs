# Pattern matching source family

## Package boundary

One `regex` package owns four implementation layers. Their dependency direction is acyclic:

- `regex::syntax`: AST, HIR, Unicode properties, UTF-8 ranges and literal extraction. Requires `alloc`, not a matcher.
- `regex::literal`: Aho–Corasick and literal prefilters. Requires `alloc`; `literal-perf` opts into memchr acceleration.
- `regex::automata`: DFA/NFA construction and search, hybrid execution, captures and the meta engine. Uses syntax and literal layers only through explicit capabilities.
- Root `Regex`, `RegexSet` and `bytes` APIs: enabled by `matcher`; reuse the same automata and syntax types rather than wrappers or duplicate crates.

Syntax-only consumers use `default-features = false, features = ["syntax"]`; Unicode and standard-library support remain independent. `jsonschema-regex` uses syntax/std/Unicode without enabling a matcher. `matchers` selects DFA construction/search; `fancy-regex` selects its actual automata requirements. `tracing-subscriber` refers to the same automata type identity. Existing high-level consumers with defaults disabled explicitly request `matcher`.

`dfa-search` alone is `no_std` and allocation-free: it can search precompiled DFAs without syntax compilation. `syntax`, `literal` and construction capabilities require allocation but not `std`. Both Linux amd64 and browser WASM remain supported. None of the four origins has a build script. No package-level cycle or new proc-macro dependency is introduced.

Feature unification is additive, so combining a parser consumer with a matcher consumer in one binary still enables both; parser isolation is guaranteed for its standalone feature closure, not for every unified dependency graph. `engine-perf` preserves the automata-only acceleration policy; high-level `perf` retains matcher-oriented engine selection. Package reduction is not a claimed runtime speedup.

## Unsupported compatibility removed

Nightly `Pattern` integration and the `pattern`, `unstable`, `use_std`, `perf-cache` compatibility feature names are not retained. The removed Pattern implementation is archived verbatim in the consolidation receipt. `arbitrary` was an advertised but unresolvable optional provider; its derive attributes and specialized impls are removed rather than exposing a broken feature. Stable parser and matcher APIs remain available. Original upstream documentation and license evidence remain immutable.

## Source and license attribution

| Layer | Original package | Version | Source | License |
| --- | --- | --- | --- | --- |
| Root API | regex | 1.13.1 | https://github.com/rust-lang/regex | MIT OR Apache-2.0 |
| Automata | regex-automata | 0.4.18 | https://github.com/rust-lang/regex | MIT OR Apache-2.0 |
| Syntax | regex-syntax | 0.8.11 | https://github.com/rust-lang/regex | MIT OR Apache-2.0; bundled Unicode data license |
| Literal | aho-corasick | 1.1.5 | https://github.com/BurntSushi/aho-corasick | Unlicense OR MIT |

The manifest combines the implementation license obligations; source-origin records retain each original license and version. Original archives, hashes, upstream manifests and Unicode notices remain under `compliance/provenance` and `compliance/licenses/upstream`. The content-addressed consolidation receipt in `compliance/rust-sources.json` records every production-file route and all 107 original external test files, including reversible import changes. No test file is discarded and each retains its original `source_sha256_before` and origin.

## Verification and limits

Runnable original syntax and literal unit suites remain attached to their actual production modules. The project-owned `test/regex-family/retained_regressions.rs` adapter runs existing high-level regression, replacement and fuzz regression suites without changing assertions or supplying fake helpers.

The full automata unit harness still requires the absent real `quickcheck` and `bstr` providers and a declared `env_logger` test dependency. The upstream TOML-driven aggregate suite requires the absent `regex-test` provider. These are unavailable suites, not passing tests; their source is preserved. Existing upstream ignored expensive fuzz cases remain explicitly ignored, not newly suppressed by this consolidation. Missing `doc_comment` README inclusion was removed; Rust API examples are still directly available to rustdoc.

### Recorded verification for this isolated consolidation

- 147 syntax unit tests and 163 literal unit tests passed.
- 52 high-level upstream regressions passed; the originally ignored `fuzz1` case was run separately and passed (53 total).
- 17 original `matchers` unit tests and 4 direct consumer behavior tests passed, covering streaming matching, fancy backreferences/lookbehind, schema translation and tracing field filters.
- 845 API doctests passed; 7 pre-existing upstream ignored explanatory snippets remain marked as such.
- 42 native feature configurations and 14 WASM configurations passed, including every retained individual feature, full features and allocation-free DFA search.
- 8 positive/negative API-boundary probes and 5 removed-feature rejection probes passed.

These results do not claim execution of unavailable quickcheck/regex-test suites. The pre-existing nightly syntax benchmark is preserved at `src/foundation/text-unicode/regex/benches/syntax.rs`, explicitly unregistered on the stable toolchain, rather than silently discarded or presented as a passing test.

## Maintained verification

The foundation CI suite runs `python3 .github/scripts/check-pattern-matching.py`.
It builds every retained individual native capability and the documented WASM boundary matrix, rejects warning diagnostics in the regex package, executes preserved syntax/literal/high-level regression tests (including the explicitly ignored fuzz regression), doctests, and real consumer behavior.
Temporary manifests and locks live outside maintained source paths; no missing upstream provider is replaced by a mock. Complete automata property tests and the aggregate regex-test TOML suite remain unexecuted because their genuine providers are absent. Seven upstream illustrative doctests remain ignored; no speedup or exhaustive input coverage is claimed.
