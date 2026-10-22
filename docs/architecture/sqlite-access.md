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
- `test/platform/unit/sqlite/driver`: locally authored direct-connection and parser
  contracts, separate from attributed upstream test material.

The shared `TransactionConnection` implementation belongs to `transaction::connection`.
Its physical file remains `src/query_engine/pool/maybe.rs`, selected with an explicit
module path to preserve immutable source-history ownership. This does not restore
a legacy public module or introduce a compatibility wrapper.

## Feature policy

Production continues to select `sqlite-bundled`. Direct filename/connect-with
operations, transactions, and the SQLite-specific string parser require neither a
Tokio runtime nor URL hostname/IDNA/Unicode normalization. Percent and form codecs
remain because SQLite URI options and percent-encoded filenames use them.

- `pool` opts into Tokio-backed multi-connection scheduling, semaphores, idle queues
  and close notifications. `platform-io/sqlite-pool` exposes the platform's retained
  async multi-connection and snapshot-pool adapters. Synchronous snapshot capture
  and the storage engine's direct workers remain available without it.
- `url` opts into interoperability with a typed general-purpose `Url`, including
  `ConnectOptions::from_url` and `to_url_lossy`. String `connect` and `FromStr` retain
  the original SQLite-specific parsing without enabling the general parser.
- Transactions own their borrowed or pooled connection source independently of
  the pool module. Disabling pooling does not introduce a second transaction
  implementation or a compatibility facade.

The family gate executes both default and retained optional capabilities, asserts
that minimal production dependency closures exclude pool scheduling and general
URL parsing, and verifies disabled APIs fail compilation. This prunes selected
build closures only; it does not reduce the repository's total active package
count or remove retained upstream source/license provenance.

The source catalog records `sqlite-pool-validation` separately as a non-production,
compile-role profile rooted at `platform-io` with exactly its `sqlite-pool` feature,
matching the family gate's optional platform tests. Its native pool notification
path still needs `parking`; that source is not a default product dependency or an
unused package to retire. Feature-specific metadata is resolved independently of
default profiles, and the requested feature set is recorded with the closure.

Direct-worker tests preserve memory isolation/sharing, absolute and relative
filenames, literal Unicode/percent/query characters, URI option encoding,
transactions/savepoints/rollback, errors and explicit closure. Non-UTF8 filenames
continue to return the existing `InvalidData` error before opening a native
connection; this change does not silently broaden the filename contract.

JSON, UUID, regexp, offline
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
Retained regexp tests use the runtime-independent futures executor and isolated
in-memory databases, without implicit SQLx fixtures. Existing first-party tests
exercise timeout, rollback, savepoints, Unicode row diagnostics, statement-cache
capacity/eviction, clear and pool closure. No tests operate on user databases.

SQLite ABI symbols, `_sqlx_savepoint_*` SQL identifiers and `file:sqlx-in-memory-*`
connection URI names are intentionally preserved: these are native/SQL identifiers,
not Cargo package names. Diagnostic worker threads use the functional name
`sqlite-worker-*`. There are no active `SQLX_*` environment selectors.
