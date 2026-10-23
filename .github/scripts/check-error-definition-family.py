#!/usr/bin/env python3
"""Keep the stable hidden derive ABI exact and exercise real error semantics."""
import json
import runpy
import subprocess
import tempfile
import tomllib
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
rebase_dependency_paths = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['rebase_dependency_paths']
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
NAMES = ('error-definition', 'error-definition-macros')

def validate_private_family(manifests):
    runtime, macros = (manifests[name] for name in NAMES)
    version = runtime['package']['version']
    if macros['package']['version'] != version:
        raise AssertionError('error definition private family package versions must match')
    dependency = runtime['dependencies'][NAMES[1]]
    if dependency.get('version') != '=' + version or dependency.get('package', NAMES[1]) != NAMES[1]:
        raise AssertionError('error-definition-macros must be exactly pinned for the shared private ABI')
    if dependency.get('path') != '../error-definition-macros':
        raise AssertionError('derive implementation must use the maintained local path')

def check_private_family(root):
    validate_private_family({name: tomllib.loads((foundation_path(root, name)/'Cargo.toml').read_text()) for name in NAMES})

def check_skew_rejected(root, base, environment):
    # A real Cargo resolver probe supplements the manifest gate's negative tests.
    import shutil
    runtime = base/'skew/error-definition'
    macros = base/'skew/error-definition-macros'
    for destination in (runtime, macros):
        shutil.copytree(foundation_path(root, destination.name), destination)
    manifest = macros/'Cargo.toml'
    original = manifest.read_text()
    import re
    original = rebase_dependency_paths(original, foundation_path(root, macros.name))
    original += '\n[workspace]\n'
    (runtime/'Cargo.toml').write_text((runtime/'Cargo.toml').read_text()+'\n[workspace]\n')
    version = tomllib.loads(original)['package']['version']
    for changed in ('2.0.22', '2.1.21', '3.0.21'):
        manifest.write_text(original.replace('version = "'+version+'"', 'version = "'+changed+'"', 1))
        result = subprocess.run(['cargo','generate-lockfile','--offline','--manifest-path',str(runtime/'Cargo.toml')],cwd=root,env=environment,capture_output=True,text=True)
        if result.returncode == 0 or 'failed to select a version' not in result.stderr or 'error-definition-macros' not in result.stderr:
            raise AssertionError('skewed derive implementation unexpectedly resolved: '+result.stderr)
        print('Confirmed Cargo rejection of macro version '+changed, flush=True)

def check(root):
    check_private_family(root)
    environment = runpy.run_path(str(root/'.github/scripts/backend_environment.py'))['matrix_environment']()
    environment['RUSTFLAGS'] = '--check-cfg=cfg(error_definition_nightly_testing) -Dwarnings'
    (root/'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='error-definition-', dir=root/'target') as temporary:
        base = Path(temporary)
        check_skew_rejected(root, base, environment)
        work = base/'consumer'; (work/'src').mkdir(parents=True)
        deps = '\n'.join(f'{name} = {{ path = {json.dumps(str(foundation_path(root, name)))} }}' for name in ('error-definition','anyhow','ref-cast'))
        tests = sorted((root/'test/foundation/error-definition/tests').glob('test_*.rs'))
        manifest = '[package]\nname="error-definition-regression"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[features]\ndefault=["std"]\nstd=[]\n[dependencies]\n'+deps+'\n'
        for test in tests:
            manifest += '\n[[test]]\nname='+json.dumps(test.stem)+'\npath='+json.dumps(str(test))+'\n'
        (work/'Cargo.toml').write_text(manifest)
        docs = '\n'.join(line[3:] for line in (root/'src/foundation/diagnostics/error-definition/src/lib.rs').read_text().splitlines() if line.startswith('//!'))
        (work/'src/lib.rs').write_text('#![doc = ' + json.dumps(docs, ensure_ascii=False) + ']\n')
        def run(args, env=environment):
            subprocess.run(['cargo',*args,'--offline','--manifest-path',str(work/'Cargo.toml'),'--target-dir',str(base/'build'),'-j2'],cwd=root,env=env,check=True)
        run(['test','--target','x86_64-unknown-linux-gnu'])
        # Exercise unstable generic-member/backtrace code explicitly, independently of normal stable tests.
        nightly = dict(environment, RUSTC_BOOTSTRAP='1', RUSTFLAGS=environment['RUSTFLAGS']+' --cfg error_definition_nightly_testing')
        run(['test','--target','x86_64-unknown-linux-gnu'], nightly)
        (work/'Cargo.toml').write_text('[package]\nname="error-definition-no-std"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nerror-definition={path='+json.dumps(str(root/'src/foundation/diagnostics/error-definition'))+',default-features=false}\n')
        (work/'src/lib.rs').write_text('#![no_std]\n#[derive(Debug,error_definition::Error)]\n#[error("error {code}")]\npub struct Failure { pub code: u32 }\n')
        for target in ('x86_64-unknown-linux-gnu','wasm32-unknown-unknown'):
            run(['check','--target',target])

if __name__ == '__main__':
    check(ROOT)
