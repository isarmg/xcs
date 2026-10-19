# Lazy branch storage review

Production changes: container_storage.rs, container_storage/README.md, history_package.rs, maintenance.rs.

- Branch.pending_file_versions is published authority; .pending files and container.json are replayable projections.
- Independent empty 0600 pending-branches/<branch-id>.pending files never enter FileVersion, version directories, checksum metadata, protection evidence, offline payload counts or downloads.
- Branch metadata changes trigger projection; first materialized writes remove individual stubs; physical container rename carries stubs with original payload inodes.
- Legacy storage_name=None can gain a new placeholder-only container without relocating, copying or rewriting old payloads.
- Only recognized empty ordinary single-link stubs are removable. Unknown names, links and nonempty files fail closed; no accidental content deletion.
- History manifests bind every pending file/source to a real same-file version and the branch HEAD snapshot; package export/restore retains records and rebuilds stubs.
- Purge does not retain source payload merely for a pending branch or empty workspace; metadata and ancestry remain intact.
- Full-history split maps pending file/version IDs into the new history, and clears migrated/audit-only endpoint reservations. File-scoped split remains explicitly unsupported.

New regression cases: physical placeholder mode and zero length; inode-preserving replay and rename; legacy path immutability and projection rebuilding; nonempty/symlink refusal; package pending export/restore and missing-stub restart replay; after_finalize/after_container_names indeterminate recovery without invented versions; purge followed by first write without old source bytes; split identity remapping and old-stub removal.

Validation: cargo test -p xcs-core --locked --offline pending passed 7/7; full changed modules container_storage passed 6/6, history_package passed 4/4, maintenance::split_tests passed 6/6 (16 total, including the legacy source case). Complete log: /tmp/xcs-lazy-branch-storage-tests.log. Initial two fixture failures were exact-private-mode violations at TempDir parents; tests now explicitly set 0700 and passed. No real runtime/data visited or changed.

The maintenance source check reuses business::pending_file_source through one crate-private re-export in business/mod.rs, so a selected pending file projected into directory main is not mistaken for main-owned content. Added a regression covering purge then first directory upload into that selected pending branch.

Snapshot-local follow-up: Commit.pending_file_versions now records historical reservations. HistoryManifest validates each snapshot mapping against same-file/same-history versions and its entries. Split maps commit and branch reservations through the same helper; migrated branch prefixes and audit endpoints retain their own snapshot maps. The restored package regression materializes a branch and then rereads its old pending snapshot without exposing source content. A new mixed-snapshot split test preserves the original pending prefix and remaps the new snapshot. Final scope tests: history_package 4/4 and maintenance 7/7 passed; log /tmp/xcs-lazy-branch-snapshot-storage-tests.log. Production source frozen after this pass.
