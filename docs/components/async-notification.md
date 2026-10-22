# Async notification capability

`async-notification` owns one notification queue implementation and the async locks
built on that implementation. It consolidates the former `event-listener` 5.4.2,
`event-listener-strategy` 0.5.4 and `async-lock` 3.4.2 source packages (three to one,
net two fewer active packages). There are no forwarding Cargo packages or dependency
aliases for the retired names.

## Boundaries

- Public `event` module: event, listener and notification primitives; available
  without `locks`. The exported `listener!` macro constructs a stack listener.
- `locks`: barrier, mutex, once cell, read/write lock, semaphore, owned guards and
  named futures. Poll/block strategy adapters are private implementation machinery.
- Default `std`: original standard-library queue implementation and native-only
  parking. It does not enable `locks`.
- Without `std`, the original atomic/spin queue fallback remains; this is not a
  new recommendation for production workloads. WASM never links `parking` and
  does not expose native blocking waits.
- SQLite retains its original standard-library event behavior without enabling
  locks. Reactive-state explicitly enables `locks` and retains its owned RwLock
  guard contracts. Consumer source names are migrated, not aliased.

This is ownership and API-boundary consolidation, not a measured speedup. Queue
ordering, notification transfer on cancellation, lock acquisition algorithms,
atomic orderings, unsafe implementations and native/WASM policy are preserved.
The generic private strategy wrapper's unused reference accessors were removed;
its generated `Future::poll` calls the same nonblocking strategy directly. The
remaining adapters still support native blocking calls. Semaphore notification
ordering is not a promise of strict FIFO permit acquisition: `try_acquire` may
barge. Mutex eventual fairness is not strengthened into a FIFO guarantee.

## Verification

Run `python3 .github/scripts/check-async-notification.py` with the maintained
Rust toolchain and installed native/WASM targets. The gate is registered in the
foundation-async suite and uses temporary fixture workspaces outside the repository.
It checks eight native/WASM std/locks combinations; compile-negative lock/blocking
boundaries; native-only parking; independent SQLite/reactive feature resolutions;
real wake-driven contention/cancellation contracts; SQLite close/acquire cancellation;
and the preserved reactive async-derived tests against the actual Tokio provider.
The helper executor never repolls after a timeout or spurious unpark without a
recorded wake, so timeouts cannot conceal a missing notification.

Three upstream inline tests remain active in the library. The rest of the eleven
original external test files remain attributed and recoverable under
`test/foundation/async-notification/<original-package>`; upstream integration
files requiring unavailable historical development dependencies are retained
source evidence, not claimed as executed coverage. Only the intrusive inline
fixture changes its pin macro to the standard-library equivalent and qualifies
the internal event namespace.

## Provenance and source recovery

The explicit consolidation receipt keeps all three original identities, licenses,
complete input inventories, exact production routes, and bounded reversible test
transfers. Original production bytes (including manifests and the event benchmark)
are additionally retained under `compliance/provenance/async-notification` as text
archives, authenticated against the reviewed input commit by the delivery proof
generator. Existing per-origin license and provenance records are unchanged.
The original event benchmark is retained without being enabled as a new benchmark.

Integration must regenerate the plan against the receiver's current catalog and
verify exact input snapshots before applying guarded mapping updates and refresh.
It must not overwrite unrelated catalog or external-test owner updates from other
parallel changes.
