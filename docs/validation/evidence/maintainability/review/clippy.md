# Historical native Clippy review

This preserves the original review outcome, not the current lint policy or a new execution result.

Command: cargo clippy --locked --offline --workspace --all-targets
Exit code: 0. The full log was recorded in a temporary directory during the historical run; this report does not retain a usable log path. No source edits or allow attributes were added in that review.

## Actual bug / correctness diagnostics
None emitted. No correctness, await_holding_lock or suspicious_open_options diagnostics. This does not prove bug absence.

## API / control-flow / formatting suggestions
- collapsible_if: 31
- too_many_arguments: 5
- ptr_arg: 1 (namespace validation can accept a slice; current API does not allocate an extra object)
- chunks_exact_to_as_chunks: 1 (decoder already verifies exact payload length and count, so current exact chunk conversion does not expose an observed panic)
- items_after_test_module: 1
- ineffective_open_options: 1 (journal.append(true) already enables write; explicit write(true) is redundant, not a missing truncate or append correctness defect)

## Nonblocking layout observation
- large_enum_variant: 1: SMB Handle Write 336 B versus Read 104 B. Boxing could reduce map entry size but adds allocation/indirection. No measured handle-count pressure or runtime failure established; avoid changing frozen transport state for this warning alone.

Total: 41 emitted source diagnostics; Cargo lib/test duplicate summary counts not counted again.

## Scope
Root workspace packages auth,contract,naming,platform,core,server,client,CLI. Independent App,Web,SMB,VFS and tools workspaces are excluded by the root manifest and are not claimed as covered by this command; their separate test/build evidence remains authoritative.

## Historical recommendation
No patch required for these Clippy diagnostics in this frozen candidate. Keep the explicit functional tests and browser/runtime acceptance as separate evidence; do not automatically rewrite public parameter lists, widespread control flow or add allow attributes to reach zero warnings.
