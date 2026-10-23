"""Small multi-owner filesystem scenario shared by staged layout tests.

No repository paths are edited. Adapters may consume the snapshots to serialize
whatever finalized receipt schema they use; the scenario itself is schema-neutral.
"""
from copy import deepcopy
from dataclasses import dataclass
import difflib
import hashlib
import json
from pathlib import Path


def digest(value):
    return hashlib.sha256(value).hexdigest()


def encoded(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, allow_nan=False).encode()


def record_id(value):
    return digest(encoded(value))


def patch_edits(before, after):
    result = []
    for kind, a, b, x, y in difflib.SequenceMatcher(None, before, after, autojunk=False).get_opcodes():
        if kind != 'equal':
            result.append(dict(start=a, end=b, before=before[a:b].decode(), after=after[x:y].decode()))
    return result


def move(path, routes):
    for old, new in routes.items():
        if path == old or path.startswith(old + '/'):
            return new + path[len(old):]
    return path


def material(root, files):
    for name, data in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def owner_package(directory, name, data, origin=None):
    return dict(directory=directory, name=name, version='1.0.0',
                origin_directory=origin or 'vendor/' + name,
                upstream=dict(name=name, version='1.0.0', archive_sha256='1' * 64),
                license='MIT', license_materials=['compliance/licenses/LICENSE-MIT'],
                repository='https://example.invalid/' + name, vcs=dict(sha1='2' * 40),
                changes='Reviewed local source.',
                files_sha256={p[len(directory) + 1:]: digest(raw) for p, raw in data.items()
                              if p.startswith(directory + '/')})


