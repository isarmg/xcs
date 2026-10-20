"""Single-workspace, local-edge and immutable-origin policy regressions."""
from contextlib import redirect_stdout
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("source_manager", Path(__file__).with_name("manage.py"))
manager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manager)


class SourcePolicy(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="source-policy-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for directory in (".cargo", "docs", "tools/rust-sources", "src/app", "src/merged", "src/foundation/mini",
                          "compliance/empty-registry", "compliance/provenance/packages/mini", "compliance/licenses/upstream/mini"):
            (self.root / directory).mkdir(parents=True)
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers = ["src/app"]\nexclude = ["src/foundation/mini"]\n')
        (self.root / "src/app/Cargo.toml").write_text(
            '[package]\nname = "app"\nversion = "1.0.0"\n[dependencies]\nmini = { path = "../foundation/mini" }\n')
        (self.root / ".cargo/config.toml").write_text(
            '[net]\noffline = true\n[source.crates-io]\nreplace-with = "empty"\n[source.empty]\ndirectory = "compliance/empty-registry"\n')
        (self.root / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "app"\nversion = "1.0.0"\n[[package]]\nname = "mini"\nversion = "1.0.0"\n')
        self.policy = dict(schema=2, workspaces=[dict(manifest="Cargo.toml", targets=["x86_64-unknown-linux-gnu"], purpose="fixture")],
                           profiles=[dict(id="native", manifests=["src/app/Cargo.toml"], targets=["x86_64-unknown-linux-gnu"], role="runtime", purpose="fixture")],
                           integrated_sources=[])
        self.write_json(manager.POLICY, self.policy)
        self.source = self.root / "src/foundation/mini"
        (self.source / "Cargo.toml").write_text('[package]\nname = "mini"\nversion = "1.0.0"\n')
        (self.source / "lib.rs").write_text("pub fn value() -> u8 { 1 }\n")
        (self.root / "src/merged/algorithm.rs").write_text("pub fn normalized() {}\n")
        (self.root / "compliance/provenance/packages/mini/Cargo.toml.orig.txt").write_text("original manifest bytes\n")
        (self.root / "compliance/licenses/upstream/mini/LICENSE").write_text("original copyright bytes\n")
        self.upstream = dict(name="mini", version="1.0.0", archive_url="https://static.crates.io/crates/mini/mini-1.0.0.crate", archive_sha256="a" * 64)
        self.merged_upstream = dict(name="normalization", version="2.0.0", archive_url="https://static.crates.io/crates/normalization/normalization-2.0.0.crate", archive_sha256="b" * 64)
        self.origin_records = [self.origin("old/mini", self.upstream), self.origin("old/normalization", self.merged_upstream)]
        self.write_json(manager.ORIGINS, dict(packages=self.origin_records))
        self.catalog = dict(packages={"src/foundation/mini": dict(name="mini", version="1.0.0", directory="src/foundation/mini",
                            repository="https://example.test/upstream", description="fixture", license="MIT", upstream=self.upstream,
                            origin_directory="old/mini", status="integrated", changes="path dependency", merged_into="foundation")},
                            merges=[dict(directory="src/merged", upstream=self.merged_upstream, origin_directory="old/normalization")])
        self.write_json(manager.CATALOG, self.catalog)
        self.metadata = dict(packages=[dict(id=name, name=name, manifest_path=str(path / "Cargo.toml"), source=None, targets=[{"kind": ["lib"]}])
                                       for name, path in (("app", self.root / "src/app"), ("mini", self.source))], workspace_members=["app"],
                             resolve={"nodes": [dict(id="app", features=[], deps=[dict(pkg="mini", dep_kinds=[{"kind": None}])]),
                                                dict(id="mini", features=["enabled"], deps=[])]})
        self.refresh()

    def origin(self, directory, upstream):
        return dict(origin_directory=directory, upstream=upstream,
                    original_files_sha256={"lib.rs": "c" * 64},
                    central_license_materials=["compliance/licenses/upstream/mini/LICENSE"],
                    central_original_metadata=["compliance/provenance/packages/mini/Cargo.toml.orig.txt"],
                    central_original_documentation=[])

    def write_json(self, relative, value):
        (self.root / relative).write_text(json.dumps(value, ensure_ascii=False))

    def save_catalog(self):
        self.write_json(manager.CATALOG, self.catalog)
        (self.root / manager.DOC).write_text(manager.document(self.catalog, self.policy))

    def refresh(self):
        with patch.object(manager, "cargo_metadata", return_value=self.metadata), redirect_stdout(io.StringIO()):
            manager.refresh(self.root)
        self.catalog = manager.read_json(self.root / manager.CATALOG)

    def verify(self, **options):
        with redirect_stdout(io.StringIO()):
            manager.verify(self.root, **options)

    def test_complete_fixture_passes_without_registry_cache(self):
        self.verify()
        with patch.object(manager, "cargo_metadata", return_value=self.metadata):
            self.verify(metadata=True)

    def test_verification_member_is_explicit_and_non_default(self):
        self.policy["verification_members"] = ["src/foundation/mini"]
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["src/app", "src/foundation/mini"]\n'
            'default-members = ["src/app"]\n')
        manager.structure(self.root, self.policy)

    def test_rejects_unregistered_or_default_verification_member(self):
        self.policy["verification_members"] = ["src/foundation/mini"]
        for members, defaults in (
            ('["src/app"]', '["src/app"]'),
            ('["src/app", "src/foundation/mini"]', '["src/app", "src/foundation/mini"]'),
            ('["src/app", "src/foundation/mini"]', '[]'),
        ):
            with self.subTest(members=members, defaults=defaults):
                (self.root / "Cargo.toml").write_text(
                    f'[workspace]\nmembers = {members}\ndefault-members = {defaults}\n')
                with self.assertRaisesRegex(ValueError, 'explicit non-default'):
                    manager.structure(self.root, self.policy)

    def test_rejects_unrecorded_foundation_workspace_member(self):
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["src/app", "src/foundation/mini"]\n'
            'default-members = ["src/app"]\n')
        with self.assertRaisesRegex(ValueError, 'explicit non-default'):
            manager.structure(self.root, self.policy)

    def test_native_target_policy_rejects_arm64_even_after_refresh(self):
        self.policy["workspaces"][0]["targets"].append("aarch64-unknown-linux-gnu")
        self.write_json(manager.POLICY, self.policy)
        with self.assertRaisesRegex(ValueError, "unsupported workspace target"):
            self.refresh()

    def test_native_profile_cannot_advertise_web_or_other_native_targets(self):
        for target in (manager.WEB_TARGET, "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-musl"):
            with self.subTest(target=target):
                self.policy["profiles"][0]["targets"] = [target]
                with self.assertRaisesRegex(ValueError, "unsupported profile target"):
                    manager.verify_target_policy(self.policy)

    def test_web_target_is_separate_from_native_platform_support(self):
        self.policy["workspaces"][0]["targets"].append(manager.WEB_TARGET)
        self.policy["profiles"].append(dict(id="web", targets=[manager.WEB_TARGET]))
        manager.verify_target_policy(self.policy)
        self.policy["profiles"][-1]["targets"] = [manager.NATIVE_TARGET]
        with self.assertRaisesRegex(ValueError, "unsupported profile target"):
            manager.verify_target_policy(self.policy)

    def test_target_coverage_cannot_advertise_unused_platforms(self):
        self.policy["workspaces"][0]["targets"].append(manager.WEB_TARGET)
        with self.assertRaisesRegex(ValueError, "workspace/profile target coverage drift"):
            manager.verify_target_policy(self.policy)

    def test_profile_target_cannot_be_duplicated(self):
        self.policy["profiles"][0]["targets"] *= 2
        with self.assertRaisesRegex(ValueError, "unsupported profile target"):
            manager.verify_target_policy(self.policy)

    def test_refresh_descriptions_is_idempotent_and_preserves_original_receipts(self):
        package = self.catalog["packages"]["src/foundation/mini"]
        package["changes"] = "See crates/zxs-naming/README.md."
        (self.root / "docs/components").mkdir()
        (self.root / "docs/components/naming.md").write_text("current documentation\n")
        self.save_catalog()
        original = self.catalog["origin_records_sha256"]
        with self.assertRaisesRegex(ValueError, "outdated source description"):
            self.verify()
        self.refresh()
        self.verify()
        first_catalog = (self.root / manager.CATALOG).read_bytes()
        first_doc = (self.root / manager.DOC).read_bytes()
        self.refresh()
        self.assertEqual((self.root / manager.CATALOG).read_bytes(), first_catalog)
        self.assertEqual((self.root / manager.DOC).read_bytes(), first_doc)
        self.assertEqual(self.catalog["origin_records_sha256"], original)

    def test_description_references_must_exist_even_if_generated_doc_matches(self):
        self.catalog["packages"]["src/foundation/mini"]["changes"] = "See docs/components/naming.md"
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "missing source description path"):
            self.verify()

    def test_repeated_provenance_prefix_is_rejected_even_with_matching_generated_doc(self):
        self.catalog["packages"]["src/foundation/mini"]["changes"] = (
            "See " + "compliance/provenance/shared/" * 16 + "vendor/web/dependencies/patches.json"
        )
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "outdated source description"):
            self.verify()

    def test_merged_descriptions_are_checked_too(self):
        self.catalog["merges"][0]["changes"] = "See crates/zxs-naming/README.md"
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "outdated source description"):
            self.verify()

    def test_changed_local_source_fails_until_explicit_refresh(self):
        (self.source / "lib.rs").write_text("pub fn value() -> u8 { 2 }\n")
        with self.assertRaisesRegex(ValueError, "source content drift"):
            self.verify()
        original = self.catalog["origin_records_sha256"]
        self.refresh()
        self.verify()
        self.assertEqual(self.catalog["origin_records_sha256"], original)

    def test_merged_source_is_receipted_without_independent_package(self):
        self.assertNotIn("src/merged", self.catalog["packages"])
        (self.root / "src/merged/algorithm.rs").write_text("changed merged algorithm\n")
        with self.assertRaisesRegex(ValueError, "merged source content drift"):
            self.verify()

    def test_missing_source_manifest_fails(self):
        (self.source / "Cargo.toml").unlink()
        with self.assertRaises(ValueError):
            self.verify()

    def retire_mini(self):
        before = self.catalog["packages"]["src/foundation/mini"]
        size = sum(path.stat().st_size for path in self.source.rglob("*") if path.is_file())
        plan = dict(schema=1, packages=[dict(directory="src/foundation/mini", package=before,
                    reason="unsupported platform backend", category="platform", bytes=size,
                    file_count=len(before["files_sha256"]))])
        (self.root / "src/app/Cargo.toml").write_text('[package]\nname = "app"\nversion = "1.0.0"\n')
        (self.root / "Cargo.lock").write_text('version = 4\n[[package]]\nname = "app"\nversion = "1.0.0"\n')
        self.metadata["packages"] = self.metadata["packages"][:1]
        self.metadata["resolve"]["nodes"] = [dict(id="app", features=[], deps=[])]
        shutil.rmtree(self.source)
        return plan

    def test_source_removal_requires_explicit_matching_retirement_plan(self):
        plan = self.retire_mini()
        with self.assertRaisesRegex(ValueError, "unregistered source directories"):
            self.refresh()
        with patch.object(manager, "cargo_metadata", return_value=self.metadata), redirect_stdout(io.StringIO()):
            manager.refresh(self.root, plan)
        self.catalog = manager.read_json(self.root / manager.CATALOG)
        self.verify()
        self.assertEqual(set(self.catalog["retired"]), {"src/foundation/mini"})
        self.assertEqual(len(self.catalog["upstream_identities"]), 2)
        self.assertEqual(self.catalog["origin_records_sha256"], manager.origin_hashes(manager.origins(self.root)))
        self.refresh()
        self.verify()

    def test_retirement_plan_rejects_altered_last_source_receipt(self):
        plan = self.retire_mini()
        plan["packages"][0]["package"] = dict(plan["packages"][0]["package"], name="another")
        with self.assertRaisesRegex(ValueError, "last source receipt"):
            manager.refresh(self.root, plan)

    def test_retired_source_cannot_silently_return_or_change_its_receipt(self):
        plan = self.retire_mini()
        with patch.object(manager, "cargo_metadata", return_value=self.metadata), redirect_stdout(io.StringIO()):
            manager.refresh(self.root, plan)
        self.catalog = manager.read_json(self.root / manager.CATALOG)
        self.source.mkdir()
        with self.assertRaisesRegex(ValueError, "retired source has returned"):
            self.refresh()
        self.source.rmdir()
        self.catalog["retired"]["src/foundation/mini"]["retirement"]["reason"] = "rewritten history"
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "retired source receipt drift"):
            self.refresh()

    def test_retirement_does_not_allow_remaining_local_dependency_edge(self):
        plan = self.retire_mini()
        (self.root / "src/app/Cargo.toml").write_text(
            '[package]\nname = "app"\nversion = "1.0.0"\n[dependencies]\nmini = { path = "../foundation/mini" }\n')
        with self.assertRaisesRegex(ValueError, "missing local dependency"):
            manager.refresh(self.root, plan)

    def test_source_link_is_rejected_even_with_target_inside_repository(self):
        (self.source / "alias.rs").symlink_to("lib.rs")
        with self.assertRaisesRegex(ValueError, "source symlink"):
            self.verify()

    def test_special_file_is_rejected_without_reading_it(self):
        if not hasattr(os, "mkfifo"):
            self.skipTest("platform has no FIFO")
        os.mkfifo(self.source / "pipe.rs")
        with self.assertRaisesRegex(ValueError, "source special file"):
            self.verify()

    def test_inventory_paths_reject_escape_and_nonportable_spellings(self):
        for path in ("../outside", "/absolute/path", "a/../b", "a//b", "a\\b", "C:/file", "./file"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                manager.safe_path(self.root, path)
        (self.root / "alias").symlink_to(self.source, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            manager.safe_path(self.root, "alias/lib.rs")

    def test_added_independent_workspace_is_rejected(self):
        (self.root / "tools/unregistered").mkdir()
        (self.root / "tools/unregistered/Cargo.toml").write_text("[workspace]\n")
        with self.assertRaisesRegex(ValueError, "workspace"):
            self.verify()

    def test_nested_lockfile_and_nested_git_file_or_directory_fail(self):
        for name, directory in (("Cargo.lock", False), (".git", False), (".git", True)):
            path = self.source / name
            if directory:
                path.mkdir()
            else:
                path.write_text("forbidden nested root\n")
            with self.subTest(name=name, directory=directory), self.assertRaisesRegex(ValueError, "nested"):
                self.verify()
            path.rmdir() if directory else path.unlink()

    def test_scattered_docs_legal_texts_and_development_roots_are_rejected(self):
        for name, directory in (("README.md", False), ("LICENSE", False), (".github", True), ("LICENSES", True)):
            path = self.source / name
            path.mkdir() if directory else path.write_text("scattered metadata\n")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "scattered|scaffolding"):
                self.verify()
            path.rmdir() if directory else path.unlink()

    def test_missing_manifest_receipt_fails(self):
        del self.catalog["manifests_sha256"]["Cargo.toml"]
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "manifest inventory drift"):
            self.verify()

    def test_first_party_manifest_drift_without_lock_change_is_rejected(self):
        with (self.root / "src/app/Cargo.toml").open("a") as manifest:
            manifest.write("\n[features]\nnew-business-behavior = []\n")
        with self.assertRaisesRegex(ValueError, "manifest.*drift"):
            self.verify()

    def test_registry_edges_are_rejected_even_when_package_is_optional(self):
        manifest = self.root / "src/app/Cargo.toml"
        manifest.write_text(manifest.read_text() + 'optional = { version = "1", optional = true }\n')
        with self.assertRaisesRegex(ValueError, "non-path dependency"):
            self.verify()

    def test_dormant_feature_forwarding_cannot_target_removed_backend(self):
        manifest = self.root / "src/app/Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[features]\nunused = ["mini/missing"]\n')
        with self.assertRaisesRegex(ValueError, "dangling feature edge"):
            self.refresh()

    def test_optional_implicit_features_and_target_variants_are_validated(self):
        manifest = self.root / "src/app/Cargo.toml"
        manifest.write_text('[package]\nname="app"\nversion="1.0.0"\n'
                            '[dependencies]\nmini={path="../foundation/mini",optional=true}\n'
                            '[target.\'cfg(unix)\'.dependencies]\nmini={path="../foundation/mini"}\n'
                            '[features]\ndefault=["mini"]\nexplicit=["dep:mini"]\n')
        # dep: hides the automatic feature, even with multiple target tables.
        with self.assertRaisesRegex(ValueError, "dangling feature edge"):
            self.refresh()
        manifest.write_text(manifest.read_text().replace('default=["mini"]', 'default=["explicit"]'))
        self.refresh()
        self.verify()

    def test_registry_or_git_source_in_root_lock_fails_after_receipt_update(self):
        with (self.root / "Cargo.lock").open("a") as lock:
            lock.write(f'source = "{manager.REGISTRY}"\nchecksum = "' + "a" * 64 + '"\n')
        self.catalog["lockfiles_sha256"]["Cargo.lock"] = manager.digest((self.root / "Cargo.lock").read_bytes())
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "non-path Rust source"):
            self.verify()

    def test_refresh_cannot_rebaseline_original_identity_or_legal_material(self):
        self.origin_records[0]["upstream"]["archive_sha256"] = "f" * 64
        self.write_json(manager.ORIGINS, dict(packages=self.origin_records))
        with self.assertRaisesRegex(ValueError, "immutable historical origin"):
            self.refresh()
        self.origin_records[0]["upstream"]["archive_sha256"] = "a" * 64
        self.write_json(manager.ORIGINS, dict(packages=self.origin_records))
        (self.root / "compliance/licenses/upstream/mini/LICENSE").write_text("changed copyright\n")
        with self.assertRaisesRegex(ValueError, "immutable original material"):
            self.refresh()

    def test_missing_centralized_notice_fails(self):
        (self.root / "compliance/licenses/upstream/mini/LICENSE").unlink()
        with self.assertRaisesRegex(ValueError, "centralized origin material"):
            self.verify()

    def test_metadata_source_outside_repository_or_registry_source_fails(self):
        for manifest, source in (("/external/cache/mini/Cargo.toml", None), (str(self.source / "Cargo.toml"), manager.REGISTRY)):
            metadata = {"packages": [dict(id="mini", manifest_path=manifest, source=source)]}
            with self.subTest(manifest=manifest, source=source), self.assertRaisesRegex(ValueError, "external Rust|registry/Git"):
                manager.verify_source_paths(self.root, metadata)

    def test_metadata_features_must_match_inventory(self):
        self.metadata["resolve"]["nodes"][1]["features"] = ["changed"]
        with patch.object(manager, "cargo_metadata", return_value=self.metadata):
            with self.assertRaisesRegex(ValueError, "closure resolution/features drift"):
                self.verify(metadata=True)

    def test_unused_retained_source_is_rejected_by_closed_scope_policy(self):
        self.policy["require_reachable_sources"] = True
        self.write_json(manager.POLICY, self.policy)
        self.metadata["resolve"]["nodes"][0]["deps"] = []
        with self.assertRaisesRegex(ValueError, "outside supported entry closures"):
            self.refresh()


