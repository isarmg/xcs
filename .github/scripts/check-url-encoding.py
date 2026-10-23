#!/usr/bin/env python3
"""Check URL capabilities without workspace feature unification, then real consumers."""
if not __debug__:
    raise RuntimeError('Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE).')

import json, os, pathlib, re, shutil, subprocess, tempfile
import runpy

ROOT=pathlib.Path(__file__).resolve().parents[2]
rebase_dependency_paths = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['rebase_dependency_paths']
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
def run(args, cwd=ROOT, success=True):
 p=subprocess.run(args,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=600)
 if 'metadata' not in args: print(p.stdout, end='')
 if success and re.search(r'^warning(?:\[|:)',p.stdout,re.M):raise AssertionError('Rust warnings in URL gate')
 if success and p.returncode:raise RuntimeError('command failed: '+str(args))
 if not success and not p.returncode:raise AssertionError('negative API probe unexpectedly compiled')
 return p.stdout

def retained_manifest(manifest, root):
 """Build the real retained-test manifest without changing production dependencies."""
 manifest=rebase_dependency_paths(manifest, root/'src/foundation/networking/url-encoding')
 manifest+='\n[workspace]\n[dev-dependencies.json-codec]\npath='+json.dumps(str(root/'src/foundation/data-storage/json-codec'))+'\n[dev-dependencies.data-serialization]\npath='+json.dumps(str(root/'src/foundation/data-storage/data-serialization'))+'\nfeatures=["derive"]\n'
 manifest+='\n[dev-dependencies.serialization-derive]\npath='+json.dumps(str(root/'src/foundation/data-storage/serialization-derive'))+'\n'
 for name,path,harness in [('url-unit','foundation/url-encoding/address/tests/unit.rs',True),('domain-unit','foundation/url-encoding/domain/tests/unitbis.rs',True),('url-wpt','foundation/url-encoding/address/tests/wpt.rs',False),('behavior','url-encoding/behavior.rs',True)]:
  manifest+='\n[[test]]\nname='+json.dumps(name)+'\npath='+json.dumps('../../../../test/'+path)+'\nharness='+str(harness).lower()+'\n'
 manifest+='\n[profile.dev]\ndebug=0\nincremental=false\n[profile.test]\ndebug=0\nincremental=false\n'
 return manifest

def check():
 target=ROOT/'target/url-family-check';scratch=ROOT/'target/scope-consumers';scratch.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='url-',dir=scratch) as tmp:
  tmp=pathlib.Path(tmp)
  for index,features in enumerate([[],['alloc'],['form'],['domain'],['parser'],['parser','std','serialization']]):
   p=tmp/str(index);(p/'src').mkdir(parents=True)
   manifest='[package]\nname="url-capability-probe"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies.url-encoding]\npath='+json.dumps(str(ROOT/'src/foundation/networking/url-encoding'))+'\ndefault-features=false\nfeatures='+json.dumps(features)+'\n[profile.dev]\ndebug=0\nincremental=false\n'
   (p/'Cargo.toml').write_text(manifest);(p/'src/lib.rs').write_text('#![no_std]\npub fn encoded(b: u8) -> &\'static str { url_encoding::percent::percent_encode_byte(b) }\n')
   common=['--manifest-path',str(p/'Cargo.toml'),'--offline']
   run(['cargo','generate-lockfile',*common]);meta=json.loads(run(['cargo','metadata','--format-version','1','--locked',*common]))
   active={n['id'] for n in meta['resolve']['nodes']};names={x['name'] for x in meta['packages'] if x['id'] in active}
   assert not names & {'url','idna','idna_adapter','percent-encoding','form_urlencoded'}
   heavy=bool(set(features)&{'domain','parser'});assert ('unicode-normalizer' in names)==heavy
   if not heavy:assert not any(n.startswith(('icu_','unicode-normalizer','unicode-properties')) for n in names)
   for target_name in ['x86_64-unknown-linux-gnu','wasm32-unknown-unknown']:
    cmd=['cargo','check','--locked',*common,'--target',target_name,'--target-dir',str(target),'--jobs','1','--color','never'];run(cmd)
   if not heavy:
    (p/'src/lib.rs').write_text('use url_encoding::address::Url;\n')
    out=run(cmd,success=False);assert 'E0432' in out and 'address' in out
  mirror=tmp/'retained';pkg=mirror/'src/foundation/networking/url-encoding';shutil.copytree(ROOT/'src/foundation/networking/url-encoding',pkg)
  shutil.copytree(ROOT/'test/foundation/url-encoding',mirror/'test/foundation/url-encoding');shutil.copytree(ROOT/'test/url-encoding',mirror/'test/url-encoding')
  (pkg/'Cargo.toml').write_text(retained_manifest((pkg/'Cargo.toml').read_text(), ROOT))
  common=['--manifest-path',str(pkg/'Cargo.toml'),'--offline'];run(['cargo','generate-lockfile',*common])
  out=run(['cargo','test','--locked',*common,'--features','parser,std,serialization','--lib','--tests','--target-dir',str(target),'--jobs','1']);assert re.search(r'test result: ok\. [1-9][0-9]* passed',out)
  run(['cargo','test','--locked',*common,'--features','parser,std,serialization,expose_internals','--doc','--target-dir',str(target),'--jobs','1'])
 subprocess.run(['python3',str(ROOT/'.github/scripts/check-url-http-consumers.py')],cwd=ROOT,check=True)
 run(['cargo','check','--locked','--offline','--jobs','1','-p','reqwest','-p','storage-client'])
 run(['cargo','check','--locked','--offline','--jobs','1','-p','web-app','--target','wasm32-unknown-unknown'])
if __name__=='__main__':check()
