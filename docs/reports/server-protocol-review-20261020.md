# Server / protocol review, 2026-10-20

Baseline: `6d0b47f`. This pass changes only project-owned server sources. It does
not change third-party dependencies, the SQLite projection, on-disk account or
upload record schemas, permission policy, or SMB wire messages.

## Boundary coverage

Line-level review covered HTTP authentication/admission and account persistence;
upload authorization, staging, progress replacement, interruption, generations
and publication; range parsing, immutable chunk verification, fallback and read
leases; native client upload/download ownership and response decoding; SMB
framing/client serialization, server transport/session/handles/publication, and
VFS synchronous/asynchronous descriptor and event ownership. Reviewed the
application/engine authorization and detached-upload/commit boundaries and
file-manager route plumbing. This is not a claim that every unrelated engine or
file-manager business branch was re-proved in this transport pass.

Contracts checked: security/permissions, client interfaces, failure handling,
architecture optimization status and SMB integration README. The existing
client FIFO fix and SMB removed-descriptor fix were recognized, not re-reported.

## Confirmed defects and changes

### Async authentication could block the entire executor

`http_auth::authorize` took the standard AuthStore mutex directly in an async
handler. Password verification and durable account/session publication hold that
same mutex in blocking jobs. Moving only the password work off-thread therefore
did not protect the executor from lock contention.

A real router on Tokio's current-thread executor, with a controlled 500 ms
AuthStore backup hook, delayed a 10 ms timer to 511.81 ms (a second run: 515.42
ms). After moving authentication and account-list lock acquisition to blocking
jobs, the same timer completed in 13.28 ms. The authenticated request correctly
still waited about 507 ms for current state, rather than reading a stale snapshot.
Account listing/upsert also recheck current administrator status while holding
the mutation/read lock. Existing session, CSRF and admission policies remain.

### Unreferenced upload attempts leaked files

After one interrupted upload, four conflicting same-generation retries returned
409 but left five `.data` files and only one JSON record. The rejected files had
no durable recovery owner. Repeated accepted-but-interrupted retries could also
leave their superseded same-generation predecessor behind.

`StagedFile` now owns a newly created file until its metadata rename. Exclusive
creation and ownership construction occur together in a blocking job, so
cancelling creation cannot strand a newly created file or delete a colliding
pre-existing path. Jobs that can publish metadata share the guard. Rename marks
the file referenced before the directory fsync: a subsequent sync error must not
delete potentially accepted bytes. Only after successful file and metadata
sync does a new same-generation record retire its predecessor. Older generations
are never supplied as cleanup candidates. Rejected/short/overlong/cancelled
replays remove their unreferenced attempt, while accepted progress survives.
The four-conflict probe now leaves one record and one data file.

### Retry reads and generation retention blocked the executor

`ExistingPrefix::open/check` and `retain_generation` performed synchronous file
operations directly inside the upload async handler. A targeted test-only
LD_PRELOAD shim delayed only a matching spool prefix `read` or retained-record
`fsync` by 400 ms. The 10 ms timer took 401.20 ms and 402.98 ms respectively before
the change; afterward it took 11.75 ms and 12.04 ms.

These operations now use blocking jobs carrying the per-item claim/admission
permit and, when created, staging ownership. Cancellation cannot admit a competing
retry while such a job remains active. Prefix comparison remains byte-for-byte;
retention still synchronizes both file and directory. Fresh uploads create no
prefix-read jobs, and retries stop creating them as soon as the durable prefix
has been checked.

These are controlled scheduling results, not measured physical-disk performance
or power-loss certification. On this local development filesystem, 24 paired
1 MiB uploads using 16 KiB body frames measured fresh-upload median/p95
4.468/7.494 ms and same-generation replay 4.515/8.073 ms after the change.
There is no comparable pre-change aggregate-throughput measurement; these numbers
only bound the observed post-change path on this fixture.

## Validation

New regressions cover executor progress while authentication waits for backup,
unreferenced/referenced staging ownership and failed exclusive creation,
conflicting replay cleanup, successive interrupted retry replacement, and
short/overlong/stream-error/cancelled retries preserving the accepted predecessor.
Existing generation-retention and cancelled-publication claim tests still pass.

- Server library: 73 passed, one environment-blocked socket test excluded.
- Real HTTP + durable engine integration: 26 passed.
- The excluded test, `cancellation_closes_blocked_rpc_sessions_and_removes_its_socket`,
  failed at its temporary UnixListener bind with EPERM in the sandbox. The
  attempted elevated invocation failed before execution in sandbox setup. It is
  not recorded as passing and needs a socket-enabled validation environment.
- Tests use `--locked --offline`; no dependency update is part of this work.
- Extra strict Clippy validation is not clean on this baseline: dependency
  checking stops at `storage-contract/src/encoding.rs` (`chunks_exact_to_as_chunks`);
  `--no-deps` reports existing server `collapsible_if` and
  `items_after_test_module` findings. These unrelated style changes were not
  folded into this correctness patch.

The temporary standalone probe used the actual public router and AuthStore, a
stub storage backend for transport isolation, and owned temporary directories.
The slow-I/O shim was test-only and was never linked into production code.

## Preserved boundaries and remaining work

AuthorizedBackend refreshes account state for local protocol operations; SMB
reads still invoke authorization rather than trusting an old FD. Immutable HTTP
ranges verify requested chunks before exposing bytes; fallback does not substitute
another version. SMB retains full verification before passing a raw FD. Framing
owns received descriptors immediately, failed exchanges close the shared
connection, and async VFS workers own buffers/descriptors rather than host pointers.

No speculative rewrite of these boundaries was made. A future structured
UploadAttempt service could group admission, prefix reader and staging record
state rather than passing ownership pieces independently. Any such change should
keep the same cancellation/rename/fsync tests and benchmark small-frame retries.
A two-phase authentication actor/read snapshot design could reduce lock contention,
but requires an explicit fail-closed publication protocol; merely copying account
state outside the lock would weaken revocation guarantees.

Existing process-crash orphan files are not garbage-collected by this patch.
If record rename succeeds but directory fsync fails, both the new referenced
file and its predecessor are deliberately retained. The predecessor may then
lack a current record reference; a later retry cannot safely infer which record
a crash would have preserved. A deterministic injected-sync-failure regression
checks this conservative boundary. This patch does not claim zero orphan files
under uncertain I/O or crash outcomes.
Automated cleanup must first prove absence of active and retained references;
blind age-based deletion could remove recoverable data. Physical-device tests,
Samba production authentication and the README's untested real-client FLUSH/
write-through/cancellation matrix remain separate acceptance work.
