# Second retained-history optimization probe (2026-10-20)

The before source is `1abbfbbc790dcc0857bc58ada26da966595a56b1`.
The after source is the accompanying second-pass implementation: stable snapshot
pages, recovery-local page repair reuse, memoized container projections and fused
canonical encoding/hash. New per-object-kind counters attribute retained bytes.
The baseline64 probe ran from a fresh `git archive` of the before revision with
only this measurement addition. The 256 before report is the retained prior run.

All five correctness checks and the bounded SQLite update gate passed at both
sizes. Sampled index changes remain 11 rows at the first save and 9 thereafter.
No old history is removed. File identifiers are random, so page partitions and
retained object byte counts can vary across independent runs.

| Retained versions | Before metadata bytes | After metadata bytes | Before restart (s) | After restart (s) | Before save (ms) | After save (ms) |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 33,465,444 | 31,220,844 | 4.703 | 3.537 | 159.204 | 100.140 |
| 256 | 453,137,022 | 305,464,122 | 65.083 | 36.139 | 537.765 | 292.799 |

At 256 versions, the after metadata is dominated by full history manifests
(170,751,231 bytes), followed by state pages (111,427,164 bytes), across all
three simulated members. The manifest representation and portable package hash
binding remain unchanged. Snapshot roots retain events; these costs still grow.
This is a partial reduction of growth costs, not constant-cost storage/recovery.

These are single unoptimized Development-mode runs on shared Linux x86_64 with
Rust 1.99.0 and disposable **tmpfs** member directories. Builds/tests were also
active. Elapsed times are observations, not speedup guarantees, real-disk
latencies, or gates. No power-loss or independent-drive failure was tested.
The harness uses synchronous core commits and legacy full-verification reads;
it does not measure HTTP range throughput or detached server contention.

Files:
- `growth-baseline64.json`: isolated old-source run with per-kind counters.
- `growth-optimized64.json`: second-pass 64-save run.
- `growth-optimized256.json`: second-pass 256-save run.
- `current-256.json`: retained before report for equal-size comparison.

Validation:

```sh
python3 tools/perf-check/compare.py tools/perf-check/results/growth-baseline64.json tools/perf-check/results/growth-optimized64.json --require-incremental
python3 tools/perf-check/compare.py tools/perf-check/results/current-256.json tools/perf-check/results/growth-optimized256.json --require-incremental
```
