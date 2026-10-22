# Shared form float formatting

`form-codec` now formats f32/f64 values directly with the existing `zmij`
implementation already used by JSON/schema code. Its sole `ryu` dependency
is removed. There is no forwarding crate, copied formatter or compatibility
adapter. Both types retain their own precision rather than promoting f32 to f64.

## Intentional canonical wire spelling

This unreleased project deliberately accepts the new canonical spelling of
positive scientific exponents: `1e16` becomes `1e+16`, and the form wire value
becomes `1e%2B16`. Likewise f32::MAX becomes `3.4028235e%2B38`. A literal `+`
would decode as a space in form encoding, so the percent-encoding check is part
of the contract, not cosmetic formatting. Negative exponents, signed zero and
nonfinite spellings are retained. No preservation of the old optional-plus
spelling is promised.

## Evidence and boundaries

The implementation audit compared 10,097,974 boundary-generated and random
f32/f64 inputs against the former formatter. Every finite candidate parsed
back to the exact original bit pattern; every observed spelling difference was
solely the optional positive-exponent plus sign. All 16,777,216 f32 nonfinite
encodings and 12 representative f64 nonfinite encodings retained their spelling.
These sampled finite results are not a proof over all f64 inputs.

`python3 .github/scripts/check-form-floats.py` checks five focused behavioral
tests, ten retained serialization tests and nine retained deserialization tests.
It exercises actual native reqwest form/query construction and http-routing
consumption, including percent encoding and bit-preserving decoding. It also
compiles the form/routing tests for browser WASM; reqwest stays explicitly
native-only in this harness. Its resolved graph requires zmij and rejects ryu.

## Provenance and size accounting

The original ryu 1.0.23 identity, licenses and source receipt remain unchanged.
`compliance/ryu-1.0.23-retirement.json.gz` preserves all 19 production files
(107,241 bytes, including 18 Rust files / 106,487 bytes) and all ten external
test files (32,919 bytes). The archive is authenticated against the retired
catalog record, complete path set and every original SHA-256 digest.

Those are removed active-source bytes, not whole-repository size savings: the
full immutable archives and original provenance remain. Archived upstream ryu
suites are preserved evidence, not a claim that those retired suites executed.

The checker is registered exactly once in the foundation-language shard and
once through the aggregate foundation gate. The explicit command inventory
and suite-routing regression enforce this placement. Existing SDK, platform,
SQLite, Samba, feature-isolation and browser gates remain unchanged.

For full audit replay, run
`python3 .github/scripts/check-form-floats.py --differential` instead of the fast
consumer gate. It restores ryu exclusively from the authenticated retirement
archive and runs `test/architecture/form-floats/differential.rs` against current
zmij/form-codec. No Git history or network is needed. The original output is
retained at `test/evidence/form-floats/differential-native.txt`; a fresh archived
oracle replay reproduced the same boundary/random and nonfinite counts after
retirement. This opt-in audit does not replace the default native/WASM consumer
checks or add retired code to any production dependency graph.
