# Detached preparation write admission

Detached workspace preparation releases the Engine mutex for payload I/O, but
it does not permit concurrent physical namespace mutations. The live Engine's
preparation claim fences ordinary write admission until the owned preparation
is dropped or consumed under exclusive Engine access. The private planning view
uses an independent admission flag so it can build its unpublished mutation.

This deliberately conservative restriction prevents a container rename from
moving a directory after payload installation resolves its path and before it
creates or opens the destination. Final state-generation checks alone cannot
repair filesystem side effects from that race.

Queries, content reads, checksums, and disk observations remain available. They
do not repair containers or purge payloads. Ordinary writes, upload completion,
protected authentication publication, purge cleanup, and degraded recovery
publication reject with retryable COMMIT_PREPARATION_BUSY. An already running
upload may finish copying its private scratch bytes, but cannot publish its
workspace manifest until admission is free. Workspace abort may still fence the
workspace because it changes only its manifest and preserves staged payloads.

The finisher releases its own claim only after regaining exclusive Engine
access, then repeats all original authority, lease, workspace, and publication
base checks. Cancellation retains the claim for as long as the worker owns it.
A busy protected-authentication write is a definite pre-publication rejection:
its local account changes roll back without latching authentication into an
indeterminate recovery state. Other backup errors retain fail-closed behavior.

Both COMMIT_PREPARATION_STALE and payload-proof PREPARATION_STALE map to HTTP
409; retryability is retained. This is read concurrency, not general concurrent
write support.
