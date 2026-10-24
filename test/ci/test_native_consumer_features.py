"""Consumer capability checks must inspect real, non-unified production/dev graphs."""
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
GATE = runpy.run_path(str(ROOT / ".github/scripts/check-native-consumer-features.py"))
MINIMAL = "rustix v1.1.5 (/source/rustix)|alloc,default,fs,process,std\n"
HTTP = "http-routing v0.8.9 (/source/http-routing)|http1,json,query,tokio,tower-log,tracing\n"


class NativeConsumerFeaturesTests(unittest.TestCase):
    def test_required_filesystem_and_query_capabilities_remain(self):
        for consumer in GATE["CONSUMERS"]:
            for edges in GATE["EDGES"]:
                GATE["validate"](MINIMAL + HTTP, consumer, edges)
        GATE["validate"](MINIMAL, "storage-client", "normal,build")

    def test_mount_and_form_regressions_are_rejected_in_every_occurrence(self):
        for extra in (MINIMAL.replace("fs,", "mount,fs,"), HTTP.replace("json,", "form,json,")):
            with self.subTest(extra=extra):
                with self.assertRaisesRegex(AssertionError, "unexpected .* capabilities"):
                    GATE["validate"](MINIMAL + HTTP + extra, "storage-server", "normal,build,dev")

    def test_missing_graph_entries_or_required_features_cannot_pass(self):
        for output in ("", HTTP, MINIMAL, MINIMAL + HTTP.replace("query,", ""),
                       MINIMAL.replace("process,", "") + HTTP):
            with self.subTest(output=output):
                with self.assertRaises(AssertionError):
                    GATE["validate"](output, "storage-server", "normal,build")
        with self.assertRaisesRegex(AssertionError, "missing expected graph entries"):
            GATE["validate"](MINIMAL, "storage-rescue", "normal,build,dev")

    def test_each_consumer_and_dependency_scope_is_selected_separately(self):
        result = subprocess.CompletedProcess([], 0, MINIMAL + HTTP)
        with mock.patch.object(GATE["subprocess"], "run", return_value=result) as run:
            GATE["check"](ROOT)
        expected = [mock.call(
            ["cargo", "tree", "--locked", "--offline", "-p", consumer,
             "--target", GATE["TARGET"], "--edges", edges, "--prefix", "none", "--format", "{p}|{f}"],
            cwd=ROOT, text=True, capture_output=True, check=True, timeout=120,
        ) for consumer in GATE["CONSUMERS"] for edges in GATE["EDGES"]]
        self.assertEqual(run.call_args_list, expected)

    def test_cargo_failure_is_not_hidden(self):
        with mock.patch.object(GATE["subprocess"], "run",
                               side_effect=subprocess.CalledProcessError(101, ["cargo"])):
            with self.assertRaises(subprocess.CalledProcessError):
                GATE["check"](ROOT)


if __name__ == "__main__":
    unittest.main()
