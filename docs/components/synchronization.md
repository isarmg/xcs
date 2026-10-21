# Synchronization

`synchronization` owns generic lock contracts, thread parking and concrete locking primitives in one package. It is optimized for this project's feature graph, not for preserving upstream crate boundaries. The three origins remain independently recorded in the source catalog and immutable provenance documents.

## Capability boundaries

- With defaults disabled, only `api` and dependency-free guard machinery plus `scopeguard` are available. This is `no_std`; it does not activate libc, thread-local parking or concrete locks.
- `arc_lock` adds allocation-backed owned guards; `atomic_usize` adds generic reentrant locks. Neither requires std.
- `parking` enables std and the low-level wait/wake queue, libc on Unix, cfg-if and smallvec. A low-level once-cell consumer does not need concrete locks.
- `locks` adds Mutex, FairMutex, RwLock, ReentrantMutex, Condvar and Once, with one shared parking queue. This is the default.
- `serialization`, `send_guard` and `hardware-lock-elision` are explicit. Guards are non-Send unless `send_guard` is enabled. Serialization remains optional and uses the project's one serialization identity.

The user-facing runtime parking switch is `parking`, and once_cell's optional backend switch is also `parking`. `futures-intrusive` explicitly activates locks only for std; spin's generic lock API remains no_std. No compatibility facade crates remain.

## Safety and validation

The underlying acquire/release ordering, fair handoff, timeout callbacks and address-stable parker lifetime contract are unchanged. The unavailable experimental deadlock detector and its no-op hooks are removed, rather than exposing a feature whose backtrace and graph providers are missing.

The pinned parking-core origin is the true consolidation primary; imported lock API and concrete-lock files are never assigned to its historical projection. Every historical primary file remains present and reversibly authenticated. Historical Unix/generic/atomic-WASM parker source files are still retained under their target cfgs due to this exact proof boundary; Linux amd64 is the supported native target and plain browser WASM is checked, not a claim of atomic-WASM runtime support.

The maintained gate checks native/WASM capability graphs, missing API negatives, default non-Send and opt-in Send guards, contention/fair handoff, mapped/owned guards, recursive locking, downgrade, condition predicates, timeouts, no poisoning and real JSON serialization. Original lock/core unit suites remain executable. The old rand-dependent reader/writer stress case now has a clearly named deterministic mixed schedule; this is not random scheduling or Loom/Miri coverage. The issue-129 test uses a startup handshake rather than a 100ms scheduling assumption. Original bincode-specific optional test cases and retired detector tests remain attributed; missing bincode and detector providers are not emulated.
