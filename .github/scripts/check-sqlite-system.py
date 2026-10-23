#!/usr/bin/env python3
"""Exercise an isolated unbundled SQLite consumer against the real system SDK."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def verify_features(metadata):
    packages = {p['id']: p['name'] for p in metadata['packages']}
    native = [n for n in metadata['resolve']['nodes'] if packages[n['id']] == 'libsqlite3-sys']
    access = [n for n in metadata['resolve']['nodes'] if packages[n['id']] == 'sqlite-access']
    assert len(native) == len(access) == 1, 'expected one native binding and one access package'
    assert not any(f.startswith('bundled') for f in native[0]['features']), 'bundled native build is forbidden'
    assert 'buildtime_bindgen' in native[0]['features'], 'system headers must supply bindings'
    assert 'sqlite-unbundled' in access[0]['features']
    assert 'sqlite-bundled' not in access[0]['features']


def check(root):
    version = subprocess.check_output(['pkg-config', '--modversion', 'sqlite3'], text=True).strip()
    include = Path(subprocess.check_output(['pkg-config', '--variable=includedir', 'sqlite3'], text=True).strip()).resolve()
    assert (include / 'sqlite3.h').is_file(), 'the real SQLite development header is required'
    assert not include.is_relative_to(root.resolve()), 'repository headers are not a system SDK'
    environment = os.environ.copy()
    for key in ('SQLITE3_LIB_DIR', 'SQLITE3_INCLUDE_DIR', 'SQLITE3_STATIC', 'BINDGEN_EXTRA_CLANG_ARGS'):
        environment.pop(key, None)
    environment['LIBSQLITE3_SYS_USE_PKG_CONFIG'] = '1'
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='sqlite-system-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        (work / 'Cargo.toml').write_text(
            '[package]\nname="sqlite-system-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[dependencies]\n'
            f'sqlite-access={{path="{root / "src/foundation/data-storage/sqlite-access"}",default-features=false,features=["sqlite-unbundled"]}}\n'
            f'tokio={{path="{root / "src/foundation/concurrency/tokio"}",default-features=false,features=["rt"]}}\n')
        (work / 'src/main.rs').write_text('''use sqlite_access::Connection;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let mut database = sqlite_access::SqliteConnection::connect("sqlite::memory:").await?;
        let version: String = sqlite_access::query_scalar("SELECT sqlite_version()")
            .fetch_one(&mut database).await?;
        let value: i64 = sqlite_access::query_scalar("SELECT ? + ?").bind(20_i64).bind(22_i64)
            .fetch_one(&mut database).await?;
        assert_eq!(value, 42);
        database.close().await?;
        println!("{version}");
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
''')
        common = ['--offline', '--manifest-path', str(work / 'Cargo.toml')]
        subprocess.run(['cargo', 'generate-lockfile', *common], env=environment, cwd=root, check=True)
        metadata = json.loads(subprocess.check_output(
            ['cargo', 'metadata', '--format-version', '1', '--locked', *common], env=environment, cwd=root))
        verify_features(metadata)
        actual = subprocess.check_output(
            ['cargo', 'run', '--locked', *common, '--target-dir', str(root / 'target/sqlite-system-check')],
            env=environment, cwd=root, text=True).strip()
        assert actual == version, (actual, version)
        print(f'System SQLite {version}: unbundled dependency graph and in-memory native query verified.')


if __name__ == '__main__':
    check(ROOT)
