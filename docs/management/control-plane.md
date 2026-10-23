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

## Unconfirmed browser actions

An interrupted account update leaves the current administration view write-fenced.
Refreshing `/api/admin/accounts` is observation only: neither an old nor an updated
snapshot proves that the original request has finished, and passwords are not
represented in that snapshot. There is currently no durable, request-correlated
account-operation receipt/status API, automatic replay, or online unlock action.
Other operation IDs can still be resolved only by their own terminal receipts.

For an unknown account update, do not issue an opposite update from another tab
or bypass the fence by reloading. An operator must quiesce account writers, stop
and restart the daemon so old queued/running requests cannot publish later, then
check the authoritative account state using a valid administrator session. Verify
credentials separately if the update changed a password; account-list equality
is not password confirmation. Only after that reconciliation should a fresh
management session issue further updates. Restarting does not undo a committed
account change or reset authentication. Durable correlated account receipts and
safe online reconciliation remain a separate protocol/persistence follow-up.

When sign-out is unconfirmed, the current page retains the old credential solely
for revocation retry. Login, session restore, setup and business actions in that
page remain unavailable until sign-out succeeds or the server confirms the
session is already unauthorized. A failed sign-out must not be replaced by a new
login in the page holding that pending revocation.

These browser fences and pending logout intent are in memory, not a durable or
cross-tab protocol. App-owned language reloads are blocked, but a manual reload,
closed tab, browser restart or independently opened tab can lose that local
intent and attempt to restore a still-valid server cookie. None of those actions
confirms logout or resolves an unknown account mutation. Keep the failed-logout
page open and retry revocation; do not treat another login or a fresh page as
proof that the earlier session was revoked. Persisted, cross-tab revocation intent
is not implemented by this containment change.

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
Native management recovery writers are owned as well: shutdown closes their
enqueue gate and waits for the current worker without allowing a generation
follow-up. HTTP setup errors and service-future cancellation also retain this
drain in runtime-owned blocking work. The example systemd unit uses
`TimeoutStopSec=infinity` to preserve this durability contract. An administrator
may explicitly force-stop a process stuck in kernel I/O, but that is a crash,
not a successful graceful drain; recovery must complete on the next start.

## Optional HTTP files at startup

HTTP upload staging is optional to management startup. If its directory cannot
be prepared, locked or safely inventoried, login, accounts, protected setup,
storage recovery and host diagnostics remain available. Upload and upload-status
requests return `503 HTTP_UPLOADS_UNAVAILABLE` after normal authentication and
CSRF admission. There is no unbudgeted or in-memory upload fallback. Existing
spool files are retained; repair the local cause and restart to recover inventory.
Immutable content reads do not require the HTTP upload spool.

A configured Web asset tree that is missing, unreadable, incomplete (no
`index.html`), over admission limits, or contains rejected links/special files
is not installed as a static file service. Non-API fallback requests instead
return `503 WEB_ASSETS_UNAVAILABLE` with `no-store`; reserved unknown API paths
remain JSON 404s. Management HTTP APIs still work, but the browser UI is not
available without a valid asset bundle. Keep assets on an independently working
system filesystem to preserve the normal Web interface during data-disk failure.

Public management status includes coarse `auxiliary.http_uploads` and
`auxiliary.web_assets` startup states (`ready`, `unavailable`; assets may also be
`not_configured`), without filesystem paths or internal error details. These are
startup admission results, not continuous probes or a claim of data readiness.
Listener/origin/proxy/admission validation, credential authority, bootstrap
capability checks, CSRF and path rules remain mandatory. Security errors outside
these isolated optional capabilities are not globally ignored.

This startup isolation does not solve the existing static-serving time-of-check /
time-of-use boundary: the admitted asset tree must remain operator-controlled and
unchanged while the daemon runs. The current static service reopens pathnames, so
post-admission replacement (including symlink races) is not safe-open protection.
The next hardening step is descriptor-relative, no-symlink file opening through
response completion, including compressed representations and validators, with
replacement/race tests. A blocked kernel I/O call during startup is also not
bounded by this change; only completed initialization failures are isolated.
