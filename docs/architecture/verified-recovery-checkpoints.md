# Verified recovery checkpoints: implementation boundary and next protocol

## Current implementation

Journal append now retains a bounded, process-local verified tail. Each journal
entry contains its device/inode, length, nanosecond modification/change times,
and receipts for only its latest decision slot. At most 64 journals and 64
receipts (and 64 KiB of receipt string data) per journal are retained. Cold access, an older receipt retry, changed
file identity/metadata, or eviction performs the ordinary full verified scan.
Failed writes invalidate the entry. Successful writes become cacheable only
after the file and containing directory are synchronized. Replacement and
truncation invalidate the entry; same-size external rewrites also invalidate it
via change time even when modification time is restored. This uses the existing
exclusive Engine ownership locks; unsupported concurrent external writers are
not made safe by a cache. Metadata fingerprints are not cryptographic evidence
against silent storage corruption or adversarial timestamp/inode manipulation.

The append cache is **not a durable checkpoint**. A separate conservative
version-1 replay checkpoint is now implemented in `engine/recovery_checkpoint.rs`.
After a successful full recovery, it writes matching content-hashed checkpoint
envelopes to all three metadata members. A later clean open still scans every
journal from byte zero, rejects forks and verifies candidate linkage and every
publication seal. It can skip historical snapshot decoding only when all three
checkpoint copies agree, the exact verified journal tips agree, the pool/config
binding agrees, and a fresh cryptographic inventory of all repair-relevant
metadata, historical proofs and layout matches the completed recovery.

The inventory hashes metadata/proof bytes and covers payload presence, type,
name and size. It excludes modern named-version payload bytes: ordinary historical recovery
also does not reverify those bytes or claim fresh protection. Legacy/content
payload bytes are hashed because historical name reconciliation revalidates
legacy bodies during migration, including previously migrated bodies. All
configured disks must validate for this optimization. Missing/corrupt/added
metadata, renamed paths, changed configuration, missing members, torn tails or
new decisions fall back to ordinary full recovery/repair. The checkpoint loads
one final authoritative snapshot and applies its recovery-result overlay.

This is deliberately a clean-restart checkpoint, not yet a suffix-replay
checkpoint: exact inventory/tip matching conservatively invalidates it after a
new decision. Checkpoint creation occurs on full recovery, never on the commit
hot path. Partial/malformed checkpoint writes are optional hints and cannot
publish a decision. No history is removed, no recovery authority is transferred
to a checkpoint, and no bounded total startup claim is made.

## Why a latest-snapshot pointer is insufficient

Recovery currently verifies candidate-to-previous-snapshot linkage, resolves
accepted but unpublished decisions, and repairs immutable snapshots, history
manifests, candidates, publication seals and historical local proofs. It also
synchronizes container names. Skipping directly to the latest sealed snapshot
would skip these repair obligations. A recovered publication can additionally
overlay its operation result on the candidate snapshot; a checkpoint must bind
that final visible state rather than accidentally restoring the unoverlaid
candidate snapshot.

Neither a checksum stored beside a checkpoint nor a publication seal by itself
proves that all preceding decisions have been checked or that their historical
repair dependencies remain available. File metadata alone is not an acceptable
durable trust anchor.

## Next stage: selective prefix coverage and suffix replay

1. Define a versioned, canonical immutable checkpoint object binding the pool,
   exact membership/configuration epoch, completed sequence, final visible state
   hash (including recovery result overlays), previous checkpoint, and each
   member's journal byte boundary, final frame hash and maximum observed slot.
2. Build a content-addressed repair-coverage manifest for the covered prefix.
   Bind all candidate/publication/seal dependencies, historical snapshot and
   history-manifest roots, and required local commit/publication proofs. Describe
   deliberate purge/loss states explicitly; never infer that missing payloads
   have been newly verified. Bind a current container-name reconciliation plan.
3. Write and synchronize checkpoint dependencies and coverage manifests on all
   required members before writing checkpoint receipts. Require a completed
   three-member checkpoint seal before selecting the checkpoint. A crash at any
   earlier step leaves the existing journal recovery path authoritative.
4. At open, retain the full journal integrity scan and accepted-candidate/fork
   checks. Verify checkpoint boundaries against those actual frames and verify
   prefix linkage/repair coverage. Load the checkpoint's final visible state,
   reconcile names, then replay only the suffix. Missing coverage triggers full
   historical repair. Actual authoritative corruption remains an error, not a
   reason to quietly accept a cache or roll back committed history.
5. Add read-only/offline selection with exactly the same publication visibility
   rules. Two available members must not advance an unsealed decision. Restore
   from data disks must discover and validate checkpoints without the system
   disk. Legacy stores must continue to open without a checkpoint.

This next stage bounds *state replay* and avoids repeatedly decoding all historical
snapshots when coverage is valid. Full journal and coverage verification can
remain linear. Benchmark and label scan, coverage verification, snapshot decode,
repair and suffix replay separately.

## Required failure and regression matrix

- Crash before/after each dependency write, checkpoint receipt and seal; partial
  checkpoint replication; restart with one member absent; restore after system
  disk loss; accepted/unpublished suffixes at every existing decision failpoint.
- Truncated, malformed and checksum-invalid checkpoints; wrong pool or membership;
  boundary not on a frame; wrong prefix hash; stale/newer checkpoint; conflicting
  checkpoints; forked candidate hidden before or after a checkpoint.
- Missing/corrupt historical snapshot page, history manifest, candidate,
  publication object, local commit proof and publication proof. Confirm repair
  parity with full replay or the same explicit error where repair is impossible.
- Renamed containers, intentionally purged versions, lost payloads and degraded
  recovery result overlays. Preserve original operation receipts and visibility.
- Legacy format compatibility, old operation idempotency, journal torn-tail
  recovery, missing checkpoint fallback, and no authoritative history deletion.
- Demonstrate reduced historical snapshot decoding and bounded suffix replay;
  report remaining scan and coverage costs honestly over growing histories.

## Truly bounded total startup is a separate protocol decision

Immediate detection of arbitrary corruption anywhere in retained historical
journal bytes requires reading those bytes (or a storage integrity mechanism
with an equivalent verified trust guarantee). A persisted hash does not verify
unread bytes. Segmented journals with sealed checkpoint roots can bound the
active segment, but skipping old-segment reads changes *when* latent corruption
is detected. Such a design needs an explicit integrity/scrubbing policy,
authenticated segment ownership/roots, durable segment closure, retention and
repair rules, and separate offline full-history verification. It must not be
introduced as an incidental performance-cache change or silently relax the
current recovery contract.
