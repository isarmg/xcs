# Binding expression parser consolidation

The host-only C binding path now owns its C expression parser at
`src/foundation/tooling/bindgen/src/cexpr`. It replaces the former package chain
`bindgen → cexpr → nom → minimal-lexical` with one package, not three renamed
or forwarded dependencies. `clang-sys` remains a separate ABI boundary.
The three removed source trees contained 47 files / 614,895 bytes; the owned
parser contains five files / 50,962 bytes, a net reduction of 563,933 bytes (91.7%)
excluding test/provenance material and the other unchanged bindgen sources.

## Reviewed call closure

- `clang.rs` converts libclang tokens into the unchanged `Token { kind, raw }`
  representation. `callbacks::{Token, TokenKind}` still expose all five token
  kinds and owned bytes for `ParseCallbacks::modify_macro`.
- `ir/var.rs` evaluates macros with `IdentifierParser::macro_definition`, uses
  `expr::expr` for literal token streams, and consumes `EvalResult` and `CChar`.
  `ir/context.rs` retains the identifier/value map. These are the only production
  consumers of the former cexpr package.
- The literal grammar needs byte tags, character sets, alternatives, mapping,
  sequencing, repetition and recognition. The expression grammar needs the same
  bounded slice operations. No bit parser, generic numeric parser, string-input
  implementation, permutation, iterator, or nom number/float machinery is used.
- cexpr's own decimal-float recognizer calls Rust's `f64::from_str`.
  Consequently none of minimal-lexical's numeric conversion machinery is in the
  executed call closure, even though the old Cargo dependency graph included it.
- The new slice operations are independently written to this closure, with
  nom 7.1.3 behavioral provenance and its retained MIT license acknowledged.
  They are not a copied private nom package or a compatibility reexport.

## Semantics and boundaries

The existing C expression algorithms remain intact: integer literals are parsed
as `u64` then bit-cast to `i64`, arithmetic uses `Wrapping<i64>`, and width/sign
suffixes retain their existing handling. Operator precedence, mixed float/int
promotion, shifts, escaped character/raw-byte handling, string concatenation,
identifier substitution and function-macro declaration parsing are retained.
Alternatives backtrack only on recoverable errors; incomplete input is distinct
and preserves its requested size and remaining input.

This does not invent C type semantics that the old evaluator did not support:
its numeric model is i64/f64, right shifts are arithmetic, width prefixes are
ignored, and it does not evaluate casts, sizeof, or hex floats. Bindgen's separate
clang-backed evaluation/fallback paths, including casts and actual ABI widths,
are unchanged. SQLite's system-header generation and Samba's build bindings
continue through the same bindgen/clang-sys path. No browser/WASM runtime parser
is added.

## Verification

`python3 .github/scripts/check-binding-expressions.py` runs six SDK-independent
regressions. Its golden corpus covers 109,250 literal inputs and 50,000 token
sequences. For each token sequence it includes expression, macro definition and
function-macro declaration results. The oracle was generated from the original
cexpr/nom implementation at `aa7ffc46e702bde224d21db6e7df6a278238a230` and includes
Debug-rendered values, remaining input and error/incomplete categories. Before
retirement a separate side-by-side run compared every result directly, not just
the corpus digest. Explicit tests also cover every supported operator, wrapping,
float promotion, populated identifier maps, function arguments, Unicode/raw
escapes, callback token shape, comments, invalid token kinds, backtracking,
incomplete propagation, and non-consuming repetition.

`python3 .github/scripts/check-binding-expressions.py --clang` additionally runs
the retained six upstream C-header suites against an actual libclang installation.
This is wired into the foundation-data CI shard, which already installs clang/libclang
and SQLite development headers. Existing system-SQLite and Samba SDK gates remain
necessary and unchanged. The parser-only tests and default/runtime-only bindgen
consumer compilation do not establish SDK, ABI, or end-to-end binding validation.

Implementation and tests are separated: production code is under bindgen/src;
standalone regressions are under test/ci/fixtures, and the historical clang suites
are under test/foundation/bindgen/cexpr. Original source identities, licenses and
retirement/consolidation evidence remain in compliance records.

The SDK-independent checker is wired into the foundation-language shard. Suite
routing tests require each mode exactly once in its owning shard and in the
aggregate foundation gate, without replacing SQLite or Samba checks.

The consolidation receipt accounts for all 62 input production files and all
30 external bindgen/cexpr test files. The two retired dependency archives retain
88 exact files in total: 42 production files and 46 external tests. Archive
integrity tests authenticate their complete path sets, original hashes, byte
counts, origin records and absence from the active tree. These preservation
checks do not claim that the retired generic parser test suites execute.
