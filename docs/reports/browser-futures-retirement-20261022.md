# Canonical browser future runtime

The supported browser surface of the removed `wasm-bindgen-futures` 0.4.79 tree was a compatibility re-export; the browser scheduler already lived in js-sys. Production callers, the CSR fixture manifest, and all generated async bindings now use the already-shipped `js-sys::futures` implementation directly. This removes one real package/tree, not merely a counted facade. Applied alone to f18c77be, the actual all-target root lock changes from 255 to 254 packages and the source catalog from 237 to 236 active trees; only wasm-bindgen-futures disappears.

- Keep `js-sys::futures::{JsFuture, spawn_local, future_to_promise}` and the original task, queue, JSPI, single-thread and atomics implementations byte-for-byte unchanged.
- Preserve the task-dispatch public `wasm-bindgen` feature, routed to optional js-sys with explicit `default-features = false, features = ["std"]`. js-sys/std also propagates wasm-bindgen/std. Existing frontend js-sys feature selection is unchanged.
- Make generated imports, generic imports, exports, async starts, async main and class markers use the configured js-sys path. Remove environment/cfg switching and obsolete futures path fields.
- Reject all experimental_tokio attributes explicitly rather than producing unresolved shim paths or silently selecting a different executor. Reject retired wasm_bindgen_futures overrides and point users to js_sys path overrides.
- Preserve the existing generator rejection of both Emscripten custom-section markers. No native application, SSR, Emscripten or compatibility runtime is introduced.
- Preserve every G4 geometry, font and Blob API and feature. This change does not trim web-sys.

## Provenance integration

`compliance/browser-futures-retirement-plan.json` contains the exact prior package receipt. `compliance/wasm-bindgen-futures-0.4.79-retirement.json.gz` retains all three final active source files, including unsupported Tokio implementation bytes, with exact hashes. Original source archives, origin records, licenses and documentation remain untouched.

Root integration must regenerate the one root lock, then run `python3 tools/rust-sources/manage.py refresh --retire-plan compliance/browser-futures-retirement-plan.json` and `python3 tools/rust-sources/manage.py verify --metadata`. The CSR fixture's current external-test receipt is already updated to match its source edit; immutable historical test-source hashes remain unchanged.

## Verification matrix

- Static archive authentication, removal of active dependencies and canonical macro routing, std feature forwarding: `python3 test/ci/test_browser_futures_retirement.py`.
- Actual proc-macro default/all-feature tests plus native and wasm32-unknown-unknown consumers, async imports/exports/start/main/class paths and explicit negative diagnostics: `python3 .github/scripts/check-wasm-binding-macros.py`.
- Real generated Web fixture with async Promise resolve/reject and microtask scheduling probes: `ZXS_CHROMIUM=/usr/bin/chromium python3 .github/scripts/check-web-bindgen-scope.py`. The browser uses an isolated temporary profile and localhost fixture server, not user browser state. The foundation-language CI shard supplies its installed Google Chrome through ZXS_CHROMIUM, so real browser execution is required in CI; local runs may omit this variable when browser launch is unavailable.
- Existing Emscripten marker rejection tests: `cargo test --locked --offline -p web-bindings --bin wasm-bindgen`.
- CSR/browser template, native properties, negative APIs and unchanged G4 closure: `python3 .github/scripts/check-csr-scope.py`, followed by execution of its generated fixture in a browser.
- Production browser consumer: `cargo check --locked --offline -p web-app --target wasm32-unknown-unknown`.

## Local validation status

- Static archive/removal/std/CI-sandbox tests: 4 passed. The retirement-plan validator accepted the exact prior source receipt. Complete external Web fixture inventory matches its updated current receipt, with historical evidence unchanged.
- All js-sys futures, queue, JSPI and atomics task implementation bytes match the baseline. This is source-preservation evidence, not an atomics execution test. web-sys geometry/fonts/Blob sources and API feature selection are unchanged.
- Host macro tests: 38 default and 38 all-feature tests passed, with no compiler warnings.
- Native/WASM consumer checks passed: synchronous and async exports/imports, class methods, async start/main, strict-macro frontend fixture and removed-runtime diagnostics on both targets. The complete macro validator exited successfully.
- Real release WASM generation passed, including async imports/exports/class bindings and all 14 removed generator API groups. All 5 web-bindings unit tests passed, including rejection of both Emscripten markers before descriptor processing.
- Workflow checks passed: 11 workflow regression tests plus the workflow context checker. The foundation-language shard now requires the sandboxed Chrome async fixture.
- Full CSR validation passed: native template and tracing regressions, release WASM generation, trace-component-props native/WASM checks, 28 retired API checks, 13 removed-feature checks, and exact web-sys source/feature closure.
- Production `cargo check --locked --offline -p web-app --target wasm32-unknown-unknown` passed, covering the changed interface/web callers and task-dispatch. No warnings were emitted by the generator, Emscripten, CSR or production validation commands. The macro validator separately verifies its deliberate unused-attribute warning and strict-mode rejection.
- Real browser execution: blocked before page load. Chromium with default security and a fresh temporary profile aborts with `socket() failed: Operation not permitted` (process singleton, exit -6). No sandbox/security bypass is used. Local HTTP binding works; the real Chromium fixture remains available for CI through `ZXS_CHROMIUM`.

The isolated validation lock changed only the removed shim package and its three consumers. It is intentionally not committed: final lock and catalog integration belong to the root change.
