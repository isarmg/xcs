#!/usr/bin/env python3
"""Reject retired SQLx APIs/features using a real offline consumer of the SQLite stack."""
if not __debug__:
    raise RuntimeError('Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE).')

from pathlib import Path
import subprocess
import tempfile
import tomllib

import runpy

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
REMOVED = {
    "sqlite-access": ["any", "migrate", "sqlx-toml", "runtime-tokio", "runtime-smol",
                      "tls-none", "tls-rustls", "_rt-tokio", "_tls-rustls"],
}
NEGATIVE = [
    ('use sqlite_access::{any, migrate, testing, __rt, _unstable};', 'E0432'),
    ('use sqlite_access::{any, config, migrate, net, testing, type_checking};', 'E0432'),
    ('use sqlite_access::sqlite::{any, describe_blocking, CREATE_DB_WAL};', 'E0432'),
    ('fn retired() { let _ = sqlite_access::Error::AnyDriverError; }', 'E0599'),
    ('fn retired() { let _ = sqlite_access::Error::Tls; }', 'E0599'),
    ('fn retired() { let _ = sqlite_access::Error::Migrate; }', 'E0599'),
    ('fn retired() { let _ = sqlite_access::Error::ConfigFile; }', 'E0599'),
    ('fn retired<T: sqlite_access::ConnectOptions>() { let _ = T::__unstable_apply_driver_config; }', 'E0599'),
]


def check(root):
    for package, retired in REMOVED.items():
        manifest = tomllib.loads((foundation_path(root, package) / 'Cargo.toml').read_text())
        features = manifest.get('features', {})
        assert not set(retired).intersection(features), (package, features)
        assert not any(name.startswith(('runtime-', '_rt-', 'tls-', '_tls-')) for name in features)
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='sqlite-scope-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        declarations = '\n'.join(
            f'{name} = {{path = "{foundation_path(root, name)}"{extra}}}'
            for name, extra in [('sqlite-access', ', features = ["sqlite-bundled"]')])
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname = "sqlite-api-scope"\nversion = "0.0.0"\nedition = "2021"\n'
                            '[workspace]\n[dependencies]\n' + declarations + '\n')
        source = work / 'src/lib.rs'
        source.write_text('pub fn accepts(_: Option<sqlite_access::SqliteConnection>) {}\n')
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=root, check=True)
        command = ['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest),
                   '--target-dir', str(root / 'target/sqlite-scope-check')]
        subprocess.run(command, cwd=root, check=True)
        for source_text, code in NEGATIVE:
            source.write_text(source_text + '\n')
            result = subprocess.run(command, cwd=root, text=True, capture_output=True)
            if not result.returncode or f'error[{code}]' not in result.stderr:
                raise AssertionError(f'retired API unexpectedly accepted or unrelated failure: {source_text}\n{result.stderr}')
        source.write_text('')
        for package, features in REMOVED.items():
            for feature in features:
                result = subprocess.run(command + ['--features', f'{package}/{feature}'], cwd=root, text=True, capture_output=True)
                if not result.returncode or f'does not have that feature' not in result.stderr:
                    raise AssertionError(f'retired feature unexpectedly accepted or unrelated failure: {package}/{feature}\n{result.stderr}')
        retained = ['sqlite-access/pool', 'sqlite-access/url', 'sqlite-access/json', 'sqlite-access/uuid', 'sqlite-access/regexp', 'sqlite-access/sqlite-deserialize',
                    'sqlite-access/sqlite-load-extension', 'sqlite-access/sqlite-preupdate-hook',
                    'sqlite-access/sqlite-unlock-notify', 'sqlite-access/offline']
        subprocess.run(command + ['--features', ','.join(retained)], cwd=root, check=True)
        print(f'SQLite scope: positive consumer, {len(NEGATIVE)} retired API and {sum(map(len, REMOVED.values()))} retired feature builds verified.')


if __name__ == '__main__':
    check(ROOT)