class DescriptionPaths(unittest.TestCase):
    def test_each_migration_is_idempotent(self):
        for before, after in manager.DESCRIPTION_PATHS:
            with self.subTest(before=before):
                self.assertEqual(manager.current_description(before), after)
                self.assertEqual(manager.current_description(after), after)
                self.assertEqual(manager.current_description(f"路径 `{before}`；说明"), f"路径 `{after}`；说明")

    def test_repeated_shared_prefix_is_repaired_once(self):
        suffix = "vendor/web/dependencies/patches.json"
        canonical = "compliance/provenance/shared/" + suffix
        for count in (2, 16, 32):
            with self.subTest(count=count):
                repaired = manager.current_description("compliance/provenance/shared/" * count + suffix)
                self.assertEqual(repaired, canonical)
                self.assertEqual(manager.current_description(repaired), canonical)

    def test_embedded_paths_and_similarly_named_paths_are_not_rewritten(self):
        for text in ("docs/upstream/web/dist", "https://example.test/web/src/file.rs",
                     "web/dist-old", "tools/zxs-bindgen-other", "src/web/src/main.rs"):
            with self.subTest(text=text):
                self.assertEqual(manager.current_description(text), text)


class MetadataProcess(unittest.TestCase):
    def test_metadata_explicitly_uses_empty_home_offline_lock_and_target(self):
        metadata = {"packages": []}
        result = subprocess.CompletedProcess([], 0, stdout=json.dumps(metadata), stderr="")
        with tempfile.TemporaryDirectory() as directory:
            root, home = Path(directory), Path(directory) / "empty-home"
            home.mkdir()
            with patch.object(manager.subprocess, "run", return_value=result) as execute:
                self.assertEqual(manager.cargo_metadata(root, "Cargo.toml", "aarch64-unknown-linux-gnu", home), metadata)
            args, options = execute.call_args
            self.assertIn("--offline", args[0])
            self.assertIn("--locked", args[0])
            self.assertEqual(args[0][-2:], ["--filter-platform", "aarch64-unknown-linux-gnu"])
            self.assertEqual(options["env"]["CARGO_HOME"], str(home))
            self.assertEqual(options["cwd"], root)

    def test_failed_offline_resolution_is_an_error(self):
        result = subprocess.CompletedProcess([], 101, stdout="", stderr="missing local source")
        with patch.object(manager.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(ValueError, "offline metadata failed"):
                manager.cargo_metadata(Path("/repository"))


class ClosureRoles(unittest.TestCase):
    def setUp(self):
        self.metadata = dict(workspace_members=["app"],
            packages=[dict(id=name, targets=[{"kind": ["proc-macro" if name == "macro" else "lib"]}])
                      for name in ("app", "normal", "build", "macro", "shared", "dev")],
            resolve={"nodes": [dict(id="app", features=[], deps=[
                dict(pkg="normal", dep_kinds=[{"kind": None}]), dict(pkg="build", dep_kinds=[{"kind": "build"}]),
                dict(pkg="macro", dep_kinds=[{"kind": None}]), dict(pkg="dev", dep_kinds=[{"kind": "dev"}])]),
                dict(id="normal", features=[], deps=[dict(pkg="shared", dep_kinds=[{"kind": None}])]),
                dict(id="build", features=[], deps=[dict(pkg="shared", dep_kinds=[{"kind": None}])]),
                dict(id="macro", features=[], deps=[]), dict(id="shared", features=["z", "a"], deps=[]),
                dict(id="dev", features=[], deps=[])]})

    def test_business_normal_build_and_macro_roles_remain_distinct(self):
        roles = manager.closure(self.metadata)
        self.assertEqual(roles["normal"]["roles"], ["runtime"])
        self.assertEqual(roles["build"]["roles"], ["compile"])
        self.assertEqual(roles["macro"]["roles"], ["compile"])
        self.assertEqual(roles["shared"], {"roles": ["compile", "runtime"], "features": ["a", "z"]})
        self.assertNotIn("dev", roles)

    def test_build_helper_entire_dependency_graph_is_compile_only(self):
        roles = manager.closure(self.metadata, initial_role="compile")
        self.assertTrue(all(package["roles"] == ["compile"] for package in roles.values()))
        self.assertEqual(manager.workspace_role("tools/web-build/Cargo.toml"), "compile")
        self.assertEqual(manager.workspace_role("src/smb/Cargo.toml"), "runtime")


if __name__ == "__main__":
    unittest.main()
