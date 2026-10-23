"""Read-only-to-repository reproduction using the real manager's temporary fixture."""
import sys,json,gzip,base64,hashlib,shutil,traceback,os
from pathlib import Path
from copy import deepcopy
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
ROOT=Path(os.environ.get('SOURCE_LAYOUT_REPOSITORY', str(HERE.parents[2] if len(HERE.parents)>=3 else Path.cwd())))
sys.path[:0]=[str(ROOT/'test/tools/rust-sources'),str(ROOT/'tools/rust-sources')]
from test_manage import SourcePolicy,manager
layout=manager.source_layout
from source_layout_fixture import patch_edits
sha=lambda data:hashlib.sha256(data).hexdigest()
canonical=layout.canonical

def retire(f,directory):
    before=deepcopy(f.catalog['packages'][directory])
    plan=dict(schema=1,packages=[dict(directory=directory,package=before,reason='unsupported platform backend',category='platform',bytes=sum(p.stat().st_size for p in f.source.rglob('*') if p.is_file()),file_count=len(before['files_sha256']))])
    (f.root/'src/app/Cargo.toml').write_text('[package]\nname = "app"\nversion = "1.0.0"\n')
    (f.root/'Cargo.lock').write_text('version = 4\n[[package]]\nname = "app"\nversion = "1.0.0"\n')
    f.metadata['packages']=f.metadata['packages'][:1]
    f.metadata['resolve']['nodes']=[dict(id='app',features=[],deps=[])]
    shutil.rmtree(f.source)
    return plan

def install(f):
    previous=deepcopy(f.catalog);previous_raw=(f.root/manager.CATALOG).read_bytes()
    old='src/foundation/mini';new='src/foundation/core-types/mini';moves={old:new}
    edits={}
    source_paths=tuple(p.relative_to(f.root).as_posix() for p in f.source.rglob('*.rs'))
    for path in ('Cargo.toml','src/app/Cargo.toml',old+'/Cargo.toml',*source_paths):
        before=(f.root/path).read_bytes();after=layout.rewrite(path,before,moves)
        if before!=after:
            replacements=[dict(e,before=base64.b64encode(e['before'].encode()).decode(),after=base64.b64encode(e['after'].encode()).decode()) for e in patch_edits(before,after)]
            edits[path]=dict(after_path=layout.route(path,moves),before_sha256=sha(before),after_sha256=sha(after),before_size=len(before),after_size=len(after),replacements=replacements)
            (f.root/path).write_bytes(after)
    (f.root/new).parent.mkdir(parents=True);(f.root/old).rename(f.root/new);f.source=f.root/new
    f.metadata['packages'][1]['manifest_path']=str(f.source/'Cargo.toml')
    stationary={}
    for directory in {p['directory'] for p in previous['merges']}|set(previous['packages']):
        if directory==old:continue
        stationary[directory]=dict(package=deepcopy(previous['packages'].get(directory)),merged_packages=[deepcopy(p) for p in previous['merges'] if p['directory']==directory],files_sha256=manager.files(f.root/directory))
    frozen={str(p.relative_to(f.root)):sha(p.read_bytes()) for p in (f.root/'compliance').rglob('*') if p.is_file() and str(p.relative_to(f.root)) not in (manager.CATALOG,manager.DOC) and not str(p.relative_to(f.root)).startswith('compliance/external-tests/')}
    installation_files={'Cargo.lock':sha((f.root/'Cargo.lock').read_bytes())}
    old_external=deepcopy(previous.get('external_tests',{}).get(old))
    new_external=deepcopy(old_external)
    if new_external is not None:
        new_external['directory']=new
        mapping=json.loads(f.external_mapping.read_bytes());mapping['owners'][0]['directory']=new
        f.external_mapping.write_text(json.dumps(mapping))
    receipt=dict(schema=2,installation_files=installation_files,reason='Isolated lifecycle regression fixture.',categories={'core-types':'Core foundational types'},predecessor_catalog_sha256=sha(previous_raw),history_hashes={k:sha(canonical(previous.get(k))) for k in layout.HISTORY_FIELDS},frozen_evidence=frozen,packages=[dict(directory=old,destination_directory=new,package=deepcopy(previous['packages'][old]),merged_packages=[],files_sha256=manager.files(f.source),external_tests=old_external,destination_external_tests=new_external)],stationary_owners=stationary,file_edits=edits)
    raw=gzip.compress(canonical(receipt),mtime=0);rid=sha(raw);file=f.root/layout.receipt_path(rid);file.parent.mkdir(parents=True);file.write_bytes(raw)
    pins=patch.object(layout,'PINNED_RECEIPTS',(rid,));pins.start();f.addCleanup(pins.stop)
    current=layout.transition_catalog(previous,receipt,rid);f.catalog=current;f.save_catalog()
    f.layout_previous=deepcopy(previous);f.layout_initial_current=deepcopy(current);f.layout_predecessor_sha=sha(previous_raw)
    view=layout.load(f.root,current);view.verify_installation(previous,current,sha(previous_raw))
    f.refresh();f.verify()
    return new
