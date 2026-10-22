# HTTP transport private primitive consolidation

Baseline: `f18c77be9d3c1da3f337e6e101c959cdf3abcd26`.

## Ownership rationale and tradeoffs

The package count is an observation, not an acceptance threshold. These three
helpers form one private consumer chain wholly owned by HTTP dispatch/body
plumbing. Consolidating that implementation removes packaging boundaries without
adding a general-purpose facade, cross-domain dependency, or duplicate type.
The stronger practical benefit is that their previously disconnected leaf tests
and scheduling-sensitive contracts now run with the transport gate.

No throughput, latency or compile-time improvement is claimed. There are fewer
Cargo compilation units, but changing a helper now invalidates the transport
crate rather than a separately cached leaf crate. The algorithms, layouts and
atomics are intentionally unchanged; behavioral validation does not substitute
for a controlled runtime benchmark. Retaining separate unsafe source modules is
more important than the numerical package reduction.

## Consumer enumeration before removal

Every maintained `Cargo.toml` was parsed recursively, including workspace,
normal, build, dev and target-qualified dependency sections. Both package names
and `package` aliases were checked, regardless of optional activation. The only
three declared edges were:

- `http-transport` → `want`, optional under `client`.
- `http-transport` → `atomic-waker`, optional under `http1`.
- `want` → `try-lock`, unconditional.

Source consumers were transport `src/client/dispatch.rs`, transport
`src/body/chan.rs`, and the want implementation respectively. Tokio and futures
have their own distinct, pre-existing internal atomics/locks; they did not depend
on these Cargo packages and are not changed. The inactive benchmarks and all
three external test files are also retained and routed explicitly.

## Scope and safety

The three real implementations move into private transport modules, with no
forwarding crates or old-package dependency aliases. Only ownership/imports,
visibility, crate-only attributes, formatting and harness plumbing change. The
original state machines, atomic orderings, unsafe blocks and guard behavior are
preserved. The demand layer adds `forbid(unsafe_code)`; the audited lock and
wakeup implementations stay separate. Public HTTP, body, service, TLS and FFI
identities are unchanged. No httparse, date, model, tower or protocol changes are
part of this slice.

Source archive: `compliance/http-transport-primitives-source-archive.json.gz`.
It records exact base64 bytes and SHA-256 hashes for every removed leaf manifest,
source, benchmark and test. The reviewed consolidation plan preserves all origin
and license metadata, including the atomic-waker third-party notice, as well as
all prior transport consolidation history. The prior multi-origin owner requires
the chain-aware source-catalog adapter; old receipts must not be flattened or
rewritten to integrate this change.

## Verification commands

`python3 .github/scripts/check-http-transport.py` now requires 18 private primitive
tests (six retained plus twelve added), three extra repeated executions of the
two race cases, the original 25 transport tests, two FFI allocation/ABI tests,
and the existing native/WASM feature matrix. Temporary harnesses depend only on
maintained local Tokio/futures providers and compile the real private sources;
no old helper package or legacy runtime dependency is restored.

Follow with `python3 .github/scripts/check-http-body-consolidation.py`,
`python3 .github/scripts/check-service-stack.py`, and
`cargo test --locked --offline -p storage-client -p storage-server`.
Catalog/architecture/workspace gates run after parent integration, using the
exact reviewed plan rather than a stale generated catalog.

The expected whole-workspace identity delta is exactly minus three packages.
A feature-only closure reduction, retained-but-unexecuted test file, or unrun
validation command is not a completed proof. Execution results are recorded in
the implementation handoff, separately from this verification specification.

## Executed validation (2026-10-22 UTC)

- `python3 .github/scripts/check-http-transport.py` passed: 26 native/WASM
  capability checks, HTTP2 rejection, 25 transport tests, two FFI tests, all six
  active retained leaf tests, twelve added contracts, and three extra runs of
  both 128-iteration race cases. Compiler warnings remain rejected by the gate.
- The HTTP-body gate passed its eight native/WASM capability combinations,
  negative feature probe, 41 unit tests, two integration tests and five doctests.
  Its final four-package consumer build hit the existing 300-second limit while
  compiling cold TLS dependencies with one job, without a compiler error.
  The identical `cargo check --locked --offline --jobs 1 -p http-routing
  -p http-transport -p tower-http -p reqwest` then passed on the warmed cache.
  The gate's source timeout/test requirements were not changed; only the
  invocation's job cap and target directory were adjusted to the assigned slot.
- `python3 .github/scripts/check-service-stack.py` passed seven native/WASM
  feature checks, four behavior tests and routing/Reqwest consumer compilation.
- Offline full-workspace metadata and lock regeneration measured 255 → 252
  package identities on this isolated baseline, removing exactly want, try-lock
  and atomic-waker with no added identities. This is not the combined branch's
  package count and is not a performance result.
- Static declared-dependency enumeration, Python syntax validation,
  `git diff --check`, and exact reconstruction of each moved implementation from
  archived bytes plus the reviewed non-algorithmic edits all passed.

The parent explicitly deferred storage-client/storage-server tests to final
integrated CI to avoid duplicate cold full-product builds. Those tests, full
source-catalog lifecycle/architecture/workspace validation and the combined
branch's final package count are pending integration, not claimed as passed
here. The source worker did not publish changes or regenerate the shared source
catalog/external-test mapping from its older baseline. Runtime/compile-time
performance was not benchmarked.
