#!/usr/bin/env python3
"""Compile the actual Web generator contract and reject retired lower-level APIs."""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def cargo(*arguments, capture=False):
    return subprocess.run(['cargo', *arguments], cwd=ROOT, check=not capture,
                          capture_output=capture, text=capture)


def check():
    (ROOT / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='web-bindgen-scope-', dir=ROOT / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname="web-bindgen-scope"\nversion="0.0.0"\nedition="2021"\n'
                            '[workspace]\n[dependencies]\nwasm-bindgen-cli-support={path="' +
                            str(ROOT / 'src/foundation/wasm-bindgen-cli-support') + '"}\n')
        source = work / 'src/lib.rs'
        source.write_text('pub fn generator() { let _ = wasm_bindgen_cli_support::Bindgen::new(); }')
        cargo('generate-lockfile', '--offline', '--manifest-path', str(manifest))
        command = ['check', '--locked', '--offline', '--manifest-path', str(manifest),
                   '--target-dir', str(ROOT / 'target/web-bindgen-scope-check')]
        cargo(*command)
        retired = {
            'typescript': 'pub fn old() { wasm_bindgen_cli_support::Bindgen::new().typescript(true); }',
            'typed_array_declarations': 'pub fn old() { wasm_bindgen_cli_support::Bindgen::new().ts_typed_array_buffers(true); }',
            'typescript_output': 'pub fn old(output: wasm_bindgen_cli_support::Output) { output.ts(); }',
            'backend_start_output': 'pub fn old(output: wasm_bindgen_cli_support::Output) { output.start(); }',
            'wasm2es6js': 'use wasm_bindgen_cli_support::wasm2es6js;',
        }
        for name in ['nodejs', 'nodejs_module', 'deno', 'bundler', 'no_modules', 'module', 'browser', 'web']:
            retired[name] = f'pub fn old() {{ wasm_bindgen_cli_support::Bindgen::new().{name}(true); }}'
        retired['no_modules_global'] = 'pub fn old() { wasm_bindgen_cli_support::Bindgen::new().no_modules_global("old"); }'
        for name, code in retired.items():
            source.write_text(code)
            result = cargo(*command, capture=True)
            if not result.returncode or not any(error in result.stderr for error in ('error[E0599]', 'error[E0432]')):
                raise AssertionError(f'{name} was accepted or failed for another reason: {result.stderr}')

        fixture = work / 'fixture'
        (fixture / 'src').mkdir(parents=True)
        fixture_manifest = fixture / 'Cargo.toml'
        fixture_manifest.write_text('[package]\nname="web-binding-contract-fixture"\nversion="0.0.0"\nedition="2021"\n'
                                    '[workspace]\n[lib]\ncrate-type=["cdylib"]\n[dependencies]\nwasm-bindgen={path="' +
                                    str(ROOT / 'src/foundation/wasm-bindgen') + '"}\n')
        (fixture / 'src/lib.rs').write_text((ROOT / 'tools/web-bindings/tests/fixtures/contract.rs').read_text())
        cargo('generate-lockfile', '--offline', '--manifest-path', str(fixture_manifest))
        target = ROOT / 'target/web-binding-contract'
        cargo('build', '--locked', '--offline', '--release', '--target', 'wasm32-unknown-unknown',
              '--manifest-path', str(fixture_manifest), '--target-dir', str(target))
        output = work / 'output'
        cargo('run', '--locked', '--offline', '-p', 'web-bindings', '--',
              str(target / 'wasm32-unknown-unknown/release/web_binding_contract_fixture.wasm'),
              '--target', 'web', '--no-typescript', '--out-name', 'fixture', '--out-dir', str(output))
        assert {p.name for p in output.iterdir()} == {'fixture.js', 'fixture_bg.wasm'}
        js = (output / 'fixture.js').read_text()
        for required in ['get value()', 'set value(', 'class Widget', 'Object.freeze(',
                         '__wbindgen_enum_Mode', '文件 Ω', 'function mode_roundtrip(',
                         'function message_roundtrip(', 'function unicode_roundtrip(']:
            assert required in js, f'missing runtime binding: {required}'
        for forbidden in ['DeclarationMustNotLeak', '@ts-self-types', 'addToLibrary(', 'require(', 'Deno.']:
            assert forbidden not in js, f'unsupported output returned: {forbidden}'
        print(f'Web/no-TypeScript contract: runtime fixture and {len(retired)} negative API groups passed')


if __name__ == '__main__':
    check()
