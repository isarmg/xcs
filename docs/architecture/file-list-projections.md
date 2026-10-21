# File listing snapshots and keyed rows

The file table uses the project's `ForSnapshot` primitive. This change is limited to listing ownership, projection, and row rendering; it does not alter server APIs, upload-ledger policy, branch/history rules, fonts, or G4 styling.

## Data flow

1. `files::Model::entries` is an immutable `Arc<[Value]>` server snapshot. Navigation replaces it with an empty snapshot; an accepted listing response replaces it with the returned entries. Existing session, location, and request-generation checks still run before publishing a response.
2. `files::rows::Listing` captures only session epoch, disk, directory, and that shared snapshot. Equality short-circuits identical snapshot allocations. Unrelated global-state notifications do not clone or traverse entry JSON. A genuinely new allocation may require a content comparison to determine whether the response changed.
3. A memo projects display fields and stable keys, then stably sorts folders before files and names by the existing lowercase rule. The rows share one location context and contain display fields only. No JSON payload is copied into rows or event closures, and retained rows cannot pin whole old listing snapshots.
4. A separate normalized-query memo filters those sorted rows. Filtering clones only row handles; it does not sort again or copy JSON/display strings.
5. `ForSnapshot` keys combine session, disk, directory, and the existing entry-ID/logical-path identity. A refreshed payload updates the retained row's read-only signal. Every mutable rendered field reads that signal. Event handlers use its stable identity to resolve current JSON, rather than capturing initial row data.
6. Selection, checked state, and disabled state are independent reactive reads. The header operates on the same projected visible rows as the table. Per-row checked-state reads can still run when the global state changes; this is not a claim that all application updates are constant time.

## Identity and event safety

Stable identity deliberately excludes request generation, selected version, size, and display name so metadata refreshes can retain the row. Different aliases of one file retain different logical-path keys. Removing a key and later reinserting it creates a new row, as required by `ForSnapshot`.

Before selecting or opening a row, the handler resolves its identity against the current immutable listing. It searches the current entries by entry ID and path without allocating candidate keys. This also works when a newer snapshot arrived before the row signal was refreshed. Removed entries and a changed session/disk/directory are rejected. This avoids a queued event reopening captured stale metadata. The unchanged downstream history/download code consequently receives the current selected entry. Metadata-only changes need not notify unchanged display fields: event-time resolution still obtains the new version. The select-all handler uses the freshly evaluated visible-row memo directly and does not perform a per-row listing scan.

All four production listing replacement sites are in `files/actions/browse.rs`: open disk, go root, go directory, and accepted listing response. There are no production in-place mutations. `PickerContext` shares the same immutable entries but retains value equality and all existing generation/context fields, preserving picker cancellation rules. The copy-on-write regression demonstrates that a future `Arc::make_mut` change cannot silently mutate a snapshot retained by the memo.

## Validation and limits

`test/web/unit/files/rows/tests.rs` exercises the actual projection memos, stable ordering and filtering, alias/location/session keys, changed payload under a retained key, fresh event-time resolution, removed entries, and copy-on-write snapshot changes. Deterministic counters count projection passes, entries projected, and filter passes; they do not estimate wall-clock speedup, allocations, or browser frame time.

Existing `test/interface/unit/for_snapshot/tests.rs` tests retained signal identity and changed payload publication in the actual `ForSnapshot` registry, including reorder/removal and unchanged snapshots. The browser-smoke file-row scenario checks real node identity, checkbox state, focus, and matching-filter retention. Native tests cannot establish DOM/focus behavior: the new browser scenario must run in the real browser CI suite before claiming those checks passed. No browser frame-time or allocation benchmark is claimed.

### Verified candidate evidence (2026-10-21)

The isolated candidate is based on commit `7a6857a9c8c6083a8f957ad237d414387fae3b65`, with only the bounded file-list changes. Native `web-app` tests passed 117/117, `web-interface` snapshot-registry tests passed 4/4, the browser-smoke harness typecheck passed, the `wasm32-unknown-unknown` application check passed, and native Clippy with all targets and warnings denied passed.

For each listing size of 100, 1,000, and 10,000 entries, 100 unrelated state updates left the measured counters at one projection/sort, exactly N projected entries, and one filter pass. Changing the search query added one filter pass and no sort. These are executed deterministic work counters, not a latency or frame-rate benchmark.

The real-browser startup probe was attempted with matching Chromium and ChromeDriver 154.0.8037.57 on the cloud computer. ChromeDriver started, but Chromium aborted with `process_singleton_posix.cc:297: socket() failed: Operation not permitted (1)`. No sandbox/security workaround was attempted. Thus the new real DOM/focus/filter checks are implemented and compile-checked but remain unexecuted until the real-browser CI environment runs them. No production release WASM build or browser performance result is claimed for this candidate.
