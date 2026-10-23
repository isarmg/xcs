#!/usr/bin/env python3
"""Exercise the actual host proc macro, target runtime, and removed API boundary."""
from pathlib import Path
import os, shutil, subprocess, tempfile, tomllib
import runpy

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
def check():
    macro = ROOT / 'src/foundation/web-runtime/wasm-binding-macros'
    manifest = tomllib.loads((macro/'Cargo.toml').read_text())
    assert manifest['features'] == {'strict-macro': [], 'extra-traits': []}
    for old in ('wasm-bindgen-macro', 'wasm-bindgen-macro-support'):
        assert not (foundation_path(ROOT, old)).exists()
    env = dict(os.environ)
    env.setdefault('CARGO_BUILD_JOBS', '2')
    env.pop('WASM_BINDGEN_USE_JS_SYS', None)
    def run(args, expected=None):
        result = subprocess.run(['cargo', *args], cwd=ROOT, env=env, text=True, capture_output=True)
        if expected is None:
            assert result.returncode == 0, result.stdout + result.stderr
        else:
            assert result.returncode != 0 and expected in result.stderr, result.stdout + result.stderr
        return result
    # Mirror only the governed package/test tree to avoid creating a nested lock.
    # Every dependency remains the real repository source, never a stub.
    with tempfile.TemporaryDirectory(prefix='zxs-wasm-host-') as temporary:
        work=Path(temporary);mirror=work/'src/foundation/web-runtime/wasm-binding-macros'
        shutil.copytree(macro, mirror)
        shutil.copytree(ROOT/'test/foundation/wasm-binding-macros', work/'test/foundation/wasm-binding-macros')
        shutil.copytree(ROOT/'test/wasm-binding-macros', work/'test/wasm-binding-macros')
        text=(mirror/'Cargo.toml').read_text()
        for row in manifest['dependencies'].values():
            text=text.replace(f'path = "{row["path"]}"', f'path = "{(macro/row["path"]).resolve()}"')
        (mirror/'Cargo.toml').write_text(text)
        flags=['--offline','--manifest-path',str(mirror/'Cargo.toml')]
        run(['generate-lockfile',*flags])
        for features in ([], ['--all-features']):
            result = run(['test','--locked',*flags,'--lib',*features])
            assert '38 passed' in result.stdout, result.stdout
            assert 'warning:' not in result.stderr, result.stderr
    with tempfile.TemporaryDirectory(prefix='zxs-wasm-macro-') as temporary:
        work=Path(temporary);(work/'src').mkdir()
        manifest=work/'Cargo.toml';source=work/'src/lib.rs'
        manifest.write_text('[package]\nname="wasm-macro-contract"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[features]\nstrict=["wasm-bindgen/strict-macro"]\n[dependencies]\nwasm-bindgen={path="'+str(ROOT/'src/foundation/web-runtime/wasm-bindgen')+'"}\nwasm-binding-macros={path="'+str(macro)+'"}\njs-sys={path="'+str(ROOT/'src/foundation/web-runtime/js-sys')+'",default-features=false,features=["std"]}\n')
        flags=['--offline','--manifest-path',str(manifest)]
        source.write_text('');run(['generate-lockfile',*flags]);flags+=['--locked']
        source.write_text((ROOT/'test/wasm-binding-macros/native.rs').read_text())
        run(['test',*flags]);run(['check',*flags,'--target','wasm32-unknown-unknown'])
        source.write_text((ROOT/'test/wasm-binding-macros/async-runtime.rs').read_text())
        run(['test',*flags]);run(['check',*flags,'--target','wasm32-unknown-unknown'])
        for attribute, item in [('start', 'pub async fn start() {}'), ('main', 'async fn main() {}')]:
            source.write_text(f'use wasm_bindgen::prelude::*;\n#[wasm_bindgen({attribute})] {item}\n')
            run(['check',*flags]);run(['check',*flags,'--target','wasm32-unknown-unknown'])
        # Broad existing frontend fixture: structs, getters/setters, enums,
        # Unicode, custom sections and payload unions through the canonical ABI.
        source.write_text((ROOT/'test/tools/web-bindings/fixtures/contract.rs').read_text())
        run(['check',*flags]);run(['check',*flags,'--target','wasm32-unknown-unknown','--features','strict'])
        source.write_text((ROOT/'test/wasm-binding-macros/strict.rs').read_text())
        result=run(['check',*flags]);assert 'unused variable: `catch`' in result.stderr, result.stderr
        run(['check',*flags,'--features','strict'], 'unused wasm_bindgen attribute catch')
        negatives=[
            ('#[wasm_bindgen(not_a_real_option)] pub fn bad() {}', 'unknown attribute'),
            ('#[wasm_bindgen] pub fn bad<T>(value: T) {}', 'lifetime or type parameters'),
            ('#[wasm_bindgen] pub const fn bad() {}', 'const'),
            ('pub fn bad() -> String { wasm_bindgen::link_to!(); }', 'requires a module'),
            ('#[wasm_bindgen(js_name = 42)] pub fn bad() {}', 'expected'),
            ('use wasm_binding_macros::expand;', 'unresolved import'),
            ('#[wasm_bindgen(experimental_tokio)] pub async fn bad() {}', 'experimental_tokio is unsupported'),
            ('#[wasm_bindgen(experimental_tokio = "isolated")] pub async fn bad() {}', 'experimental_tokio is unsupported'),
            ('#[wasm_bindgen(jspi, experimental_tokio)] pub async fn bad() {}', 'experimental_tokio is unsupported'),
            ('#[wasm_bindgen(start, experimental_tokio)] pub async fn bad() {}', 'experimental_tokio is unsupported'),
            ('#[wasm_bindgen(wasm_bindgen_futures = legacy)] pub async fn bad() {}', 'wasm_bindgen_futures is retired'),
        ]
        for code,diagnostic in negatives:
            source.write_text('use wasm_bindgen::prelude::*;\n'+code+'\n')
            run(['check',*flags], diagnostic)
            run(['check',*flags,'--target','wasm32-unknown-unknown'], diagnostic)
    print('WASM macro boundary: 38+38 host tests, native execution, native/WASM sync/async/import/start/main fixtures, and explicit removed-runtime diagnostics passed')
if __name__ == '__main__': check()
