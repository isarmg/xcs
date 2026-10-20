"""No local-source or known-finding exemptions may weaken the advisory gate."""
from contextlib import redirect_stdout
import io
from pathlib import Path
import tempfile
import unittest

import audit


class AdvisoryGate(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="zxs-advisory-test-")
        self.addCleanup(temporary.cleanup)
        self.lock = Path(temporary.name) / "Cargo.lock"
        self.lock.write_text(audit.render([dict(name="mini", version="1.0.0", source=audit.manage.REGISTRY, checksum="a" * 64)]))
        self.report = dict(database={"advisory-count": 100}, lockfile={"dependency-count": 1},
                           settings={"ignore": [], "informational_warnings": ["unmaintained", "unsound", "notice"],
                                     "target_arch": [], "target_os": []},
                           vulnerabilities={"list": [], "found": False, "count": 0}, warnings={})

    def check(self, status=0):
        with redirect_stdout(io.StringIO()):
            audit.classify(self.report, self.lock, status)

    def test_clean_complete_report_passes(self):
        self.check()

    def test_all_warning_categories_fail_including_yanked_and_unmaintained(self):
        for category in ("unmaintained", "yanked", "notice", "unsound"):
            with self.subTest(category=category):
                self.report["warnings"] = {category: [{"package": {"name": "mini"}, "advisory": None}]}
                with self.assertRaisesRegex(ValueError, "unresolved advisory"):
                    self.check(1)

    def test_ignore_and_target_filters_fail_even_without_findings(self):
        for field, value in (("ignore", ["RUSTSEC-TEST"]), ("target_arch", ["x86_64"]), ("target_os", ["linux"])):
            with self.subTest(field=field):
                self.report["settings"][field] = value
                with self.assertRaisesRegex(ValueError, "exclusions|ignores"):
                    self.check()
                self.report["settings"][field] = []

    def test_severity_threshold_cannot_hide_lower_cvss_findings(self):
        for threshold in ("low", "medium", "high", "critical", 0):
            with self.subTest(threshold=threshold):
                self.report["settings"]["severity"] = threshold
                with self.assertRaisesRegex(ValueError, "severity"):
                    self.check()
        self.report["settings"]["severity"] = None
        self.check()

    def test_missing_package_or_database_coverage_fails(self):
        self.report["lockfile"]["dependency-count"] = 0
        with self.assertRaisesRegex(ValueError, "coverage"):
            self.check()
        self.report["lockfile"]["dependency-count"] = 1
        self.report["database"]["advisory-count"] = 0
        with self.assertRaisesRegex(ValueError, "database"):
            self.check()

    def test_scanner_failure_without_findings_fails(self):
        for status in (1, 2):
            with self.subTest(status=status), self.assertRaises(ValueError):
                self.check(status)

    def test_all_local_packages_keep_original_identities_after_path_conversion(self):
        origin = dict(name="upstream", version="2.0.0", archive_sha256="a" * 64)
        catalog = {"packages": {"tools/web-bindings": {"upstream": origin},
                                "src/foundation/ordinary": {"upstream": dict(origin, name="ordinary")}}}
        packages = audit.identities(catalog)
        self.assertEqual(len(packages), 2)
        self.assertEqual({p["name"] for p in packages}, {"ordinary", "upstream"})

    def test_merged_code_original_identities_are_scanned_without_independent_package(self):
        catalog = {"packages": {}, "merges": [
            {"upstream": dict(name="unicode-normalization", version="0.1.24", archive_sha256="a" * 64)},
            {"upstream": dict(name="leptos", version="0.8.20", archive_sha256="b" * 64)}]}
        self.assertEqual({p["name"] for p in audit.identities(catalog)}, {"unicode-normalization", "leptos"})

    def test_conflicting_original_checksums_fail(self):
        catalog = {"packages": {}, "upstream_identities": [
            dict(name="mini", version="1.0.0", archive_sha256="a" * 64),
            dict(name="mini", version="1.0.0", archive_sha256="b" * 64)]}
        with self.assertRaisesRegex(ValueError, "conflicting upstream"):
            audit.identities(catalog)

    def test_retired_origins_are_preserved_but_only_present_source_is_scanned(self):
        active = dict(name="retained", version="1.0.0", archive_sha256="a" * 64)
        removed = dict(name="removed", version="2.0.0", archive_sha256="b" * 64)
        catalog = dict(schema=3, packages={"src/foundation/retained": dict(upstream=active)},
                       merges=[], retired={"src/foundation/removed": dict(upstream=removed)},
                       upstream_identities=[active, removed])
        self.assertEqual([package["name"] for package in audit.identities(catalog)], ["retained"])

    def test_upgrades_scan_current_identity_without_historical_or_local_exemptions(self):
        old = dict(name="mini", version="1.0.0", archive_sha256="a" * 64)
        current = dict(name="mini", version="1.1.0", archive_sha256="b" * 64)
        merged = dict(name="merged", version="2.0.0", archive_sha256="c" * 64)
        catalog = dict(schema=4, packages={"src/foundation/mini": dict(name="local-name", upstream=current)},
                       merges=[dict(upstream=merged)], upstream_identities=[old, current, merged],
                       superseded={"old/mini": dict(package=dict(upstream=old), successor_origin_directory="new/mini")})
        self.assertEqual([(package["name"], package["version"]) for package in audit.identities(catalog)],
                         [("merged", "2.0.0"), ("mini", "1.1.0")])

    def test_upgraded_source_scans_new_identity_without_advisory_exemption(self):
        old = dict(name="rustls-webpki", version="0.103.15", archive_sha256="a" * 64)
        new = dict(name="rustls-webpki", version="0.103.16", archive_sha256="b" * 64)
        catalog = dict(schema=4, packages={"src/foundation/rustls-webpki": dict(upstream=new)},
                       merges=[], superseded={"old": {"package": dict(upstream=old)}},
                       upstream_identities=[old, new])
        self.assertEqual([(package["name"], package["version"]) for package in audit.identities(catalog)],
                         [("rustls-webpki", "0.103.16")])
        # A future finding against the upgraded identity still fails normally.
        self.report["vulnerabilities"] = dict(found=True, count=1, list=[
            {"package": {"name": "rustls-webpki", "version": "0.103.16"},
             "advisory": {"id": "RUSTSEC-TEST"}}])
        with self.assertRaisesRegex(ValueError, "unresolved advisory"):
            self.check(1)


if __name__ == "__main__":
    unittest.main()
