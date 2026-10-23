# Concurrent collections

`src/foundation/concurrency/concurrent-collections` is one Rust package with four explicit API
namespaces: `utilities`, `epoch`, `queue`, and `deque`. It replaces the separate
Crossbeam utility, reclamation, queue, and work-stealing packages. Rayon and SQLite access
consume this same package, not compatibility aliases.

- No features: no-std utility atomics, backoff, and cache padding.
- `alloc`: epoch reclamation and allocating queues when pointer atomics exist.
- `std` (default): scoped threads, synchronization utilities, the default epoch
  collector, and work-stealing deques. `std` implies `alloc`.
The empty upstream `nightly` compatibility feature is removed: it selected no
retained implementation. A negative Cargo-resolution probe prevents its return.

Each original module keeps its own statics, thread-local collector and primitive
wrappers. Imports are qualified within those boundaries. Atomic orders, fences,
unsafe operations, ownership/drop behavior, and the reclamation algorithms are
unchanged. Sanitizer/no-atomic build detection is preserved. The combined build
calls the three retained detection entries rather than silently dropping one.

## Origins

The active primary is crossbeam-utils 0.8.23. Imported origins are crossbeam-epoch
0.9.21, crossbeam-queue 0.3.14, and crossbeam-deque 0.8.8. All originate from
https://github.com/crossbeam-rs/crossbeam and retain MIT OR Apache-2.0 licensing.
Exact upstream identities, file routes, retained tests and reversible import
edits are recorded by the consolidation receipt and central compliance inventory.
Original license material and upstream documentation are not rewritten.

## Validation and limits

The isolated candidate runs 41 retained unit tests; 60 independently runnable
retained integration tests; four additional Rayon, queue, destructor and stealing
checks; 265 passing documentation tests (five upstream ignored); six native and
WASM feature checks; and a negative no-alloc API check. Rayon and SQLite access consumers
compile against the same package. Order/unsafe/static/TLS token inventories were
compared as an additional review aid, not as a semantic proof.

The six integration sources requiring an absent `rand` dev provider remain
preserved but are not counted as executed. The Loom test source and configuration
branches remain preserved; no Loom provider is present and no Loom or Miri run is
claimed. WASM results are compile checks, not browser-thread execution. These
checks do not establish exhaustive schedule coverage or a performance gain.

## Maintained verification

Run `python3 .github/scripts/check-concurrent-collections.py`. The foundation CI suite runs this gate: six native/WASM feature builds, allocation-API rejection, 41 retained units, 60 retained integration tests, four concurrency/Rayon checks, and 265 doctests (five original ignored examples). Disposable manifests and locks never alter the root lock or governed source bytes. Six integrations requiring missing rand, Loom, Miri and exhaustive scheduling remain explicitly unverified.
