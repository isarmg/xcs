# Storage-independent management

The daemon owns a system-disk management root independently of configured data-disk roots. HTTP assets, login, accounts and host diagnostics do not open the storage engine on their request path. Engine open/recovery runs in one bounded background worker. A hung worker retains its admission slot; HTTP timeouts do not spawn replacement workers indefinitely.

## Authority and revocation

`management-auth.json` is the durable primary for accounts, grants and sessions. Engine copies are disaster-recovery snapshots, never an authorization fallback. The old `auth.json` cache cannot authorize offline login. A durable `management-authority.json` fence in the system state root prevents automatic replay of old engine credentials after primary loss. A missing or invalid primary is recovery-required, not an empty installation or a default administrator.

Account changes first commit to the system primary. Storage admission closes while the newest account snapshot is asynchronously copied; only a matching current snapshot can reopen admission. Sessions are not included in disaster-recovery copies. Replica failure cannot roll back a password change, revocation or durable account change.

At actual engine dispatch, after acquiring the engine lock, managed requests revalidate against the current primary, including session expiry, logout and account generation. There is no engine-snapshot fallback if the management authority disappears. Already-admitted operations may finish; this boundary is not rollback of work already executing. Management authentication never acquires the engine lock.

## States and recovery

Public `/api/management/status` exposes only management liveness, authentication availability and storage state: `ready`, `read_only`, `unavailable`, `unconfigured`, or `recovery_required`. It does not disclose disk paths or credentials. Missing storage configuration alone does not authorize setup.

Storage observations and capabilities are cached. A stale observation closes storage admission after fifteen seconds rather than leaving readiness advertised indefinitely. Host telemetry remains available with disk sampling marked unavailable. Data operations fail closed when admission is unavailable. Readiness/capabilities and protected setup use management capacity independently of blocked data requests.

Authenticated administrators may request `/api/system/storage/retry`. Recovery is serialized. A faulted engine cannot be replaced while a request, observer or transfer retains it; `STORAGE_DRAINING` explicitly asks the caller to retry after those holders finish. A permanently blocked disk call may require a daemon restart or host-level intervention. A restart does not reset authentication.

First setup and explicit disaster recovery use the separately documented protected Web setup protocol. A local, expiring bootstrap capability is required when no valid primary exists. Restoring old credentials requires acknowledging that older passwords or grants may return; restored sessions are always discarded. Authenticated storage restoration preserves a valid system primary.

Initialization completion is deliberately narrower than credential recovery: only a pristine pool bound to this Web setup's durable intent and post-initialization proof, before any authority or credential publication, can complete. Existing authority markers always block this path, even if the primary is missing. Other interrupted stages remain fail-closed for explicit local diagnosis.

Locally provisioned configuration presets are identifiers, not browser-supplied paths or arbitrary JSON. Their files are not read during HTTP startup or preset listing. An authorized preview reads only the selected protected file, hides disk paths, and returns a configuration digest. Initialization checks that digest again before writing so a changed local preset cannot silently substitute another topology or policy.
