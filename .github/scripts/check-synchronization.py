#!/usr/bin/env python3
"""Check real feature isolation, lock behavior and guard-send contracts."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'test/ci/fixtures/synchronization'
FEATURE_MATRIX = (
    (),
    ('atomic_usize',),
    ('arc_lock',),
    ('arc_lock', 'serialization'),
    ('parking',),
    ('locks',),
    ('locks', 'send_guard', 'arc_lock'),
    ('locks', 'hardware-lock-elision'),
)


def run(arguments, directory, expected_success=True, diagnostic=None):
    # Diagnostic matching must not depend on a caller's forced color settings.
    command = [arguments[0], '--color', 'never', *arguments[1:]]
    result = subprocess.run(
        command, cwd=directory, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    if (result.returncode == 0) != expected_success:
        raise RuntimeError(result.stdout)
    if diagnostic is not None and diagnostic not in result.stdout:
        raise RuntimeError('Required diagnostic missing:\n' + result.stdout)
    if expected_success and 'warning:' in result.stdout:
        raise RuntimeError('Unexpected warning:\n' + result.stdout)
    return result.stdout


def dependency(name, features=(), defaults=False):
    path = json.dumps(str(ROOT / 'src/foundation' / name))
    feature_list = json.dumps(list(features))
    default_features = 'true' if defaults else 'false'
    return (
        f'{name}={{path={path},default-features={default_features},'
        f'features={feature_list}}}\n'
    )


def prepare(directory, features, body=None, extra_dependencies=''):
    # Fixtures can live outside the checkout; select its pinned toolchain before Cargo.
    (directory / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
    if body is None:
        body = '#![no_std]\npub use synchronization::api::{RawMutex, GuardNoSend};\n'
    manifest = (
        '[package]\nname="synchronization-check"\nversion="0.0.0"\n'
        'edition="2021"\n[workspace]\n[features]\narc=[]\nserialization=[]\n'
        '[dependencies]\n' + dependency('synchronization', features)
        + extra_dependencies
    )
    (directory / 'Cargo.toml').write_text(manifest)
    (directory / 'src/lib.rs').write_text(body)
    run(['cargo', 'generate-lockfile', '--offline'], directory)


def check_matrix(directory):
    for features in FEATURE_MATRIX:
        prepare(directory, features)
        for target in (None, 'wasm32-unknown-unknown'):
            command = ['cargo', 'check', '--offline', '--locked', '--lib']
            if target is not None:
                command += ['--target', target]
            run(command, directory)
        if not {'parking', 'locks'} & set(features):
            graph = json.loads(run(
                ['cargo', 'metadata', '--offline', '--locked', '--format-version', '1'],
                directory,
            ))
            active = {node['id'] for node in graph['resolve']['nodes']}
            names = {package['name'] for package in graph['packages'] if package['id'] in active}
            assert not names & {'libc', 'smallvec', 'cfg-if', 'tokio'}, names

    for symbol in ('Mutex', 'parking'):
        prepare(directory, (), f'pub use synchronization::{symbol};\n')
        run(
            ['cargo', 'check', '--offline', '--locked'], directory, False,
            f'no `{symbol}` in the root',
        )
    (directory / 'Cargo.toml').write_text(
        '[package]\nname="removed-feature"\nversion="0.0.0"\nedition="2021"\n'
        '[workspace]\n[dependencies]\n'
        + dependency('synchronization', ('deadlock_detection',))
    )
    run(['cargo', 'check', '--offline'], directory, False, 'deadlock_detection')

    guard = (FIXTURES / 'guard_send.rs').read_text()
    prepare(directory, ('locks',), guard)
    run(
        ['cargo', 'check', '--offline', '--locked'], directory, False,
        'cannot be sent between threads safely',
    )
    prepare(directory, ('locks', 'send_guard'), guard)
    run(['cargo', 'check', '--offline', '--locked'], directory)


def check_behavior(directory):
    prepare(
        directory, ('locks', 'arc_lock', 'serialization'),
        extra_dependencies=dependency('json-codec', defaults=True),
    )
    behavior = directory / 'tests/behavior.rs'
    behavior.write_bytes((FIXTURES / 'behavior.rs').read_bytes())
    run(
        ['cargo', 'test', '--offline', '--locked', '--features', 'arc,serialization',
         '--test', 'behavior', '--', '--test-threads=1'],
        directory,
    )
    behavior.unlink()


def check_consumers(directory):
    for standard in (False, True):
        features = ('std',) if standard else ()
        extra = dependency('futures-intrusive', features) + dependency('spin', defaults=True)
        fixture = 'spin_consumer.rs'
        if standard:
            extra += dependency('tokio', ('rt', 'sync', 'parking'))
            extra += dependency('once_cell', ('parking',), defaults=True)
            extra += dependency('tracing-subscriber', ('std', 'registry', 'parking'))
            fixture = 'consumers.rs'
        prepare(
            directory, ('locks',) if standard else (),
            (FIXTURES / fixture).read_text(), extra,
        )
        run(
            ['cargo', 'test' if standard else 'check', '--offline', '--locked', '--lib'],
            directory,
        )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--matrix-only', action='store_true')
    arguments = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='synchronization-check-') as temporary:
        directory = Path(temporary)
        (directory / 'src').mkdir()
        (directory / 'tests').mkdir()
        check_matrix(directory)
        if not arguments.matrix_only:
            check_behavior(directory)
            check_consumers(directory)
    print('Synchronization feature graphs, capability negatives and guard-send contract passed')
    if not arguments.matrix_only:
        print('Six lock behaviors and real native/no-std consumers passed')


if __name__ == '__main__':
    main()
