# Storage-independent management

The daemon owns a system-disk management root independently of configured data-disk roots. HTTP assets, login, accounts and host diagnostics do not open the storage engine on their request path. Engine open/recovery runs in one bounded background worker. A hung worker retains its admission slot; HTTP timeouts do not spawn replacement workers indefinitely.

## Authority and revocation

`management-auth.json` is the durable primary for accounts, grants and sessions. Engine copies are disaster-recovery snapshots, never an authorization fallback. The old `auth.json` cache cannot authorize offline login. A durable `management-authority.json` fence in the system state root prevents automatic replay of old engine credentials after primary loss. A missing or invalid primary is recovery-required, not an empty installation or a default administrator.

Account changes first commit to the system primary. Storage admission closes while the newest account snapshot is asynchronously copied; only a matching current snapshot can reopen admission. Sessions are not included in disaster-recovery copies. Replica failure cannot roll back a password change, revocation or durable account change.

At actual engine dispatch, after acquiring the engine lock, managed requests revalidate against the current primary, including session expiry, logout and account generation. There is no engine-snapshot fallback if the management authority disappears. Already-admitted operations may finish; this boundary is not rollback of work already executing. Management authentication never acquires the engine lock.

## States and recovery

Public `/api/management/status` exposes only management liveness, authentication availability and storage state: `ready`, `read_only`, `unavailable`, `unconfigured`, or `recovery_required`. Authentication availability is `ready`, `busy` (a credential operation currently owns the state lock), or `recovery_required`. `busy` is transient, does not diagnose credential corruption, and must not trigger logout or recovery. Status probing never waits for that lock. It does not disclose disk paths or credentials. Missing storage configuration alone does not authorize setup.

Storage observations and capabilities are cached. A stale observation closes storage admission after fifteen seconds rather than leaving readiness advertised indefinitely. Host telemetry remains available with disk sampling marked unavailable. Data operations fail closed when admission is unavailable. Readiness/capabilities and protected setup use management capacity independently of blocked data requests.

Authenticated administrators may request `/api/system/storage/retry`. Recovery is serialized. A faulted engine cannot be replaced while a request, observer or transfer retains it; `STORAGE_DRAINING` explicitly asks the caller to retry after those holders finish. A permanently blocked disk call may require a daemon restart or host-level intervention. A restart does not reset authentication.

First setup and explicit disaster recovery use the separately documented protected Web setup protocol. A local, expiring bootstrap capability is required when no valid primary exists. Restoring old credentials requires acknowledging that older passwords or grants may return; restored sessions are always discarded. Authenticated storage restoration preserves a valid system primary.

Initialization completion is deliberately narrower than credential recovery: only a pristine pool bound to this Web setup's durable intent and post-initialization proof, before any authority or credential publication, can complete. Existing authority markers always block this path, even if the primary is missing. Other interrupted stages remain fail-closed for explicit local diagnosis.

Locally provisioned configuration presets are identifiers, not browser-supplied paths or arbitrary JSON. Their files are not read during HTTP startup or preset listing. An authorized preview reads only the selected protected file, hides disk paths, and returns a configuration digest. Initialization checks that digest again before writing so a changed local preset cannot silently substitute another topology or policy.

## HTTP resource isolation

Password verification runs outside the shared credential-state lock. Session
publication rechecks the account generation, password hash, enabled state and
recovery fence under the lock; a concurrent password/role change cannot issue a
session from the old proof. Durable publication remains serialized.

Each router admits at most eight ordinary authorization, account-list and logout
jobs before scheduling blocking work. Upload-status authorization, backend status and spool inspection
share one job holding both backend and account-scoped read admission. These
permits remain owned by actual blocking work after HTTP cancellation.

Telemetry status and history share an independent two-job pool, with admission
retained until blocking work completes, including after HTTP cancellation. They
do not consume data-backend or management-readiness capacity.

Mutating API requests have a separate 64-request ingress bound before body
collection. Ordinary JSON/login body collection has a 30-second total deadline
as well as its existing byte limit. Setup bodies retain their stricter 10-second outer
deadline. Streaming uploads retain their separate upload limits and deadlines. Read-only health and management status
requests do not consume the mutating-request pool.

Optional SMB startup waits and directory preparation run off async workers. A
termination signal requests HTTP graceful shutdown and immediately stops optional
SMB admission and closes its connected sockets, without waiting for HTTP drain.
HTTP completion, error, or service-future cancellation also drops the owned bridge;
it is not a detached background listener. An optional bridge failure still leaves
management HTTP available.

Admitted HTTP requests drain normally; no arbitrary timeout abandons publication
or mistakes dropping a server future for cancelling the HTTP library's connection
tasks. Already-running SMB/backend blocking work retains its permits and recovery
evidence until it returns. Socket shutdown wakes idle RPC reads, but cannot cancel
a filesystem system call. HTTP drain and the Tokio runtime's blocking-worker join
are therefore not bounded when kernel I/O cannot return; host-level intervention
may still be required. No shutdown path force-exits the process to claim success.
