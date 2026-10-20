# Retained-history growth: current implementation

The current implementation reduces repeated work without deleting retained
history or weakening the authoritative journal, exact candidate identity, or
corruption/repair requirements. Historical performance reports describe their
original commits; they do not establish performance for the current source.

## Stable snapshot boundaries

Snapshot map pages use deterministic content-defined boundaries on entity keys.
A key hash selects a boundary with nominal probability 1/32; the existing
128-entry hard cap remains. Values do not select boundaries, so a value update
does not move other entities between pages. An ordinary insertion usually
changes a nearby page rather than shifting all later positional pages.
Adversarial key distributions can still hit the hard cap and have worse sharing;
this is not a constant-write guarantee.

The project uses one current, unnumbered root/page representation. Valid page
partitions are checked structurally, not selected by a release-version branch.
Tombstone admission uses the same page builder for a new publication and the
actual hash-bound pages for an existing recovery candidate.
Recovery repairs the original root and pages, never substitutes a new-layout
root for an old candidate.

The page builder is an iterator, avoiding a second complete collection of cloned
map pages. Immutable replica storage canonical-encodes once and hashes those
same bytes, preserving the domain-separated digest and byte limits.

## Recovery-local repair reuse

A recovery invocation remembers page hashes only after all required replicas
were safely verified/repaired. Repeated references to those immutable pages do
not repeat the same repair work. The cache is limited to 16,384 hashes; entries
beyond the limit take the original full repair path. It does not survive a
restart and never replaces the initial byte verification.

Exact snapshot repair also replaces redundant reserialization during replay.
This avoids generating replacement roots that are not bound by the original
candidate during journal replay.
Historical-only pages remain covered by repair; unsafe targets and irreparable
corruption still fail.

## Container inspection projection

One call-local projection indexes merge events and memoizes first-parent tag
inheritance. Version records are constructed once per file/history and reused
across replica caches. Metadata precedence, explicit parent overrides, parent
ordering, cycles, missing nodes, and serialized fields retain their behavior.

A deterministic 4,096-node chain requires 4,096 ancestry visits rather than
8,390,656 repeated visits, with no new visits when another replica requests the
same records. This is an algorithmic counter, not a throughput benchmark.

## Paged history roots and remaining costs

Each candidate binds a `PagedHistoryRoot` for every history. Its fixed map
references reach domain-separated `history-index` and `state-page` objects;
portable-package proofs preserve that exact root identity, not the hash of a
flattened logical manifest. Readers validate field sets, routing prefixes,
depth, duplicates, object hashes, and the reconstructed semantic graph. The
obsolete flat-manifest representation is not a compatibility reader.

History dependencies can be selected object by object from different verified
members. This does not generalize to State snapshots: snapshot verification
currently requires one member with a complete correct root and page set.
Recovery must not silently substitute a newly encoded root for a bound one.
See [the paged-history decision](../adr/0004-paged-history-manifests.md) and
[implementation limits](optimization-status.md).

State construction/cloning, complete logical-history projection, retained
events, and proof/coverage verification still grow with retained state. A
bounded page or individual metadata object is not a bound on total package
objects, import traversal memory, or complete save cost. This is not end-to-end
O(delta), constant-memory operation, or a physical-disk durability guarantee.