@dataclass
class Scenario:
    root: Path
    routes: dict
    before_bytes: dict
    after_bytes: dict
    previous: dict
    current: dict
    frozen: dict
    edits: dict

    @classmethod
    def create(cls, root):
        root = Path(root)
        routes = {'src/foundation/alpha': 'src/foundation/concurrency/alpha',
                  'src/foundation/plain': 'src/foundation/platform/plain',
                  'src/foundation/trimmed': 'src/foundation/web-runtime/trimmed'}
        before = {
            'Cargo.toml': b'[workspace]\nmembers = ["src/foundation/alpha", "src/foundation/plain", "src/foundation/trimmed", "src/interface"]\n',
            'Cargo.lock': b'version = 4\n',
            'src/foundation/alpha/Cargo.toml': b'[package]\nname = "alpha"\nversion = "1.0.0"\nreadme = "../../../docs/upstream/alpha/README.md"\nautobenches = false\n[dependencies]\nplain = { path = "../plain", default-features = false }\n',
            'src/foundation/alpha/src/lib.rs': b'#![doc = include_str!("../../../../docs/upstream/alpha/README.md")]\nmod value;\npub fn answer() -> u8 { value::VALUE }\n',
            'src/foundation/alpha/src/value.rs': b'pub const VALUE: u8 = 42;\n',
            'src/foundation/alpha/docs/internal.txt': b'Package-private documentation stays internal.\n',
            'src/foundation/plain/Cargo.toml': b'[package]\nname = "plain"\nversion = "1.0.0"\n',
            'src/foundation/plain/src/lib.rs': b'pub fn identity(value: u8) -> u8 { value }\n',
            'src/foundation/trimmed/Cargo.toml': b'[package]\nname = "trimmed"\nversion = "1.0.0"\nbuild = false\n',
            'src/foundation/trimmed/src/lib.rs': b'pub fn live() {}\n',
            'src/interface/Cargo.toml': b'[package]\nname = "interface"\nversion = "1.0.0"\n[dependencies]\nalpha = { path = "../foundation/alpha" }\n',
            'src/interface/src/lib.rs': b'pub fn interface() {}\n',
            'test/foundation/alpha/tests/smoke.rs': b'include!("../../../../src/foundation/alpha/src/value.rs");\n#[test]\nfn smoke() { assert_eq!(VALUE, 42); }\n',
        }
        after = {move(p, routes): raw for p, raw in before.items()}
        after['Cargo.toml'] = before['Cargo.toml']
        for old, new in routes.items():
            after['Cargo.toml'] = after['Cargo.toml'].replace(old.encode(), new.encode())
        after[routes['src/foundation/alpha'] + '/Cargo.toml'] = (
            before['src/foundation/alpha/Cargo.toml']
            .replace(b'../../../docs/', b'../../../../docs/')
            .replace(b'path = "../plain"', b'path = "../../platform/plain"'))
        after[routes['src/foundation/alpha'] + '/src/lib.rs'] = (
            before['src/foundation/alpha/src/lib.rs']
            .replace(b'../../../../docs/', b'../../../../../docs/'))
        after['src/interface/Cargo.toml'] = before['src/interface/Cargo.toml'].replace(
            b'../foundation/alpha', b'../foundation/concurrency/alpha')
        after['test/foundation/alpha/tests/smoke.rs'] = before['test/foundation/alpha/tests/smoke.rs'].replace(
            b'src/foundation/alpha/', b'src/foundation/concurrency/alpha/')
        packages = {directory: owner_package(directory, directory.rsplit('/', 1)[1], before)
                    for directory in [*routes, 'src/interface']}
        merged = deepcopy(packages['src/foundation/alpha'])
        merged.update(name='alpha-helper', origin_directory='vendor/alpha-helper',
                      upstream=dict(name='alpha-helper', version='1.0.0', archive_sha256='3' * 64),
                      merged_into='src/foundation/alpha')
        test_path = 'test/foundation/alpha/tests/smoke.rs'
        test_owner = dict(directory='src/foundation/alpha', origin_directory='vendor/alpha',
            roots=['test/foundation/alpha'], file_count=1, bytes=len(before[test_path]),
            files=[dict(source='src/foundation/alpha/tests/smoke.rs', path=test_path,
                        sha256=digest(before[test_path]), source_sha256_before=digest(before[test_path]))])
        dead = b'pub fn dead() {}\n'
        trimmed_before = deepcopy(packages['src/foundation/trimmed'])
        trimmed_before['files_sha256']['src/dead.rs'] = digest(dead)
        trim = dict(directory='src/foundation/trimmed', package=trimmed_before,
                    files=[dict(path='src/dead.rs', sha256_before=digest(dead), reason='Unreachable local implementation removed without dropping provenance.')])
        frozen_bytes = {
            'compliance/provenance/origins.json': encoded({'schema': 1, 'owners': list(packages)}),
            'compliance/licenses/LICENSE-MIT': b'Exact immutable license bytes.\n',
            'docs/upstream/alpha/README.md': b'Exact immutable upstream documentation.\n',
            'compliance/platform-test-relocation.json': encoded({'schema_version':1, 'roots':{}, 'files':[{
                'source':'src/foundation/plain/src/lib.rs', 'path':'src/foundation/plain/src/lib.rs',
                'sha256':digest(before['src/foundation/plain/src/lib.rs']),
                'before_sha256':digest(before['src/foundation/plain/src/lib.rs'])}]}),
        }
        previous = dict(schema=8, packages=packages, merges=[merged],
                        external_tests={'src/foundation/alpha':test_owner},
                        renames={}, rename_records_sha256={}, source_moves={},
                        consolidations={}, consolidation_records_sha256={},
                        source_trims={record_id(trim):trim}, source_trim_records_sha256={record_id(trim):record_id(trim)},
                        proof_routing_receipts=[], retired={}, retired_records_sha256={},
                        superseded={}, superseded_records_sha256={})
        current = deepcopy(previous)
        current['packages'] = {}
        for directory, package in previous['packages'].items():
            new = move(directory, routes)
            current['packages'][new] = dict(deepcopy(package), directory=new,
                files_sha256={p[len(new)+1:]:digest(raw) for p,raw in after.items() if p.startswith(new+'/')})
        current['merges'][0].update(directory=routes['src/foundation/alpha'],
                                  merged_into=routes['src/foundation/alpha'],
                                  files_sha256=deepcopy(current['packages'][routes['src/foundation/alpha']]['files_sha256']))
        new_owner = deepcopy(test_owner)
        new_owner.update(directory=routes['src/foundation/alpha'], bytes=len(after[test_path]))
        new_owner['files'][0]['sha256'] = digest(after[test_path])
        current['external_tests'] = {routes['src/foundation/alpha']:new_owner}
        material(root, after); material(root, frozen_bytes)
        (root/'compliance/rust-sources.json').write_bytes(encoded(current))
        edits = {p:dict(before_sha256=digest(raw), after_sha256=digest(after[move(p,routes)]),
                        replacements=patch_edits(raw,after[move(p,routes)]))
                 for p,raw in before.items() if raw != after[move(p,routes)]}
        return cls(root, routes, before, after, previous, current,
                   {p:digest(raw) for p,raw in frozen_bytes.items()}, edits)

    def write_current(self):
        (self.root/'compliance/rust-sources.json').write_bytes(encoded(self.current))

    def reset_live(self):
        material(self.root, self.after_bytes)

    def full_inventory(self, package, after=False):
        table = self.after_bytes if after else self.before_bytes
        return {p:digest(raw) for p,raw in table.items() if p.startswith(package+'/')}
