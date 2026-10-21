#!/usr/bin/env python3
"""Verify unified serialization capabilities, exact derives and attributed tests."""
import json
import os
import shutil
from pathlib import Path
import runpy
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PROFILES = {'core': [], 'alloc': ['alloc'], 'rc': ['alloc', 'rc'],
            'derive': ['derive'], 'path': ['error-path'],
            'std': ['std'], 'full': ['std', 'derive', 'rc', 'error-path']}
TARGETS = ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown')


def manifest(root, features, tests=False):
    data = root / 'src/foundation/data-serialization'
    result = ('[package]\nname="serialization-consolidation-probe"\nversion="0.0.0"\n'
              'edition="2021"\n[workspace]\n[dependencies]\n'
              f'data-serialization={{path={json.dumps(str(data))}, default-features=false, '
              f'features={json.dumps(features)}}}\n')
    if tests:
        result += 'log={path='+json.dumps(str(root/'src/foundation/log'))+',features=["serialization"]}\n'
        result += 'url={path='+json.dumps(str(root/'src/foundation/url'))+',features=["serialization"]}\n'
        result += 'fraction={path='+json.dumps(str(root/'src/foundation/fraction'))+',default-features=false,features=["with-serialization-support"]}\n'
        result += 'json-codec={path=' + json.dumps(str(root / 'src/foundation/json-codec')) + ',features=["arbitrary_precision","raw_value"]}\n'
    return result


def source(profile):
    text = '#![no_std]\nuse data_serialization::{Serialize, Deserialize};\n'
    text += 'fn traits<T: Serialize + for<\'a> Deserialize<\'a>>() {}\npub fn verify() { traits::<Result<u128, i128>>(); traits::<core::net::Ipv6Addr>(); traits::<core::num::Saturating<u64>>(); }\n'
    if profile in ('alloc', 'rc', 'path', 'std', 'full'):
        text += 'extern crate alloc;\npub fn allocated() { traits::<alloc::vec::Vec<u64>>(); }\n'
    if profile in ('rc', 'full'):
        text += 'pub fn shared() { traits::<alloc::rc::Rc<u64>>(); traits::<alloc::sync::Arc<u64>>(); }\n'
    if profile in ('derive', 'full'):
        text += '#[derive(Serialize, Deserialize)]\npub struct Derived { value: u64 }\n'
    if profile in ('path', 'full'):
        text += 'pub fn track() -> data_serialization::error_path::Track { data_serialization::error_path::Track::new() }\n'
    return text


def validate_source(root):
    package = root / 'src/foundation/data-serialization'
    data = tomllib.loads((package / 'Cargo.toml').read_text())
    if data['package'].get('build') is not False or (package / 'build.rs').exists():
        raise AssertionError('unified serialization must not execute an obsolete build script')
    if data['package']['rust-version'] != '1.99':
        raise AssertionError('modern compiler assumptions require the project toolchain minimum')
    for name in ('serialization-core', 'serialization-error-path'):
        if (root / 'src/foundation' / name).exists():
            raise AssertionError('old serialization package remains')
    if (package / 'src/core').exists():
        raise AssertionError('duplicate documentation-only implementation remains')
    for path in (package / 'src').rglob('*.rs'):
        text = path.read_text()
        if 'env!("OUT_DIR")' in text or 'no_serialization_derive' in text or 'no_core_net' in text:
            raise AssertionError('obsolete generated support or compiler compatibility remains')


def run_cargo(args, root, env, expected=True):
    result = subprocess.run(['cargo', *args], cwd=root, env=env, text=True, capture_output=True)
    if expected and result.returncode:
        raise AssertionError(result.stdout + result.stderr)
    if expected and ('warning:' in result.stderr or 'warning[' in result.stderr):
        raise AssertionError('unexpected Rust warning: ' + result.stderr)
    return result


