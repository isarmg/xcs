# ring Clippy member diagnostics repair (2026-10-20)

## Scope and provenance

The newly registered non-default ring workspace member exposed two diagnostics in
normal `quality.sh` dependency compilation. Strict all-target ring Clippy additionally
exposed five equivalent test-only style repairs. This batch changes exactly seven
source lines in six files:

- `src/testutil.rs`: `current_section.truncate(0)` to `current_section.clear()`.
- `src/aead/aes/ffi.rs`: remove one excess SAFETY comment indentation space.
- `tests/aead_tests.rs`: `in_out.truncate(0)` to `in_out.clear()`.
- `tests/agreement_tests.rs`: remove redundant formatting borrow of `private_key`.
- `tests/hmac_tests.rs`: remove redundant formatting borrows of `key` and `ctx`.
- `cavp/tests/shavs.rs`: `msg.truncate(0)` to `msg.clear()`.

No cryptographic algorithm, ABI, test assertion, feature, manifest, lock or lint
policy changes. The HMAC patch retains unchanged intervening context and records
both changed lines as one reversible source replacement.

`compliance/provenance/supplemental/ring-0.17.14-style-patches.json` records the
restoration baseline commit, the upstream commit, both immutable historical proof
hashes, and each exact old/new replacement with before/after Git blob, SHA256 and
byte length. The checker accepts only these six paths and exact transformations,
authenticates the current source, reverses each replacement and then checks the
unchanged restoration and original-package proofs, including CAVP material blobs, against reconstructed baseline
bytes. It does not replace original upstream hashes with modified source hashes.
Current source-governance receipts separately describe the new local source state.
The original restoration report and evidence are historical and remain unchanged.

## Validation

- Complete `bash .github/scripts/quality.sh`: passed, including first-party
  Clippy with `-D warnings`, workspace all-target compiler diagnostics, and WASM
  release checks. No `allow`, excluded member or lowered lint level was added.
- `cargo test --locked --offline -j2 -p ring --features std`: 94 library,
  89 integration (including 8 CAVP), and 9 doctests passed; one pre-existing
  ignored doctest remains. No filtering or skipped executable test entries.
- `bash .github/scripts/check.sh static`: 105 source-tool, 51 script and 5 perf
  tool tests passed, along with the other existing static gates.
- Source governance refresh and `manage.py verify --metadata`: passed, retaining
  339 active identities, 64 retired identities and the single root lock/workspace.
- Explicit `cargo clippy --locked --offline -p ring --all-targets --features std
  --message-format=json -- -D warnings`: final run passed with zero diagnostics.
  Two initial exploratory runs exposed integration/CAVP style diagnostics in
  stages; neither was counted as passing. A `--keep-going` discovery run confirmed
  the remaining inventory before repairs. No diagnostic remains from the final run.


Machine-readable command results and log hashes are in
`docs/validation/evidence/foundation/ring-style-20261020.json`.
