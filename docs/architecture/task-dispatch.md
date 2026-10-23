# Task dispatch boundary

`task-dispatch` owns one process-wide executor selection. Its supported backends
are the browser event loop, Tokio, and the futures executor used by native tests.
Initialization is atomic and exactly one caller can succeed. A failed duplicate
initialization does not replace the active backend.

Both `spawn` and `spawn_local` panic before initialization in every build profile.
Tasks must never disappear silently because of release mode or logging features.
Browser dispatch queues both Send and local futures on the browser microtask
executor. Tokio callers must enter their runtime, and local tasks additionally
need a `LocalSet`. The futures backend owns one shared lazy thread pool and one
local pool per calling thread; recursive polling does not borrow a pool twice.
`poll_local` intentionally does nothing before initialization or for event-driven
backends.

The unused generic custom-executor API and unsupported glib/async-executor
adapters are removed. In particular, a thread-local executor may not be published
through process-global function pointers. New backends require an actual consumer
and an explicit execution contract, not compatibility stubs.

Maintained execution tests live in `test/task-dispatch`. The foundation async gate
runs initialization and cross-thread behavior in debug and release and checks
WASM compilation. The web bindings gate also executes the browser dispatch
contract in Chromium. Original upstream tests in `test/foundation/task-dispatch`
remain attributed source evidence; those for removed adapters are not active
contracts. Original source identities, versions and licenses remain recorded by
the repository's compliance inventory and provenance material.
