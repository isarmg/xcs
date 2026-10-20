# Native client and App review

> 历史范围说明：2026-10-20 起产品仅保留 Web 与 SMB；本文 App/Tauri 的路径、命令和结果属于移除前记录，不再作为当前构建或验收要求。

Reviewed crates/zxs-client, apps/bridge, apps/host source and manifests, CLI entry/read/write call sites (CLI deep refactor delegated), and cross-checked common API boundary with Web.

Implemented:
- Shared zxs_contract::validate_api_path prevents native URL normalization of percent-encoded dot segments from leaving /v1; preserves query values and rejects controls/encoded separators. Native URL origin and final path are checked again.
- Client transfer and response handling separated from connection/session/commit interface. Downloads use private NamedTempFile/TempPath cleanup and atomic persist_noclobber, removing empty destination reservation and rename race. Static temporary prefix also fixes valid 240-byte destination names. Actual length and immutable hash are checked before publication. Session Debug redacts bearer and CSRF.
- Empty successful HTTP responses (including HEAD/204) return null; non-JSON HTTP failures retain HTTP status and retryability instead of becoming status-zero parsing errors.
- Native Bridge separates login attempt ids from active session revisions. Failed login retains old session; superseded login cannot attach after disconnect/newer intent; stale logout/password/transfer/commit/cache verification results cannot mutate or report into the new connection. Superseded successful login's token is revoked best-effort.
- Cache target helper is shared by download and verified reuse, uses versions/<version>/<current filename> instead of version-prefix-name, preserving 240-byte names and user-facing basename. Strict version id + PortableName validates path/Windows ADS boundaries, private subdirectories reject symlinks. Existing old cache entries are not deleted or implicitly migrated.
- Cache reuse is limited to DESTINATION_EXISTS, not every I/O failure. Cached data is verified against the active session's immutable version before opener invocation.
- Provider import staging is a Rust RAII StagedImport across spawn_blocking result handoff. Read/sync failure and cancelled outer async future remove partial/detached temporary data; successful caller explicitly keeps a 0600 file. Host provider handle/security-scoped resource remains alive for the blocking read only.

Tests: native client real localhost HTTP tests cover verified/private/long-name download, checksum cleanup, concurrent destination preservation, HTTP status and empty replies; API boundary and redacted debug unit tests. App tests cover cache paths/permissions/symlink refusal, stale logout and stale-cache-401 across new session, superseded login, private import handoff/error/cancellation, IPC credential removal. Final native client: 8 tests, contract: 8 tests, App bridge: 11 tests; root workspace includes the client and contract results. App host Linux check passed offline with its locked dependencies and cached local GTK/WebKit SDK. Consolidated native workspace: 219 tests passed. Logs are retained in this evidence directory.

Limits: tests use temporary directories and loopback HTTP; Linux Tauri check uses cached local GTK/WebKit SDK. No Windows/mobile provider/system-opener or signed six-platform package/device acceptance. Existing cache pathname selection rejects symlinked subdirectories, but this is not a claim of cross-platform directory-handle race isolation from a hostile process with identical cache-write privileges. No actual user data or credentials used in these tests.

Independent cross-review found and corrected the stale cache-verification error path and detached import ownership before the final candidate. A malformed seal response now produces an explicit workspace-identity error rather than indexing a non-object or attempting a different workspace.
