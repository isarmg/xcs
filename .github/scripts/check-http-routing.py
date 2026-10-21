#!/usr/bin/env python3
"""Check routing/kernel identity, pure WASM boundaries, macros and response behavior."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[2]
FEATURES = ((), ('json', 'form', 'query', 'macros'), ('tokio', 'http1'), ('default',))
def run(work, *args):
    result = subprocess.run(['cargo', *args, '--color', 'never', '--offline'], cwd=work,
                            text=True, capture_output=True, timeout=600)
    if result.returncode: raise RuntimeError(result.stdout + result.stderr)
    if re.search(r'^warning:', result.stderr, re.M): raise AssertionError(result.stderr)
    return result.stdout

def main():
    os.environ.setdefault('CARGO_BUILD_JOBS','2')
    os.environ.setdefault('CARGO_PROFILE_DEV_DEBUG','0')
    os.environ.setdefault('CARGO_INCREMENTAL','0')
    with tempfile.TemporaryDirectory(prefix='http-routing-') as temporary:
        work=Path(temporary);(work/'src').mkdir();(work/'test').mkdir()
        (work / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        (work/'src/lib.rs').write_text('pub fn router() -> http_routing::Router { http_routing::Router::new() }\n')
        (work/'test/behavior.rs').write_bytes((ROOT/'test/architecture/http-routing/behavior.rs').read_bytes())
        for features in FEATURES:
            (work/'Cargo.toml').write_text(
                '[package]\nname="routing-contract"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n'
                f'http-routing={{path={json.dumps(str(ROOT/"src/foundation/http-routing"))},default-features=false,features={json.dumps(features)}}}\n'
                f'service-stack={{path={json.dumps(str(ROOT/"src/foundation/service-stack"))},default-features=false,features=["util"]}}\n'
                f'http-body={{path={json.dumps(str(ROOT/"src/foundation/http-body"))},features=["utilities"]}}\n'
                '[[test]]\nname="behavior"\npath="test/behavior.rs"\nrequired-features=["http-routing/query","http-routing/macros"]\n')
            run(work,'generate-lockfile')
            metadata=json.loads(run(work,'metadata','--locked','--format-version','1'))
            nodes={n['id'] for n in metadata['resolve']['nodes']}
            names={p['name'] for p in metadata['packages'] if p['id'] in nodes}
            assert not names.intersection({'axum','axum-core'}), names
            runtime=bool(set(features)&{'tokio','default'})
            if not runtime:
                assert not names.intersection({'tokio','hyper','hyper-util'}), names
                run(work,'check','--locked','--target','wasm32-unknown-unknown')
            run(work,'check','--locked')
            if 'macros' in features:
                result=run(work,'test','--locked','--test','behavior')
                assert re.search(r'test result: ok\. 5 passed; 0 failed',result), result
        print('http-routing: 6 native/WASM feature checks and 5 routing/body/status/macro tests passed')
if __name__=='__main__': main()
