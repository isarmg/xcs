# Verified browser downloads

Two feature-detected disk paths keep application memory bounded. They are browser
capabilities, not desktop/mobile user-agent rules. No server cache is enlarged.
Browser/device acceptance is tracked separately from compiled implementation.

## Transactional save picker

In a secure context, if `window.showSaveFilePicker` exists, the click handler
synchronously requests a destination before scheduling asynchronous work. The
browser owns its permission prompt. Cancelling/denying it does not silently fall
back to another destination. The picker is also available in some mobile browsers;
its availability must not be inferred from an OS name.

The response is streamed to `FileSystemWritableFileStream`. Exact Content-Length
and BLAKE3 ETag are checked before `close()`. Overrun, truncation, corruption,
read/write failure, permission denial, disk-full, cancellation and stale sessions
never invoke that commit step. Abort discards the browser's temporary replacement;
a picker can still leave an empty newly selected file. Existing destination
content is not replaced before close.

Once close has been invoked the browser controls the commit. Cancellation/logout
racing that final operation cannot retract an already committed file; the UI does
not report success if the session changed while awaiting close.

## Private-disk staging and explicit export

Without a save picker, the app checks for secure-context OPFS `getDirectory` and
writable file handles. It creates an owned cryptographically random job directory
under `zxs-verified-downloads`; per-job Web Locks are held where supported. No
other job directories are scanned or deleted. There is one active or retained job
per page. New downloads require cleanup of an existing retained copy first.

The initial network stream is length/hash verified before closing the private
writable. The resulting file's actual size and a second incremental BLAKE3 read
are then checked. Only after these checks and session fences does the UI offer
**Save to device**. No URL is exposed while bytes are unverified.

A fresh user click creates a Blob URL directly from the native OPFS-backed File
snapshot, without a whole-file ArrayBuffer, WASM vector, or Blob-parts assembly.
This is **handoff to the browser, not confirmation of destination saving**. The
private backing file and URL remain alive after handoff: modifying/deleting that
file can invalidate its File snapshot. There is no 30-second/other guessed expiry.

The UI offers **Clean up after saving** and **Cancel and clean up**, with a warning
that explicit cleanup can interrupt an ongoing browser save. The user should
keep the page open and inspect the browser's downloads before cleanup. Explicit
cleanup revokes the URL, invalidates export, aborts any writer, then removes the
owned job directory and releases its lock. Failed cleanup keeps a non-exportable
retryable cleanup control. Cleanup uses the same single physical-worker admission
as downloads: after a 120-second no-progress timeout (or cancellation), foreground
feedback reports pending cleanup while the worker continues to own its slot.
Logout or repeat clicks cannot start additional cleanup workers. If cleanup
finishes after foreground feedback, a single root-scoped completion observer
refreshes the UI copy state; retrying already completed cleanup is idempotent.
The observer is removed on UI unmount, uses no polling, and reads fresh session-
checked labels/export permission. Login/session replacement also reconciles once.
Late failed cleanup therefore exposes a retry control without another download. A partial setup directory whose removal fails
before a complete spool exists instead quarantines one small admission record;
new jobs remain blocked with explicit site-storage/reload recovery instructions.
No polling task continues after that physical operation has ended.

Logout immediately invalidates export and revokes its URL, and requests cancellation
of active reads and writes. A retained copy is not automatically deleted: a browser
export may still be reading its backing file. Explicit cleanup remains available,
with a generic label rather than the previous session's filename.
Unexported failed jobs are removed only after their physical I/O and abort settle. Browser/process crashes, page closure,
or storage deletion failures can leave private staging data. This version does
not recover/export prior-session jobs or use expiry-based sweeping; browser
site-storage controls are the fallback for removing residual data. Normal error
cleanup failures include an actionable site-storage message. No persistent-storage
permission is requested automatically. Available storage estimates are advisory:
if the estimate is smaller than the file, admission fails early with an actionable
message. A larger or unavailable estimate does not promise capacity; actual
write/close failures (including quota exhaustion) are handled without an
unlimited-memory fallback.

## Bounds, timeouts and required capabilities

- One transfer at a time, at most 256 KiB per WASM hash copy / write call, with one
  awaited write at a time. Browser-controlled network/readback chunks and browser/
  OS internal buffers are not claimed to be bounded by this WASM limit.
- Cancel requests cancellation of network, current stream reader and writer. A monotonic-clock
  watchdog checks credential generation and configured idle/total deadlines.
  A modern browser monotonic performance clock is required; wall-clock fallback
  cannot change the timeout clock domain.
  Strict millisecond-string limits come from `/api/capabilities`, independently
  of upload support. Idle progress means a completed read/write. There is no
  unconditional 120-second total deadline on file content. The user-controlled
  picker dialog is exempt; private-storage setup and picker writable creation
  have a 120-second no-progress watchdog before server limits are available.
- Foreground cancellation returns promptly even if a browser write, create, close,
  or abort promise never settles. One physical worker retains the admission slot
  until pending I/O and cleanup actually finish. New attempts return `DOWNLOAD_BUSY`;
  cancelling again cannot create abandoned workers or queues. A permanently stuck
  operation may require a page reload, not an ever-growing set of replacement jobs.
  Check browser download status first: reload may interrupt saving and is not
  proof of rollback or automatic recovery.
  Cancellation after close was requested cannot promise rollback. There is one
  foreground Rust waker, not a Promise race listener added for every chunk. If
  setup fails while removing its partial directory, a stuck removal can defer the
  original diagnostic until the setup watchdog reports cancellation/cleanup pending.
- If neither disk path is available, downloads fail before fetching file content
  with `DOWNLOAD_STORAGE_UNSUPPORTED` and HTTPS/modern-browser guidance. There is
  no whole-file memory fallback, even for small files. Bounded text previews remain
  independent. OPFS Blob handoff is not reported as confirmed saving.
- Both disk paths depend on browser capabilities, quota, permissions and available
  storage. This implementation is not evidence of successful native saves on every
  Safari/Firefox/mobile device. OPFS quota may be smaller than the desired file.

## Sources reviewed

- [WHATWG File System](https://fs.spec.whatwg.org/#api-filesystemfilehandle-createwritable)
  defines writable replacement and close semantics.
- [File snapshot semantics](https://fs.spec.whatwg.org/#api-filesystemfilehandle-getfile)
  govern the lifetime of files backed by private storage.
- [File System Access](https://wicg.github.io/file-system-access/#api-showsavefilepicker)
  specifies activation, permissions and save-picker behavior.
- [Chrome file system guide](https://developer.chrome.com/docs/capabilities/web-apis/file-system-access)
  documents gesture ordering and writing streams.
- [Storage Standard](https://storage.spec.whatwg.org/) defines estimates and quota,
  not a guarantee that an arbitrary file will fit.

## Verification

Rust tests cover incremental digest/length boundaries, corruption, truncation,
excess body bytes, empty content, a real 65 MiB stream hashed using a reused small
buffer, >4 GiB admission, exact browser-integer bounds, and session-admission regression.
`test/evidence/large-downloads/browser-probe.mjs` is a real-browser-testable harness
for shipped WASM. Its injected faults and browser execution status must be reported
separately from native device/disk acceptance. Neither compilation nor the presence
of that harness proves that a native picker, quota failure or mobile save was run.
