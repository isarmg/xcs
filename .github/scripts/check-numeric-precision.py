#!/usr/bin/env python3
"""Exercise optional JSON numeric consumers, without directly enabling numeric."""
from pathlib import Path
import json
import os
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def check(root=ROOT):
    for name in ('jsonschema', 'jsonschema-value'):
        manifest = tomllib.loads((root / 'src/foundation' / name / 'Cargo.toml').read_text())
        forwarding = manifest['features']['arbitrary-precision']
        assert 'numeric/bigint' in forwarding, (name, forwarding)
        assert 'json-codec/arbitrary_precision' in forwarding, (name, forwarding)
        if name == 'jsonschema':
            assert 'jsonschema-value/arbitrary-precision' in forwarding
    with tempfile.TemporaryDirectory(prefix='zxs-numeric-precision-') as directory:
        work = Path(directory)
        (work / 'src').mkdir()
        (work / 'src/lib.rs').write_bytes((root / 'test/ci/fixtures/numeric-precision.rs').read_bytes())
        foundation = root / 'src/foundation'
        (work / 'Cargo.toml').write_text(f'''[package]
name = "numeric-consumer-precision"
version = "0.0.0"
edition = "2021"
[workspace]
[features]
value-only = ["jsonschema-value/arbitrary-precision"]
validator = ["dep:jsonschema", "jsonschema/arbitrary-precision"]
[dependencies]
jsonschema = {{ path = "{foundation / 'jsonschema'}", default-features = false, optional = true }}
jsonschema-value = {{ path = "{foundation / 'jsonschema-value'}", default-features = false, features = ["json-codec"] }}
json-codec = {{ path = "{foundation / 'json-codec'}" }}
[profile.dev]
debug = 0
incremental = false
[profile.test]
debug = 0
incremental = false
''')
        env = os.environ.copy()
        env.setdefault('CARGO_BUILD_JOBS', '2')
        for features in ('value-only', 'validator'):
            args = ['--manifest-path', str(work / 'Cargo.toml'), '--offline', '--features', features]
            result = subprocess.run(['cargo', 'metadata', *args, '--format-version', '1'], capture_output=True, text=True, check=True, cwd=root, env=env)
            metadata = json.loads(result.stdout)
            packages = {p['id']: p['name'] for p in metadata['packages']}
            active = {packages[n['id']]: n['features'] for n in metadata['resolve']['nodes']}
            assert 'bigint' in active['numeric'], active
            assert 'arbitrary_precision' in active['json-codec'], active
            assert 'arbitrary-precision' in active['jsonschema-value'], active
            if features == 'value-only':
                assert 'jsonschema' not in active
            else:
                assert 'arbitrary-precision' in active['jsonschema']
            assert not ({'num', 'num-traits', 'num-bigint', 'num-rational', 'num-integer', 'num-complex', 'num-iter'} & active.keys())
            subprocess.run(['cargo', 'test', *args, '--locked', '--lib'], check=True, cwd=root, env=env)
            print(features, {name: active[name] for name in ('numeric', 'json-codec', 'jsonschema-value')}, flush=True)
    print('Arbitrary-precision value and validator forwarding, large integers, exact rationals and extreme exponents verified.')


if __name__ == '__main__':
    check()
