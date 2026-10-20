# Architecture workload probe

This is a non-destructive **Development-mode simulation**, never proof of physical disk independence, failure durability, or production throughput. Each run creates a private tempfile pool, three simulated member directories and a system directory. It accepts no target directory or device argument, never opens a raw device, and removes only its own tempfile tree on exit. It does not change an existing ZXS pool.

## Reproduce

Use the repository's Rust 1.99 toolchain:

```sh
cargo run --locked --manifest-path tools/perf-check/Cargo.toml -- 64 > current.json
python3 tools/perf-check/compare.py tools/perf-check/results/baseline-92cd662.json current.json --require-incremental
```

The positional argument is the number of retained one-file saves (4–4096). Default is 64. Run identical counts, dependency lockfiles, build profiles, machines and filesystem conditions for comparisons. For larger sweeps use 64, 256 and 1024, allowing substantial time and space: historical metadata is still retained and startup recovery still scans durable evidence. Timings include fsync and are highly sensitive to shared-machine load, cache warmth and filesystem implementation. They are deliberately not pass/fail gates.

To reproduce the historical baseline, archive commit `92cd662` into a fresh scratch directory and copy `tools/perf-check` from commit `728358018330027e67a4db5d389d32b42cc0c192` (the pre-rename harness) into that archive. Do not use the renamed harness against the old crate paths. Run its manifest against those baseline crates with the historical lockfile and profile. Do not replace the live checkout or copy simulated pool files between revisions. Each report is generated from a newly initialized pool. UUIDs and timestamps vary, so metadata byte sizes are descriptive rather than byte-for-byte golden fixtures.

## Workload and checks

- Retain history for one 1 KiB file, with three copies; sample at 1, N/4, N/2 and N saves.
- Measure a complete create-workspace, stage, commit save, including its durable I/O.
- At sample points install SQLite triggers on `entities` to count actual inserted/updated/deleted rows and entity JSON bytes written by the next save. Trigger bookkeeping adds measurement overhead. It excludes `current_state`, journal and filesystem writes and is not total write amplification.
- Close/reopen the engine at each sample and require exact recovered `State` equality. Reinstall triggers after restart because projection rebuild may replace tables.
- Open and seek to eight 64-byte ranges; assert exact bytes. This uses the public **legacy fully verified `open_content` API**, not the optimized HTTP range endpoint. These timings cannot establish HTTP range-path speedup.
- Write a 4 MiB version; measure first and repeated legacy open/range reads. The first open is not an OS-cache-cold read: the save has just written the bytes.
- Hold a read descriptor and its lease while another thread starts reading that immutable version and the single writer commits a newer version. Require the old bytes throughout and after the save. Scheduler overlap is not guaranteed; this is a held-read correctness exercise, not a server concurrency/latency claim.
- Require a clean final engine integrity check.

The optional comparison gate checks bounded SQLite entity writes (at most 100 per sampled save and at most 10 growth after first save). It does not require a specific speedup. The baseline is expected to fail this incremental-write gate. Keep architecture-specific deterministic unit tests as the more precise contracts: journal scan-count bounds, chunk-reader byte bounds and projection changed-row counts.

## Limits

This does not make the whole save path O(delta): full state copies, comparisons, serialization, snapshots, historical metadata retention and startup replay may still grow. The journal optimization removes repeated hot append scans, not startup replay. Triggers observe SQLite logical rows, not disk sectors or write amplification. Directory-size totals are apparent file lengths, not allocated space. Range HTTP authentication, transport, cancellation and contention are covered by server tests, not this core harness.
