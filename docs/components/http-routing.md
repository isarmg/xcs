# HTTP routing

`http-routing` is the single owner of routing, extraction, request extensions, response conversion and the HTTP body type previously split between Axum and axum-core. The kernel is an internal organization boundary, not a second Cargo package or duplicated trait implementation.

Default features preserve the existing application configuration. With defaults disabled, routing and kernel logic compile for Linux amd64 and browser WASM without Tokio, Hyper or network sockets. HTTP server/runtime integration stays explicit behind `tokio` and `http1`/`http2`. Form and query rejection constructors now follow their extractor features instead of leaving unused feature-inactive constructors in the merged crate.

`axum-macros` remains a separate host procedural-macro package. Its generated paths now target `http_routing`; no old crate alias is retained. The kernel's original unsafe prohibition is retained explicitly. HTTP parsers, TLS, routing semantics and response status/body behavior are not rewritten.

`.github/scripts/check-http-routing.py` exercises six native/WASM feature configurations and five maintained in-memory behavior cases: path/query routing, 404/405 distinction, request/response bodies and status, extractor rejection, and real derive/debug-handler macro expansion. No bound socket or fake test provider is required. Historical upstream tests remain individually attributed, not misreported as all executed.

Original sources: Axum 0.8.9 and axum-core 0.5.6. Exact versions, licenses, upstream commits, immutable originals and relocated test identities remain in provenance/catalog/consolidation receipts. The upstream macro package's original identity is also preserved there.
