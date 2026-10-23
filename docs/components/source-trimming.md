# Reviewed production-source trimming

Use `python tools/rust-sources/manage.py refresh --trim-plan PLAN.json` for
intentional removal of individual production Rust files. Ordinary refresh still
rejects unaccounted removal; this is not a test-relocation or package-retirement
escape hatch.

The plan is `{ "schema": 1, "packages": [...] }`. Each package row contains only:

- `directory`: an active standalone `src/foundation/<owner>` directory.
- `package`: the exact full owner snapshot from the previous
  `compliance/rust-sources.json` catalog, including its complete file hashes.
- `files`: a nonempty list of objects with `path` (owner-relative),
  `sha256_before` (from that snapshot), and a nonempty per-file `reason`.

Only `build.rs` and production `src/**/*.rs` files are supported. Test, benchmark,
example, manifest, compliance and license files cannot be removed this way.
Each listed file must already be absent. Every other removed file still needs
its original explicit test-relocation accounting. A plan does not authorize loss
of any external-test fragment or receipt.

Refresh appends full content-addressed receipts under `source_trims` and hashes
under `source_trim_records_sha256`. Verify checks receipt integrity, duplicate
paths, owner identity, current inventory and continued on-disk absence. Refresh
retains all prior receipts unchanged. Original provenance and license material
are never rewritten by this mechanism. As with the existing catalog histories,
Git review is the external trust anchor; these hashes are integrity checks, not
signatures against a party able to replace the entire catalog.

Trimming merged/consolidated owners is deliberately unsupported. Renaming,
moving, upgrading, consolidating or retiring an owner with trim history requires
a future explicit chain-aware adapter. Do not remove receipts or retain dead
stubs to circumvent this boundary.

The governance module performs no builds and writes no source files. Run
`python -m unittest discover -s test/tools/rust-sources -p test_source_trimming.py`
for its focused regression suite.
