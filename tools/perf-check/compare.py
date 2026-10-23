#!/usr/bin/env python3
"""Compare probe reports; optional deterministic entity-write regression gate."""
import argparse
import json

REQUIRED_CHECKS = (
    "restart_state_equality", "range_bytes_correct", "held_read_during_save",
    "historical_version_after_save", "integrity_check",
)
TIMINGS = ("save_us", "restart_us", "range_open_read_8_us")
WRITES = ("inserts", "updates", "deletes")


def require(condition, message):
    # This is an external-input acceptance gate, not a debug assertion. Python
    # optimization must never turn invalid evidence into a successful report.
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate report field: {key}")
        result[key] = value
    return result


def invalid_constant(value):
    raise ValueError(f"invalid JSON constant: {value}")


def integer(value, label, minimum=0):
    require(type(value) is int and value >= minimum, f"invalid {label}: expected integer >= {minimum}")
    return value


def writes(sample):
    return sum(sample["sqlite_entity_writes"][key] for key in WRITES)


def validate(report):
    require(isinstance(report, dict), "report must be an object")
    require(report["mode"] == "Development", "only Development simulation reports are comparable")
    require(report["physical_redundancy_evidence"] is False, "simulation is not physical redundancy evidence")
    for key in ("history_limit", "small_file_bytes", "copies"):
        integer(report[key], key, 1)
    require(isinstance(report["checks"], dict), "missing correctness checks")
    require(all(report["checks"].get(key) is True for key in REQUIRED_CHECKS), "correctness check failed")
    samples = report["samples"]
    require(isinstance(samples, list) and samples, "missing samples")
    previous = 0
    for sample in samples:
        require(isinstance(sample, dict), "sample must be an object")
        history = integer(sample["history_versions"], "history_versions", 1)
        require(previous < history <= report["history_limit"], "sample history must increase within history_limit")
        previous = history
        for key in TIMINGS:
            integer(sample[key], key)
        require(isinstance(sample["sqlite_entity_writes"], dict), "missing write instrumentation")
        for key in WRITES:
            integer(sample["sqlite_entity_writes"][key], key)


def compare(old, new, require_incremental=False):
    for report in (old, new):
        validate(report)
    for key in ("mode", "history_limit", "small_file_bytes", "copies"):
        require(old[key] == new[key], f"incompatible report parameter: {key}")
    history = [sample["history_versions"] for sample in new["samples"]]
    require([sample["history_versions"] for sample in old["samples"]] == history,
            "incompatible sample histories")
    if require_incremental:
        # Observe multiple non-initial history sizes before claiming that writes
        # do not grow with history. The probe always emits these checkpoints.
        limit = new["history_limit"]
        require(limit >= 4 and history == sorted({1, limit // 4, limit // 2, limit}),
                "incremental gate requires the complete history checkpoints")
        counts = [writes(sample) for sample in new["samples"]]
        require(min(counts) > 0, f"missing write instrumentation: {counts}")
        require(max(counts) <= 100, f"publication rewrites too many entities: {counts}")
        require(max(counts[1:]) <= min(counts[1:]) + 10, f"entity writes grow with history: {counts}")
    print("Development simulation; timing ratios are observations, never gates.")
    for before, after in zip(old["samples"], new["samples"]):
        ratios = ", ".join(f"{key}: {after[key] / max(before[key], 1):.3f}x" for key in TIMINGS)
        print(f"history={after['history_versions']}: entity writes {writes(before)} -> {writes(after)}; {ratios}")
    if require_incremental:
        print("PASS: bounded entity writes and correctness checks")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline")
    parser.add_argument("current")
    parser.add_argument("--require-incremental", action="store_true")
    args = parser.parse_args()
    try:
        reports = []
        for path in (args.baseline, args.current):
            with open(path, encoding="utf-8") as file:
                reports.append(json.load(file, object_pairs_hook=unique_object, parse_constant=invalid_constant))
        compare(*reports, args.require_incremental)
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"Performance comparison failed: {error}\n")


if __name__ == "__main__":
    main()
