# Inherited branch server/API/SMB final review, 2026-10-19

This record supersedes the earlier no-read pending-branch interpretation. The latest requested behavior is a logical inherited view: registration persists records and empty physical placeholders without a new immutable content version; an unwritten branch displays its parent file, and first publication creates an own FULL version. Directory branches do this independently for every file.

## Shared response semantics

The server uses content_wire for file entries, file-branch views, file-manager history's current view, resolve results and audit entries:

- Unwritten: version_id=visible parent ID, materialized=false, inherited=true, own_version_id=null, source_version_id=parent, content_state=PENDING, visible_content_state=the parent's actual logical state.
- Written: version_id=own ID, materialized=true, inherited=false, own_version_id=own ID, source_version_id=null. Both content-state fields reflect the actual version state.
- Generic tree entries keep the strict Entry contract used by SMB and return the visible parent/own ID and size. pending_file_versions identifies inheritance without adding incompatible unknown Entry fields.
- History versions remain real FileVersion records. A visible parent can be current, but it is not reassigned to the inherited branch. Pending/own markers apply to branch and current-view responses, not to the parent version record itself.
- Explicit historical registration snapshots still display their recorded parent after later writes, retaining inherited ownership. Genuine earlier content snapshots remain readable.
- capabilities.branch_content is inherit-parent-until-first-write. branch_creation stays placeholder-until-write.

## SMB behavior

Read-open of an inherited file returns a read-only parent descriptor and its read lease. Write-open creates a private copy of the visible parent, allowing ordinary random edits to retain unchanged bytes. Merely opening and closing, or aborting modified staging, creates no own version. Actual Dirty/save and explicit truncation materialize an own FULL version, including zero-byte files. Old parent descriptors keep reading the original immutable bytes after publication.

The obsolete BRANCH_PENDING_CONTENT error, read prohibition, empty-base write logic, errno mapping and test cases have been removed. FILE_BRANCH_CONFLICT remains HTTP 409 / SMB EAGAIN to fence an outdated main-directory workspace from following a changed selection.

## Final validation

All tests use temporary fixture data. No real runtime/user directory or existing service was read or modified by this agent.

- HTTP integration: 12/12 passed, the historical temporary test directory/log
- SMB server: 12/12 passed on host, the historical temporary test directory/log
- SMB protocol/client: 6/6 passed on host, the historical temporary test directory/log
- Three permanent cross-review regressions: 3/3 passed, the historical temporary test directory/log
- Server without core feature: check passed, the historical temporary test directory/log

HTTP coverage includes recursive branching of three files, all-parent reads after selection, a first write to A yielding A-own plus B/C-inherited, successful downloads of all three immutable parents, sibling isolation, unchanged version counts before writes, first-write ancestry, abort/restart, own-version flags, explicit historical parent views and selection fencing. SMB coverage checks parent read descriptors, a private parent write copy, untouched open/close, modified abort, a one-byte edit yielding Xld from old, immutable parent reads afterward, and explicit zero-byte truncation. The core range regression now expects XOURCE_BYTES when a stable inherited file is edited at offset zero, while selection changes are still rejected before range staging; historical ranges and split-main delegation also pass.

No smbd deployment, platform-device acceptance or performance claim is made.
