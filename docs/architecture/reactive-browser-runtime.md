# Stable client-side reactive runtime

The maintained browser stack has three distinct responsibilities:

- `reactive-state` owns reactive dependency tracking, owner/child lifetime,
  cleanup, signals, memoization, and effects. It has no server/client shared
  hydration context and no hot-reload function-pointer registry.
- `view-renderer` owns DOM construction, template cloning, event attachment,
  incremental rebuild, and unmount. Its `hydration` cursor module remains live:
  it walks cloned client-side templates, not server-rendered HTML.
- `dom-helpers` supplies browser helpers; `src/interface` composes application
  components and lifecycle behavior. Browser timers in the application use the
  callback API, not the optional timer futures adapter.

The workspace pins stable Rust. The removed `reactive-state/nightly` feature
provided unstable function-call shorthand for signals; supported `.get()` and
`.set()` operations and stable closure conversion implementations remain. No
compiler-channel build probe is needed. `hydration`, `subsecond`, and the
`dom-helpers/hydration` forwarding feature are also removed rather than retained
as unsupported configuration promises.

This is a reduction of unsupported source and dependency edges, not a claim of
measured runtime acceleration. Graph/owner, DOM rendering, and application
component boundaries remain separate because they represent different behavior
and test contracts. Fonts and G4 geometry are unchanged.

The CSR scope gate runs all 14 retained reactive unit tests in isolated manifests,
with and without effects, using a test-only typed-builder dependency. It also
checks the real template probe on native and WASM targets
and rejects retired owner APIs and feature requests. The interface integration
suite covers cleanup plus parent/weak-owner lifecycle. Static regression tests
in `test/ci/test_reactive_csr_scope.py` prevent retired modes, the compiler probe,
and unnecessary direct dependency requests from returning.

Upstream identities, original manifests, licenses, and source receipts remain
under the existing `compliance/provenance` and `compliance/rust-sources.json`
records; these are not renamed to imply original authorship.
