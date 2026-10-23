#!/usr/bin/env python3
"""Exercise migrated AST consumers, generated bindings and CPU dispatch offline."""
import argparse
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
TARGET = 'x86_64-unknown-linux-gnu'
BORROWED_HANDLER = ('pub struct State;\nimpl State {\n'
                    '    #[http_routing::debug_handler]\n'
                    '    pub async fn borrowed(&self) -> &\'static str { "no" }\n}\n')


def run_cargo(work, target_dir, environment, *, tests=False, rejection=None):
    command = ['cargo', 'test' if tests else 'check', '--locked', '--offline',
               '--manifest-path', str(work / 'Cargo.toml'), '--target', TARGET,
               '--target-dir', str(target_dir), '--message-format=json', '-j2']
    if tests:
        command += ['--tests', '--no-run']
    diagnostics = runpy.run_path(str(ROOT / '.github/scripts/check-rust-diagnostics.py'))['CargoDiagnostics'](sys.stderr)
    executables, errors = [], []
    with subprocess.Popen(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                          text=True, encoding='utf-8') as process:
        for number, line in enumerate(process.stdout, 1):
            diagnostics.consume(line, number)
            try:
                event = json.loads(line)
            except ValueError:
                continue
            if not isinstance(event, dict):
                continue
            if event.get('reason') == 'compiler-message' and event['message'].get('level') == 'error':
                errors.append(event['message'].get('message', ''))
            if (event.get('reason') == 'compiler-artifact' and event.get('profile', {}).get('test')
                    and event.get('executable')):
                executables.append(event['executable'])
        status = process.wait()
    if rejection:
        if (status == 0 or not errors or any(rejection not in error for error in errors)
                or diagnostics.warnings or diagnostics.invalid or not diagnostics.finished or diagnostics.succeeded):
            raise RuntimeError('expected owned-handler rejection was not cleanly established')
        return
    if diagnostics.result(status):
        raise RuntimeError('syntax migration consumer diagnostics rejected')
    if tests:
        if len(executables) != 5:
            raise RuntimeError(f'expected five registered test executables, got {len(executables)}')
        for executable in executables:
            subprocess.run([executable, '--test-threads=1'], cwd=ROOT, env=environment, check=True)


def manifest():
    deps = {
        'tracing': ('tracing', []), 'tracing-attributes': ('tracing-attributes', []),
        'http-routing': ('http-routing', ['macros']), 'strum': ('strum', ['derive']),
        'rust-syntax': ('rust-syntax', ['full']), 'rust-source-format': ('rust-source-format', []),
        'cpu-feature-detect': ('cpu-feature-detect', []), 'typed-builder': ('typed-builder', []),
        'argon2': ('argon2', []), 'password-hash': ('password-hash', []), 'base64ct': ('base64ct', []),
        'libsqlite3-sys': ('libsqlite3-sys', ['bundled', 'loadable_extension', 'buildtime_bindgen']),
    }
    lines = ['[package]', 'name = "syntax-migration-regression"', 'version = "0.0.0"',
             'edition = "2024"', 'autobins = false', '[lib]', 'test = false', 'doctest = false', '[workspace]', '[dependencies]']
    for name, (directory, features) in deps.items():
        lines.append(f'{name} = {{ path = {json.dumps(str(foundation_path(ROOT, directory)))}, features = {json.dumps(features)} }}')
    for name, relative in [('migration', 'test/ci/fixtures/rust-syntax-migration.rs'),
                           ('typed-builder-main', 'test/foundation/typed-builder/tests/tests.rs'),
                           ('typed-builder-no-std', 'test/foundation/typed-builder/tests/no_std.rs'),
                           ('typed-builder-no-leakage', 'test/foundation/typed-builder/tests/no_type_leakage.rs'),
                           ('argon2-phc', 'test/foundation/argon2/tests/phc_strings.rs')]:
        lines += ['[[test]]', f'name = "{name}"', f'path = {json.dumps(str(ROOT / relative))}']
    # The preserved Argon2 tests use their original crate feature guards.
    lines += ['[features]', 'default = ["alloc", "password-hash"]', 'alloc = []', 'password-hash = []', '[profile.dev]', 'debug = 0', 'incremental = false', '[profile.test]', 'debug = 0', 'incremental = false']
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target-dir', type=Path, default=ROOT / 'target/syntax-migration')
    args = parser.parse_args()
    environment = runpy.run_path(str(ROOT / '.github/scripts/backend_environment.py'))['matrix_environment']()
    environment['RUSTFLAGS'] = '--deny=warnings'
    # Own this fixture's header search list; do not inherit a different SDK.
    include = Path(subprocess.check_output(['cc', '-print-file-name=include'], text=True).strip())
    if not (include / 'stdarg.h').is_file():
        raise RuntimeError(f'C compiler resource stdarg.h unavailable: {include}')
    sqlite = ROOT / 'src/foundation/data-storage/libsqlite3-sys/sqlite3'
    if not (sqlite / 'sqlite3ext.h').is_file():
        raise RuntimeError('bundled SQLite extension headers missing')
    environment['BINDGEN_EXTRA_CLANG_ARGS'] = f'-I{sqlite} -isystem {include}'
    args.target_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='consumer-', dir=args.target_dir) as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        (work / 'src/lib.rs').write_text('')
        (work / 'Cargo.toml').write_text(manifest())
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(work / 'Cargo.toml')], cwd=ROOT, env=environment, check=True)
        run_cargo(work, args.target_dir / 'build', environment, tests=True)
        (work / 'src/lib.rs').write_text(BORROWED_HANDLER)
        run_cargo(work, args.target_dir / 'build', environment, rejection='Handlers must only take owned values')
    print('Syntax migration regression passed: runtime instrumentation, formatting, FnPtr generated SQLite API, typed builders, Argon2 and receiver rejection.')


if __name__ == '__main__':
    main()
