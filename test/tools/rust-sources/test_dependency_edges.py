"""Workspace inheritance matches the pinned Cargo 1.99 dependency semantics."""
from copy import deepcopy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "dependency_edges", Path(__file__).resolve().parents[3] / "tools/rust-sources/dependency_edges.py")
dependency_edges = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(dependency_edges)


class InheritedDependencies(unittest.TestCase):
    def test_default_feature_matrix_for_explicit_and_inherited_editions(self):
        # Checked against cargo 1.99 metadata --offline --no-deps. Earlier
        # editions ignore member false when the workspace enables defaults.
        for edition in ("2015", "2018", "2021", "2024"):
            for inherited_edition in (False, True):
                for workspace_default in (None, False, True):
                    for consumer_default in (None, False, True):
                        with self.subTest(edition=edition, inherited_edition=inherited_edition,
                                          workspace=workspace_default, consumer=consumer_default):
                            workspace = {"package": {"edition": edition}, "dependencies": {
                                "renamed": {"path": "src/mini", "package": "mini", "features": ["base"]}}}
                            declaration = {"workspace": True, "features": ["extra"], "optional": True}
                            if workspace_default is not None:
                                workspace["dependencies"]["renamed"]["default-features"] = workspace_default
                            if consumer_default is not None:
                                declaration["default-features"] = consumer_default
                            manifest = {"package": {"edition": {"workspace": True} if inherited_edition else edition}}
                            before = deepcopy((manifest, workspace, declaration))
                            dependency, base = dependency_edges.resolve(
                                Path("/repository"), "src/app/Cargo.toml", manifest, workspace, "renamed", declaration)
                            expected = workspace_default is not False
                            if consumer_default is not None and (edition == "2024" or consumer_default):
                                expected = consumer_default
                            self.assertEqual(dependency, {
                                "path": "src/mini", "package": "mini", "features": ["base", "extra"],
                                "optional": True, "default-features": expected})
                            self.assertEqual(base, Path("/repository"))
                            self.assertEqual((manifest, workspace, declaration), before)

    def test_unset_edition_uses_legacy_defaults(self):
        dependency, _ = dependency_edges.resolve(Path("/repository"), "src/app/Cargo.toml", {},
            {"dependencies": {"mini": {"path": "src/mini"}}}, "mini",
            {"workspace": True, "default-features": False})
        self.assertTrue(dependency["default-features"])

    def test_consumer_options_do_not_leak_to_workspace_or_siblings(self):
        workspace = {"dependencies": {"mini": {"path": "src/mini", "features": ["base"], "default-features": False}}}
        first, _ = dependency_edges.resolve(Path("/repository"), "src/app/Cargo.toml", {}, workspace, "mini",
            {"workspace": True, "features": ["extra"], "optional": True, "default-features": True})
        first["features"].append("mutated")
        second, _ = dependency_edges.resolve(Path("/repository"), "src/tool/Cargo.toml", {}, workspace,
                                              "mini", {"workspace": True})
        self.assertEqual(second, {"path": "src/mini", "features": ["base"], "default-features": False, "optional": False})
        self.assertNotIn("optional", workspace["dependencies"]["mini"])

    def test_direct_dependencies_stay_relative_to_the_consumer(self):
        declaration = {"path": "../mini", "features": ["base"], "optional": True, "default-features": False}
        dependency, base = dependency_edges.resolve(Path("/repository"), "src/app/Cargo.toml", {}, {}, "mini", declaration)
        self.assertEqual(dependency, declaration)
        self.assertIsNot(dependency, declaration)
        self.assertEqual(base, Path("/repository/src/app"))

    def test_missing_or_non_table_dependencies_are_rejected(self):
        for workspace, declaration in (({}, "1"), ({}, {"workspace": True}),
                                       ({"dependencies": {"mini": "1"}}, {"workspace": True})):
            with self.subTest(workspace=workspace, declaration=declaration):
                with self.assertRaisesRegex(ValueError, "non-path dependency: src/app/Cargo.toml mini"):
                    dependency_edges.resolve(Path("/repository"), "src/app/Cargo.toml", {}, workspace, "mini", declaration)


if __name__ == "__main__":
    unittest.main()
