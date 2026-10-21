# Entropy consolidation regression coverage

`error.rs` is included by random-generator-core's unit tests. Test with no default features, `system-entropy` only (no_std), and `std`. The frozen upstream numeric-ABI test remains in its original provenance-owned test location; it is no longer an active test of a deliberately retired API.

`failure_probe.rs` is a no_std external harness. Its manifest must be generated outside the repository, with absolute path dependencies on random-generator-core (feature `system-entropy`), system-entropy, and Linux-only ring. Features `std` and `browser` forward respectively to core/std + entropy/std, and entropy/wasm_js. Run the harness with `RUSTFLAGS='--cfg system_entropy_backend="unsupported"'` for both no_std and std. Without the forced unsupported backend the fail-closed test intentionally fails, rather than silently skipping the regression. Check the same harness (do not execute its forced-failure test) for wasm32-unknown-unknown with `browser` enabled.

Checks cover typed internal/custom errors, formatting, preserved explicit numeric user codes, std sources/downcasting/io conversion, failed OsRng byte fill/integer generation, failed entropy seeding before seed construction, empty requests, and ring's error return. The harness adds no production randomness fallback, unsafe Rust, or injected production backends. Browser compilation is not browser execution evidence.

## Reproduce

Run `python3 test/entropy/check.py` with the repository's Rust toolchain and installed native/WASM targets. It creates its standalone harness outside the repository, copies maintained packages into temporary isolated workspaces, generates separate locks offline, uses locked/offline test commands, and leaves the single product workspace intact. It also runs the retained entropy provider's native source tests.

## Consolidation review

- Exact removed implementation: getrandom 0.2.17, previously referenced only by ring and random-generator-core. Existing getrandom 0.4.3 consumers now share the same functionally named system-entropy package. getrandom 0.3.4 had already been retired.
- Error redesign: no_std stores `Code(NonZeroU32)` or the original typed entropy error. Entropy errors are never translated into invented numeric user codes. `code()` returns `None` for typed entropy errors; `raw_os_error()`, `Display`, and `Debug` delegate to the retained error. The std representation preserves downcasting and exposes the stored error as its source. Explicit numeric user errors still retain their codes.
- Linux amd64 retains the 0.4.3 backend, including kernel entropy readiness, interrupted/partial-read handling and its existing secure OS-file fallback. Both retired and retained default Linux implementations fall back on ENOSYS/EPERM, not on arbitrary errors. No new fallback, unsafe block, or random-number algorithm is introduced.
- Browser WASM retains explicit Web Crypto opt-in and caught Web Crypto failures. The platform does not become a native target. A WASM compile pass is not a fresh browser runtime result.
- Package/crate/features and backend configuration are renamed coherently. Kernel syscall/module names remain literal OS interface names. Upstream origins and versions remain in source documentation, original licenses and archived package records. Custom implementation symbol becomes `__system_entropy_custom`; no legacy API-name compatibility is claimed.
- Original numeric-ABI test bytes, ring/platform evidence, prior runtime evidence and all upstream/legal records remain intact. A pinned compressed retirement archive preserves the removed 0.2 production/test bytes. Authenticated active routing records cover the survivor rename and ring call-site migration; they do not assert current runtime correctness.

## Earlier isolated candidate results (2026-10-21)

- Source governance: 311 active trees, 86 retired, two superseded; metadata/provenance verification and generated consumer-index verification passed.
- Ring historical proof: 69 materials and 15 integration test entries verified. All 720 historical native runtime hashes replayed exactly; original runtime evidence was not rewritten.
- Retained provider native source tests: all 9 passed, including empty, large/uninitialized buffers and multithreaded calls.
- Core unit tests: 8 without entropy, 11 with no_std entropy, 11 with std; all passed. Four provider doctests passed, one upstream uninitialized-buffer doctest remains intentionally ignored.
- Forced unsupported provider: no_std and std failure probes passed, including ring and entropy seeding.
- Both the minimal browser harness and actual web-app passed wasm32-unknown-unknown cargo check. No fresh browser execution, Samba SDK integration or deployment acceptance is claimed.

## Rebase verification

Rebased against committed `aab06fd57c85a51ed76c12d3ae99faa8760c5241`, preserving the current HTTP, futures, regex, numeric, SQLite and synchronization consolidations. The earlier candidate results above are historical; this rebase reruns the focused entropy, source governance and proof checks, not the full web-app or ring cryptographic execution matrices. The old crypto-common 0.1 manifest edge no longer exists and was not restored. Source plans and provenance were generated from this exact baseline, not copied from the earlier candidate.

The live entropy backend matrix now exercises seven retained-provider modes (default, forced fallback, forced syscall, linux_getrandom, linux_raw, custom and unsupported), a browser compile and all three retired-backend negative checks. Retired 0.2 code is authenticated archival material, not a second implementation to execute. The independent ahash fail-closed consumer still executes both tests, rejects browser builds without Web Crypto opt-in, and compiles with it. The migration error matrix is wired into the foundation suite.

## Current receiving-tree integration (2026-10-21)

Applied to `f92b074e91e8de3db3d1fa5d0435557cdb360e45`, retaining the HTTP deadline and crypto namespace corrections. Fresh source plans authenticate this receiving commit. Metadata and consumer-index verification report 275 total active source trees (273 foundation packages), not the historical counts above.

The receiving-tree run passes all seven native backend modes, the browser compilation check, three retired-backend negatives, two native fail-closed consumer tests, explicit browser opt-in rejection/success, and the complete core/no_std/std/error/provider-doc matrix. Real storage-server, storage-admin and web-build consumers pass locked offline type checks and strict all-target Clippy. These are compilation and native behavior results, not browser entropy execution.

Every standalone entropy fixture receives the repository Rust toolchain pin before its first Cargo invocation. Five portable runner tests cover isolated source/lock ownership and pin presence at invocation time. Nested UUID private `getrandom` module paths and the benchmark function name are deliberately preserved rather than blindly renamed.
