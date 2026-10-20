import copy
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import upstream_audit as audit

class UpstreamForkAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packages = audit.identities()

    def report(self, identifier=None, name="glib"):
        package = next(p for p in self.packages if p["name"] == name)
        return {
            "database": {"advisory-count": 1000},
            "lockfile": {"dependency-count": 20},
            "settings": {"ignore": [], "informational_warnings": ["unmaintained", "unsound", "notice"]},
            "vulnerabilities": {"list": []},
            "warnings": {"unsound": [{"advisory": {"id": identifier}, "package": package}]} if identifier else {},
        }

    def run_classify(self, report, text=None, status=1):
        with tempfile.TemporaryDirectory() as tmp, patch.object(audit, "identities", return_value=self.packages):
            path = pathlib.Path(tmp) / "Cargo.lock"
            path.write_text(audit.render(self.packages) if text is None else text)
            audit.classify(report, path, status)

    def test_all_original_fork_identities_present(self):
        self.assertEqual(len(self.packages), 20)
        self.assertEqual({p["name"] for p in self.packages}, audit.NATIVE | audit.LEPTOS | {"leptos", "leptos_macro", "unicode-normalization"})

    def test_new_local_dependency_cannot_skip_inventory(self):
        with tempfile.TemporaryDirectory() as tmp:
            lock = pathlib.Path(tmp) / "Cargo.lock"
            lock.write_text('version = 4\n[[package]]\nname = "new-vendored-third-party"\nversion = "1.0.0"\n')
            with self.assertRaisesRegex(ValueError, "uncovered local dependency"):
                audit.validate_coverage([lock], {(p["name"], p["version"]) for p in self.packages})

    def test_exact_reviewed_backport_is_explicitly_classified(self):
        self.run_classify(self.report("RUSTSEC-2024-0429"))

    def test_new_advisory_on_any_local_fork_fails(self):
        for name in ("glib", "tachys", "leptos", "leptos_macro", "unicode-normalization"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "NEW OR UNREVIEWED"):
                self.run_classify(self.report("RUSTSEC-2099-9999", name))

    def test_omitted_or_tampered_shadow_identity_fails(self):
        bad = copy.deepcopy(self.packages)
        bad[0]["checksum"] = "0" * 64
        for text in (audit.render(self.packages[:-1]), audit.render(bad)):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError, "tampered"):
                self.run_classify(self.report(), text, 0)

    def test_incomplete_scan_and_finding_identity_fail(self):
        report = self.report("RUSTSEC-2024-0429")
        report["lockfile"]["dependency-count"] = 18
        with self.assertRaisesRegex(ValueError, "complete upstream inventory"):
            self.run_classify(report)
        report = copy.deepcopy(self.report("RUSTSEC-2024-0429"))
        report["warnings"]["unsound"][0]["package"]["checksum"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "exact verified upstream identity"):
            self.run_classify(report)
        with self.assertRaisesRegex(ValueError, "incomplete upstream scan"):
            self.run_classify(self.report(), status=2)

if __name__ == "__main__":
    unittest.main()
