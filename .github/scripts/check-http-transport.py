#!/usr/bin/env python3
"""Validate HTTP/1 transport/adapters, real duplex IO and native/WASM capabilities."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parents[2]
FEATURES=((),('client','http1'),('server','http1'),('adapters',),('service-adapter',),('adapter-client',),('client-pool',),('client-proxy',),('tokio',),('client-legacy','http1','tokio'),('adapter-server',),('server-auto','tokio'),('server-graceful','http1','tokio'),('full',),('rt-tracing-exec-force',))
NATIVE_ONLY={'tokio','client-legacy','full','rt-tracing-exec-force'}
def run(work,*args,negative=False,env=None):
    result=subprocess.run(['cargo',*args,'--color','never','--offline'],cwd=work,text=True,capture_output=True,timeout=600,env=env)
    if negative:
        assert result.returncode and 'with feature `http2`' in result.stderr and '`http-transport` does not have that feature' in result.stderr,result.stderr
    else:
        if result.returncode:raise RuntimeError(result.stdout+result.stderr)
        if re.search(r'^warning:',result.stderr,re.M):raise AssertionError(result.stderr)
    return result.stdout

def main():
    os.environ.setdefault('CARGO_BUILD_JOBS','2');os.environ.setdefault('CARGO_PROFILE_DEV_DEBUG','0');os.environ.setdefault('CARGO_INCREMENTAL','0')
    with tempfile.TemporaryDirectory(prefix='http-transport-') as temporary:
        work=Path(temporary);(work/'src').mkdir();(work/'test').mkdir()
        (work / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        (work/'src/lib.rs').write_text('pub use http_transport::{Error,Request,Response};\n')
        count=0
        for features in FEATURES:
            (work/'Cargo.toml').write_text('[package]\nname="transport-contract"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n'+f'http-transport={{path={json.dumps(str(ROOT/"src/foundation/http-transport"))},default-features=false,features={json.dumps(features)}}}\n')
            run(work,'generate-lockfile');metadata=json.loads(run(work,'metadata','--locked','--format-version','1'))
            nodes={n['id']:n for n in metadata['resolve']['nodes']};packages={p['name']:p for p in metadata['packages'] if p['id'] in nodes}
            assert not set(packages).intersection({'hyper','hyper-util','h2'}),set(packages)
            if not set(features)&NATIVE_ONLY:
                tf=set(nodes[packages['tokio']['id']]['features']);assert not tf.intersection({'rt','rt-multi-thread','net','time'}),tf
                run(work,'check','--locked','--target','wasm32-unknown-unknown');count+=1
            run(work,'check','--locked');count+=1
        run(work,'check','--locked','--features','http-transport/http2',negative=True)
        for extra,flag in [('tracing','hyper_unstable_tracing'),('ffi','hyper_unstable_ffi')]:
            environment=dict(os.environ)
            environment['RUSTFLAGS']=(environment.get('RUSTFLAGS','')+' --cfg '+flag).strip()
            run(work,'check','--locked','--features',f'http-transport/{extra},http-transport/client,http-transport/http1',env=environment)
            count+=1
            if extra == 'ffi':
                with (work/'Cargo.toml').open('a') as manifest:
                    manifest.write('[[test]]\nname="ffi"\npath="test/ffi.rs"\n')
                (work/'test/ffi.rs').write_bytes((ROOT/'test/architecture/http-transport/ffi.rs').read_bytes())
                output=run(work,'test','--locked','--test','ffi','--features','http-transport/ffi,http-transport/client,http-transport/http1',env=environment)
                assert re.search(r'test result: ok\. 2 passed; 0 failed',output),output
        # Real runtime + HTTP client/server over Tokio's in-memory duplex stream, no sockets.
        (work/'Cargo.toml').write_text('[package]\nname="transport-behavior"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n'+
            f'http-transport={{path={json.dumps(str(ROOT/"src/foundation/http-transport"))},features=["full"]}}\n'+
            f'http-routing={{path={json.dumps(str(ROOT/"src/foundation/http-routing"))},default-features=false,features=["tokio","http1"]}}\n'+
            f'tokio={{path={json.dumps(str(ROOT/"src/foundation/tokio"))},features=["rt","macros","io-util","time","test-util"]}}\n'+
            f'http-body={{path={json.dumps(str(ROOT/"src/foundation/http-body"))},features=["utilities"]}}\n'+
            f'service-stack={{path={json.dumps(str(ROOT/"src/foundation/service-stack"))},features=["util"]}}\n'+
            '[[test]]\nname="behavior"\npath="test/behavior.rs"\n')
        (work/'test/behavior.rs').write_bytes((ROOT/'test/architecture/http-transport/behavior.rs').read_bytes() + (ROOT/'test/architecture/http-transport/combined.rs').read_bytes() + (ROOT/'test/architecture/http-transport/upgrade.rs').read_bytes() + (ROOT/'test/architecture/http-transport/client_state.rs').read_bytes() + (ROOT/'test/architecture/http-transport/probe_timeout.rs').read_bytes())
        run(work,'generate-lockfile');output=run(work,'test','--locked','--test','behavior')
        assert re.search(r'test result: ok\. 25 passed; 0 failed',output),output
        print(f'http-transport: {count} native/WASM feature checks, absent HTTP2 rejection, 25 transport/routing/readiness/upgrade/state tests and 2 FFI allocation/ABI tests passed')
if __name__=='__main__':main()
