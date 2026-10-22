#!/usr/bin/env python3
"""Check schema capability boundaries and retained real canonical assertions offline."""
import json
import os
from pathlib import Path
import subprocess
import re
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'src/foundation/schema-validation/Cargo.toml'

def run(*args):
    return subprocess.run(args, cwd=ROOT, env=dict(os.environ, CARGO_BUILD_JOBS=os.environ.get('CARGO_BUILD_JOBS', '2')), check=True)

def check(manifest):
    for feature in ('', 'pattern', 'value', 'reference', 'validator', 'idna', 'resolve-async', 'jsonb,arbitrary-precision'):
        args = ['--offline', '--manifest-path', str(manifest), '--no-default-features']
        if feature:
            args += ['--features', feature]
        data = json.loads(subprocess.check_output(['cargo', 'metadata', *args, '--format-version', '1'], cwd=ROOT))
        names = {package['id']: package['name'] for package in data['packages']}
        graph = {names[node['id']]: node['features'] for node in data['resolve']['nodes']}
        retired = set(graph) & {'uuid-simd', 'vsimd', 'outref'}
        if retired:
            raise ValueError(f'retired UUID dependency in schema graph: {sorted(retired)}')
        assert not set(graph) & {'url', 'idna', 'reqwest', 'pyo3', 'magnus', 'jsonschema-macros', 'jsonschema', 'jsonschema-value', 'jsonschema-regex', 'referencing'}, graph
        if feature == 'pattern':
            assert set(graph) == {'schema-validation', 'regex'}, graph
            assert not set(graph['regex']) & {'std', 'matcher', 'automata'}, graph
        if feature in ('', 'value', 'reference'):
            assert 'regex' not in graph, graph
        if feature != 'idna':
            assert 'domain' not in graph.get('url-encoding', []), graph
        if feature == 'resolve-async':
            assert set(graph['futures']) == {'alloc', 'utilities'}, graph
        run('cargo', 'check', *args)
    common = ['--offline', '--manifest-path', str(manifest)]
    run('cargo', 'check', *common, '--all-features')
    run('cargo', 'test', *common, '--test', 'canonical')
    run('cargo', 'test', *common, '--test', 'uuid_format')
    run('cargo', 'test', *common, '--features', 'arbitrary-precision', '--test', 'canonical')
    run('cargo', 'test', *common, '--features', 'resolve-async,jsonb-testkit,arbitrary-precision', '--test', 'capabilities')
    print('Schema capability graph, canonical fixture assertions, async retrieval and typed JSONB: PASS')

def main():
    # A separate test workspace avoids unrelated root feature unification while
    # using the exact retained implementation and real external fixtures.
    with tempfile.TemporaryDirectory(prefix='schema-capabilities-') as temporary:
        work = Path(temporary)
        package = work / 'src/foundation/schema-validation'
        shutil.copytree(MANIFEST.parent, package)
        shutil.copytree(ROOT / 'test/foundation/schema-validation', work / 'test/foundation/schema-validation')
        shutil.copytree(ROOT / 'test/schema-validation', work / 'test/schema-validation')
        manifest = package / 'Cargo.toml'
        text = re.sub(r'path = "\.\./([^/"\n]+)"',
                      lambda m: 'path = ' + json.dumps(str(ROOT / 'src/foundation' / m[1])),
                      manifest.read_text())
        manifest.write_text(text + '\n[workspace]\n')
        check(manifest)
        probe = work / 'probe'
        (probe / 'src').mkdir(parents=True)
        cases = [
            ('pattern-api', ['pattern'], 'pub fn check() { let _ = schema_validation::patterns::to_rust_regex("x"); }', True, None),
            ('pattern-no-validator', ['pattern'], 'pub fn check() { let _ = schema_validation::validator_for; }', False, 'validator_for'),
            ('empty-no-values', [], 'use schema_validation::values::Value;', False, 'values'),
            ('removed-macros', ['macros'], '', False, 'does not have'),
        ]
        for name, features, source, success, diagnostic in cases:
            (probe / 'Cargo.toml').write_text('[package]\nname = "schema-boundary-probe"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n[dependencies.schema-validation]\npath = ' + json.dumps(str(package)) + '\ndefault-features = false\nfeatures = ' + json.dumps(features) + '\n')
            (probe / 'src/lib.rs').write_text(source)
            result = subprocess.run(['cargo', 'check', '--offline', '--manifest-path', str(probe / 'Cargo.toml')], cwd=ROOT, capture_output=True, text=True)
            assert (result.returncode == 0) == success, (name, result.stdout, result.stderr)
            if diagnostic:
                assert diagnostic in result.stderr, (name, result.stderr)
            print('Schema compiler boundary:', name, flush=True)

if __name__ == '__main__':
    main()
