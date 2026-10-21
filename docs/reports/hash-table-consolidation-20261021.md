# Hash-table dependency consolidation, 2026-10-21

## Scope and rationale

Consolidate the remaining hashbrown 0.16.1 consumers onto the already-present 0.17.1 implementation. Change only the version constraints and local paths in sqlx-core, hashlink and optional wasmparser dependencies; retain all feature/default/optional settings. referencing and indexmap already use 0.17.1. A complete Cargo.toml scan found no other old-version consumers.

Keep the collection abstraction layers. hashlink has insertion-controlled order, order-sensitive equality and LRU behavior; indexmap also promises ordering. Neither is replaceable with an unordered HashMap. Wasmparser's std mode retains std::RandomState for collision-attack resistance, while no_std retains its existing weaker foldhash-based implementation. No hasher choice or security boundary changes.

Both hashbrown manifests have identical default features and dependencies, including foldhash 0.2.0; src/hasher.rs is byte-identical. The 0.17 MSRV of Rust 1.85 is below workspace Rust 1.99. The upstream EntryRef insertion change to ToOwned can break arbitrary external consumers, but no repository production consumer calls that affected API. String/str borrowed insertion is regression-tested. SQLx publicly reexports hashbrown types, so the workspace must rebuild as one coherent dependency graph.

## Evidence

Before integration, isolated baseline/candidate tests passed:

- Eight custom tests, four per version: forced collisions, borrowed lookup/insertion, reserve/shrink/retain, equality, linked ordering, LRU, statement cache and drop-once behavior.
- Preserved hashlink linked-map/set/LRU suites: 68 tests per version, including existing panic-sensitive clear/drop cases. Total including custom tests: 144 passed, zero failed.
- Candidate sqlx-core library check passed with candidate hashlink and hashbrown.
- Candidate wasmparser checks passed for std/hash/serde/validation, no_std/hash/validation, hash-disabled fallback and browser-Wasm no_std/hash configurations.
- Valid/invalid/truncated Wasm module validation passed in std, no_std and preferred-BTree modes, each alongside the eight custom checks.

Six maintained first-party tests now live at test/engine/integration/hash_table_compatibility.rs, explicitly registered in storage-engine. They use the actual SQLx StatementCache, not a copied implementation, and retain no old hashbrown implementation. They cover collision handling and borrowed/raw lookup, unordered versus linked equality, linked replacement order, LRU touch versus membership checks, eviction/drop-once and disabled-capacity statement caching. These tests are original repository regression contracts, not upstream test-source relocations.

## Bounds and provenance

HashMap bucket layout and iteration order are unspecified; logical contents are tested instead. Linked order is tested explicitly. HashTable clone_from and panic-during-rehash fixes in 0.17 are intentional behavior improvements, not bytewise equivalence. No claim of exhaustive fuzzing, Miri, allocator-failure or serde-specific hashlink testing is made. Browser-Wasm evidence is compilation, not browser execution.

The original hashbrown names, upstream versions, licenses, manifests and source/test hashes remain in provenance. Retirement of the 0.16.1 source and external tests uses an explicit audited receipt: 33 production files / 688217 bytes and 14 external-test files / 125149 bytes. Functional versionless naming of the surviving implementation is a separate audited transition, not part of this dependency substitution.

## Integrated workspace validation

- `cargo test --offline -j2 -p storage-engine --test hash_table_compatibility`: 6 passed, zero failed.
- `cargo check --offline --locked -j2 --target wasm32-unknown-unknown -p web-app`: passed.
- `cargo check --offline --locked -j2 --target x86_64-unknown-linux-gnu -p storage-engine -p web-bindings`: passed, covering SQLx/hashlink and Wasm parser consumers.

Cargo.lock resolves one hashbrown package, version 0.17.1. The serde_core dependency formerly enabled on 0.16.1 is now enabled on the survivor; no new third-party package versions are introduced. Engine-only development dependencies register the regression against the same internal implementations. Foundation crates are excluded workspace components, so native checks target their actual workspace consumers rather than selecting foundation package IDs directly.

## Applied accounting

The reviewed retirement is applied: 324 active source trees and 78 retired identities remain. The complete last production and external-test inventories are embedded in the immutable retirement receipt; original provenance and legal material are unchanged. Metadata/architecture verification and the complete static suite passed. Maintained-package formatting, Clippy, all supported host-target diagnostics and release WASM checking passed after the merge. Remote CI is checked separately for the published commit.
