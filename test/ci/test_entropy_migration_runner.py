"""Entropy regressions must never generate locks inside governed source trees."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

PATH = Path(__file__).resolve().parents[2] / 'test/entropy/check.py'
SPEC = importlib.util.spec_from_file_location('entropy_migration_check', PATH)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class EntropyMigrationRunner(unittest.TestCase):
    def test_unit_manifest_and_lock_stay_outside_governed_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.0"\n')
            package = root / 'src/foundation/cryptography/random-generator-core'
            package.mkdir(parents=True)
            (package / 'Cargo.toml').write_text(
                '[package]\nname="random-generator-core"\nversion="0.0.0"\n'
                '[dependencies.system-entropy]\npath = "../system-entropy"\n')
            (package / 'Cargo.lock').write_bytes(b'original lock must not change')
            for relative in ('src/foundation/cryptography/random-generator-core/src/lib.rs',
                             'test/foundation/random-generator-core/unit/test.rs',
                             'test/entropy/error.rs', 'docs/components/system-entropy.md'):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('exact original material')
            original = {p.relative_to(root): p.read_bytes() for p in root.rglob('*') if p.is_file()}
            commands = []

            def run(arguments, env=None):
                commands.append(arguments)
                manifest = Path(arguments[arguments.index('--manifest-path') + 1])
                self.assertFalse(manifest.is_relative_to(root))
                self.assertIn('--offline', arguments)
                self.assertIn(str(root / 'src/foundation/cryptography/system-entropy'), manifest.read_text())
                self.assertEqual((manifest.parent / 'src/lib.rs').read_text(), 'exact original material')
                self.assertEqual((manifest.parents[4] / "rust-toolchain.toml").read_bytes(),
                                 (root / "rust-toolchain.toml").read_bytes())
                external = manifest.parents[4] / 'test/foundation/random-generator-core/unit/test.rs'
                self.assertEqual(external.read_text(), 'exact original material')
                if arguments[0] == 'generate-lockfile':
                    manifest.with_name('Cargo.lock').write_bytes(b'isolated generated lock')
                else:
                    self.assertIn('--locked', arguments)
                    self.assertEqual(manifest.with_name('Cargo.lock').read_bytes(), b'isolated generated lock')

            with patch.object(checker, 'ROOT', root), patch.object(checker, 'run', side_effect=run):
                checker.unit('random-generator-core', ['--lib', '--no-default-features'])
            self.assertEqual([x[0] for x in commands], ['generate-lockfile', 'test'])
            self.assertEqual(original, {p.relative_to(root): p.read_bytes() for p in root.rglob('*') if p.is_file()})

    def test_all_explicit_compile_commands_require_offline_and_locked(self):
        source = PATH.read_text()
        import ast
        for node in ast.walk(ast.parse(source)):
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Name) or node.func.id != 'run':
                continue
            args = node.args[0]
            if not isinstance(args, ast.List):
                continue
            values = [item.value for item in args.elts if isinstance(item, ast.Constant)]
            if values and values[0] in ('test', 'check'):
                self.assertIn('--offline', values)
                self.assertIn('--locked', values)


if __name__ == '__main__':
    unittest.main()
