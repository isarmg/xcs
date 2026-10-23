#!/usr/bin/env python3
"""Compile real routing/proxy consumers without pulling Unicode into byte/form paths."""
import json,os,pathlib,subprocess,tempfile
import runpy

ROOT=pathlib.Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
def check(root=ROOT):
 scratch=root/'target/scope-consumers';scratch.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='url-http-',dir=scratch) as folder:
  work=pathlib.Path(folder);(work/'src').mkdir();(work/'src/lib.rs').write_text('pub use http_routing::extract::Query;\npub use http_transport::adapters::client::proxy::matcher::Matcher;\n')
  manifest='[package]\nname="url-http-consumers"\nversion="0.0.0"\nedition="2024"\n[workspace]\n'
  for name,features in [('http-routing',['query','form']),('http-transport',['client-proxy'])]:
   manifest+='[dependencies.'+name+']\npath='+json.dumps(str(foundation_path(root, name)))+'\ndefault-features=false\nfeatures='+json.dumps(features)+'\n'
  manifest+='[[test]]\nname="contracts"\npath='+json.dumps(str(root/'test/url-encoding/http-consumers.rs'))+'\n[profile.dev]\ndebug=0\nincremental=false\n[profile.test]\ndebug=0\nincremental=false\n';(work/'Cargo.toml').write_text(manifest)
  env=dict(os.environ,CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_TARGET_DIR=str(root/'target/url-family-check'))
  def run(args,capture=False):
   p=subprocess.run(['cargo',*args,'--manifest-path',str(work/'Cargo.toml'),'--offline','--color','never'],cwd=root,env=env,text=True,capture_output=capture,check=True);return p.stdout
  run(['generate-lockfile']);metadata=json.loads(run(['metadata','--locked','--format-version','1'],True));nodes={n['id'] for n in metadata['resolve']['nodes']};names={p['name'] for p in metadata['packages'] if p['id'] in nodes}
  assert 'url-encoding' in names
  assert not names.intersection({'url','idna','idna_adapter','percent-encoding','form_urlencoded','unicode-normalizer','unicode-properties','axum','axum-core','hyper','hyper-util'}),names
  for target in ['x86_64-unknown-linux-gnu','wasm32-unknown-unknown']:run(['check','--locked','--target',target])
  run(['test','--locked','--test','contracts'])
if __name__=='__main__':check()
