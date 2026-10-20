# Server / API / SMB lazy branch implementation

- Engine branch lifecycle remains active/archived; pending_file_versions records file identity to lineage source separately.
- /v1/capabilities advertises branch_creation=placeholder-until-write, branch_content=written-files-only, full_branches=false.
- File-manager pending entries and branch DTOs expose materialized=false, content_state=PENDING, version_id=null and source_version_id. File-manager entries expose selected_branch_id. Generic tree preserves Entry compatibility (empty version_id, zero size), with all snapshots routed through Engine::list_tree_at. Commit.pending_file_versions preserves historical pending snapshots; genuinely materialized old snapshots retain their content.
- One branch_wire formatter serves generic branch/list/create and file-manager history branches, avoiding separate pending-state interpretation.
- Pending heads cannot claim the source content version in file-history ancestry ownership. Actual versions alone remain in the tree; first real write points to the requested lineage source.
- Backup listings inspect source placement membership only to show pending metadata; they never read its bytes as pending branch content.
- Registration audit events use file_branch_pending / branch_pending and null current version. Pending snapshots cannot be interpreted as deletion of files outside their scope.
- SMB read-open of pending data returns 409 BRANCH_PENDING_CONTENT. Writes open an empty staging file; open/close and abort produce no versions. Explicit Dirty or truncate followed by normal publication materializes content, including an intentional zero-byte version.
- SMB client maps pending-content diagnostics to EAGAIN rather than an I/O corruption error; the framed remote diagnostic remains available.

## Verification

All tests use tempfile fixtures; no production/runtime data or live service was read or changed.

- CARGO_TARGET_DIR=target/native cargo check --locked --offline -p zxs-server: passed.
- Same command --no-default-features: passed.
- HTTP integration suite: 12 passed, the historical temporary test directory/log Extended scenarios prove creation adds no version, idempotency, pending API tree/resolve/download metadata, selection, backup visibility, abort, reopen/recovery and first-write lineage, recursive per-file materialization and sibling isolation.
- Host SMB server suite: 12 passed, the historical temporary test directory/log The first sandbox run had one Unix socket permission failure; it was rerun successfully on the host. Two new tests exercise no-read/no-open-close materialization, abort, actual first-write bytes and explicit empty truncate.
- Host SMB protocol/client suite: 6 passed, the historical temporary test directory/log existing remote-error framed test now also validates pending-content errno and diagnostics.

Final reruns after Commit snapshot markers: HTTP 12/12, SMB server 12/12 host, SMB client 6/6 host, three new core regressions 3/3, no-core server check passed. Logs and resolved cross-review findings are enumerated in the historical temporary test directory/log No smbd session, release, platform device or performance claim is made.
