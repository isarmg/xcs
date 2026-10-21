import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from unittest import mock
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('gate', ROOT / '.github/scripts/check-command-line-family.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)

class NamingGate(unittest.TestCase):

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        packages = {}
        for old, new in gate.N.items():
            p = self.root / 'src/foundation' / new
            (p / 'src').mkdir(parents=True)
            (p / 'Cargo.toml').write_text(f'''[package]\nname="{new}"\n[lib]\nname="{new.replace('-', '_')}"\n''')
            (p / 'src/lib.rs').write_text('attributes(command, arg, group)' if new == 'command-derive' else 'mod lexer;')
            packages['src/foundation/' + new] = {'directory': 'src/foundation/' + new, 'origin_directory': old + '-4.6.7'}
        for name in ('main.rs', 'args.rs'):
            p = self.root / 'src/admin/src' / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text('use command_line::Parser;')
        (self.root / 'compliance').mkdir()
        (self.root / 'compliance/rust-sources.json').write_text(json.dumps({'packages': packages, 'merges': [{'directory': 'src/foundation/command-line', 'origin_directory': origin} for origin in ('clap_builder-4.6.7', 'clap_lex-1.1.1')]}))

    def test_accepts_functional_identity(self):
        self.assertEqual(gate.check(self.root), 2)

    def test_rejects_removed_capabilities(self):
        for name in ('unicode', 'wrap_help', 'unstable-markdown'):
            with self.subTest(feature=name):
                path = self.root / 'src/foundation/command-line/Cargo.toml'
                before = path.read_text()
                path.write_text(before + '[features]\n' + name + '=[]\n')
                with self.assertRaisesRegex(AssertionError, 'unsupported capability reintroduced'):
                    gate.check(self.root)
                path.write_text(before)

    def test_rejects_unmerged_package(self):
        (self.root / 'src/foundation/command-builder').mkdir()
        with self.assertRaisesRegex(AssertionError, 'unmerged source root'):
            gate.check(self.root)

    def test_rejects_public_lexer(self):
        (self.root / 'src/foundation/command-line/src/lib.rs').write_text('pub mod lexer;')
        with self.assertRaisesRegex(AssertionError, 'lexer exposed'):
            gate.check(self.root)

    def test_rejects_lost_merged_origin(self):
        path = self.root / 'compliance/rust-sources.json'
        catalog = json.loads(path.read_text())
        catalog['merges'] = []
        path.write_text(json.dumps(catalog))
        with self.assertRaisesRegex(AssertionError, 'lost merged runtime upstream origin'):
            gate.check(self.root)

    def test_rejects_legacy_root(self):
        (self.root / 'src/foundation/clap').mkdir()
        with self.assertRaisesRegex(AssertionError, 'old source root'):
            gate.check(self.root)

    def test_rejects_legacy_emitted_path(self):
        (self.root / 'src/foundation/command-derive/src/lib.rs').write_text('attributes(command, arg, group) impl clap::Parser for X {}')
        with self.assertRaisesRegex(AssertionError, 'old Rust path'):
            gate.check(self.root)

    def test_rejects_legacy_attribute(self):
        (self.root / 'src/foundation/command-derive/src/lib.rs').write_text('attributes(command, arg, group) attributes(clap)')
        with self.assertRaisesRegex(AssertionError, 'old derive alias'):
            gate.check(self.root)

    def test_rejects_dependency_alias(self):
        p = self.root / 'src/foundation/command-line/Cargo.toml'
        p.write_text(p.read_text() + '[dependencies.clap]\npackage="command-line"\npath="../command-line"\n')
        with self.assertRaisesRegex(AssertionError, 'old dependency alias'):
            gate.check(self.root)

    def test_cargo_commands_are_locked_and_offline(self):
        for operation in ('test', 'check', 'build', 'metadata'):
            self.assertEqual(gate.cargo_command(operation, '--manifest-path', 'fixture/Cargo.toml'), ['cargo', operation, '--locked', '--offline', '--color', 'never', '--manifest-path', 'fixture/Cargo.toml'])

    def test_captured_cargo_diagnostics_override_forced_ci_color(self):
        with mock.patch.dict(os.environ, {"CARGO_TERM_COLOR": "always"}):
            command = gate.cargo_command("check", "--manifest-path", "fixture/Cargo.toml")
            self.assertEqual(command[command.index("--color") + 1], "never")
            self.assertEqual(os.environ["CARGO_TERM_COLOR"], "always")

    def test_unsafe_rejection_accepts_actual_colored_compiler_content(self):
        stderr = "\x1b[1m\x1b[91merror\x1b[0m\x1b[1m: usage of an `unsafe` block\x1b[0m\nrejected_unsafe_boundary_probe"
        gate.require_unsafe_rejection(subprocess.CompletedProcess([], 101, "", stderr), "src/lib.rs")

    def test_unsafe_rejection_requires_error_and_injected_code(self):
        expected = "error: usage of an `unsafe` block\nrejected_unsafe_boundary_probe"
        for status, stderr in ((0, expected), (101, "unrelated failure"),
                               (101, "error: usage of an `unsafe` block"),
                               (101, "warning: usage of an `unsafe` block\nrejected_unsafe_boundary_probe")):
            with self.subTest(status=status, stderr=stderr), self.assertRaises(AssertionError):
                gate.require_unsafe_rejection(subprocess.CompletedProcess([], status, "", stderr), "src/lib.rs")

    def test_only_temporary_fixture_generates_a_lock(self):
        fixture = self.root / 'test/command-line-family.rs'
        fixture.parent.mkdir()
        fixture.write_text('// fixture')
        (fixture.parent / 'command-line-lexer-examples.rs').write_text('// lexer examples')
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.object(gate.subprocess, 'run') as run:
                manifest, source = gate.prepare_fixture(self.root, Path(directory))
            run.assert_called_once_with(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=self.root, check=True)
            self.assertIn('// fixture', source.read_text())
            self.assertIn('mod lexer;', source.read_text())
            self.assertFalse((self.root / 'Cargo.lock').exists())

    def test_rejects_erased_origin(self):
        p = self.root / 'compliance/rust-sources.json'
        d = json.loads(p.read_text())
        d['packages']['src/foundation/command-line']['origin_directory'] = 'command-line'
        p.write_text(json.dumps(d))
        with self.assertRaisesRegex(AssertionError, 'lost upstream origin'):
            gate.check(self.root)
if __name__ == '__main__':
    unittest.main()
