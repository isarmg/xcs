"""Focused report-shape checks; never run a storage workload."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECKS = (
    "restart_state_equality", "range_bytes_correct", "held_read_during_save",
    "historical_version_after_save", "integrity_check",
)


def report():
    return {
        "mode": "Development", "history_limit": 4, "small_file_bytes": 1024,
        "copies": 3, "physical_redundancy_evidence": False,
        "checks": dict.fromkeys(CHECKS, True),
        "samples": [
            {"history_versions": n, "save_us": 1, "restart_us": 1,
             "range_open_read_8_us": 1, "index_sqlite_bytes": 16384,
             "sqlite_history_writes": {"inserts": 7, "updates": 2, "deletes": 0}}
            for n in (1, 2, 4)
        ],
    }


class CompareTests(unittest.TestCase):
    def compare(self, before, after, optimized=False):
        return self.compare_json(json.dumps(before), json.dumps(after), optimized)

    def compare_json(self, before, after, optimized=False):
        with tempfile.TemporaryDirectory(prefix="zxs-compare-") as folder:
            paths = [Path(folder) / name for name in ("before.json", "after.json")]
            for path, value in zip(paths, (before, after)):
                path.write_text(value)
            return subprocess.run(
                [sys.executable, *(["-O"] if optimized else []),
                 str((Path(__file__).resolve().parents[3] / "tools/perf-check" / "compare.py")),
                 *(str(path) for path in paths), "--require-incremental"],
                capture_output=True, text=True, check=False,
            )

    def test_current_reports_need_no_numbered_format(self):
        self.assertEqual(self.compare(report(), report()).returncode, 0)

    def test_unrelated_metadata_does_not_dispatch_report_formats(self):
        before = report()
        before["archived_probe_note"] = "unchanged evidence"
        self.assertEqual(self.compare(before, report()).returncode, 0)

    def test_workload_mismatch_is_rejected(self):
        after = report()
        after["copies"] = 2
        self.assertNotEqual(self.compare(report(), after).returncode, 0)

    def test_missing_or_false_correctness_checks_are_rejected(self):
        for key in CHECKS:
            for value in (False, None):
                after = report()
                if value is None:
                    del after["checks"][key]
                else:
                    after["checks"][key] = value
                self.assertNotEqual(self.compare(report(), after).returncode, 0)

    def test_missing_samples_are_rejected(self):
        before = report()
        before["samples"] = []
        self.assertNotEqual(self.compare(before, copy.deepcopy(before)).returncode, 0)


    def test_optimization_cannot_disable_the_acceptance_gate(self):
        self.assertEqual(self.compare(report(), report(), optimized=True).returncode, 0)
        for mutate in (
            lambda value: value["checks"].update(integrity_check=False),
            lambda value: value.update(copies=99),
            lambda value: value["samples"][-1]["sqlite_history_writes"].update(inserts=10000),
        ):
            after = report()
            mutate(after)
            result = self.compare(report(), after, optimized=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("PASS:", result.stdout)

    def test_invalid_counters_and_timings_cannot_hide_regressions(self):
        for value in (-1, True, 1.5, float("nan"), float("inf"), "1"):
            for key in ("inserts", "updates", "deletes"):
                after = report()
                after["samples"][-1]["sqlite_history_writes"][key] = value
                self.assertNotEqual(self.compare(report(), after).returncode, 0)
            for key in ("save_us", "restart_us", "range_open_read_8_us", "index_sqlite_bytes"):
                after = report()
                after["samples"][-1][key] = value
                self.assertNotEqual(self.compare(report(), after).returncode, 0)

    def test_truncated_or_repeated_history_cannot_claim_bounded_growth(self):
        for histories in ((1,), (1, 4), (1, 1, 4), (4, 2, 1), (1, 2, 5)):
            before = report()
            before["samples"] = [dict(before["samples"][0], history_versions=n) for n in histories]
            self.assertNotEqual(self.compare(before, copy.deepcopy(before)).returncode, 0)

    def test_missing_instrumentation_is_rejected(self):
        for key in ("inserts", "updates", "deletes"):
            after = report()
            del after["samples"][-1]["sqlite_history_writes"][key]
            self.assertNotEqual(self.compare(report(), after).returncode, 0)
        for key in ("sqlite_history_writes", "index_sqlite_bytes"):
            after = report()
            del after["samples"][-1][key]
            self.assertNotEqual(self.compare(report(), after).returncode, 0)
        after = report()
        after["samples"][-1]["sqlite_history_writes"] = dict.fromkeys(("inserts", "updates", "deletes"), 0)
        self.assertNotEqual(self.compare(report(), after).returncode, 0)

    def test_history_growth_and_excessive_writes_are_rejected(self):
        for inserts in (20, 101):
            after = report()
            after["samples"][-1]["sqlite_history_writes"]["inserts"] = inserts
            self.assertNotEqual(self.compare(report(), after).returncode, 0)

    def test_strict_json_rejects_duplicates_nonfinite_and_trailing_data(self):
        valid = json.dumps(report())
        for invalid in (
            valid.replace('"inserts": 7', '"inserts": 7, "inserts": 1', 1),
            valid + " {}",
            *(valid[:-1] + ', "extra": ' + value + '}'
              for value in ("NaN", "Infinity", "-Infinity", "1e400")),
        ):
            for optimized in (False, True):
                result = self.compare_json(valid, invalid, optimized)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("PASS:", result.stdout)

    def test_output_uses_matching_history_and_index_metrics(self):
        after = report()
        after["samples"][-1]["index_sqlite_bytes"] = 8192
        after["samples"][-1]["restart_us"] = 999
        result = self.compare(report(), after)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("history writes 9 -> 9", result.stdout)
        self.assertIn("index.sqlite bytes 16384 -> 8192", result.stdout)
        self.assertIn("restart_us: 999.000x", result.stdout)
        self.assertIn("exclude the fixed current_state marker", result.stdout)

    def test_archived_entity_reports_are_not_comparable(self):
        root = Path(__file__).resolve().parents[3] / "tools/perf-check/results"
        before = json.loads((root / "growth-baseline64.json").read_text())
        after = json.loads((root / "growth-optimized64.json").read_text())
        self.assertNotEqual(self.compare(before, after).returncode, 0)
        after = report()
        for sample in after["samples"]:
            sample["sqlite_entity_writes"] = sample.pop("sqlite_history_writes")
        self.assertNotEqual(self.compare(report(), after).returncode, 0)


if __name__ == "__main__":
    unittest.main()
