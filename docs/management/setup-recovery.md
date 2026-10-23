# Protected setup and offline management boundary

`storage_server::setup_recovery::SetupRecovery` is the synchronous service boundary for explicit first setup and isolated recovery. Its public and protected HTTP route groups are merged by the managed runtime, which serializes operations with backend open/retry and installation.

## Local deployment inputs

The system directory, management directory, and allowlist mapping opaque disk IDs to existing absolute disk directories are trusted local configuration. HTTP requests cannot supply filesystem paths. Missing disk directories are not created: a missing mount is an unavailable disk, not a new disk. Unavailable allowlisted members remain status descriptors rather than preventing checks or restore from other, selected surviving members. First initialization still requires the complete configured allowlist to be available and empty. Production setup requires each configured disk root to be an active mountpoint, followed by core independent-Btrfs verification. Development simulation is denied unless explicitly enabled in trusted local deployment settings. Directory device/inode identities are bound and rechecked.

For first setup, a local administrator must provision `management_root/bootstrap.key`: exactly 64 hexadecimal ASCII characters, no newline, mode 0600, regular file, one hard link, no symbolic link. The service never generates or logs a bootstrap secret. Loopback access alone is not authorization. An adapter transmits this secret in `X-ZXS-Bootstrap`, never a URL, browser persistent storage, or logs. This capability is distinct from a normal account session.

The bootstrap key is bound to the configured system and management directory. Removing or changing the key revokes previously created in-memory permits. Keys expire 30 minutes after their filesystem modification time; in-memory permits additionally expire after 10 minutes. Existing management credentials cannot be bypassed by the bootstrap restore operation, including a corrupt credential file. No default account, missing-auth credential reset, disk formatting, or anonymous remote administration is implemented.

## DTOs and adapter obligations

- `SetupStatus`: state (`empty`, `existing`, `recovery_required`, `unavailable`), opaque disk IDs, and `local_authorization_required`. The status also reports whether trusted local deployment explicitly enabled development simulation. Only the coarse state belongs in public status; release the disk candidates only after authorization.
- `SetupRequest`: `disk_ids` containing one to 32 distinct allowlisted IDs, `mode` (`production` or `development`), `username`, `password`. One storage disk is sufficient; additional backup levels are optional. Credentials deliberately do not implement Debug or Serialize.
- `RestoreRequest`: one to 32 distinct allowlisted IDs, and `accept_stale_credentials: true` for bootstrap restore. The selected survivors must satisfy the original configured membership quorum, not a quorum recomputed from reachable disks. Existing account passwords and grants may be stale after loss of the management disk; all restored sessions must be cleared.
- `RecoveryPreviewRequest`: `operation_id`, `reason`.
- `RecoveryCommitRequest`: `recovery_id`, `digest`, `accept_permanent_loss`.

All request DTOs reject unknown fields. An HTTP adapter must use a 16 KiB maximum decoded body and 10-second body deadline, the ordinary exact-origin and CSRF protection, request admission/rate limits (including bootstrap verification attempts), and fresh current-system-admin authentication for administrator operations. Do not use legacy credential snapshots as offline admission. Use an independent bounded management worker semaphore, so blocked data workers do not exhaust management admission. Hold the runtime lifecycle gate for the entire blocking operation, not merely request submission. Do not run healthy `/api/admin/check` a second time through the offline checker. Do not expose offline export paths or file payloads through this boundary.

## Initialization and interrupted setup

A new installation requires all configured disk directories to be empty, no configuration, no legacy or current management credentials, no authority marker, and no previous setup intent. Unknown directory contents are existing data, not permission to erase them. Invalid credentials are rejected before writing storage.

Initialization acquires an exclusive process-level `setup.lock`, validates the selected core configuration, and durably creates `setup-intent.json` before enrolling any disk. Core initialization also acquires its normal system and disk writer locks. Multi-disk initialization cannot be made atomic by renaming one system file: an interruption therefore leaves a durable fence. It never silently cleans up, rolls back disk identities, or retries as a fresh installation.

The trusted installer callback must durably publish the management-authority marker before establishing `AuthStore::initialize_management`, then replicate the initial management snapshot using the managed runtime protocol. It must not call legacy engine-authoritative `open_auth` or `protect_auth`. Report success only after that callback succeeds, then let the managed runtime schedule a serialized backend reopen. Its response describes whether recovery is pending or a restart is required. The setup intent remains as evidence even after success, so later loss of configuration does not look like first setup.

## Recovery

Authenticated offline checking invokes the existing proof-verifying core checker on one allowlisted disk. Normal system restore requires a current system administrator; when all management credentials are absent, explicit local bootstrap authorization can instead restore existing, verified authoritative credentials. Restore never creates new credentials. The core requires a strict majority of the original configured metadata members and never initializes old disks. A configured single-member pool needs its sole surviving member; one survivor cannot authorize restore of a configured three-member pool.