def check(root=ROOT):
    validate_source(root)
    env = runpy.run_path(str(root / '.github/scripts/backend_environment.py'))['matrix_environment']()
    env.update(CARGO_BUILD_JOBS='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_TERM_COLOR='never')
    (root / 'target').mkdir(exist_ok=True)
    target = root / 'target/serialization-consolidation'
    with tempfile.TemporaryDirectory(prefix='serialization-consolidation-', dir=root / 'target') as directory:
        work = Path(directory); (work / 'src').mkdir()
        for profile, features in PROFILES.items():
            (work / 'Cargo.toml').write_text(manifest(root, features))
            (work / 'src/lib.rs').write_text(source(profile))
            run_cargo(['generate-lockfile', '--offline', '--manifest-path', str(work / 'Cargo.toml')], root, env)
            for triple in TARGETS:
                run_cargo(['check', '--locked', '--offline', '--manifest-path', str(work / 'Cargo.toml'), '--target', triple, '--target-dir', str(target), '-j1'], root, env)
            graph = run_cargo(['tree','--locked','--offline','--manifest-path',str(work/'Cargo.toml'),'--edges','normal','--prefix','none'],root,env).stdout
            if profile in ('core','alloc','rc','std') and any(name in graph for name in ('itoa ', 'serialization-derive ', 'rust-syntax ', 'json-codec ')):
                raise AssertionError('lightweight capability pulled unrelated implementation')
            print(f'{profile}: native/WASM and dependency boundary pass', flush=True)
        (work/'Cargo.toml').write_text(manifest(root, []))
        run_cargo(['generate-lockfile','--offline','--manifest-path',str(work/'Cargo.toml')],root,env)
        for code, expected in [
            ('pub use data_serialization::error_path::Track;', 'error_path'),
            ('pub use data_serialization::__derive_support::de;', 'de'),
            ('#[derive(data_serialization::Serialize)] pub struct NoDerive;', 'Serialize'),
            ('data_serialization::serialization_if_integer128! { pub struct Legacy; }', 'serialization_if_integer128'),
        ]:
            (work/'src/lib.rs').write_text(code)
            result=run_cargo(['check','--locked','--offline','--manifest-path',str(work/'Cargo.toml'),'--target-dir',str(target),'-j1'],root,env,False)
            if result.returncode == 0 or expected not in result.stderr or not re.search(r'error\[E043[23]\]', result.stderr):
                raise AssertionError('missing capability rejection '+expected+': '+result.stderr)
        (work/'Cargo.toml').write_text(manifest(root, PROFILES['full'], tests=True))
        text=(root/'test/ci/fixtures/serialization-consolidation.rs').read_text()
        for name in ('deserialize','serialize'):
            path=root/f'test/foundation/data-serialization/error-path/tests/{name}.rs'
            text+='\n#[path = '+json.dumps(str(path))+']\nmod original_'+name+';\n'
        (work/'src/lib.rs').write_text(text)
        run_cargo(['generate-lockfile','--offline','--manifest-path',str(work/'Cargo.toml')],root,env)
        result=run_cargo(['test','--locked','--offline','--manifest-path',str(work/'Cargo.toml'),'--target-dir',str(target),'-j1','--lib'],root,env)
        print(result.stdout)
    # Compile the exact library source in a disposable harness. Original include!
    # paths retain their same-depth test layout; no Cargo.lock is written in src.
    with tempfile.TemporaryDirectory(prefix='serialization-source-tests-', dir=root/'target') as directory:
        stage=Path(directory)
        package=stage/'src/foundation/serialization-source-tests'
        package.mkdir(parents=True)
        shutil.copytree(root/'test/foundation/data-serialization',stage/'test/foundation/data-serialization')
        main=tomllib.loads((root/'src/foundation/data-serialization/Cargo.toml').read_text())
        text=manifest(root,PROFILES['full'],tests=True).replace('serialization-consolidation-probe','serialization-source-tests')
        text+='serialization-derive={path='+json.dumps(str(root/'src/foundation/serialization-derive'))+'}\n'
        text+='itoa={path='+json.dumps(str(root/'src/foundation/itoa'))+'}\n'
        text+='[lib]\npath='+json.dumps(str(root/'src/foundation/data-serialization/src/lib.rs'))+'\n'
        text+='[features]\ndefault=["std","derive","rc","error-path"]\nstd=["alloc"]\nalloc=[]\nderive=[]\nrc=[]\nerror-path=["alloc"]\n'
        (package/'Cargo.toml').write_text(text)
        run_cargo(['generate-lockfile','--offline','--manifest-path',str(package/'Cargo.toml')],root,env)
        for scope in ('--lib', '--doc'):
            result=run_cargo(['test','--locked','--offline','--manifest-path',str(package/'Cargo.toml'),scope,'--target-dir',str(target),'-j1'],root,env)
            print(result.stdout)
    print('Unified serialization capabilities, original path tests and semantic regressions pass')


if __name__ == '__main__':
    check()
