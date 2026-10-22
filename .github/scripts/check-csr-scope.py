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
TARGET = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target/csr-template-check'))


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
            'view-renderer={path="' + str(ROOT / 'src/foundation/view-renderer') + '",features=["reactive-state","shared-value"]}',
            'dom-helpers={path="' + str(ROOT / 'src/foundation/dom-helpers') + '"}',
            'reactive-state={path="' + str(ROOT / 'src/foundation/reactive-state') + '",features=["effects"]}',
            'wasm-bindgen={path="' + str(ROOT / 'src/foundation/wasm-bindgen') + '"}',
            'js-sys={path="' + str(ROOT / 'src/foundation/js-sys') + '"}',

            'web-sys={path="' + str(ROOT / 'src/foundation/web-sys') + '",features=["Window","Document","HtmlInputElement","Event"]}',
        ]) + '\n')
        source = work / 'src/lib.rs'
        source.write_text((ROOT / 'test/tools/web-bindings/fixtures/csr-template.rs').read_text())
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
        # Compile exported tracing macros from an alias, without web-interface in scope.
        source.write_text((ROOT / 'test/ci/fixtures/tracing-props.rs').read_text())
        cargo(*command, '--features', 'dom-helpers/trace-component-props')
        cargo('test', '--locked', '--offline', '--lib', '--manifest-path', str(manifest),
              '--target-dir', str(TARGET), '--features', 'dom-helpers/trace-component-props')
        cargo(*command, '--target', 'wasm32-unknown-unknown', '--features', 'dom-helpers/trace-component-props')
        retired = {
            'server_module': 'use view_renderer::ssr;',
            'nightly_static_views': 'use view_renderer::view::static_types;',
            'nightly_static_attributes': 'use view_renderer::html::attribute::StaticAttr;',
            'islands': 'use view_renderer::html::islands;',
            'serialized_keys': 'use view_renderer::view::keyed::SerializableKey;',
            'doctype': 'pub fn old() { view_renderer::html::doctype("html"); }',
            'server_hydration': 'pub fn old<T:view_renderer::view::RenderHtml>(value:T) { value.hydrate::<true>(); }',
            'attribute_output': 'pub fn old<T:view_renderer::html::attribute::Attribute>(value:T) { value.to_html(); }',
        }
        for method in ['to_html', 'to_html_with_buf', 'to_html_branching', 'to_html_stream_in_order',
                       'to_html_stream_out_of_order', 'to_html_async_with_buf', 'resolve', 'dry_resolve', 'hydrate_async']:
            retired[method] = f'pub fn old<T:view_renderer::view::RenderHtml>(mut value:T) {{ value.{method}(); }}'
        for name, code in {
            'typed_canvas': 'use view_renderer::html::element::canvas;',
            'typed_audio': 'use view_renderer::html::element::audio;',
            'typed_select': 'use view_renderer::html::element::select;',
            'typed_svg_animation': 'use view_renderer::svg::animate;',
            'typed_svg_use': 'use view_renderer::svg::Use;',
            'typed_mathml': 'use view_renderer::mathml;',
            'inline_event_attributes': 'use view_renderer::html::attribute::GlobalOnAttributes;',
            'typed_touch_event': 'use view_renderer::html::event::touchstart;',
            'typed_touch_type': 'use view_renderer::html::event::TouchEvent;',
            'unused_attribute': 'use view_renderer::html::attribute::accept;',
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
            result = cargo(*command, '--features', 'view-renderer/' + feature, capture=True)
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
            'tracing_macro_checks': ['native-check', 'native-test', 'wasm-check'],
            'tracing_macro_fixture_sha256': hashlib.sha256((ROOT / 'test/ci/fixtures/tracing-props.rs').read_bytes()).hexdigest(),
            'retired_api_checks': len(retired),
            'retired_feature_checks': 13,
            'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
            'fixture_source_sha256': hashlib.sha256((ROOT / 'test/tools/web-bindings/fixtures/csr-template.rs').read_bytes()).hexdigest(),
            'outputs_sha256': {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                               for name in ['fixture.js', 'fixture_bg.wasm']},
        }
        (output / 'validation.json').write_text(json.dumps(record, indent=2) + '\n')
        print(f'CSR contract: browser template compiled; {len(retired)} retired API and 13 feature checks passed')


if __name__ == '__main__':
    check()