Degraded recovery uses `Engine::open_for_recovery`, never ordinary open which may advance a decided candidate. Existing core checks restrict it to an already accepted unpublished operation with every original disk accessible. Preview produces a bound plan; commit requires its ID, digest, the same principal, and explicit permanent-loss acknowledgement. This is not quorum bypass for new writes.

These operations are application-level recovery, not SMART, fsck, hardware repair, or arbitrary filesystem administration.

## HTTP endpoints

`GET /api/setup/status` verifies `X-ZXS-Bootstrap` and returns the authorized status plus `csrf_token`. `POST /api/setup/initialize` and `POST /api/setup/restore-system` require the same bootstrap header, `X-CSRF-Token` from status, exact same-origin validation, and JSON. Query strings are rejected on setup routes. Bootstrap authorization does not replace CSRF protection.

Authenticated administrator operations are `POST /api/management/offline-check`, `/api/management/restore-system`, `/api/management/recovery-preview`, and `/api/management/recovery-commit`. They retain ordinary current-session, CSRF, and origin checks. Runtime results are returned unchanged; the UI must not automatically resubmit a mutation after an unknown transport outcome.

## Precisely bounded interrupted-initialization completion

`POST /api/setup/complete-initialization` accepts only `username` and `password`, with the same local bootstrap, CSRF, origin, size, and admission checks. Authorized status reports `completion_available`; it is not inferred from a missing credential file. The original initialization writes a private intent binding the complete configuration, pool identity, selected disk IDs, and original bootstrap authorization. Only after storage initialization succeeds does it durably record the exact pristine disk trees before entering authority/credential installation.

Completion requires both matching records, the original locally authorized key, unchanged configuration and disk contents, current mount/mode checks, empty workspaces, and the core's pristine initial state with no auth snapshot. It never accepts old unbound intents or an arbitrary existing pool. Existing authority markers or any credential file, including corrupt files, always require explicit recovery and cannot be replaced. This covers only the proven post-storage/pre-authority interruption window. A crash after authority publication but before primary credentials remains fail-closed; this is not a claim that all interrupted setups can be completed automatically.

## Locally provisioned configuration presets

Deployment can register opaque preset IDs mapped to local absolute configuration files. Registration does not read those files and unavailable presets never stop the management listener. Only an explicitly selected, bootstrap-authorized action opens a preset: no leaf symlink following, regular single-link file, effective-user ownership, mode 0400 or 0600, and at most 64 MiB, matching the local draft boundary. Invalid IDs are not exposed as Web candidates. HTTP never accepts a configuration path or configuration JSON.

`POST /api/setup/preset-preview` accepts `preset_id` and returns `preset_id`, `config_digest`, and the validated configuration with every disk root removed and replaced by its opaque `candidate_id`. It preserves the meaningful topology, resource limits, and other Config parameters for review. `POST /api/setup/initialize-preset` requires `preset_id`, the reviewed `config_digest`, `username`, and `password`. A changed local preset requires a fresh preview; it cannot silently change the confirmed installation.

Every selected preset root must map to a locally allowlisted disk. Core configuration, full-installation emptiness, current disk identity, production mount/independence, development opt-in, bootstrap expiry/revocation, serialization, and durable initialization proof checks remain unchanged. The same initialization transaction preserves all validated Config parameters, including additional non-metadata members; it does not squeeze a preset into the default single-disk setup or discard its optional backups. The precise pre-authority completion procedure also applies to interrupted preset initialization using its original bound configuration.

## Rejected or interrupted storage recovery

Bootstrap restore rechecks the local permit after disk recovery and immediately before installing management credentials. Deleting, rotating, expiring, or making the bootstrap key unsafe during recovery rejects publication. Storage configuration and a recovered credential snapshot may already have been written; those files remain recovery evidence, not authorization.

Background storage discovery and daemon restart never install credentials from an engine snapshot, even when both the management credential file and authority marker are absent. Missing or corrupt management credentials remain fail-closed. Existing valid local management credentials retain authority, and a successful explicitly authorized restore revokes recovered sessions before installing the credential snapshot.

The offline `zxs-rescue restore-system` command restores storage metadata only; `credential_recovery_snapshot_available` does not authorize management login. Preserve partial or rejected recovery state for inspection. To recover management credentials, stop the related service and perform explicitly authorized bootstrap restore into separately provisioned clean system state, acknowledging potentially stale credentials. Do not delete or overwrite live authority, credential files, or partial recovery evidence to make an installation appear empty. There is no automatic migration or CLI password-reset fallback.
