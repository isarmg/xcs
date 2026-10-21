# HTTP routing

`http-routing` is the single owner of routing, extraction, request extensions, response conversion and the HTTP body type previously split between Axum and axum-core. The kernel is an internal organization boundary, not a second Cargo package or duplicated trait implementation.

Default features preserve the existing application configuration. With defaults disabled, routing and kernel logic compile for Linux amd64 and browser WASM without Tokio, Hyper or network sockets. HTTP server/runtime integration stays explicit behind `tokio` and `http1`. Form and query rejection constructors now follow their extractor features instead of leaving unused feature-inactive constructors in the merged crate.

`axum-macros` remains a separate host procedural-macro package. Its generated paths now target `http_routing`; no old crate alias is retained. The kernel's original unsafe prohibition is retained explicitly. HTTP parsers, TLS, routing semantics and response status/body behavior are not rewritten.

`.github/scripts/check-http-routing.py` exercises eleven native/WASM checks including every remaining declared feature together and the full runtime-free WASM subset and five maintained in-memory behavior cases: path/query routing, 404/405 distinction, request/response bodies and status, extractor rejection, and real derive/debug-handler macro expansion. No bound socket or fake test provider is required. Historical upstream tests remain individually attributed, not misreported as all executed.

Original sources: Axum 0.8.9 and axum-core 0.5.6. Exact versions, licenses, upstream commits, immutable originals and relocated test identities remain in provenance/catalog/consolidation receipts. The upstream macro package's original identity is also preserved there.

## Transport integration boundary

The `http1` feature enables only the protocol engine. Runtime/server adapters are activated by `tokio`, not by the optional transport dependency itself. The maintained gate verifies native and WASM HTTP/1-only graphs without transport adapters or Tokio networking/runtime/time features. Application routing and protocol reception retain distinct body types; readiness-aware middleware is adapted explicitly, not aliased to the protocol service trait. The maintained server installs `TokioTimer`, activating the transport’s existing 30-second header deadline, including protocol detection. The timeout is not restarted when detection hands the connection to the HTTP/1 parser.

## Capability audit and inactive source evidence

The maintained component does not provide a WebSocket or multipart extractor.
`ws`, `multipart`, `__private`, `__private_docs` and `http2` are rejected by Cargo,
not accepted as empty compatibility features. Product manifests did not enable
these unavailable features. The application default feature set is unchanged.

Compiling the previously advertised optional features exposed unavailable
providers: `ws` imported `sha1`, `tokio_tungstenite` and `futures::sink::SinkExt`;
`multipart` required `multer`; the old `__private` test-client API required the
unavailable `reqwest::multipart` interface; `__private_docs` activated unavailable
`async_compression` through broad `tower-http` features. We remove the unsupported feature declarations and active exports rather than
adding replacement engines or claiming unexecuted behavior.
The exclusively associated optional dependency declarations are also removed.

The four existing files `src/extract/ws.rs`, `src/extract/multipart.rs`,
`src/test_helpers/mod.rs` and `src/test_helpers/test_client.rs` remain byte-identical
at their original locations inside `http-routing`, including embedded notices.
They are inactive historical source evidence, not compiled modules or supported
APIs. No production source is deleted and no duplicate archive is introduced.
The gate checks all four exact hashes and rejects restored feature declarations,
inactive-only dependencies and module exports. Existing historical tests in
`test/foundation/http-routing/` remain source evidence, not a passing test suite.
The maintained routing gate now compiles all declared features natively, the
runtime-free subset for browser WASM, and rejects each removed capability.

HTTP/1.1 generic protocol upgrade support remains in `http-transport`. Its actual
in-memory duplex tests cover the 101 exchange, buffered bytes crossing the
upgrade boundary, bidirectional data, EOF, and rejection of an HTTP/2.0 wire
request before reaching the service. These are transport tests, not a WebSocket
handshake/frame implementation. No WebSocket support or performance improvement
is inferred from them; canonical HTTP version constants remain unchanged.
