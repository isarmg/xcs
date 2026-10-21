# External test sources

Executable test code and test-only fixtures live under the root `test/` tree. Cargo
package identities, manifests, and production entry points remain under `src/`
(or the existing tool roots). Explicit Cargo test targets and gated Rust includes
retain package-local feature and private-module semantics. There is no test Cargo
workspace or independent lockfile.

`compliance/external-tests/*.json` accounts for current third-party test material
by owning source directory. Each owner records exact destination roots, current
file hashes and original source paths, including fragments extracted from a
production file that still exists. The source catalog binds these mappings to the
unchanged original upstream identity and hashes the mapping documents. Verification
rejects missing, changed, added, unattributed, overlapping, escaping and symlinked
material. Refresh verifies these explicit inventories before recording the source
catalog; it does not learn new test files implicitly. Removing a file from an
unchanged upstream source identity requires an explicit external source mapping.

Ring additionally uses `compliance/ring-test-relocation.json`, whose digest is
pinned by its verifier. Exact live production and test inventories are authenticated
before uniquely reversible source-routing changes reconstruct the prior source
view. Existing target-pruning, retired archive, style, original-overlap and test
material proofs retain their original bytes and are checked against that view.
The historical view cannot supply a missing current file. Production DER inputs
and unconditionally compiled public test-support APIs remain production sources;
executable assertions and their input vectors are external. The unconditional
compile-time assertions in `polyfill/cstr` preserve their original compilation
behavior through an external include.

The initial separation also has a compact, compressed reverse-line-delta receipt:
`compliance/test-source-relocation.json.gz`. Its compressed digest is pinned by
`source_relocation.py`; expansion is bounded. Bootstrap refresh checks every
remaining source and extracted fragment, rebuilds each pre-separation source from
those current bytes and reverse edits, and requires the result to match the prior
catalog's source hash. An empty/truncated replacement, different source attribution,
or altered current fragment cannot silently pass this first relocation audit.
The receipt is immutable historical evidence, not the mutable current inventory.
Later deliberate source/test edits follow the existing review-and-refresh process:
update the explicit current mapping and review the diff before refreshing. Such a
refresh accepts reviewed current code changes, not a claim that edited tests remain
byte-identical to the initial separation. Missing files and unrecorded additions
still fail, and removing previously attributed fragments is not ordinary refresh.

## Retirement and upgrade of an external-test owner

Ordinary refresh still refuses disappearance of an external owner or fragment.
An explicit `--retire-plan` or `--upgrade-plan` item for such an owner must include
`external_action: "remove"` and `external_tests` equal to the complete last owner
receipt from the catalog, including origin identity, roots, files, hashes, count,
and bytes. Remove that owner's entry from the current mapping and remove its
entire recorded test roots before refreshing. Missing approval, mismatched last
receipt, remaining mapping, leftover root (even empty), or symlink fails closed.

Retirement archives this exact external receipt under
`retired[directory].retirement.external_tests`. Its counts and bytes are added to
the retired summary totals. Upgrade archives it under
`superseded[prior_origin].external_tests`; unchanged old-version tests cannot be
silently attributed to a successor release. These nested receipts are protected
by the existing immutable retired/superseded record hashes. The initial relocation
proof remains checked even when no current external owners remain.

Upgrade supports removal, not in-place substitution of the old external corpus.
Restore successor tests inside the release-authenticated `source_prefixes` first,
then relocate successor tests in a separate reviewed change. If an upgrade needs
to retain or replace external tests directly, it must supply a separately designed
successor-preservation proof; the current workflow gives an explicit removal
instruction rather than exempting that owner from checks. Upgrade and retirement
of the same package require separate refreshes. Later, properly attributed tests
may reuse a superseded owner's former test root; a retired owner's root must stay
absent. Other active owners remain subject to the ordinary anti-drop checks.

## Correcting a whole-file classification

A file mistakenly classified as test-only may return to its exact former
production path during ordinary refresh, but only as a complete byte-preserving
restoration. The previous owner must have exactly one attribution for that source;
its original-source and external-file hashes must agree. The previous production
inventory must lack that file, the new production inventory and actual bytes must
match the recorded hash, the old external path must be absent, and both package
identities must retain the same owner and origin. Extracted fragments, altered
content, alternate paths, other owners, and retained external copies do not
qualify. This exception conserves source material rather than approving deletion.

The catalog retains `initial_test_relocation_sha256` after the last current
external owner is retired or restored, so the immutable initial separation proof
continues to be required independently of the current source classification.

Subsequent platform-test include-path fixes are authenticated separately by
`compliance/platform-test-routing-corrections.json`. The verifier pins this new
receipt, checks the corrected live bytes, and reverses its exact routing edit and
whole-file fixture move before applying the unchanged platform relocation proof.
Modified current bytes, incorrect reversals, reappearing old fixture paths, and
receipt tampering fail closed; the old platform and runtime receipts stay frozen.
