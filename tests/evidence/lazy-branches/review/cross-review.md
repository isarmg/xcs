# Lazy branch cross-review, 2026-10-19

Read-only production review covered business/file.rs, history.rs, workspace.rs, historical_save.rs and their server/SMB read boundaries. Production fixes remained with the root agent; this agent appended three regression tests to business/file.rs and adapted server/API/SMB code in its assigned scope.

## Findings and resolved behavior

1. A pending full branch can become a repository main branch after a history split. Its old direct pending map previously masked a newly materialized selected file branch and could make the updated snapshot fail history-package validation. Effective pending state now follows the selected independent file branch, and publishing a selection removes the obsolete main reservation. The public-API probe and permanent split/reopen regression pass.
2. Range staging previously consulted the mutable current selection before copying its base. A workspace captured on pending branch A, followed by selecting materialized main, could copy SOURCE_BYTES into private staging and produce XOURCE_BYTES. The final commit was fenced, but the working contents already violated the empty-base rule. Normal range staging now validates captured selection and current file version before any base-byte read. The permanent test verifies early FILE_BRANCH_CONFLICT, no staged file, then a stable pending range yielding only X.
3. Applying that current-version preflight to historical workspaces incorrectly rejected explicit edits to an older real snapshot. Historical range staging now honors its explicitly bound source and its snapshot pending map. The regression edits old body to Xld body while current body remains unchanged, then publishes via the historical-save plan.
4. A first write must not retroactively turn the branch's historical registration snapshot into source-owned content. Commit.pending_file_versions now retains pending state per snapshot. Engine::list_tree_at is the common read projection. HTTP tests prove registration P -> actual write V2 -> explicit read/resolve P still has no version/body, while a genuine older content snapshot is still readable.

Server DTOs and audit events use per-snapshot pending markers as well as current branch reservations. Mixed historical saves cannot label unchanged source references as newly materialized content or fabricate deletion of the original namespace. Archived audit endpoints do not claim a pending snapshot's source version as owned content. FILE_BRANCH_CONFLICT maps to HTTP 409 and SMB EAGAIN; BRANCH_PENDING_CONTENT maps to the same filesystem errno while retaining its explicit diagnostic.

## Final verification

All payloads were temporary fixtures; no runtime directory, user data or existing service was read or modified.

- HTTP integration: 12 passed, /tmp/xcs-lazy-branch-http-final.log.
- SMB server: 12 passed on host, /tmp/xcs-lazy-branch-smb-final-host.log.
- SMB protocol/client: 6 passed on host, /tmp/xcs-lazy-branch-smb-client-final-host.log. Sandbox socket/SCM_RIGHTS denial was rerun on host; it is not counted as a sandbox pass.
- Three new permanent cross-review regressions: 3 passed, /tmp/xcs-lazy-crossreview/permanent-tests-final.log.
- Server without core feature: check passed, /tmp/xcs-lazy-branch-no-core-final.log.
- Independent Engine API probe after the first two fixes: /tmp/xcs-lazy-crossreview/result-current.log. It confirms early selection rejection and a visible materialized main after the split case. The later permanent tests cover the final Commit marker implementation.

Normal staged writes of identical or empty bytes still create the first real version because pending status bypasses the ordinary unchanged-content shortcut. Pure registration, selecting pending branches, opening/closing an untouched SMB write handle, aborted staging and unrelated file edits do not create content versions for the pending file. Metadata keeps source identity for lineage; reading an already existing source version by its own authorized version ID remains an ordinary history read.
