# Compression foundation

The `compression` package consolidates four Cargo packages into one functional
component. This reduces this family by three packages; it is not a measured
runtime, binary-size, or memory-performance improvement.

## Capabilities and boundaries

The default feature set is empty. `compression::checksum` supplies safe scalar
Adler-32 with no allocator, standard library, or dependencies. Module-level
`forbid(unsafe_code)` remains in both the scalar checksum and DEFLATE codec.

- `std` enables standard-library checksum I/O; it does not pull in stream formats.
- `codec` enables allocation-free low-level DEFLATE inflate.
- `alloc` enables allocated codec APIs when combined with `codec`, including
  deflate and bounded-output convenience decompressors.
- `streams` enables `std`, `alloc`, `codec` and the real `crc32fast` provider.
  The `streams::{read,bufread,write}` modules support raw DEFLATE, zlib and gzip.
- `simd` enables `accelerated_checksum` and uses it inside the codec. Stable
  Linux amd64 dispatch selects available SSE2, SSSE3 or AVX2; no-std selects
  compile-time features. Browser WASM falls back to scalar, even if its compiler
  enables simd128. Unsafe intrinsics stay inside the original scoped kernels.
- `runtime-detection` enables std and optional CRC runtime detection. Enable
  it with `streams` for the release builder. Adler SIMD dispatch uses `std`.
- `block-boundary` enables the real codec block-boundary capability.
- `serialization` adds the existing codec-state adapter through the real
  data-serialization provider, with alloc and derive. It remains optional.

Only tools/web-build consumed flate2 in the reviewed product graph. Its gzip
asset writer now explicitly opts into streams, SIMD and runtime detection.
No old Cargo package alias remains. The existing compression algorithms and
DEFLATE/zlib/gzip wire formats are unchanged; this is structural consolidation,
not a new codec or a cryptographic-checksum replacement.

## Streaming contracts

Read wrappers may consume bytes beyond the compressed stream; bufread wrappers
preserve remaining buffered bytes. A single-member gzip decoder stops after the
first member; MultiGzDecoder processes concatenated members. Callers still need
to finish writers and honor low-level consumed/written counters. The bounded
codec convenience APIs remain available for untrusted input. No new guarantee
is made that every raw/zlib stream adapter diagnoses truncation identically;
the retained upstream behavior and regression contracts are tested separately.

## Removed unsupported surfaces

Native-zlib/C backend switches previously lacked a provider in this repository.
They and old backend-selection/compatibility switches are removed, as are
compiler-internal rustc-dep-of-std and unstable alloc_io selection. Native-only
window-size/dictionary/level-adjustment APIs are not advertised by the Rust
backend. Nightly AVX512, NEON and WASM-SIMD modules are not part of the retained
Linux amd64/WASM-scalar support. The four removed complete implementation files
have exact original bytes, hashes and reasons in the consolidation receipt.
Their original license and upstream identities have not been removed.

## Origins and immutable attribution

- flate2 1.1.10: https://github.com/rust-lang/flate2-rs
- miniz_oxide 0.9.1: https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide
- adler2 2.0.1: https://github.com/oyvindln/adler2
- simd-adler32 0.3.10: https://github.com/mcountryman/simd-adler32

All original manifests, upstream documents and legal materials remain governed
by compliance/provenance. Each input remains independently attributed in the
content-addressed source consolidation receipt, including the codec's earlier
source move. Thirty-seven external files have exact origin/source hashes and
bounded reversible transfer edits. The inert Adler benchmark is retained
byte-identically as source history, not advertised as executed coverage.

## Maintained validation and explicit gaps

`.github/scripts/check-compression-family.py` runs in the foundation CI suite.
It checks 26 isolated native/WASM capability configurations, verifies that
checksum/codec-only dependency closures remain empty, rejects disabled APIs and
removed backend features with real compiler/Cargo failures, runs retained unit
and integration tests plus doctests, tests a real serialized decoder checkpoint/resume, and tests the actual web-build consumer.
It mirrors only this package and its tests into temporary storage; dependency
providers remain the real maintained sources and no governed manifest is edited.

`test/compression/contracts.rs` covers independent zlib/gzip vectors, levels
and empty data, malformed/truncated/checksum-corrupt data, bounded output,
streaming flush and short buffers, concatenated gzip and trailing bytes, and
scalar/SSE2/SSSE3/AVX2 agreement at alignment/chunk boundaries. Each SIMD kernel
is exercised only when its original runtime/compile-time availability guard
allows it. WASM is compile-checked; no WASM execution or cross-machine benchmark
result is claimed.

Three randomized stream unit modules and SIMD randomized backend modules need
upstream rand/quickcheck providers that are not vendored. Those external test
files remain byte-identical and inactive, with no substitutes or fake providers.
Native-backend-only capability tests and two native-only memory tests remain
inactive. These gaps are not counted as passing tests. Maintained deterministic
contracts augment, but do not pretend to replace, unavailable upstream coverage.

## Receiving-tree integration checks

The receiving-tree run also checks all maintained workspace formatting, the
server/client/admin native builds and strict Clippy, and documentation contracts.
Three lightweight consumer regressions inspect generated serialization harness
manifests for existing local paths, matching package identities and supported
features, and verify the release builder's explicit compression capabilities.
These checks prevent a renamed package from surviving only in an older test
harness. They do not replace the compiler and runtime checks above.

Both scalar Adler implementations remain deliberately separate: the serial
kernel and four-lane kernel have different small-buffer/large-buffer tradeoffs.
Structural consolidation is not a reason to impose one kernel on all callers.
The existing dispatch and canonical checksum semantics remain unchanged.
