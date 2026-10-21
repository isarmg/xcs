# SQLite access architecture

The maintained `sqlite-access` package combines the former SQLx facade, query engine,
and SQLite driver. It is one Cargo package, not three renamed wrapper packages.
The native `libsqlite3-sys` binding remains a separate package because it owns the
SQLite C build and native linking contract.

## Module boundaries

- `src/query_engine`: pooling, transactions, statements, typed values, query builders,
  errors and statement caching. This module retains `forbid(unsafe_code)`.
- `src/sqlite`: SQLite-specific types, options, worker connections and native FFI.
  This is the only module allowed to use unsafe code.
- `src/lib.rs`: the unified public query API and SQLite types. Generic query-engine
  modules remain available where actual consumers need them, including
  `common::StatementCache`; the implementation module itself is private.
- `test/foundation/sqlite-access/{facade,query_engine,sqlite}`: preserved upstream
  tests, separately attributed to their original source packages. Archived upstream
  tests for removed database backends/macros remain provenance, not supported APIs.

## Feature policy

Production continues to select `sqlite-bundled`. JSON, UUID, regexp, offline
metadata, serialization, bit-vector and IP address type exports remain opt-in.
`sqlite-deserialize`, `sqlite-load-extension`, `sqlite-preupdate-hook` and
`sqlite-unlock-notify` retain explicit capability gates. `sqlite-unbundled` uses
real system SQLite headers and libraries; it is not a second driver implementation.
No PostgreSQL, MySQL, migrations, generic Any driver, runtime selector or TLS
selector is restored by this consolidation.

## Upstream provenance and validation

The upstream origins are `sqlx` 0.9.0, `sqlx-core` 0.9.0 and `sqlx-sqlite` 0.9.0,
from https://github.com/launchbadge/sqlx, under MIT OR Apache-2.0. Immutable original
manifests, licenses and source hashes remain in `compliance/provenance`; the
multi-origin receipt records every production file and reversible external test
adaptation. Original upstream documentation is not renamed.

The SQLite scope gate builds a real consumer and rejects removed APIs/features.
Retained regexp tests create an explicit Tokio runtime and isolated
in-memory databases, without implicit SQLx fixtures. Existing first-party tests
exercise timeout, rollback, savepoints, Unicode row diagnostics, statement-cache
capacity/eviction, clear and pool closure. No tests operate on user databases.

SQLite ABI symbols, `_sqlx_savepoint_*` SQL identifiers and `file:sqlx-in-memory-*`
connection URI names are intentionally preserved: these are native/SQL identifiers,
not Cargo package names. Diagnostic worker threads use the functional name
`sqlite-worker-*`. There are no active `SQLX_*` environment selectors.
