# Modern Web large-transfer integration verification

Verification snapshot: 2026-10-22. Isolated QA started from a28c2f89; receiving
integration starts from b27c8d7 and regenerates its own catalog. Isolated build
artifacts are not exact receiving-repository release artifacts.
Composed bounded File uploads, server-advertised deadlines, transactional picker,
verified OPFS export/cleanup, modern disk-only admission, monotonic clock, and
wasm32-only BLAKE3 SIMD. Preview remains bounded; server RAM cache is unchanged.

## Executed successfully
- Final `cargo test --locked --offline -j 2 -p web-build -p browser-smoke -p web-app`:
  Web 136, browser-harness 10, builder 10 unit tests; one builder doctest.
- Final wasm32-unknown-unknown Web `cargo check`.
- Final all-targets clippy for those three packages with `-D warnings`.
- Integrated server advertised-timeout regression: one passed.
- Real disposable CLI server init, login, capabilities HTTP 200: max upload
  1073741824, idle timeout 60000 ms, total timeout 3600000 ms, decimal strings.
- Built actual wasm-bindgen CLI 0.2.129; generated contract fixture contains
  required runtime bindings and excludes retired TypeScript/backend outputs.
- SIMD worker: 72 deterministic cases agree across native, portable WASM and SIMD
  WASM in Node; SIMD artifact declares simd128. This is not browser execution or
  a measured speedup.
- Both test-only JS files pass syntax check; scoped rustfmt and whitespace check.
- Canonical metadata refresh passed in isolated tree. Catalog is build-local and
  excluded from source patch; receiving repository must regenerate its own.

The pre-cancellation-fix candidate completed a production asset build, both
canonical source checks, and HTTP 200 serving in a disposable server. The final
cancellation-aware candidate production build is still pending at this snapshot;
actual receiving-repository production/browser acceptance belongs to its exact CI
commit. An earlier build correctly refused changed input inventory; stale outputs
were not delivered as verified.

The final source also separates unit tests under test/, removes the unreleased
version suffix from the private storage namespace, and retains a reproducible
optional SIMD correctness gate at test/tools/web-blake3-simd, documented in
[the manual gate guide](../tools/web-blake3-simd.md). It is not wired
CI or a browser performance benchmark.

## Browser execution blocker
Chromium 155.0.8059.79 and Playwright were found; ChromeDriver was absent. TCP
loopback bind succeeded. Actual Chromium launch with `chromiumSandbox: true`
exited SIGABRT before creating a page:

    FATAL:chrome/browser/process_singleton_posix.cc:297
    Check failed: . socket() failed: Operation not permitted (1)

No sandbox bypass, alternate process-channel workaround, user desktop or access
escalation was attempted. No browser pass evidence or screenshots were produced.

## CI-wired, not executed in this environment
The existing Rust `browser-smoke` CI command invokes these real-WASM cases:
- Real 65 MiB filesystem File, actual publication, original File Request body,
  no slice or content arrayBuffer read above 1 MiB.
- Real advertised maximum +1: reject before write-session/workspace creation and
  before content hashing; updated transfer screenshots/filter contract.
- Controlled picker backed by real OPFS writable: real server bytes, click
  activation, verified close, stale-session/logout abort and no false close.
- Shared test-only fixture: checksum corruption, truncated/overlong content,
  injected quota/close errors, picker cancel, write cancel, short advertised
  idle/total deadlines; real OPFS writes and one awaited write at a time.
- Production OPFS: 65 MiB verified staging/readback, real browser File export,
  retained backing bytes, explicit cleanup/cancel and single-job admission.
- Production OPFS corruption/quota-estimate cleanup, no false export state.
- Independently stalled write/abort and retained removeEntry: prompt foreground
  cancellation, physical-worker quarantine, no duplicate cleanup, eventual
  recovery and automatic retained-panel reconciliation. These injected cases
  are prepared, not claimed executed in a browser here.
- Neither disk API available: DOWNLOAD_STORAGE_UNSUPPORTED before capabilities
  or content fetch. No whole-file memory-only compatibility path remains.
- Six responsive viewport captures from actual application state, when CI runs.

The optional Playwright wrapper uses the same fixture; standalone import is not
claimed as an independent CI job. Synthetic stream/quota/deadline faults are
explicit and do not represent measured real network delay/storage exhaustion.

## Still unverified
Native OS picker dialogs, physical-disk durability, mobile Safari/Firefox, actual
storage exhaustion, peak browser RSS, real slow-network timeout observations,
upload logout during hashing/in flight, mutation of underlying files during File
hashing, and full device support. Modern capability requirements do not establish
brand-wide browser support. Interrupted process cleanup can leave private orphans.
