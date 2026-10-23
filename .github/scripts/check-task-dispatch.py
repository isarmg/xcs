#!/usr/bin/env python3
"""Verify supported executor capabilities and fail-loud initialization in both profiles."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def run(work, *args, minimum_tests=0):
    result = subprocess.run(['cargo', '--color', 'never', *args], cwd=work,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=600)
    print(result.stdout, end='', flush=True)
    assert result.returncode == 0, result.stdout
    assert 'warning:' not in result.stdout, result.stdout
    passed = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', result.stdout)))
    assert passed >= minimum_tests, result.stdout


def prepare(work, features, tests):
    dependencies = f'task-dispatch={{path={json.dumps(str(ROOT / "src/foundation/task-dispatch"))},features={json.dumps(features)}}}\n'
    if 'tokio' in features:
        dependencies += f'tokio={{path={json.dumps(str(ROOT / "src/foundation/tokio"))},default-features=false,features=["rt","sync"]}}\n'
    if 'wasm-bindgen' in features:
        dependencies += f'js-sys={{path={json.dumps(str(ROOT / "src/foundation/js-sys"))},default-features=false,features=["std"]}}\n'
        dependencies += f'wasm-bindgen={{path={json.dumps(str(ROOT / "src/foundation/wasm-bindgen"))}}}\n'
        (work / 'src/lib.rs').write_text('use wasm_bindgen::prelude::*;\n' + (ROOT / 'test/task-dispatch/browser.rs').read_text())
    else:
        (work / 'src/lib.rs').write_text('')
    manifest = '[package]\nname="task-dispatch-contracts"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n' + dependencies
    for test in tests:
        manifest += f'\n[[test]]\nname={json.dumps(test)}\npath={json.dumps(str(ROOT / "test/task-dispatch" / (test + ".rs")))}\n'
    manifest += '\n[profile.dev]\ndebug=0\nincremental=false\n[profile.release]\ndebug=0\nincremental=false\n'
    (work / 'Cargo.toml').write_text(manifest)
    run(work, 'generate-lockfile', '--offline')


def main():
    os.environ.setdefault('CARGO_BUILD_JOBS', '1')
    with tempfile.TemporaryDirectory(prefix='task-dispatch-') as directory:
        work = Path(directory)
        (work / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        (work / 'src').mkdir()
        (work / 'src/lib.rs').write_text('')
        for features, tests in (([], ['uninitialized']), (['futures-executor'], ['uninitialized', 'futures']), (['tokio'], ['uninitialized', 'tokio'])):
            prepare(work, features, tests)
            for profile in ([], ['--release']):
                run(work, 'test', '--locked', '--offline', '--jobs', '1', '--tests', *profile, minimum_tests=len(tests))
        prepare(work, ['wasm-bindgen'], [])
        run(work, 'check', '--locked', '--offline', '--jobs', '1', '--target', 'wasm32-unknown-unknown')
    print('Task dispatch initialization and supported executor contracts passed.')


if __name__ == '__main__':
    main()
