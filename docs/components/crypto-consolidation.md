# Unified cryptographic traits and BLAKE2 integration

BLAKE2 and Argon2 now use the same digest 0.11.3, block-buffer 0.12.1 and crypto-common 0.2.2 implementation family as SHA-2, HMAC and the optional BLAKE3 digest integration. The shared storage representation is hybrid-array 0.4.15. The former standalone digest 0.10.7, block-buffer 0.10.4, crypto-common 0.1.7 and generic-array 0.14.7 dependency stack is no longer reachable or present as packages.

This is not an algorithm replacement. BLAKE2 compression rounds, constants, parameter encoding, counters, final-block handling and SIMD sources are preserved. Runtime-selected output remains available for Argon2 H'. Optional reset, allocated output and std::io::Write behavior remain available; the obsolete digest::VariableOutput import becomes blake2::VariableOutput, without an old-path compatibility alias.

## Imported implementation attribution

The buffering adapters retain implementation fragments from RustCrypto digest 0.10.7. That identity is recorded as a merged origin in the BLAKE2 directory, not falsely classified as wholly eliminated source. Its advisory identity and MIT/Apache-2.0 license materials remain active alongside BLAKE2's own origin.

- digest 0.10.7 src/core_api/wrapper.rs → BLAKE2 src/buffered.rs: fixed-output buffering, conditional Reset, keyed traits and std::io::Write. XOF/OID wrappers not used by BLAKE2 are not imported.
- digest 0.10.7 src/core_api/rt_variable.rs → BLAKE2 src/variable.rs: runtime-sized output wrapper and its conditional reset/Write implementations.
- digest 0.10.7 src/lib.rs → BLAKE2 src/variable.rs: VariableOutput, VariableOutputReset and InvalidBufferSize.

The adapters use the retained digest's block_api/common/array traits; they do not introduce a second trait family. BufferedHash implements the retained CoreProxy interface. The original last-active source and external-test inventories are preserved byte-for-byte in compliance/crypto-consolidation-source-archive.json.gz, whose compressed hash and decoded per-file hashes are pinned by test/ci/test_crypto_consolidation.py. Historical origin records, license materials, original test source labels and earlier receipts are unchanged.

## Reproducible acceptance

Run python3 .github/scripts/check-crypto-consolidation.py. It creates only an external temporary Cargo workspace, uses local path dependencies and offline/locked resolution, and rejects duplicate digest/crypto-common/block-buffer packages or any generic-array dependency.

The maintained test/ci/crypto_vectors.py generator decodes all five preserved BLAKE2 blob fixtures, verifies all 518 known vectors independently using Python hashlib, and emits bounded deterministic Rust tests. It adds 1,152 independently calculated unkeyed cases covering every valid BLAKE2b/BLAKE2s output length at twelve input boundaries with six chunking patterns, plus 54 keyed/personalized/truncated cases. It never alters the original fixtures. Existing Argon2 KAT and PHC tests are compiled with equivalent literal arrays instead of an unavailable hex-literal development dependency; their original ignored cases remain ignored and must not be counted as passing.

Capability tests cover fixed/runtime output, invalid length without state consumption on reset APIs, keyed/personalized reset, constant-time MAC verification interfaces, boxed output and Write. The runner checks default, reset, no-default, no-default+reset, no-default+alloc, size_opt, and native/WASM no_std configurations. Existing nightly SIMD source is unchanged; this consolidation does not assert that its historical platform_intrinsics API works on the current stable compiler.

## Separate parameter-validation fix

After consolidation, a focused fix makes the fallible BLAKE2 new_with_salt_and_personal constructor return InvalidLength for all oversized key, salt and persona inputs instead of allowing intermediate lengths to reach core assertions. Keys are limited to 64 bytes for BLAKE2b and 32 for BLAKE2s; salt/persona fields are limited to 16 and 8 bytes respectively. These limits do not shrink with truncated output sizes. The low-level Self-returning new_with_params constructor intentionally retains its assertions.

Maintained capability tests exhaust lengths from zero through one beyond the former block-sized bound for each parameter and both full/truncated BLAKE2b/BLAKE2s MACs, including simultaneous exact maxima and invalid maxima. The existing independent vectors and feature matrix prove valid cryptographic output, reset and no_std behavior remain unchanged. Original snapshots and provenance receipts are not rewritten by this correction.
