# Recorded architecture probe: 2026-10-20

One baseline and one in-progress architecture working-tree run; **not a release benchmark**. Baseline source was a fresh `git archive` of `92cd662`; current source includes incremental SQLite projection and journal/read work under concurrent development. Re-run `current.json` after integration for a final-revision measurement.

Environment: Linux x86_64, kernel 6.18.55, rustc 1.99.0 (b940084d7 2026-09-28), unoptimized dev profile, identical standalone Cargo.lock. Both private test pools were on `/tmp`, which is **tmpfs** on this machine. Timings therefore do not measure real disk latency/durability. Other compiler/test jobs were active; no warm-up, repetitions or confidence intervals were collected.

| History versions | Baseline entity writes | Current entity writes | Baseline entity JSON bytes | Current entity JSON bytes |
|---:|---:|---:|---:|---:|
| 1 | 20 | 11 | 4,973 | 3,297 |
| 16 | 202 | 9 | 47,906 | 3,318 |
| 32 | 394 | 9 | 93,730 | 3,318 |
| 64 | 778 | 9 | 185,378 | 3,318 |

These are actual SQLite trigger counts for inserts + updates + deletes, excluding current_state and other I/O. They demonstrate bounded projected entity changes for this one-file workload. Both reports passed exact recovered-state equality, range-byte comparisons, held historical read during save and final integrity checks. The incremental comparison gate passes the current report and intentionally fails the baseline report.

At 64 versions the sampled complete save was 222.711 ms baseline versus 152.561 ms current; restart was 4,180.606 ms versus 5,207.059 ms. This is **not evidence of a startup improvement**, nor a stable percentage performance claim. Retained metadata-object bytes were identical at 33,465,444 across all members. Full snapshots and startup replay remain important growth costs.

The 4 MiB legacy open/read loop was 74.985 ms baseline versus 87.966 ms current for 16 opens. The optimized HTTP verified-chunk range path is not exercised here, so these numbers say nothing about that optimization. Its deterministic byte-count tests belong to the server/read implementation.

Validation commands:

```sh
cargo clippy --locked --offline --manifest-path tools/perf-check/Cargo.toml -- -D warnings
python3 tools/perf-check/compare.py tools/perf-check/results/baseline-92cd662.json tools/perf-check/results/current.json --require-incremental
```

Both passed during this run. See `../PHYSICAL-DISK-VALIDATION.md` for the unexecuted, authorization-gated real hardware plan.
