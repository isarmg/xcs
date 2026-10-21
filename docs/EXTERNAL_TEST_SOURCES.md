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

## Explicit source naming transitions

`refresh --rename-plan PLAN.json` supports reviewed foundation package-root,
package-name and corresponding external-test-root renames. Schema 1 plans contain
`packages` entries with `directory`, the exact prior `package`,
`destination_directory`, `destination_name`, `reason`, the exact reviewed post-move
`files_sha256`, the exact previous `external_tests` owner (or null), and the exact
reviewed `destination_external_tests` owner (or null). The enriched owner includes
origin, file count and byte total. Destination directory basename and package name
must match. Relative production filenames and external fragment counts remain
bijective; changed routing/import bytes require explicit reviewed after hashes.

Schema 5 catalogs preserve the original package and external owner in append-only
`renames`, authenticated by `rename_records_sha256`. External `source` and
`source_sha256_before` labels remain historical; only validated same-origin rename
history allows that old source prefix under the current owner. Old production and
test roots must be absent. Ordinary refresh still rejects lost external fragments,
unknown files, changed ownership and rewritten history. Later local content edits
follow ordinary explicit refresh without rewriting the naming receipt. Prior
schema 4 catalogs without naming history remain verifiable.

Rename and retirement/upgrade of one identity require separate changes. A renamed
owner can later be explicitly retired with its exact external receipt. Repeat
renames, upstream upgrades after renaming, renames of already-upgraded sources,
and sources covered by fixed ring/platform live-view proofs fail closed until a
chain-aware or historical-view adapter is supplied. Original upstream names,
archives, licenses and security identities are never renamed or overwritten.

## Identity-preserving internal file moves

`refresh --move-plan PLAN.json` moves internal production filenames or external
test filenames without changing the package root, name, version or upstream
identity. This also applies after a previous package-root rename. Plans use
`schema: 1` and `packages` entries containing `directory`, exact previous
`package`, all exact overlapping `merged_packages` (or an empty list), `reason`,
full reviewed post-move `files_sha256`, `file_moves`, exact previous
`external_tests` (or null), reviewed `destination_external_tests` (or null), and
`external_file_moves`.

Production file maps use package-relative paths; external maps use repository-
relative paths. Enumerate every changed file, including each descendant of a
moved directory. Unlisted files remain at their current paths. The implicit and
explicit maps together must be bijections: no lost file, invented source,
unknown destination, or overwrite of an unmoved file is accepted. After content
hashes and byte totals are checked against disk. External owner roots, original
`source` labels, original hashes, fragment counts and ownership remain unchanged.
At least one map must be nonempty; an external-only move is supported.

Schema 6 catalogs append content-addressed `source_moves` receipts containing the
exact before snapshots and reviewed maps. Subsequent moves append new receipts,
never rewrite old source moves or root-renaming records. Ordinary content edits
remain possible through explicit refresh. Historical receipt origins are checked
against the independently authenticated origin inventory; a move does not create
or remove an origin. Schemas 4 and 5 remain readable, and refresh emits schema 6.

A source cannot be internally moved and root-renamed, upgraded or retired in the
same refresh. First finish the root-identity transition, then snapshot its current
receipt for a separate move plan. Disjoint owners may use different explicit plans
in one refresh. This accounting proves declared file preservation, not behavioral
compatibility; affected consumers and protocol/format invariants still require
their normal tests.

## Reviewed multi-origin consolidation

`refresh --consolidate-plan PLAN.json` explicitly combines implementations. Schema
8 catalogs retain content-addressed `consolidations` and matching
`consolidation_records_sha256`; older empty-history catalogs remain readable.
Catalog schemas never decrease during refresh.

A schema-1 plan has `packages` entries naming a `primary_directory`,
`destination_directory`, `destination_name`, and review `reason`. Each `inputs`
entry contains the exact previous `package` and enriched `external_tests` receipt
(or null). `destination_files_sha256` is the complete reviewed production tree.
`file_routes` names every previous package-relative file per input directory:
`destination` is a current relative file; only redundant root Cargo manifests
may instead use byte-exact `archive_base64`. Explicit `new_files` accounts for
new glue independently of routed files. Production routes may be many-to-one.
They document reviewed content migration, NOT byte preservation or semantic
compatibility. Functional tests remain mandatory.

`destination_external_tests` is the complete enriched combined owner. Every row
retains its historical source/hash and metadata, adds the original input's
`origin_directory`, and updates only the current path/hash.
`external_file_routes` maps every previous repository-relative test path to one
unique `destination` plus `replacements`: ordered base64 `old`/`new` byte pairs.
Initial verification reverses unique nonempty bounded edits and reproduces the
exact last-current test hash. Whole-file substitutions, disappearance, extra
files, symlinks, ambiguous edits and wrong-origin attribution fail closed.
Unchanged files require no replacements. Grants name exact origin/source/original
hash triples, never a broad merged-source prefix.

Primary and absorbed origins retain separate upstream versions, security and
license identities; absorbed origins are merged, not reported as deleted. Old
source and external roots must be absent. Historical receipts and original
materials are immutable. Later reviewed current-file edits use ordinary refresh
without rewriting initial transfer evidence or dropping origin tags.

The initial bounded implementation rejects pinned proof families, previously
renamed/moved/upgraded/merged inputs and chained consolidated-owner lifecycle
changes until an authenticated adapter supports them. Disjoint ordinary lifecycle
plans may run together, but overlapping plans never silently bypass checks.
