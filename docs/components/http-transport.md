# HTTP transport

`http-transport` combines the maintained Hyper protocol engine and HyperUtil adapters in one package. Protocol types remain owned by the engine; `adapters` uses those same types rather than duplicate providers. The original edition and unsafe boundaries are unchanged. Client/server, runtime, pooling, proxy and service adapters remain explicit features. Bare protocol configurations do not enable Tokio networking or runtime features.

## Maintained scope

HTTP/1 is maintained. The former HTTP/2 feature depended on an absent `h2` provider and was not activated by any maintained product profile. It is removed, not replaced with an empty feature. Cargo rejects requests for `http2`; `full` means the supported HTTP/1 and adapter union. Existing protocol version constants and C ABI symbols remain semantically unchanged. The existing C HTTP/2 option entry point consistently reports `HYPERE_FEATURE_NOT_ENABLED`; it does not claim HTTP/2 support. The private, opaque options structure no longer stores an unused HTTP/2 flag.

Only retired HTTP/2 cfg conditions were evaluated in the syntax-aware pruning pass. Mixed conditions preserve unrelated predicates. Eight original implementation files are archived byte-for-byte; original test ownership is retained, including dormant upstream tests. Parser algorithms and TLS implementations were not rewritten. Adapter-only lint scope and explicit Future imports preserve the original module/edition boundary.

## Validation

The maintained gate checks 26 native/WASM feature profiles, rejects compiler warnings and the retired HTTP/2 feature, and validates original unstable tracing/FFI opt-in configurations. Twenty-five behavior tests exercise real HTTP/1 client/server handshakes, body/status/header transfer, malformed requests, service errors, service-adapter identity and readiness, route integration, upgrades, HTTP/1 pool reuse, unsupported version/ALPN rejection, and owned connection/graceful futures over in-memory duplex streams. Two focused FFI tests exercise distinct non-zero-sized opaque option allocations, setters/free, and the preserved unsupported HTTP/2 entry point without consuming the options. No listening socket or fake transport provider is used. Historical attributed tests are not represented as executed.

## Provenance

Sources: Hyper 1.12.0 and HyperUtil 0.1.21, as recorded in the repository's immutable source catalog and consolidation receipt. The receipt, not this prose, is authoritative for original versions, input hashes, licenses and exact source/test routes. The standalone consolidation generator regenerates all transformed routes from original Git objects. The explicit cfg edit inventory and pruner fixtures accompany the staged change. Host/native/WASM checks do not imply non-Linux product support.

## Integrated feature design

`full` directly names the supported client, proxy, pool, server, service and runtime capability union. There is no redundant `adapters-full` aggregation feature. Transitive capability edges are omitted where their prerequisite already guarantees them. The merged futures and service-stack providers each have one type identity. Combined tests exercise a real route and observable middleware through the HTTP/1 engine, and deterministically verify that an adapter never calls a service before readiness. No throughput or latency improvement is claimed without measurement.

### Upgrade boundary regression

`test/architecture/http-transport/upgrade.rs` exercises generic HTTP/1.1 upgrade
using real Tokio duplex I/O: 101 headers, preservation of already-buffered bytes,
subsequent bidirectional I/O, clean EOF, and parser rejection of an unsupported
HTTP/2.0 wire request before the application service executes. It does not
implement or claim WebSocket handshake/frame support. The routing component's
unusable WebSocket feature is explicitly absent rather than silently enabled.

## Header deadlines and protocol detection

The auto server retains its protocol probe: fragmented HTTP/2 prefaces must be
rejected without emitting an HTTP/1 response. Direct HTTP/1 parsing does not have
that same fragmented-input behavior, so deleting the probe is not an equivalent
simplification. All 23 preface split positions have regression coverage.

The probe uses the existing HTTP/1 timer policy and starts the first-header
deadline on its first poll. It transfers the same pinned timer into HTTP/1 parsing,
rather than granting a fresh budget after detection. Subsequent keepalive headers
retain their ordinary fresh deadline. This bounds idle or matching-prefix stalls
without changing buffered upgrade bytes, protocol errors or graceful shutdown.

The maintained routing server installs the Tokio timer and activates the existing
30-second header policy. Generic consumers still have no default deadline without
a timer; explicitly disabling the header timeout remains supported. An explicitly
configured timeout requires a timer. Paused-time tests cover those policies,
first-poll timing, deadline handoff, keepalive renewal and the actual routing serve
path using a duplex listener, not a real network socket.

Paused-time upload and streamed-download regressions each keep a partial 64 KiB
body open for 61 seconds after complete headers, then verify byte-exact successful
completion. The header policy is not a replacement for application body or
operation limits.
