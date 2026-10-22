# Logging capability consolidation, 2026-10-22

## Reviewed scope

The only production edge is web-bindings → env_logger → env_filter. Neither
implementation has an independent consumer requiring a separate Cargo package.
Both become one real `logging` capability: the existing formatter, writer,
environment configuration and logger own a `filter` module containing the actual
directive parser and matcher. This removes one package (two to one), not an extra
facade over unchanged packages. The shared terminal-style implementation and API
remain separate and unchanged.

The logger always required std and explicitly enabled env_filter/std, so the
integrated filter uses std unconditionally. Native Linux amd64 and browser-Wasm
compilation remain supported. The default feature set, regex versus literal
message matching, target-prefix precedence, default error level, environment
variable names, color adaptation, synchronized output and flushing are preserved.
Canonical kv remains; the unused unstable-kv alias is rejected rather than kept
as a hollow compatibility flag. Stale nonexistent packaging include entries are
removed. No algorithm, terminal-style implementation or application scope changes.

## Attribution and evidence

The explicit consolidation plan binds both exact previous package and external
test snapshots, their immutable rename/move lineage, every production destination
and every external test route. All 15 former production files are accounted for:
14 mapped files and the exact redundant env_filter Cargo.toml archived in the
receipt. Original upstream identities, source hashes, licenses, source archives
and README documents remain unchanged.

All five original external Rust test fragments are retained. Four are byte-identical;
the parser fragment drops only its already-unused ParseError import, with a bounded
reversible substitution proving the exact previous bytes. Original attribution
and before-source hashes are retained for both origins. Three original README Rust
examples are retained under test/logging/readme_doctests.rs with only the crate name changed;
a static test verifies exact example equivalence. Current-use documentation is
separate from the immutable upstream documents.

The small no-color StyledValue alias now exists only when kv uses it, eliminating
a pre-existing unused-type warning without changing active formatting behavior.
All new behavioral contracts are under test/logging, not production source.

## Validation

The logging gate checks independent Cargo dependency closures, native inherited
unit tests, output/filter/color/flush contracts and doctests, and browser-Wasm
compilation for no features, regex, color, kv, auto-color+regex, and
all canonical capabilities. A real negative Cargo probe rejects unstable-kv.
The terminal-style gate exercises existing shared CLI/logger composition,
environment color precedence and positive/negative terminal capability boundaries.

Actual web-bindings tests pass all five cases, including target/regex log filtering,
ANSI formatting, browser-only argument acceptance, help/version and unsupported
backend rejection. The actual binary help and version smoke checks pass. Browser
validation is compilation, not execution in a browser. No claim of performance
improvement or byte-for-byte equivalence of unrelated APIs is made.

Integration requires applying the exact two-owner delta to the shared external
owner map, regenerating the root lockfile, applying the guarded consolidation plan
through manage.refresh, and refreshing architecture evidence. Generated catalog,
lock and whole-owner files must never be transplanted from this isolated checkout.

The six feature sets completed 634 tests (400 inherited unit-test executions,
30 authored contract executions and 204 doctest executions), with no warnings in
the final matrix. All six browser-Wasm checks passed. After changing the upstream
README fixture from Markdown to a genuine Rust doctest file to satisfy the existing
layout guard, all 34 doctests passed again. The complete terminal-style gate passed,
as did the actual web-bindings binary help/version smoke checks and its five tests.
The initial complete test/ci pass ran 269 tests; the final changed logging gate
contracts are also checked separately. Final metadata/architecture/static results
are recorded in the handoff validation receipt.
