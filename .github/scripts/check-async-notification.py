#!/usr/bin/env python3
"""Exercise explicit event/lock capabilities and real SQLite/reactive consumers."""
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'test/async-notification'


def run(args, directory, expected=None, minimum_tests=None):
    result = subprocess.run(['cargo', '--color', 'never', *args], cwd=directory,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=600)
    if expected is not None:
        assert result.returncode != 0 and expected in result.stdout, result.stdout
    else:
        assert result.returncode == 0, result.stdout
        assert 'warning:' not in result.stdout, result.stdout
        if minimum_tests is not None:
            passed = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', result.stdout)))
            assert passed >= minimum_tests, result.stdout
    print(result.stdout, end='', flush=True)
    return result.stdout


def dependency(name, features=()):
    return f'{name}={{path={json.dumps(str(ROOT / "src/foundation" / name))},default-features=false,features={json.dumps(list(features))}}}\n'


def prepare(work, dependencies, source, tests=()):
    (work / 'src').mkdir(exist_ok=True)
    (work / 'src/lib.rs').write_text(source)
    text = (FIXTURES / 'Cargo.toml.in').read_text().replace('@DEPENDENCIES@', dependencies)
    for name, path in tests:
        text += f'\n[[test]]\nname={json.dumps(name)}\npath={json.dumps(str(ROOT / path))}\n'
    (work / 'Cargo.toml').write_text(text)
    run(['generate-lockfile', '--offline'], work)


def check(work):
    event_source = 'pub fn event() { let e = async_notification::event::Event::new(); let _listener = e.listen(); e.notify(1); }'
    lock_source = 'pub fn lock() { let _ = async_notification::RwLock::new(1); }'
    for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
        for features in ((), ('std',), ('locks',), ('std', 'locks')):
            prepare(work, dependency('async-notification', features), event_source + (lock_source if 'locks' in features else ''))
            run(['check', '--offline', '--locked', '--jobs', '1', '--target', target], work)
            tree = run(['tree', '--offline', '--locked', '--target', target, '--prefix', 'none'], work)
            assert ('parking v' in tree) == ('std' in features and target.startswith('x86_64'))
            assert all(old not in tree for old in ('event-listener v', 'event-listener-strategy v', 'async-lock v'))
    prepare(work, dependency('async-notification', ('std',)), lock_source)
    run(['check', '--offline', '--locked'], work, expected='could not find `RwLock`')
    prepare(work, dependency('async-notification', ('std',)), 'pub fn blocked() { use async_notification::event::Listener; async_notification::event::Event::new().listen().wait(); }')
    run(['check', '--offline', '--locked', '--target', 'wasm32-unknown-unknown'], work, expected='no method named `wait`')
    prepare(work, dependency('async-notification', ('std', 'locks')), '', [('behavior', 'test/async-notification/contract.rs')])
    run(['test', '--offline', '--locked', '--jobs', '1', '--test', 'behavior'], work, minimum_tests=13)
    for consumer, features, expected_locks in (('sqlite-access', ('sqlite-bundled',), False), ('reactive-state', (), True)):
        prepare(work, dependency(consumer, features), '')
        raw = subprocess.check_output(['cargo', 'metadata', '--offline', '--locked', '--format-version', '1'], cwd=work, text=True)
        metadata = json.loads(raw)
        package = next(p for p in metadata['packages'] if p['name'] == 'async-notification')
        node = next(n for n in metadata['resolve']['nodes'] if n['id'] == package['id'])
        assert ('locks' in node['features']) == expected_locks, (consumer, node['features'])
        assert 'std' in node['features'], (consumer, node['features'])
    dependencies = dependency('sqlite-access', ('sqlite-bundled',)) + dependency('reactive-state') + dependency('task-dispatch', ('tokio',)) + dependency('tokio', ('macros', 'rt-multi-thread', 'time', 'sync'))
    prepare(work, dependencies, '', [('sqlite_close', 'test/async-notification/consumer.rs'), ('reactive_derived', 'test/foundation/reactive-state/tests/async_derived.rs')])
    run(['test', '--offline', '--locked', '--jobs', '1', '--tests', '--', '--test-threads=1'], work, minimum_tests=8)
    prepare(work, dependency('reactive-state'), 'pub fn guard() { let _ = reactive_state::owner::Owner::new(); }')
    run(['check', '--offline', '--locked', '--jobs', '1', '--target', 'wasm32-unknown-unknown'], work)


def main():
    os.environ.setdefault('CARGO_BUILD_JOBS', '1')
    os.environ.setdefault('CARGO_PROFILE_DEV_DEBUG', '0')
    os.environ.setdefault('CARGO_PROFILE_TEST_DEBUG', '0')
    os.environ.setdefault('CARGO_TARGET_DIR', str(ROOT / 'target/async-notification-check'))
    with tempfile.TemporaryDirectory(prefix='async-notification-') as directory:
        check(Path(directory))
    print('Async notification capability and consumer contracts passed.')


if __name__ == '__main__':
    main()
