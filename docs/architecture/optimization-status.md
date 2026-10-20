# Architecture optimization status

This records the architecture work started from `92cd662`. It is not a claim of
production fault certification or constant-time operations.

## Implemented

- Web and native bridge share commit preparation, response parsing, size limits,
  and mutation-outcome rules in `zxs-contract::client_protocol`.
- The server uses typed application commands for high-risk business mutations.
  Namespace relocation centralizes identity-preserving path/name updates.
- SQLite projection updates only changed entities inside a transaction. The
  authoritative filesystem/journal recovery model remains unchanged.
- Journal appends reuse a bounded verified-tail cache, invalidating it when the
  underlying journal identity or contents change.
- HTTP download ranges verify touched checksum chunks with replica fallback.
  SMB's direct file-descriptor API still requires full verification.
- Upload preparation, download planning/streaming, and monitoring I/O are
  separated from the live Engine mutex.
- Workspace commit copying, hashing, and metadata preparation use an owned
  non-publishing snapshot outside that mutex. Publication remains serialized.
  See [the preparation contract](detached-workspace-commits.md) for admission,
  stale-state checks, cancellation, and bounded-descriptor fallback.
- Verified recovery checkpoints reduce historical snapshot decoding and permit
  suffix replay only after authoritative evidence and repair dependencies pass
  validation. See [checkpoint guarantees](verified-recovery-checkpoints.md).
- A reproducible development workload measures logical SQLite entity writes,
  verifies recovered-state equality and historical reads, and compares against
  the pre-optimization baseline.
- A [second retained-history pass](history-growth-optimization.md) stabilizes
  snapshot page boundaries, reuses invocation-local page repairs and container
  projections, and avoids duplicate canonical serialization.
- Event arrays now use ordered, validated snapshot pages rather than one scalar
  root field. Existing scalar-event snapshots remain readable. Duplicate or
  discontinuous event indexes and ambiguous scalar/page representations fail
  validation instead of silently losing or reordering events.
- Publication prepares bounded immutable snapshot and history-manifest bytes
  once, then reuses their exact hashes and bytes for preflight and replica
  installation. The prepared snapshot descriptor is also reused by tombstone
  admission. Existing immutable-file collision verification remains enabled.
- Snapshot encoding borrows typed state entries page by page instead of creating
  an intermediate JSON tree for the complete State. An exhaustive field
  declaration prevents silent omissions; differential tests retain the exact
  v2 descriptor, canonical object bytes and hashes. The temporary page builder
  buffers at most 128 top-level entries, while final encoded objects remain
  retained until installation.
- New immutable object hashes use a two-hex-digit directory prefix. Verified
  legacy flat objects are reused, not moved or duplicated. Duplicate locations
  must agree; discovery, repair, checkpoint verification and performance
  inventory cover both layouts.

## Deliberate limits

The complete save path is not O(delta). State cloning/comparison, snapshots,
historical manifests, and retained metadata can still grow with history. The
initial workspace preparation snapshot is copied under the mutex. Other mutation
classes and recovery remain synchronous.

Event paging removes the previous single event-array object bottleneck, not all
metadata limits: page references, individual pages and publication proofs remain
bounded. No operation or event evidence is expired, and no historical objects
are garbage-collected by this optimization.

Concurrent namespace relocation must not race detached pathname-based payload
installation. Write admission is therefore conservatively fenced during a
workspace preparation; reads remain available. This is not parallel publication.
Large candidates retain bounded descriptors by using full sequential payload
reverification in the decision critical section rather than weaker proof data.

Checkpoints do not make total startup work constant. Journal prefixes and
coverage dependencies remain verified; oversized coverage and degraded opens
fall back to full replay. Skipping this work would change corruption-detection
and repair guarantees and requires a separate protocol/integrity-policy design.

## Evidence still required

Local fixtures, tmpfs workload timings, and CI do not establish physical disk
independence, power-loss durability, real-device latency, or write amplification.
The [physical-storage validation plan](../../tools/perf-check/PHYSICAL-DISK-VALIDATION.md)
has not been executed. It requires explicitly approved disposable independent
devices and fault-injection methods. Do not treat this hardware-dependent part
of the original plan as completed.

For workload commands and measurement limitations, see
[the probe](../../tools/perf-check/README.md) and
[recorded results](../../tools/perf-check/results/README.md).
