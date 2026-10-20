import copy
import pathlib
import unittest
from classify_audit import classify

BASE = {"database": {"advisory-count": 1},
        "settings": {"ignore": [], "informational_warnings": ["unmaintained", "unsound", "notice"]},
        "vulnerabilities": {"list": []}, "warnings": {}}

class AuditClassificationTests(unittest.TestCase):
    def test_clean_report(self):
        classify(BASE, pathlib.Path("Cargo.lock"), 0)

    def test_fail_closed(self):
        reports = []
        bad = copy.deepcopy(BASE)
        bad["settings"]["ignore"] = ["RUSTSEC-2024-0429"]
        reports.append((bad, 0))
        bad = copy.deepcopy(BASE)
        bad["settings"]["informational_warnings"] = []
        reports.append((bad, 0))
        reports.append((BASE, 2))
        reports.append((BASE, 1))
        for identifier, source in [("RUSTSEC-9999-9999", None),
                                   ("RUSTSEC-2024-0429", "registry+https://github.com/rust-lang/crates.io-index")]:
            bad = copy.deepcopy(BASE)
            bad["warnings"]["unsound"] = [{"advisory": {"id": identifier},
                "package": {"name": "glib", "version": "0.18.5", "source": source}}]
            reports.append((bad, 1))
        for report, status in reports:
            with self.subTest(report=report, status=status), self.assertRaises(ValueError):
                classify(report, pathlib.Path("Cargo.lock"), status)

if __name__ == "__main__":
    unittest.main()
