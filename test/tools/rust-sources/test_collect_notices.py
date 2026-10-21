"""Notice collection is explicit, pinned and isolated from maintained sources."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("notice_collector", (Path(__file__).resolve().parents[3] / "tools/rust-sources" / "collect_notices.py"))
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class NoticeReview(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="notice-policy-")
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.root = self.base / "maintained"
        self.root.mkdir()
        self.origins = self.root / "origins.json"
        self.record = dict(origin_directory="old/mini", repository="https://github.com/example/mini", central_license_materials=[],
                           upstream=dict(name="mini", version="1.0.0"), vcs=dict(git=dict(sha1="a" * 40)))
        self.origins.write_text(json.dumps(dict(packages=[self.record])))
        self.output = self.base / "review"
        self.addCleanup(patch.stopall)
        patch.object(collector, "ROOT", self.root).start()
        patch.object(collector, "ORIGINS", self.origins).start()

    def test_default_writes_review_only_and_never_fetches(self):
        before = self.origins.read_bytes()
        with patch.object(collector, "urlopen") as fetch:
            result = collector.review(self.output)
        fetch.assert_not_called()
        self.assertEqual(result[0]["status"], "requires-manual-notice-review")
        self.assertEqual(self.origins.read_bytes(), before)
        self.assertTrue((self.output / "provenance.json").is_file())

    def test_maintained_tree_and_nonempty_output_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "outside the maintained"):
            collector.review(self.root / "new-notices")
        self.output.mkdir()
        (self.output / "existing").write_text("keep")
        with self.assertRaisesRegex(ValueError, "new or empty"):
            collector.review(self.output)
        self.assertEqual((self.output / "existing").read_text(), "keep")

    def test_fetch_uses_exact_original_commit_and_receipts_downloaded_bytes(self):
        content = b"copyright fixture and license text\n" * 10
        expected = f"https://raw.githubusercontent.com/example/mini/{'a' * 40}/LICENSE-MIT"
        class Response(io.BytesIO):
            def geturl(self):
                return expected
        self.output.mkdir()
        with patch.object(collector, "urlopen", return_value=Response(content)) as fetch:
            result = collector.collect(self.record, self.output, fetch=True)
        fetch.assert_called_once_with(expected, timeout=20)
        self.assertEqual(result["sha256"], hashlib.sha256(content).hexdigest())
        self.assertEqual((self.output / result["file"]).read_bytes(), content)

    def test_retained_central_material_is_not_refetched(self):
        self.record["central_license_materials"] = ["compliance/licenses/mini/LICENSE"]
        with patch.object(collector, "urlopen") as fetch:
            self.assertIsNone(collector.collect(self.record, self.output, fetch=True))
        fetch.assert_not_called()


if __name__ == "__main__":
    unittest.main()
