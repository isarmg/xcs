# Project-owned tracing

The `tracing` package owns instrumentation macros, `primitives` dispatch and metadata,
`collector` registry/layers/filtering/formatting, `log_bridge` adapters and `pattern`
incremental field matching. The host `tracing-attributes` procedural macro remains
separate. The facade keeps its functional name and generated macro identity; no
old package compatibility crates or duplicate provider identities remain.

## Capabilities

- No defaults: no_std + alloc instrumentation with only pin-project-lite. No
  collector, regex, log, time, thread-local registry or serialization provider.
- `collector`: alloc-capable layer abstractions; `registry` adds std and span storage.
- `fmt`, `ansi`, `env-filter`, `json`, `time` opt into their actual providers.
- `pattern` opts into shared regex DFA syntax/search; `unicode` adds Unicode support.
- `log` is trace-to-log fallback; `log-tracer` enables log-to-trace. They are distinct
  from `log-always`, which deliberately emits both and may duplicate events.
- `collector-default` selects formatting, ANSI, registry, bridge and small-vector
  support for the administrative executable. Library defaults remain std+attributes.

JSON event fields stream directly through SerializeMap instead of requiring the
missing tracing-serde package or building an intermediate event value tree. Typed
values, string/debug escaping, duplicate-key order and first-error propagation are
covered. Existing span-field storage is unchanged; no end-to-end performance claim
is made without a benchmark. Span macros now short-circuit without an overwritten
initial interest value, while retaining callsite and field evaluation semantics.

The global maximum-level atomic uses explicit integer encoding, not compiler-chosen
Option niches. Tests verify that actual encoding. Unexpected encodings fail closed
in all profiles rather than taking an unchecked release-only branch.

## Retained limits

Only Linux amd64 and browser WASM compilation are validated. WASM checks do not
claim browser-runtime, filesystem or terminal behavior. Unsupported, unused
interest-cache (missing LRU provider), system-local time lookup, and inert valuable
integration are not advertised as capabilities. UTC/fixed-offset formatting remains.
Original tests requiring tracing-mock, futures test providers or wasm-bindgen-test
are retained with provenance, not counted as executed. One inherited ignored test
is explicitly reported by the retained suite. Original benchmarks remain inert
under retained source/origin paths until an authenticated production-to-test transfer is implemented.

## Sources

- tracing 0.1.44, tracing-core 0.1.36, tracing-subscriber 0.3.23,
  tracing-log 0.2.0: https://github.com/tokio-rs/tracing
- matchers 0.2.0: https://github.com/hawkw/matchers

Original manifests, versions, licenses and source receipts remain in compliance
provenance. Their preservation is for attribution and audit, not a requirement to
keep upstream layout, compatibility names, or unavailable features in maintained code.
