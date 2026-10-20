# Recorded architecture probe: 2026-10-20

One baseline and one integrated architecture run; **not a release benchmark**. Baseline source was a fresh `git archive` of `92cd662`. The current report was regenerated on 2026-10-20 at approximately 03:12 UTC against the integrated architecture source subsequently committed as `401f3756ad6b2a7df64731a692084460fb488f02`, including detached workspace preparation and recovery-checkpoint extensions. Final admission-gate tests were completed separately; this serial core probe is not evidence for that concurrency behavior.

Environment: Linux x86_64, kernel 6.18.55, rustc 1.99.0 (b940084d7 2026-09-28), unoptimized dev profile, identical standalone Cargo.lock. Both private test pools were on `/tmp`, which is **tmpfs** on this machine. Timings therefore do not measure real disk latency/durability. The baseline was collected with other compiler/test jobs active; the current run also occurred in a shared development environment without controlled machine load. No warm-up, repetitions or confidence intervals were collected. `/tmp` was rechecked as tmpfs for the current run.

| History versions | Baseline entity writes | Current entity writes | Baseline entity JSON bytes | Current entity JSON bytes |
|---:|---:|---:|---:|---:|
| 1 | 20 | 11 | 4,973 | 3,297 |
| 16 | 202 | 9 | 47,906 | 3,318 |
| 32 | 394 | 9 | 93,730 | 3,318 |
| 64 | 778 | 9 | 185,378 | 3,318 |

These are actual SQLite trigger counts for inserts + updates + deletes, excluding current_state and other I/O. They demonstrate bounded projected entity changes for this one-file workload. Both reports passed exact recovered-state equality, range-byte comparisons, held historical read during save and final integrity checks. The incremental comparison gate passes the current report and intentionally fails the baseline report.

At 64 versions the sampled complete save was 222.711 ms baseline versus 169.084 ms current; restart was 4,180.606 ms versus 4,890.008 ms. This is **not evidence of a startup improvement**, nor a stable percentage performance claim. Retained metadata-object bytes were identical at 33,465,444 across all members. Full snapshots and startup replay remain important growth costs.

The 4 MiB legacy open/read loop was 74.985 ms baseline versus 91.654 ms current for 16 opens. The current first 4 MiB open/read took 6.181 ms; the payload had just been written, so this is not an OS-cache-cold measurement. The optimized HTTP verified-chunk range path and detached HTTP/SMB workspace commit boundary are not exercised here: the harness calls synchronous core `commit_workspace` and legacy `open_content`. These numbers say nothing about those optimizations. Deterministic range byte-count and detached-commit boundary tests belong to the server/core test suites.

Validation commands:

```sh
cargo run --locked --offline --manifest-path tools/perf-check/Cargo.toml -- 64 > tools/perf-check/results/current.json
python3 tools/perf-check/compare.py tools/perf-check/results/baseline-92cd662.json tools/perf-check/results/current.json --require-incremental
```

Both commands passed for the regenerated current report. All five reported correctness checks passed. The earlier probe clippy result is not a fresh integrated-tree clippy result. The 64-save comparison and separate 256-save growth run below were completed; a 1024-save sweep, controlled actual HTTP contention/latency measurements and physical fault tests remain unexecuted. See `../PHYSICAL-DISK-VALIDATION.md` for the unexecuted, authorization-gated real hardware plan.

## Additional 256-save growth run

`current-256.json` records one separate integrated architecture run for source revision `401f3756ad6b2a7df64731a692084460fb488f02`, completed on 2026-10-20 at approximately 03:16 UTC with the same toolchain, dev profile and disposable tmpfs setup. It passed exact recovered-state equality, range bytes, held reads during save, historical-version reads after save and final integrity. A direct report check confirmed positive entity-write counts, at most 100 writes per sample and at most 10 writes of growth after the first sample, using the same bounds as the comparison gate. No unequal-history baseline timing comparison was made.

| History versions | Entity writes | Entity JSON bytes written | Observed save (ms) | Observed restart (ms) | Metadata-object bytes, all members |
|---:|---:|---:|---:|---:|---:|
| 1 | 11 | 3,297 | 28.756 | 35.260 | 85,146 |
| 64 | 9 | 3,318 | 132.361 | 6,317.816 | 33,465,444 |
| 128 | 9 | 3,325 | 424.630 | 19,977.828 | 128,006,145 |
| 256 | 9 | 3,325 | 537.765 | 65,082.993 | 453,137,022 |

This extends evidence of bounded SQLite entity changes to 256 retained saves while exposing substantial remaining metadata and restart growth: the 256-save restart took approximately **65.083 seconds**, and retained metadata objects occupied **453,137,022 bytes** across the three simulated members. Bounded index row updates do not make overall restart or retained metadata costs bounded. Timings are single observations under uncontrolled shared load, not throughput or latency guarantees. The same synchronous-core/legacy-read and non-physical-storage limitations apply.

Reproduce separately from the 64-save comparison:

```sh
cargo run --locked --offline --manifest-path tools/perf-check/Cargo.toml -- 256 > tools/perf-check/results/current-256.json
```
