# Native upload boundary

The Web and App interfaces share one upload queue and publication protocol, but
not their byte transport. The browser accepts `File` inputs, checks the 64 MiB
per-file bound before creating a write session, then uses same-origin fetch.
The App opens a native picker and passes only an opaque selection ID, display
name, and decimal-string size to the WebView. Native bytes never cross IPC.

## Ownership and authorization

- A selection is registered against the current native work epoch. Its source
  is an already-open regular-file descriptor, not a path supplied by JavaScript.
  Desktop paths are opened once; replacing a pathname later cannot substitute a
  different file. This does not freeze in-place edits to the selected inode.
- Provider files are copied in Rust to a private RAII-owned import. Unused
  selections, failed registration, session replacement, and release remove owned
  imports. User-owned original files are never deleted.
- A selection is one-shot: attempting upload consumes it, including on failure.
  The UI requests reselection for a new upload attempt. An uncertain commit is
  recovered using the original workspace and operation ID, not by reuploading.
- Picker cancellation returns an empty list. Both normal multi-select and
  single-file branch replacement capture their destination before opening the
  picker. A late result after navigation, branch change, or logout is discarded.
- The Rust registry and UI cap a selection queue at 128 files. Explicit release
  is idempotent. Logout initiation and successful replacement login invalidate
  old work and clear unused sources; in-flight leases keep cleanup ownership.

## Dispatch and publication fences

Credential identity is distinct from the work epoch. Starting logout advances
work immediately, while retaining the credential needed to retry an unconfirmed
logout. A failed logout does not silently restore old work. Fresh explicit work
is not permanently disabled by the transport layer.

Hashing and rewinding a selected descriptor produces a `PreparedUpload`, without
network dispatch. The bridge verifies its original epoch before sending it.
Seal/commit uses the same boundary: a seal response cannot authorize a commit
once logout has invalidated that work. Requests already sent are not recalled or
rolled back; unknown publication outcomes still require original-operation
recovery. Native system-open captures one epoch across download, cache reuse
verification, and opener dispatch.

## Verification and limits

Bridge regressions cover opaque metadata, arbitrary-path/replayed-ID rejection,
private-import release, stale picker completion, pathname replacement with real
HTTP transfer, prepared upload invalidation, and a held seal/logout-response
race. Client transfer regressions retain regular-file/symlink support, FIFO
rejection, checksum validation, and atomic download behavior.

The Linux host build needs system GTK/WebKit development libraries; a bridge or
WebAssembly test pass alone is not a native GUI/device acceptance test. Native
picker and provider behavior still need each supported platform's validation.
In particular, providers returning `content://` URIs without an authoritative
filename are explicitly rejected before import. The UI directs the user to the browser or desktop App instead; document IDs are never silently used as remote filenames. File
URLs use filesystem URL decoding and retain their real UTF-8 basename. iOS
scoped-access cleanup covers both open failure and provider-copy failure.
Offline cache opening and durable upload recovery across application restart are
not implemented by this transport change. Logout is not a cache-deletion action.

The iOS cleanup rule follows [Apple’s scoped-resource documentation](https://developer.apple.com/documentation/Foundation/NSURL/stopAccessingSecurityScopedResource%28%29): releases balance acquired access; a release without access is a no-op. Provider cleanup is installed before the fallible open and tested independently of the native GUI SDK.

For the per-platform implementation and device acceptance gates, see the
[six-platform capability matrix](app-platform-matrix.md). The Android
DISPLAY_NAME route is a future project-owned adapter, not a third-party plugin
patch or an implemented provider capability.
