# Large browser download fault regressions

`browser-probe.mjs` exports `verifyDownloads(page)` for a Playwright caller that already
has an authenticated production-WASM file-details page open against a real ZXS
server with the new advertised transfer timeout fields. Use an isolated browser
profile and disposable server pool. Import it by absolute file URL, then call
`await verifyDownloads(page)` and persist its returned case array only after the
promise resolves successfully. Set the caller's timeout to at least 180 seconds.
Use official Chromium with its sandbox enabled. Do not bypass host socket or
sandbox restrictions.

The standalone Playwright function is optional. The existing Rust `browser-smoke`
CI job directly invokes the same test-only JS fault fixture and covers real 65 MiB File uploads, advertised-limit rejection, bounded
reads and original File Request bodies, plus real response download session
isolation through a controlled picker backed by actual OPFS writables. It also
invokes picker fault/deadline tests and production OPFS corruption/quota cleanup,
retained export, explicit cleanup and single-job admission. Neither writable-disk
capability must produce DOWNLOAD_STORAGE_UNSUPPORTED before requesting content;
there is no old-browser whole-file memory fallback.

The Playwright probe substitutes only download responses and explicit injected
quota/cancel/corruption faults. Picker writes, close, abort, file reads and cleanup
use actual OPFS. It checks 65 MiB verified commit, checksum mismatch, truncated
and excessive response lengths, quota and close failures, picker cancellation,
write cancellation and one-write-at-a-time backpressure. Idle/total timeout cases
explicitly substitute shorter advertised timeout values and stalled/slow response
streams; they do not claim a real slow-network measurement. Fallback cases use the
production OPFS implementation and verify retained backing bytes, explicit fresh
export, one-job admission, and explicit cleanup/cancel. Synthetic quota exceptions
do not prove behavior under actual storage exhaustion.

Not verified by this probe: native OS picker dialogs, mobile Safari/Firefox,
physical disk durability, real network timeout timing, logout during hashing,
peak browser RSS, or filesystem mutation during File hashing. Never report
syntax checks, Rust compilation, or this file's existence as browser execution.
