#!/usr/bin/env python3
"""Exercise maintained bindgen, attribute diagnostics and terminal output through real consumers."""
from pathlib import Path
import os
import json
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FEATURES = {"attribute-derive": ["syntax-full"]}
CONSUMER = (ROOT / "test/ci/fixtures/foundation-consumers.rs").read_text()


def check(root):
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='foundation-consumers-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        dependencies = '\n'.join(f'{name} = {{path = "{root / "src/foundation" / directory}", features = {json.dumps(FEATURES.get(name, []))}}}'
                                 for name, directory in [('bindgen', 'bindgen'), ('anstream', 'anstream'),
                                                         ('attribute-derive', 'attribute-derive'), ('rust-syntax', 'rust-syntax'),
                                                         ('winnow', 'winnow'), ('toml', 'toml')])
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname = "foundation-consumer-regression"\nversion = "0.0.0"\n'
                            'edition = "2021"\n[workspace]\n[dependencies]\n' + dependencies + '\n')
        (work / 'src/lib.rs').write_text(CONSUMER)
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=root, check=True)
        environment = dict(os.environ, BINDGEN_EXTRA_CLANG_ARGS='-DVALUE=7 -DLABEL=\'"hello world"\'')
        subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest),
                        '--target-dir', str(root / 'target/foundation-consumer-check')],
                       cwd=root, env=environment, check=True)
        command = ['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest),
                   '--target-dir', str(root / 'target/foundation-consumer-check')]
        (work / 'src/lib.rs').write_text('use winnow::combinator::trace;\n')
        result = subprocess.run(command, cwd=root, capture_output=True, text=True)
        if not result.returncode or 'error[E0432]' not in result.stderr:
            raise AssertionError('retired parser tracing API was accepted or failed for another reason: ' + result.stderr)
        (work / 'src/lib.rs').write_text('')
        for feature in ['winnow/debug', 'toml/unstable-debug']:
            result = subprocess.run(command + ['--features', feature], cwd=root, capture_output=True, text=True)
            if not result.returncode or 'does not have that feature' not in result.stderr:
                raise AssertionError('retired parser debug feature was accepted or failed for another reason: ' + result.stderr)



if __name__ == '__main__':
    check(ROOT)
