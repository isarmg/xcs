# Core / contract / naming review

## Scope and coverage
- crates/xcs-core: lib, engine, business(mod/file/history/namespace/repository/workspace), disk, journal, container_storage, historical_save, history_package, maintenance, metadata_budget, protection, repair, degraded_recovery, wire, tests.
- crates/xcs-contract: lib, encoding, integer_json, model.
- crates/xcs-naming: lib, README, fixed Unicode table/data boundaries.
- Initial size: 26 Rust source files, 15,681 lines including 2,410 generated Unicode table lines.
- No runtime data, real disks, service, protocol or disk format changes authorized by this slice.

## Data flow
- Engine owns single writer, grants, idempotency and current state. Business operations compose cloned State + prepared full payloads.
- Publication validates bounds and fixed route, installs immutable payloads, persists snapshots/manifests, then PREPARE / ACCEPT / FINALIZE / PUBLICATION and local seals.
- Container names and notes are replayable post-decision projections; recovery replays frozen candidates. SQLite is a rebuildable projection.
- Normalized namespace identity is fixed Unicode 16 NFC + case folding; physical file containers use stable file ids.

## Findings (all implemented below)
- Namespace rename selects descendants by raw display prefix while parent lookup is normalized: A vs a, Straße vs STRASSE, Σ vs ς differ; legal descendants fail parent validation after rename.
- File-branch snapshot uses the same raw-prefix ancestor assumption, potentially omitting required parents.
- Namespace parent validation and history closure repeatedly scan full collections; worst case quadratic.
- Upload copies immutable generation before detecting a directory target, leaving failed generation bytes and breaking safe retry. Copy quota uses initial source size rather than remaining workspace quota.
- engine.rs has 1,560 lines spanning lifecycle, config, publication, recovery, reads, SQLite projection and workspace persistence.
- Note write bound128 versus legacy history-package read bound4096 needs explicit compatibility boundary; not silently tightening historical storage.

## Changes
- engine.rs1560→595 lines: six private responsibility modules engine/{config,index,publication,reads,recovery,workspace_storage}. Public API, State/disk schemas and durable ordering retained. Only private persist/recover entry visibility adjusted to parent scope.
- Namespace functions moved from business/mod.rs into namespace.rs; normalized-key ancestor predicate reused by delete/rename/dependency checks and file snapshot. Rename selects canonical descendants but removes raw display prefix by component count, preserving UTF-8 and stable ids.
- Parent validation builds one normalized index, commit comparison builds borrowed key indexes while retaining result ordering, history package file membership uses one set. Shared business/ancestry.rs indexed reachability used by manifest validation and split; 12,000-node reverse chain uses iterative traversal, detached cycles excluded.
- Upload base/IS_DIRECTORY validation moved before immutable copy. full_copy streaming maximum is min(file cap, remaining retained workspace quota); range offset/id constraints checked before copying old content.
- Direct namespace updates now reject inactive directory branches with BRANCH_NOT_ACTIVE.
- Engine::plan now verifies requested plan id and object_hash(plan.input) for all kinds, so purge/split cannot execute locally modified input with an old preview digest.
- Shared xcs_contract::{validate_note,MAX_NOTE_BYTES} validates new128-byte UTF-8/control-free/empty-clears notes; core uses it. Historical package4096 reader bound explicitly named and preserved.
- Shared SUPPORTED_CHECKSUM_CHUNK_BYTES used by checksum encoder, core config, history package reader.
- New crates/xcs-core/README.md covers responsibility map, authority/projections, publish sequence, extension points and test boundary.

## Validation
- Before fix: directory_rename_follows_normalized_ancestry test reproduced PARENT_NOT_DIRECTORY with A/a descendants.
- Final cargo test --locked --offline -p xcs-core -p xcs-contract -p xcs-naming --lib: core56 + contract8 + naming3 =67 passed, zero failed. Contract includes root's two api_path tests.
- Added9 tests across core+contract: normalized raw-prefix mismatch (ASCII/German/Greek), reverse long ancestry, detached cycles, real file branch+directory rename+restart closure, upload failure leaves no immutable generation and valid same-id retry, inactive namespace unchanged, indexed comparison order/case rename, modified plan digest/id rejection, shared note boundary. (Core baseline48→56 =8, contract notes1; ancestry has2.)
- Final cargo check --locked --offline -p xcs-core -p xcs-contract -p xcs-naming passed after all edits without warnings.
- rustfmt applied only owned Rust files; no root Cargo/server/Web edits.

## Ownership
- This agent owns core edits and contract text.rs/encoding.rs + initial text module exports. Root owns contract api_path.rs and its exports. No naming production code change; fixed Unicode data treated as generated input and existing tests verified.


## Remaining risks
- Production physical-disk/fault scenarios require separate real hardware acceptance; temporary Development fixtures are not redundancy evidence.
- Recovery still replays verified historical snapshots/publication dependencies; no unmeasured startup-speed claim or checkpoint trust-boundary rewrite was made.
- Existing max_entries/metadata/frame limits and legacy disk/package byte representation intentionally preserved.
- In-tree test inventory reviewed by domain and all executed; generated Unicode tables are not hand-refactored.
