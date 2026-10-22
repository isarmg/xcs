# UUID syntax validation without unused decoding

The schema-validation UUID format check consumes only success/failure, never
UUID bytes. It now uses a safe, fixed-size 36-byte predicate with hyphens at
positions 8, 13, 18 and 23 and ASCII hexadecimal digits everywhere else.
Uppercase and lowercase are accepted; there is no version or variant restriction.
The fixed hexadecimal lane array permits ordinary compiler vectorization without
unsafe code, architecture dispatch, a copied decoder or a compatibility adapter.

The sole uuid-simd dependency and its otherwise unused vsimd/outref chain are
retired. Their original identities, versions, licenses and full production/test
bytes remain in authenticated compliance archives. This removes 29 active Rust
files / 195,516 bytes before counting the replacement and manifest changes; it
is not a claim about whole-repository size because archives are retained.

## Correctness and consumer boundaries

The retained differential probe compares 628,043 cases with the original parser:
lengths, every byte at every position, valid mixed case, Unicode/non-ASCII,
arbitrary inputs and malformed separator/digit layouts. The retired oracle is
restored only after checking the exact catalog receipt, complete file set,
SHA-256 hashes, canonical encoding, counts and safe output paths. It requires no
Git history or network, so shallow checkouts preserve the same validation.
Retired code is only an oracle, never an active product dependency.

The existing 1,509 canonical fixtures and UUID regression cases through both
2019-09 and 2020-12 schema drafts passed. The schema checker explicitly runs the
UUID consumer test; it is not left as an uninvoked test manifest entry. The
predicate and full schema validator compiled for browser WASM with the existing
wasm_js configuration. Root contract-check tests and repository documentation-
contract validation also passed; these do not substitute for storage, model or
platform tests. The current contract schemas do not use UUID format assertions.

## Measured trade-off, not a speedup claim

Compared with the prior SIMD decoder, direct valid/bad-hex cases cost about
1–4 additional nanoseconds. The final mixed predicate sample was 13.43 → 13.67
ns per input. Four alternating optimized object-validation runs, each containing
11 samples of one million objects, had aggregate medians 58.713 → 59.653 ns
(+1.6%), with substantially overlapping jitter. No stable material host impact
was established, and no speedup is claimed. This is a host validation-tool
simplification, not a demonstrated application hot-path optimization.

`python3 .github/scripts/check-uuid-format.py` replays the differential corpus;
`--bench` additionally reports the direct predicate comparison. The ignored
`uuid_validation_throughput` consumer test is retained for intentional optimized
old/new host comparisons, not as a timing assertion on shared CI machines.

The three retirement archives preserve all 32 production files / 197,446 bytes
and all six external test files / 9,148 bytes, with their original receipts.
Archive integrity checks authenticate this evidence; they do not assert that
all retired uuid-simd/vsimd/outref upstream suites execute.

The default differential gate runs exactly once in foundation-language and in
the aggregate foundation gate, with explicit inventory/routing checks. It also
compiles the predicate for browser WASM. The existing schema capability gate
runs the UUID consumer regression and rejects uuid-simd/vsimd/outref in every
checked feature graph. Archive restoration has fail-closed corruption and path
escape regressions that also pass under optimized Python (`-O`).
