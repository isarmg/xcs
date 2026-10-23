# Foundation functional layout proof

The 219 foundation packages are organized beneath 12 functional directories.
This changes source locations only: package names, versions, origin identities,
licenses and upstream records remain unchanged. The taxonomy and its boundaries
are documented in [foundation-groups.md](foundation-groups.md).

## Independent, additive receipt

`compliance/rust-sources.json` binds `foundation_layout_receipt` to the SHA-256 of
a gzip-compressed JSON receipt under `compliance/source-layout/`. The same ID is
pinned in `tools/rust-sources/source_layout.py`. The receipt records every old and
new root, all active and shared-origin owners, the exact predecessor catalog,
source inventories, stationary owners, external test attribution and unchanged
historical evidence. Test roots remain at `test/foundation/<package>`.

The receipt does not rename or recalculate any historical origin, source label,
rename, internal move, consolidation, benchmark relocation or trimming record.
It is a separate current-path boundary above those proofs. `src/interface` and
`src/naming` remain where they were, including their original shared identities.
There are no compatibility directories, symlinks, duplicated sources or fallback
archive reads.

## Path edits and one exact formatting adjustment

One maintained test statement in `test/tools/web-build/unit/main_tests.rs`
also receives the exact rustfmt-required line breaks around `fixture.0.join(...)`
after its path becomes longer. The receipt binds those whitespace bytes and the
validator permits only that exact statement replacement, not general formatting
or implementation edits.

Every changed source, test or Cargo manifest has its before/after digest and a
bounded, reversible list of byte replacements. Verification reads the present
file, authenticates its after digest, reverses the replacements, authenticates
the before digest, and reproduces the deterministic path transformation. Cargo
TOML semantics and resolved targets must match after applying the root map.
Recognized Rust include/module attributes and explicitly reviewed build/runtime
references use their actual source or manifest base. Unknown changed string
references are rejected for review; unrelated strings and package-internal
relative documentation are not blindly rebased.

Older proof readers see a projection of the *current* catalog, not a cached
predecessor catalog. Only active owner paths and exact reviewed path-edit hashes
are projected; historical record internals are unchanged. Pinned source proofs
still reject altered live bytes. Missing live files never fall back to stored
snapshots. Trimmed sources and retired build scripts must remain absent at the
new physical owner paths.

## Installation and verification

Installation uses an explicit `refresh --layout-plan PLAN.json` whose only keys
are `schema: 1` and the pinned `receipt_id`. It is separate from retirement,
upgrade, rename, internal move, consolidation and trim operations. Installation
checks the complete source/test file bijection, modes via independent audit,
exact current external ownership and every previous identity before recording
the new catalog. Existing historical receipt bytes are immutable.

Normal `manage.py verify --metadata` revalidates the binding and the existing
proof chains. Authentication is scoped to one exact repository operation. The
catalog is hashed on entry and exit; full filesystem identity/timestamp stamps
trigger immediate reauthentication if it changes between helper reads. A
refresh binds only the exact serialized catalog it validated and wrote. No
trusted context survives the operation.

`test/tools/rust-sources/test_source_layout*.py` covers missing, extra, renamed,
mutated, symlinked and unattributed files; old-root reappearance; lost merged
owners; frozen evidence and source-label corruption; non-path semantic edits;
module/include bases; scoped context drift; terminal consolidation routing;
and grouped source-trim/build-script absence.

## Supported post-layout lifecycle

Receipt schema 2 separates `installation_files` from `frozen_evidence`.
`Cargo.lock` is checked byte-for-byte at installation, then remains governed by
the normal current catalog rather than being frozen permanently by this layout.
The pre-existing provenance and proof files remain immutable.

Ordinary refresh remains supported. Existing explicit retirement and internal
source-file move plans are also supported for standalone, unrenamed,
unconsolidated, untrimmed, non-proof-pinned mapped owners. New grouped retirement
entries must preserve the original stable identity, exact historical external
labels, valid retirement receipt, and absent source/test roots. New grouped
internal move entries are individually checked by the original internal-move
validator, their content-addressed IDs, source-file bijection, and the original
stable owner identity. Repeated internal moves and subsequent retirement are
covered by actual-manager tests. Neither operation can implicitly introduce an
external file move or discard source attribution. Moving either to or from a
file with a reviewed layout path edit is rejected until an explicit descendant
inverse adapter exists. Authenticated ordinary moved destinations remain readable
through the current logical inventory; unrelated extra files remain forbidden.

Only independently validated new grouped entries are removed when comparing the
baseline history digest. No pre-layout key can be filtered, removed, rewritten
or rehashed. Installation still requires the complete exact source/test
bijection; later ordinary retirement is not confused with a failed installation.

Rename, consolidation, upstream upgrade, source trimming, external-file moves,
and lifecycle changes to shared/chained or fixed-proof owners require a future
explicit descendant-chain adapter. Preflight rejects these plans with a named
limitation. The adapter does not claim that all previous lifecycle workflows are
unchanged, and it never relaxes an older pinned live-source proof to accept one.
