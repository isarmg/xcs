# Streamed typed snapshots and sharded objects: fresh comparison

Development-only observations collected 2026-10-20, not a physical-storage benchmark.

**No overall elapsed-time or peak-RSS win was demonstrated at either size.**
All correctness and bounded logical-write gates passed. See the results below;
concurrent compilation on the shared host prevents causal timing attribution.

## Source and method

- Baseline: `3ec8e1a5f354982856210258558ffd3b4bad8502`.
- Current: `b391b0a48981fa192443841f7c28e34bad7dd503`.
- Both source trees came from fresh `git archive` exports, with their own unchanged
  `tools/perf-check` harnesses and separate new build targets. Dependency lockfiles
  and Cargo profile configuration are identical. Current's metadata classifier
  recognizes both flat and hash-prefix object layouts; baseline's parent-directory
  classifier is valid for its flat layout. No existing checkout, pool, or device was
  changed by the runs.
- Rust `1.99.0 (b940084d7 2026-09-28)`; Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Build: locked, offline, two jobs, dev profile, debug information disabled and
  incremental compilation disabled. Builds finished before measurement.
- Linux, shared runner, workspace overlay filesystem (not tmpfs). About 17 GiB
  free before building and 16 GiB after building. All synthetic pools used a fresh
  temporary directory under the scratch workspace and were removed by the harness.
- Order: baseline 64, current 64, baseline 256, current 256, sequentially. One run
  per revision/count; no averaging, warmup runs, or selection of best results.
- `run-one.py` records wall elapsed seconds and Linux child-process peak RSS in KiB
  for exactly one directly executed harness subprocess. Build resources are excluded.
  Process elapsed includes initialization, all saves, restart samples, the 4 MiB read
  exercise, integrity checking and temporary-pool cleanup. Each run has a 1,200 s
  safety timeout. Harness stage timings are microseconds.

## Reproduction

Use Rust 1.99.0 and a scratch directory with sufficient free disk, outside existing
user data. The harness itself chooses a new disposable pool on every invocation.
The following uses the active checkout only as the archive source and for reading
this runner; it does not modify it. Set `scratch` to a newly created directory on
an appropriate filesystem (the recorded run used workspace overlay, not `/tmp`).

```sh
scratch=$(mktemp -d ./snapshot-probe-XXXXXXXX)
scratch=$(cd "$scratch" && pwd)
mkdir -p "$scratch/baseline" "$scratch/current" "$scratch/tmp"
git archive 3ec8e1a5f354982856210258558ffd3b4bad8502 | tar -x -C "$scratch/baseline"
git archive b391b0a48981fa192443841f7c28e34bad7dd503 | tar -x -C "$scratch/current"
for rev in baseline current; do
  CARGO_TARGET_DIR="$scratch/target-$rev" cargo build --locked --offline -j 2 \
    --manifest-path "$scratch/$rev/tools/perf-check/Cargo.toml"
done
for count in 64 256; do
  for rev in baseline current; do
    python3 tools/perf-check/results/20261020-snapshot-pages/run-one.py "$scratch" "$rev" "$count"
  done
  python3 tools/perf-check/compare.py "$scratch/baseline-$count.json" \
    "$scratch/current-$count.json" --require-incremental
done
```

Offline building requires dependencies already present in the selected Cargo cache.
Do not replace a developer's toolchain/cache settings or reuse another source
revision's target directory to save time.

## Limits

All byte totals are apparent file lengths, not allocated disk space or physical
writes. Metadata counts cover all three simulated copies. Random identifiers and
wall-clock timestamps differ between independent runs; state-page partitions and
retained byte totals are therefore not byte-for-byte comparable fixtures. One
shared-runner observation cannot isolate streaming/sharding effects from variance,
cache warmth, filesystem behavior or other concurrent jobs. Streamed encoding
reduces a particular intermediate representation, not necessarily whole-process
peak memory in this workload. Small 64-byte reads use legacy fully verified
`open_content`, not the optimized HTTP range endpoint. Held-read checking does not
guarantee scheduler overlap. No power-loss, independent-drive failure, physical
durability, production-throughput, or whole-save O(delta) claim follows.

