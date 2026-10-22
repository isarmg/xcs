# Daily management CLI replacement gates

The Web administrator page uses the real HTTP request builders in
`src/web/src/admin/request.rs`. Storage mutations require an active administrator
session and `management.storage.writable()`. Account administration instead uses
`/api/admin/accounts` and the system-authoritative AuthStore, independent of data
storage. The server remains responsible for authorization and storage admission.

## Command → endpoint → Web surface

| Former command | HTTP endpoint | Web replacement |
|---|---|---|
| account | GET/POST /api/admin/accounts | Accounts: select existing, create/update password, admin, disabled, repository admission grants |
| create-repo | POST /api/repositories | Repository form |
| repositories | GET /api/repositories | Inspect repositories |
| disks | GET /api/admin/disks | Inspect disks / existing file-manager disk navigation |
| topology | GET /api/backup-topologies/{id} | Inspect topology |
| policy | GET/PUT /api/policies/{id} | Policy form, explicit expected generation; inspect current |
| rule | GET/PUT /api/primary-placement-rules/{id} | Placement form, explicit expected generation; inspect current |
| smb-binding | GET/PUT /api/repositories/{id}/smb-binding | Binding form with branch, enabled, expected history generation |
| list | GET /api/branches/{id}/tree | Inspect branch tree / existing file-manager browse |
| history | GET /api/files/{id}/history | Inspect file history / existing file details |
| graph | GET /api/history-trees/{id}/graph | Inspect history graph / existing history tree |
| branch | POST /api/repositories/{id}/branches | Historical branch form with source commit and name |
| link | POST /api/history-trees/{id}/merge-links | Merge-link form with exact source/target branch and commits |
| namespace | file-manager namespace and namespace-batch routes | Existing folder create/rename/move/copy/delete controls; no new content transfer |
| purge | POST /api/history-trees/{id}/purge-plans | Purge preview, explicit version IDs |
| split | POST /api/history-trees/{id}/split-plans | Split preview, explicit cut commit |
| historical-save | POST /api/history-trees/{id}/historical-save-plans | Historical save preview; no content upload; optional slot/copies |
| plan-commit | GET /api/plans/{id}; POST /api/plans/{id}/commit | Full immutable preview + digest + explicit consequence acknowledgment; ready plans only |
| plan-cancel | POST /api/plans/{id}/cancel | Separate explicit cancellation action |
| operation | GET /api/operations/{id} | Inspect operation; unknown mutations retain original operation ID |
| check | GET /api/admin/check | Inspect integrity check |
| repository ACL (additional) | PUT /api/repositories/{id}/grants | Separate read/write/manage ACL form; not confused with account repository admission |

## Other boundaries

- Configuration templates and registration of disks in uninitialized drafts are
  provided by `zxs-rescue configuration`. Applying a locally provisioned Config
  uses the protected Web setup preset preview and digest-bound confirmation.
  Initial administrator creation belongs to that same setup transaction, not a
  general password-reset command. Only a proven pre-authority interruption may
  be completed; an existing authority marker with missing credentials fails closed.
- serve becomes the direct service entry.
- restore-system/recovery-preview/recovery-commit/offline-check/offline-export
  remain explicit offline rescue operations.
- Local history package export, check and restore are implemented by
  `zxs-rescue package`, retaining validation and no-overwrite safeguards. They
  are not daily Web operations and do not reintroduce Web upload/download/preview.
- Historical-save's optional existing workspace is not a content-transfer UI.
  This page accepts an optional existing workspace ID; it never stages content.

## Verification gates (must record actual outcomes before retirement)

`test/server/integration/http_engine/daily_management.rs` imports the exact Web
request builders and sends those requests through a real Router + durable Engine:

1. Account creation, repository admission + repository ACL, unauthorized account
   mutation rejection, disabled-account session revocation.
2. Repository creation, policy/rule updates and stale-generation rejection,
   SMB binding, branch creation, merge link.
3. Historical-save preview/cancel/commit; no commit without acknowledgment.
4. Purge and split preview/cancel/commit; wrong digest rejected.
5. Positive integer/generation validation, encoded plan IDs, immutable digest,
   completed-plan rejection, preview requests never commit.

Independent AuthStore / unavailable-storage account lifecycle is covered by the
runtime owner's tests, not by this baseline Engine fixture. Native and WASM
compilation plus integrated shell wiring are separate gates. These HTTP tests do
not claim browser-click automation or visual QA. Existing namespace operation
integration tests must also pass before its CLI path is removed.

## Non-default namespace equivalence

The folder-branch selector in `src/web/src/files/view.rs` calls directory-select
for a chosen writable directory scope. The existing batch review UI exposes
explicit source/target branch choices; `files/batch/request.rs` captures
`directory_branch_id` and `expected_tree_id` for each side, plus stable entry ID.
This is a controlled branch-scoped CAS transaction, not an arbitrary JSON console.

The real HTTP test
`branch_namespace_batch_preserves_unselected_views_and_is_idempotent` exercises
move/copy/rename/delete across multiple non-default branches, verifies untouched
branches and the SMB default do not change, and checks retry idempotency and
branch-scoped audit events. This is the relevant business-capability replacement
for the generic namespace CLI rather than mechanically reproducing its syntax.
