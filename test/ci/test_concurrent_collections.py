import importlib.util
from pathlib import Path
import unittest
from unittest.mock import call, patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("concurrency_gate", ROOT / ".github/scripts/check-concurrent-collections.py")
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class ConcurrentCollectionsGateTests(unittest.TestCase):
    def test_sqlite_default_and_optional_pool_consumers_are_both_compiled(self):
        with patch.object(GATE, 'run') as execute:
            GATE.check_consumers()
        self.assertEqual(execute.call_args_list, [
            call(['cargo', 'check', '--locked', '--offline', '-p', 'rayon', '-p', 'sqlite-access']),
            call(['cargo', 'check', '--locked', '--offline', '-p', 'platform-io',
                  '--features', 'platform-io/sqlite-pool']),
        ])

    def test_native_and_wasm_feature_matrix(self):
        self.assertEqual(len(GATE.TARGETS) * len(GATE.FEATURES), 6)
        self.assertEqual(GATE.FEATURES[0], ())
        self.assertIn(("alloc",), GATE.FEATURES)
        self.assertEqual(set(GATE.TARGETS), {"x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"})

    def test_all_retained_integrations_exist(self):
        self.assertEqual(len(GATE.INTEGRATIONS), 6)
        for module, name in GATE.INTEGRATIONS:
            self.assertTrue((ROOT / "test/foundation/concurrent-collections" / module / "tests" / (name + ".rs")).is_file())

    def test_dormant_material_is_not_discarded_or_counted(self):
        self.assertEqual(len(GATE.DORMANT), 6)
        for path in GATE.DORMANT:
            self.assertTrue((ROOT / "test/foundation/concurrent-collections" / path).is_file())
        self.assertFalse(set(GATE.DORMANT) & {f"{module}/tests/{name}.rs" for module, name in GATE.INTEGRATIONS})

    def test_semantics_fixture_covers_four_real_cases(self):
        source = (ROOT / "test/tools/concurrent-collections/semantics.rs").read_text()
        self.assertEqual(source.count("#[test]"), 4)
        for name in ("rayon_parallel_consumer", "bounded_mpmc_exactly_once", "unbounded_fifo_and_drop_exactly_once", "batch_steal_preserves_all_tasks"):
            self.assertIn("fn " + name, source)

    def test_empty_nightly_compatibility_feature_is_removed(self):
        import tomllib
        manifest = tomllib.loads((ROOT / "src/foundation/concurrent-collections/Cargo.toml").read_text())
        self.assertNotIn("nightly", manifest["features"])
        self.assertEqual(GATE.FEATURES, ((), ("alloc",), ("std",)))
        self.assertTrue(callable(GATE.check_removed_feature))
