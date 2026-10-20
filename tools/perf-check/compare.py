#!/usr/bin/env python3
"""Compare probe reports; optional deterministic entity-write regression gate."""
import argparse
import json

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("baseline")
p.add_argument("current")
p.add_argument("--require-incremental", action="store_true")
a = p.parse_args()
with open(a.baseline) as f:
    old = json.load(f)
with open(a.current) as f:
    new = json.load(f)
for key in ("schema_version", "mode", "history_limit", "small_file_bytes", "copies"):
    assert old[key] == new[key], f"incompatible report parameter: {key}"
assert old["mode"] == "Development"
assert not old["physical_redundancy_evidence"] and not new["physical_redundancy_evidence"]
assert all(old["checks"].values()) and all(new["checks"].values())
assert [s["history_versions"] for s in old["samples"]] == [s["history_versions"] for s in new["samples"]]
print("Development simulation; timing ratios are observations, never gates.")
for before, after in zip(old["samples"], new["samples"]):
    def writes(s):
        w = s["sqlite_entity_writes"]
        return sum(w[k] for k in ("inserts", "updates", "deletes"))
    ratios = ", ".join(f"{k}: {after[k]/max(before[k],1):.3f}x" for k in ("save_us", "restart_us", "range_open_read_8_us"))
    print(f"history={after['history_versions']}: entity writes {writes(before)} -> {writes(after)}; {ratios}")
if a.require_incremental:
    # This one-file overwrite workload changes a bounded set of entities. These
    # bounds deliberately allow schema evolution; they are not timing thresholds.
    counts = [sum(s["sqlite_entity_writes"][k] for k in ("inserts", "updates", "deletes")) for s in new["samples"]]
    assert min(counts) > 0, f"missing write instrumentation: {counts}"
    assert max(counts) <= 100, f"publication rewrites too many entities: {counts}"
    assert max(counts[1:]) <= min(counts[1:]) + 10, f"entity writes grow with history: {counts}"
    print("PASS: bounded entity writes and correctness checks")