This is a **combined-change comparison** of snapshot encoding and metadata object
layout, not an isolated JSON encoder microbenchmark. Directory counts, mkdir/fsync
counts and per-phase allocation peaks are not recorded by this harness. Sharding
may add directory-creation/sync overhead, but that is an unmeasured hypothesis,
not a demonstrated explanation of the observations. All integrity/recovery checks
remain enabled. The file counts below must not be read as directory counts.

## Results

**Neither fresh pair demonstrates an overall elapsed-time or peak-RSS improvement.**
Current process elapsed and peak RSS were higher at both sizes. Other workers
compiled and ran tests concurrently on the shared host: these are not isolated
causal timings. CPU seconds and RSS remain observations, not performance gates.
The final sampled 256-save time was slightly lower, while final restart time was
higher; that mixed observation does not establish an encoder speedup.

All four runs passed exact recovered-state equality, exact range bytes, held-read
correctness, historical-version readability, and the final integrity check.
Sampled logical SQLite entity mutations were identically 11, 9, 9, 9 at both
sizes/revisions. Both current reports pass `compare.py --require-incremental`;
baseline reports also satisfy the same bounds. The counters are logical entity
rows, not whole-save writes.

| Saves | Revision | Process wall (s) | User CPU (s) | System CPU (s) | Peak RSS (KiB) | First sampled save (ms) | Final sampled save (ms) | Final restart (s) |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 64 | baseline | 19.928 | 9.864 | 3.216 | 33,240 | 155.642 | 189.530 | 4.502 |
| 64 | current | 22.902 | 10.449 | 5.215 | 34,724 | 256.159 | 196.418 | 4.848 |
| 256 | baseline | 126.632 | 90.633 | 13.832 | 65,696 | 176.042 | 304.013 | 45.615 |
| 256 | current | 143.991 | 93.127 | 23.117 | 71,752 | 173.659 | 299.561 | 47.687 |

Final retained-history checkpoint, before the extra large-file/held-read exercises:

| Saves | Revision | All file bytes | Files | Application-log bytes | Metadata object bytes |
|---:|---|---:|---:|---:|---:|
| 64 | baseline | 34,171,032 | 4,262 | 268,212 | 27,342,552 |
| 64 | current | 36,476,541 | 4,265 | 268,212 | 29,217,852 |
| 256 | baseline | 347,238,507 | 16,949 | 1,062,684 | 320,174,478 |
| 256 | current | 301,445,082 | 16,838 | 1,062,684 | 273,549,876 |

All-member metadata breakdown (bytes / files):

| Kind | Baseline 64 | Current 64 | Baseline 256 | Current 256 |
|---|---:|---:|---:|---:|
| candidate | 320,979 / 195 | 320,979 / 195 | 1,279,239 / 771 | 1,279,239 / 771 |
| finalize | 283,254 / 195 | 283,254 / 195 | 1,121,604 / 771 | 1,121,604 / 771 |
| history-manifest | 11,043,327 / 195 | 11,043,327 / 195 | 170,751,231 / 771 | 170,751,231 / 771 |
| publication | 945,840 / 195 | 945,840 / 195 | 3,754,176 / 771 | 3,754,176 / 771 |
| snapshot | 360,435 / 195 | 344,757 / 195 | 2,480,967 / 771 | 2,618,250 / 771 |
| state-page | 14,388,717 / 1,590 | 16,279,695 / 1,593 | 140,787,261 / 6,405 | 94,025,376 / 6,294 |

The large history-manifest totals are unchanged at equal workload size. State-page
bytes vary with random identifier placement, so aggregate metadata differences do
not establish a storage-efficiency regression or improvement. Both per-kind sums
were checked against each sample's total metadata bytes. No real user data or
physical device was used.

Raw `baseline-{64,256}.json` and `current-{64,256}.json` are untouched harness
outputs; `*-process.json` contains resource observations, and `compare-*.txt`
contains the comparison gate output. `SHA256SUMS` pins these results and runner.
