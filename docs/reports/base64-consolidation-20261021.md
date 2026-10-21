# Base64 scalar consolidation, 2026-10-21

## Decision and scope

The HTTP/WebSocket/browser-tool consumers formerly on upstream `base64@0.22.1`
can use the existing upstream `base64@0.23.1` implementation without changing
callsites. Each migrated dependency explicitly disables defaults and enables
`std`, avoiding accidental activation of the newer default `simd-unsafe` feature.
Existing 0.23 consumers and their feature choices are unchanged. All migrated
callsites select `general_purpose::STANDARD` (including its prelude alias), which
remains scalar even if another workspace consumer enables SIMD through feature
unification. No claim is made that feature unification removes previously enabled
SIMD code from the product.

`base64ct@1.8.3` and its constant-time password/Argon2 consumers are not merged or
modified. Neither old nor new GeneralPurpose is constant-time. This is source
and maintenance consolidation, not a new constant-time or runtime-speed claim.

## API review

- reqwest: standard EncoderWriter, incremental writes and drop-based flush.
- axum: standard encoding of the WebSocket SHA-1 accept digest.
- tower-http: standard encoding for Basic authorization headers.
- browser-smoke: standard engine for screenshot decoding.
- No migrated caller pattern-matches DecodeError::InvalidLastSymbol. Upstream
  0.23 changes this error from a tuple to a struct, adds decoded symbol bits, and
  changes diagnostic formatting. The compatibility test normalizes only this
  intentional representation change, preserving rejection, category, offset and
  offending byte. Exact diagnostic strings are not asserted as identical.

## Evidence collected before retirement

Both implementations were compiled independently as scalar `std` + `alloc`
libraries with rustc. A direct differential harness compared 4,551,752 decoding
results, plus 4,100 encodings and 4,100 seven-byte-chunk EncoderWriter outputs.
All compared results matched.

The corpus covers all four standard/URL-safe padded/unpadded engines; exhaustive
inputs of lengths zero through five over `ABZaz09+/-_=`, space, newline, NUL and
0xff; deterministic binary inputs of lengths zero through 1,024; and invalid or
padding mutations at the start, midpoint and end of their encodings.

A separately executed one-implementation fingerprint harness produced identical
results for each implementation:

    decode_cases=4551752 fingerprint=4283e9f020b56287

This FNV-1a fingerprint is a deterministic regression checksum, not a security
primitive. Length prefixes, offsets and counts use explicit little-endian u64
representation. The maintained golden test retains the corpus and fixed baseline,
not a permanent second dependency or millions of expected-output records.

Baseline production hashes:

- `base64@0.22.1` src/lib.rs:
  `5887ca96bfa96fed3b5a1d4c1ed73af8a5977c1ae54822b15a1731de75f5eaa6`
- `base64@0.22.1` src/engine/general_purpose/decode.rs:
  `80476b681c255ccbee0367e61f15f225553fdfa11f618937335f0478f1b79595`
- Full original identity, upstream archive hash, license and original file hashes
  remain in central provenance. The retirement receipt retains all 19 final
  production file hashes (130,075 bytes) and the complete external test receipt.

The maintained browser-smoke tests also check Basic authorization's fragmented
writer/drop flush, RFC 6455's handshake example and corrupt/noncanonical browser
input rejection. The standalone debug test executable passed both tests in
3.60 seconds. The build-tools CI group invokes browser-smoke tests without
launching Chrome; browser execution remains a separate validation task.

## Reproduce

    cargo test --locked --offline -p browser-smoke base64_compatibility_tests

To independently reproduce the old baseline, reconstruct the recorded final
0.22 implementation outside the repository, compile the maintained test corpus
against that implementation, and map InvalidLastSymbol's old tuple `(i, b)` to
`(3, i, b)` in its normalized error branch. The expected checksum and count must
not be regenerated from the successor during ordinary test maintenance.

## Governance

Retirement must be an explicit reviewed plan with the complete last package and
external test owner receipts. Remove the old production/test roots and current
external mapping only through that plan; retain all original legal/provenance
materials. Source refresh and metadata verification are separate required steps.

## Integrated validation and limits

- `cargo test --locked --offline -p browser-smoke`: all 10 tests passed,
  including both new regressions.
- `cargo check --locked --offline -p reqwest -p axum -p tower-http`: passed
  for the registered native feature graph.
- `cargo tree --locked --offline -p browser-smoke -e features`: Base64 has
  `std` and `alloc`, not `simd-unsafe`.
- An isolated scratch consumer with `tower-http/auth` enabled compiled cleanly.
- An extra isolated `axum/ws` check exposed an existing unsupported optional
  branch: missing `sha1` and `tokio_tungstenite` dependencies and disabled
  `futures_util::sink`. The product does not enable `ws`. This check did not
  establish whole-feature WebSocket support; its Base64 operation is covered by
  the maintained RFC vector. No removed dependency was restored to hide the
  unrelated failure.

## Applied source accounting

The explicit retirement refresh completed with 325 active source trees and 77 retired identities. The old 19 package files and 15 external test files were removed from active maintenance; their final hashes and upstream legal materials remain recorded. Metadata verification passed. Native maintained-package formatting, Clippy and all-target diagnostics, plus release WASM checking passed locally. Final remote CI belongs to the published commit and is reported separately.
