# Detached workspace commit preparation

The HTTP and SMB application boundary uses an owned preparation API for ordinary
workspace content commits. The original synchronous core APIs remain available
for embedded callers and recovery workflows.

## Boundaries

1. While holding the live Engine mutex, authorize the request, capture its exact
   workspace, configuration, state and publication base, and acquire one
   per-Engine preparation admission. No payload is read in this step.
2. Release that mutex. A private, non-publishing planning snapshot validates the
   request and builds the unpublished mutation. Source hashing, complete payload
   copying, verification, metadata preflight and content-addressed metadata writes
   run here. Independent file payload preparations use at most four workers;
   versions of one file and that file's protection route remain sequential.
   This phase writes no PREPARE, ACCEPT, FINALIZE or PUBLICATION record and does
   not mark the live workspace committed.
3. Reacquire the live mutex. Refresh protected account status at the application
   boundary and recheck current repository grants, workspace owner and fence,
   lease expiry, exact workspace contents, configuration, publication snapshot,
   sequence, decision slot, and business/topology generations. Then publish the
   single serialized decision and update the live workspace.

The live Engine conservatively fences other ordinary writes, protected account
publication, purge cleanup, and degraded recovery publication while preparation
owns its admission. They return retryable COMMIT_PREPARATION_BUSY; reads remain
available. This prevents physical container relocation during payload install.
Workspace abort may still invalidate the prepared workspace without removing its
staged input. The final authority and exact-base checks remain in force: any
stale base yields a definite retryable conflict before ACCEPT, not an
indeterminate operation. Retry with the same operation ID and request digest.
See [write-admission details](detached-write-admission.md) for scope and limits.

## Verification and lifetime

Detached publication preparation fully verifies each replica and retains opaque,
non-serializable evidence bound to its held file descriptors, metadata, disk
identity and candidate. The locked decision path rechecks that exact evidence
instead of rereading the payload. Per-payload held descriptors are capped at 128
(four per replica). Larger candidates retain only disk identity handles and fully
reverify replicas sequentially during preparation, before the first decision
record, and before issuing each payload receipt. This bounded-descriptor fallback
supports large batches but performs full payload reads in the final critical
section. It does not substitute timestamp-only evidence for held inode proofs.
Publication still requires verified complete replicas; recovery has no live proof
and performs complete verification.

The preparation owns duplicate handles for the pool's existing process locks.
Those locks remain alive if the original Engine owner is dropped during work.
The preparation admission follows a running blocking task through request
cancellation and is released on completion or drop. Cancelled or stale preparation
may leave immutable unreferenced payloads/content-addressed metadata. It cannot
publish a stale workspace or remove the original staged input.

## Cost and limits

There is one detached workspace preparation per Engine, not a process-global
limit shared by unrelated pools. This bounds the additional metadata snapshot to
one extra full State clone: the peak is live state, frozen planning state and
planned next state. Workers share the snapshot; they do not clone State per file.
The snapshot copy itself still takes the mutex and is proportional to metadata
size. This phase does not introduce a copy-on-write State representation.

Other mutation classes, package import/export, historical-save operations and
recovery remain on their existing synchronous paths. This is not a claim that
all Engine work is lock-free. Deterministic tests check boundaries, verification,
staleness, authorization, fault points and bounded worker count; no throughput
claim follows from these structural tests or tmpfs timings.
