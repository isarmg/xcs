# Retained inactive benchmark source separation

The pinned post-consolidation transition moves exactly 32 retained Rust benchmark
files (145,025 bytes), without changing their bytes:

- 17 files: `src/foundation/date-time/benchmarks` → `test/foundation/date-time/benchmarks`.
- 15 files: `src/foundation/schema-validation/benches` → `test/foundation/schema-validation/benches`.

This is source-layout/provenance preservation, not benchmark execution or behavior
validation. No other benchmark, fixture, production file, package identity, Cargo
manifest, dependency or target is changed. In particular, both packages retain
`autobenches = false` and no explicit benchmark targets.

## Guarded transition

`tools/rust-sources/benchmark_relocation.py` supports this single pinned transaction,
not a generic move language. The receipt in `compliance/benchmark-relocation/`
contains exact before package, merged-owner and external-test snapshots; complete
before/after inventories; current and original source paths; each byte hash and
length; consolidation IDs; and hashes of the existing frozen proof files. Its
recorded base commit/catalog hash is provenance context, not a lock on unrelated
catalog changes. Installation validates all involved snapshots exactly.

The existing consolidation histories, original test-separation receipt, original
upstream identities and other frozen proofs are unchanged. The new adapter derives
only exact owner/origin/historical-source/hash grants and normalizes only its 32
newly authenticated current-path removals. All ordinary origin, shared merged
inventory, anti-drop, external mapping and unattributed-file checks still run.
Installation also requires the newly loaded external mapping to equal the exact
reviewed after snapshot: old rows and roots cannot be reassigned or discarded.

Later refresh/verify replays the pinned evidence and authenticates all 32 retained
files, their attribution and absence of the old source directories. Ordinary edits
to other implementation files remain supported by the existing checks. The
adapter does not permanently freeze the two packages' implementation inventories.
Changes to these relocated benchmark bytes or activation of benchmark targets
require a separately reviewed extension; an ordinary refresh cannot bless them.
### Explicit lifecycle limit

Current supported operations:

- Ordinary refresh/verify, including unrelated owner integrations.
- Nonbenchmark implementation, documentation and manifest edits in these two
  owners, while retaining their identities and leaving benchmarks disabled.
- Existing external-test edits outside the 32 relocated rows, subject to normal
  provenance, attribution and no-loss checks.

Current unsupported operations without new reviewed chain evidence:

- Editing, deleting, duplicating or moving any of the 32 retained benchmark rows.
- Reintroducing their old source directories or enabling benchmark targets.
- Renaming, reconsolidating, retiring or upgrading either consolidated owner.

This initial adapter does **not** implement subsequent rename, reconsolidation,
retirement, upgrade, or further relocation of either consolidated owner. The
existing manager already rejects such operations on consolidated owners without
a chain-aware adapter; this change does not remove that restriction. It supports
ordinary edits to nonbenchmark implementation files, not every future lifecycle.
The historical 32 routes and receipt must never be overwritten to accommodate a
new owner/path. The smallest future extension is an append-only successor receipt
that names this receipt as its predecessor and binds an existing formal lifecycle
record: validate the exact old/current owner and merged identities, map each
retained benchmark to exactly one new live destination with the same original
hash/attribution, derive grants for that successor owner, and retain all prior
proof checks. Retirement needs an explicit reviewed disposition and cannot be
inferred from a missing owner or directory. That extension and its negative tests
are intentionally outside this change; general lifecycle support is not claimed.

Missing/replaced evidence, symlinks, path aliases, partial moves, changed bytes,
wrong origins and dropped historical attribution fail closed. Catalogs unrelated
to both scoped owners and their evidence have no relocation obligation.

## Reproduction

After applying the exact physical moves and external mapping in this change,
install once with `python tools/rust-sources/manage.py refresh --relocate-benchmarks`.
It cannot be combined with another lifecycle plan in the same refresh. Subsequent
checks use ordinary `refresh` and `verify`. The option does not move files itself.
Use the repository's configured Rust toolchain for metadata resolution.

Run the guard suite with:
`python -m unittest discover -s test/tools/rust-sources -p test_benchmark_relocation.py`.
Run all source-accounting regression tests with the same discovery directory and
`-p 'test_*.py'`, followed by source `verify`, ordinary `refresh`, and a no-diff
second `verify`.

## Execution limitations

Historical date-time benchmark entrypoints include an intentional `compile_error`
and old criterion invocations. Schema benchmarks reference historical criterion
providers, crate names and fixture paths that are not restored by this transition.
These benchmarks remain inactive. This change makes no claim that they compile or
run; provider/API/fixture restoration and reactivation need separate review.
