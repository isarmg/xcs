# Unified data serialization

The project maintains one `data-serialization` implementation for serialization traits, standard/core type implementations, derive support, and optional structured error paths. `serialization-derive` remains a separate procedural macro crate, pinned to the exact matching private-support version. This split is required by Rust's proc-macro compilation model, not by upstream repository layout.

## Capabilities

- No default features: allocation-free traits and core type implementations, with no normal dependency on proc-macro tooling, JSON, or integer formatting.
- `alloc`: owned collections, strings and buffered derived values.
- `std`: standard-library types; includes `alloc`.
- `rc`: `Rc`/`Arc` implementations when allocation is available.
- `derive`: optional macro reexports; plain manually implemented traits do not enable the macro dependency.
- `error-path`: an alloc-only module at `data_serialization::error_path`, with the optional integer formatter used by path display.

All retained APIs target the project's Rust 1.99 toolchain on Linux amd64 and browser WASM. The old compiler probes, generated private module, `serialization_if_integer128!` no-op compatibility macro, separate-core misuse guard and unsupported nightly feature are removed. Result serialization is an ordinary fundamental implementation, not a fake compatibility feature.

## Implementation changes

The former runtime core and documentation-only duplicated core now share a single implementation and documentation tree. The two copies differed: the documentation-only version omitted Result implementations and pointed to its own extracted formatting test. The unified implementation retains the actual runtime Result behavior and executes both attributed original formatting tests. Facade and core private helpers are merged, removing cross-crate forwarding and the duplicate private Content representation. Derive output and the runtime agree on the existing functional `__derive_support` namespace.

These changes reduce duplicate source and compilation boundaries. No runtime speedup, build-speed improvement or reduced binary size is claimed without comparative measurements. JSON/TOML protocol markers, signed/unsigned integer semantics, borrowed deserialization and custom derive crate paths remain contracts.

## Sources and checks

Original identities, versions and licenses remain in `compliance/provenance`, `docs/upstream`, and content-addressed consolidation receipts: serde 1.0.229, serde_core 1.0.229, and serde_path_to_error 0.1.20. The three inputs are not presented as new upstream releases.

Business implementation lives in `src/foundation/data-storage/data-serialization`; original tests remain attributed under `test/foundation/data-serialization`. The CI language shard runs both the coordinated serialization consumer gate and the new consolidation gate: seven native/WASM feature profiles, dependency-boundary and missing-capability checks, original path tests, precise integer/Result/derive/content/borrowing/path regressions, original formatting unit tests and doctests. Source governance separately validates exact original snapshots, deduplicated routes, build-script retirement evidence and immutable prior history; it does not equate a reviewed source transformation with behavioral equivalence.

## Receiving-tree validation (2026-10-21)

The combined receiver based on `eb813854` passes seven native/WASM profiles, four capability negatives, 20 semantic/error-path/log serialization tests, two attributed source tests and 72 doctests. Storage server, admin CLI and web-build pass locked offline type checks and strict all-target Clippy. The 231-test portable CI suite and documentation contracts pass. Fresh source metadata and the generated consumer index cover 273 active sources, including 271 foundation packages. These checks are not a browser runtime or performance claim.

Log serialization is guarded by its actual `serialization` feature rather than the removed dependency name. UUID's separate system-entropy dependency and browser feature wiring are preserved. The four previously retired HTTP capability source files and the complete HTTP header-deadline implementation remain unchanged.
