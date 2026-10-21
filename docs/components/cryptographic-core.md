# Cryptographic core

One no_std crate contains common types, hash/MAC traits and optional block buffering. Algorithms remain in their separate components.

Sources: RustCrypto digest 0.11.3 (primary), block-buffer 0.12.1 and crypto-common 0.2.2 (merged implementations, not retired code). Original versions, source snapshots, documentation and MIT/Apache-2.0 licenses remain preserved.

The buffer module contains its unchanged existing unsafe operations; hash/common modules forbid unsafe code. Features retain alloc, block-api, mac, oid and zeroize. The unified zeroize feature enables the prior buffer zeroization capability and hybrid-array's optional zeroization support.

## Maintained boundaries

The production gate checks empty features, alloc, block-api, mac, oid, zeroize, dev, and their supported union (plus true --all-features) on Linux amd64 and WebAssembly. It also runs the original block-buffer tests and the retained hash serialization vector, including the exported serialization macro, without changing the archived upstream data. The zeroize feature additionally supplies hybrid-array zeroization implementations; it does not add automatic array destruction or a std dependency.

The new package does not advertise the unused rand_core/getrandom features or dangling rand_core reexport: their upstream providers were absent from the repository. The dev feature now works without an external fixture parser and retains fixed-hash, XOF, MAC and serialization helpers. The unused new_test/new_mac_test binary-loader macros and blobby reexport are removed; the stale hash_rt_outsize_serialization_test macro is also removed because its VariableOutput trait was absent from digest 0.11. Their exact original definitions remain in the authenticated source archive. No algorithm, runtime-variable BLAKE2 API, reset behavior or I/O capability is removed. Maintained probes reject the removed feature names and exercise the supported dev exports.

The three original source and test inventories are authenticated in compliance/cryptographic-core-source-archive.json.gz. Their original MIT and Apache-2.0 license materials remain under compliance/licenses/upstream. The earlier digest 0.10.7 implementation adapted into BLAKE2 remains a separate merged origin; this consolidation does not retire or relabel it.
