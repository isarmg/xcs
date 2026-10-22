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

The append cache is **not a durable checkpoint**. The separate durable
replay checkpoint is implemented in `engine/recovery_checkpoint.rs`. After a
successful full or suffix recovery, it writes identical content-hashed envelopes
to all configured metadata members. Selection requires agreement between all configured
copies, the same pool/configuration, and each saved tip appearing in that
member's fully verified journal chain. Startup still scans journals from byte
zero and rejects forks and candidate-linkage errors.

A bounded coverage manifest records repair-relevant files and directory rules:
- Metadata and proof bytes are cryptographically hashed and reread on selection.
- Existing payload paths, sizes, types and link counts remain covered. Modern
  named-version payload bytes are excluded because historical replay does not
  reverify them. Legacy/content bytes ARE hashed because legacy migration does
  reverify them, even after migration. Namespaced containers are identified by
  their ownership markers, not a fixed path depth.
- New entries are allowed only in recognized append-only namespaces. Stable-file
  and named-container directory layouts remain exact to catch rename ambiguity.
- Absent historical version/history/container paths are recorded explicitly.
  Restoring such a path forces historical proof/name repair instead of silently
  skipping an obligation that was previously inapplicable.
- A derived `container.json` may differ only for a file/history with genuinely
  new version evidence in a continuous, fully sealed suffix. Suffix replay
  reconstructs that cache. Forks, pending decisions, reused version IDs and
  unrelated suffix activity do not authorize this exception.

A valid checkpoint loads one authoritative boundary snapshot, applies its
recovery-result overlay, and replays subsequent decisions normally. Missing or
changed covered dependencies, renames, unknown layout changes, missing members,
legacy byte corruption and torn tails conservatively trigger full recovery.
No checkpoint is built on the commit hot path. Coverage is capped at 8 MiB and
32,768 entries per disk, with a 16 MiB envelope limit; exceeding a cap disables
this optional acceleration rather than weakening verification.

Partial/malformed checkpoint writes cannot publish a decision. No history is
removed, no publication authority is transferred to a checkpoint, and no bounded
**total startup** claim is made. Journal scan and coverage verification still
scale with retained history. A seven-decision prefix followed by three ordinary
same-file saves is tested to decode four snapshots instead of ten; the next
unchanged open decodes one.

Recovery also tries healthy replicas of the exact candidate-bound snapshot root
when its first source is missing or corrupt. After a complete snapshot verifies,
missing/corrupt regular snapshot/page replicas can be atomically repaired from
that hash-verified source. Unsafe targets still fail, and failure of every source
fails closed. This never substitutes a different snapshot or suppresses a fork.

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

## Remaining extensions

The current bounded inline coverage format deliberately falls back once its
size limit is reached. A paged, independently hash-bound coverage tree could
extend acceleration to larger stores without an unbounded checkpoint object.
Incremental coverage refresh could reduce checkpoint construction work. Degraded
(missing configured members) opens currently keep ordinary recovery and its visibility rules;
they do not use the replay shortcut. Further optimization must preserve those
rules and all historical repair obligations.

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
