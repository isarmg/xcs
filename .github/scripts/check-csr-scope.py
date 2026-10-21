#!/usr/bin/env python3
"""Prove the CSR/template contract and reject removed server-only APIs/features."""
from pathlib import Path
import os
import hashlib
import json
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
TARGET = ROOT / 'target/csr-template-check'


def cargo(*args, capture=False):
    return subprocess.run(['cargo', *args], cwd=ROOT, check=not capture,
                          capture_output=capture, text=capture)


def check():
    TARGET.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='csr-scope-', dir=ROOT / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname="csr-template-contract"\nversion="0.0.0"\nedition="2021"\n'
                            '[workspace]\n[lib]\ncrate-type=["cdylib", "rlib"]\n[dependencies]\n' + '\n'.join([
            'tachys={path="' + str(ROOT / 'src/foundation/tachys') + '",features=["reactive_graph","oco"]}',
            'reactive_graph={path="' + str(ROOT / 'src/foundation/reactive_graph') + '",features=["effects"]}',
            'wasm-bindgen={path="' + str(ROOT / 'src/foundation/wasm-bindgen') + '"}',
            'wasm-bindgen-futures={path="' + str(ROOT / 'src/foundation/wasm-bindgen-futures') + '"}',
            'js-sys={path="' + str(ROOT / 'src/foundation/js-sys') + '"}',

            'web-sys={path="' + str(ROOT / 'src/foundation/web-sys') + '",features=["Window","Document","HtmlInputElement","Event"]}',
        ]) + '\n')
        source = work / 'src/lib.rs'
        source.write_text((ROOT / 'tools/web-bindings/tests/fixtures/csr-template.rs').read_text())
        cargo('generate-lockfile', '--offline', '--manifest-path', str(manifest))
        command = ['check', '--locked', '--offline', '--manifest-path', str(manifest), '--target-dir', str(TARGET)]
        cargo(*command)
        cargo("test", "--locked", "--offline", "--lib", "--manifest-path", str(manifest), "--target-dir", str(TARGET))
        cargo('build', '--locked', '--offline', '--release', '--target', 'wasm32-unknown-unknown',
              '--manifest-path', str(manifest), '--target-dir', str(TARGET))
        output = TARGET / 'output'
        if output.exists():
            shutil.rmtree(output)
        wasm = TARGET / 'wasm32-unknown-unknown/release/csr_template_contract.wasm'
        arguments = [str(wasm), '--target', 'web', '--no-typescript', '--out-name', 'fixture', '--out-dir', str(output)]
        if tool := os.environ.get('ZXS_WEB_BINDGEN'):
            subprocess.run([tool, *arguments], cwd=ROOT, check=True)
        else:
            cargo('run', '--locked', '--offline', '-p', 'web-bindings', '--', *arguments)
        (output / 'index.html').write_text('''<!doctype html><meta charset="utf-8"><body>pending<script type="module">
try { const api=await import('./fixture.js'); await api.default(); document.body.textContent=await api.run_template_probe(); }
catch(error) { document.body.textContent=JSON.stringify({status:'failed',message:String(error),stack:error?.stack}); }
</script>''')
        retired = {
            'server_module': 'use tachys::ssr;',
            'nightly_static_views': 'use tachys::view::static_types;',
            'nightly_static_attributes': 'use tachys::html::attribute::StaticAttr;',
            'islands': 'use tachys::html::islands;',
            'serialized_keys': 'use tachys::view::keyed::SerializableKey;',
            'doctype': 'pub fn old() { tachys::html::doctype("html"); }',
            'server_hydration': 'pub fn old<T:tachys::view::RenderHtml>(value:T) { value.hydrate::<true>(); }',
            'attribute_output': 'pub fn old<T:tachys::html::attribute::Attribute>(value:T) { value.to_html(); }',
        }
        for method in ['to_html', 'to_html_with_buf', 'to_html_branching', 'to_html_stream_in_order',
                       'to_html_stream_out_of_order', 'to_html_async_with_buf', 'resolve', 'dry_resolve', 'hydrate_async']:
            retired[method] = f'pub fn old<T:tachys::view::RenderHtml>(mut value:T) {{ value.{method}(); }}'
        for name, code in {
            'typed_canvas': 'use tachys::html::element::canvas;',
            'typed_audio': 'use tachys::html::element::audio;',
            'typed_select': 'use tachys::html::element::select;',
            'typed_svg_animation': 'use tachys::svg::animate;',
            'typed_svg_use': 'use tachys::svg::Use;',
            'typed_mathml': 'use tachys::mathml;',
            'inline_event_attributes': 'use tachys::html::attribute::GlobalOnAttributes;',
            'typed_touch_event': 'use tachys::html::event::touchstart;',
            'typed_touch_type': 'use tachys::html::event::TouchEvent;',
            'unused_attribute': 'use tachys::html::attribute::accept;',
            'removed_web_api': 'use web_sys::WebGlRenderingContext;',
        }.items():
            retired[name] = code
        for name, code in retired.items():
            source.write_text(code)
            result = cargo(*command, capture=True)
            if not result.returncode or not any(code in result.stderr for code in ['error[E0432]', 'error[E0425]', 'error[E0599]', 'error[E0107]']):
                raise AssertionError(f'{name} accepted or failed for another reason: {result.stderr}')
        source.write_text('')
        for feature in ['ssr', 'hydrate', 'islands', 'mark_branches', 'testing', 'sledgehammer', 'reactive_stores', 'error-hook', 'nightly']:
            result = cargo(*command, '--features', 'tachys/' + feature, capture=True)
            if not result.returncode or 'does not have that feature' not in result.stderr:
                raise AssertionError(f'{feature} accepted or failed for another reason: {result.stderr}')
        for feature in ['AudioContext', 'WebGlRenderingContext', 'Gamepad', 'XrSession']:
            result = cargo(*command, '--features', 'web-sys/' + feature, capture=True)
            if not result.returncode or 'does not have that feature' not in result.stderr:
                raise AssertionError(f'{feature} accepted or failed for another reason: {result.stderr}')
        policy = json.loads((ROOT / 'tools/rust-sources/web-scope.json').read_text())
        declared = tomllib.loads((ROOT / 'src/foundation/web-sys/Cargo.toml').read_text())['features']
        if set(declared) != set(policy['web_sys_features']):
            raise AssertionError('web-sys feature policy drift')
        generated = {p.stem.removeprefix('gen_') for p in (ROOT / 'src/foundation/web-sys/src/features').glob('gen_*.rs')}
        if generated != set(declared) - {'default', 'std'}:
            raise AssertionError('web-sys source/feature closure drift')
        record = {
            'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'git_tree': subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], cwd=ROOT, text=True).strip(),
            'worktree_has_changes': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)),
            'source_receipt_sha256': hashlib.sha256((ROOT / 'compliance/rust-sources.json').read_bytes()).hexdigest(),
            'web_scope_sha256': hashlib.sha256((ROOT / 'tools/rust-sources/web-scope.json').read_bytes()).hexdigest(),
            'retired_api_checks': len(retired),
            'retired_feature_checks': 13,
            'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
            'fixture_source_sha256': hashlib.sha256((ROOT / 'tools/web-bindings/tests/fixtures/csr-template.rs').read_bytes()).hexdigest(),
            'outputs_sha256': {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                               for name in ['fixture.js', 'fixture_bg.wasm']},
        }
        (output / 'validation.json').write_text(json.dumps(record, indent=2) + '\n')
        print(f'CSR contract: browser template compiled; {len(retired)} retired API and 13 feature checks passed')


if __name__ == '__main__':
    check()
