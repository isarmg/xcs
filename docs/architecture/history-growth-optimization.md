# Retained-history growth: second optimization pass

This pass reduces repeated work without deleting history or changing the
authoritative journal, snapshot hash, or corruption/repair requirements.

## Stable snapshot boundaries

Snapshot map pages use deterministic content-defined boundaries on entity keys.
A key hash selects a boundary with nominal probability 1/32; the existing
128-entry hard cap remains. Values do not select boundaries, so a value update
does not move other entities between pages. An ordinary insertion usually
changes a nearby page rather than shifting all later positional pages.
Adversarial key distributions can still hit the hard cap and have worse sharing;
this is not a constant-write guarantee.

The root and page wire representation remains v2. Readers accept old fixed-count
pages and new pages. Tombstone admission uses the same page builder for a new
publication and the actual hash-bound pages for an existing recovery candidate.
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
This avoids generating orphan new-layout roots when opening legacy journals.
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

## Remaining costs and compatibility limits

State construction, full history manifests, and retained event lists still grow.
Full manifests remain bound by their existing logical hashes in candidate and
portable-package proofs; they have not been silently replaced with a different
storage format. Journal/coverage verification is still required. No retained
history was removed, and no physical disk or power-loss guarantee follows from
the development probes.
