#!/usr/bin/env python3
"""Check ICU runtime/table consolidation without changing root source or lock files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown')


def run(root, args, *, capture=False, env=None, negative=False):
    result = subprocess.run(['cargo', *args, '--color', 'never'], cwd=root,
                            capture_output=capture, text=True, env=env, timeout=600)
    if not negative:
        result.check_returncode()
    elif result.returncode == 0:
        raise AssertionError('provider-only build unexpectedly exposes baked data')
    return result


def validate_graph(metadata, name, compiled, tree):
    nodes = {row['id']: row for row in metadata['resolve']['nodes']}
    packages = {row['name']: row for row in metadata['packages'] if row['id'] in nodes}
    if name.replace('unicode-', 'icu_') + '_data' in packages:
        raise AssertionError('absorbed generated-data package remains active')
    own = nodes[packages[name]['id']]['features']
    if ('compiled_data' in own) != compiled:
        raise AssertionError('compiled_data feature isolation failed')
    if ('baked' in nodes[packages['icu_provider']['id']]['features']) != compiled:
        raise AssertionError('provider-only build activates baked support')
    # Cargo metadata includes weak optional feature edges; cargo tree reports
    # the actually built normal dependencies without this over-approximation.
    if name == 'unicode-normalizer' and 'unicode-properties' in tree:
        raise AssertionError('minimal normalizer unexpectedly activates optional properties')


def validate_table_inputs(root, family):
    """Table bytes and build selection stay equal to their exact input receipt."""
    catalog = json.loads((root / 'compliance/rust-sources.json').read_text())
    destination = 'src/foundation/unicode-' + family
    matches = [record for record in catalog['consolidations'].values()
               if record['destination_directory'] == destination]
    if len(matches) != 1:
        raise AssertionError('Unicode data consolidation receipt missing or ambiguous')
    record = matches[0]
    old = 'src/foundation/icu_' + family + '_data'
    package = next(row['package'] for row in record['inputs'] if row['package']['directory'] == old)
    for source, expected in package['files_sha256'].items():
        if source.startswith('data/') or source == 'build.rs':
            target = record['file_routes'][old][source]['destination']
            actual = hashlib.sha256((root / destination / target).read_bytes()).hexdigest()
            if actual != expected:
                raise AssertionError('generated Unicode table/build input changed: ' + source)


def check_family(root, temporary, target, family):
    validate_table_inputs(root, family)
    name = 'unicode-' + family
    crate = 'unicode_' + family
    if (root / 'src/foundation' / (name.replace('unicode-', 'icu_') + '_data')).exists():
        raise AssertionError('retired generated-data source package remains')
    feature_sets = ((), ('compiled_data',)) if family == 'normalizer' else ((), ('alloc',), ('compiled_data',), ('compiled_data', 'alloc'))
    for features in feature_sets:
        compiled = 'compiled_data' in features
        work = temporary / (family + '-' + ('-'.join(features) or 'provider'))
        (work / 'src').mkdir(parents=True)
        lib = work / 'src/lib.rs'
        lib.write_text('#![no_std]\npub use ' + crate + '::provider;\n')
        manifest = ('[package]\nname = "unicode-data-probe"\nversion = "0.0.0"\n'
                    'edition = "2024"\n[workspace]\n[dependencies.' + name + ']\n'
                    'path = ' + json.dumps(str(root / 'src/foundation' / name)) + '\n'
                    'default-features = false\nfeatures = ' + json.dumps(features) + '\n'
                    '[profile.dev]\ndebug = 0\nincremental = false\n'
                    '[profile.test]\ndebug = 0\nincremental = false\n')
        (work / 'Cargo.toml').write_text(manifest)
        common = ['--offline', '--manifest-path', str(work / 'Cargo.toml')]
        run(root, ['generate-lockfile', *common])
        metadata = run(root, ['metadata', '--locked', '--format-version', '1', *common], capture=True)
        tree = run(root, ['tree', '--locked', *common, '--edges', 'normal', '--prefix', 'none', '--format', '{p}'], capture=True)
        validate_graph(json.loads(metadata.stdout), name, compiled, tree.stdout)
        for platform in TARGETS:
            check = ['check', '--locked', *common, '--jobs', '1', '--target', platform,
                     '--target-dir', str(target)]
            run(root, check)
        if compiled:
            # Use the real unchanged generated tables, never a fabricated provider.
            custom = dict(os.environ, ICU4X_DATA_DIR=str(root / 'src/foundation' / name / 'src/data'))
            run(root, check, env=custom)
        else:
            lib.write_text('#![no_std]\npub use ' + crate + '::provider::Baked;\n')
            failure = run(root, check, capture=True, negative=True)
            if 'E0432' not in failure.stderr or 'Baked' not in failure.stderr:
                raise AssertionError('negative baked-data probe failed for an unrelated reason')
    work = temporary / (family + '-behavior')
    work.mkdir()
    features = ['compiled_data', 'utf8_iter'] if family == 'normalizer' else ['compiled_data', 'alloc']
    manifest = ('[package]\nname = "unicode-data-behavior"\nversion = "0.0.0"\n'
                'edition = "2024"\n[workspace]\n[dependencies.' + name + ']\n'
                'path = ' + json.dumps(str(root / 'src/foundation' / name)) + '\n'
                'default-features = false\nfeatures = ' + json.dumps(features) + '\n')
    source = root / ('test/unicode-data/' + family + '.rs')
    manifest += ('[[test]]\nname = "unicode_behavior"\npath = ' + json.dumps(str(source)) + '\n'
                 '[profile.dev]\ndebug = 0\nincremental = false\n'
                 '[profile.test]\ndebug = 0\nincremental = false\n')
    (work / 'Cargo.toml').write_text(manifest)
    common = ['--offline', '--manifest-path', str(work / 'Cargo.toml')]
    run(root, ['generate-lockfile', *common])
    run(root, ['test', '--locked', *common, '--jobs', '1', '--target-dir', str(target)])
    if family == 'properties':
        run(root, ['test', '--locked', *common, '--lib', '-p', name, '--jobs', '1', '--target-dir', str(target)])


    # Documented public parser APIs require unstable; keep minimal graph probes above separate.
    doc_features = ['compiled_data', 'utf8_iter', 'unicode-properties'] if family == 'normalizer' else ['compiled_data', 'alloc', 'unstable']
    (work / 'Cargo.toml').write_text(manifest.replace('features = ' + json.dumps(features), 'features = ' + json.dumps(doc_features)))
    run(root, ['generate-lockfile', *common])
    run(root, ['test', '--locked', *common, '--doc', '-p', name, '--jobs', '1', '--target-dir', str(target)])


def check(root=ROOT, families=('normalizer', 'properties')):
    root = Path(root).resolve()
    lock = hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest()
    scratch = root / 'target/scope-consumers'
    scratch.mkdir(parents=True, exist_ok=True)
    target = Path(os.environ.get('ICU_FAMILY_TARGET_DIR', str(root / 'target/unicode-data-consumers')))
    with tempfile.TemporaryDirectory(prefix='unicode-data-', dir=scratch) as directory:
        for family in families:
            check_family(root, Path(directory), target, family)
    if hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest() != lock:
        raise AssertionError('Unicode gate changed the product lock file')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--family', choices=('normalizer', 'properties'))
    arguments = parser.parse_args()
    check(families=(arguments.family,) if arguments.family else ('normalizer', 'properties'))
