#!/usr/bin/env python3
"""Verify pinned ring test materials offline and run the GNU amd64 regression matrix.

No download, synthetic test vectors, algorithm edits, skipped assertions, alternate
Cargo workspace, or replacement original provenance is used. Local-generation mode
copies the registered repository workspace before adding ring's development marker.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shutil
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
RING = Path('src/foundation/ring')
PROOF = Path('compliance/provenance/supplemental/ring-0.17.14-test-materials.json')
OVERLAP = Path('compliance/provenance/supplemental/ring-0.17.14-original-overlap.json')
STYLE = Path('compliance/provenance/supplemental/ring-0.17.14-style-patches.json')
COMMIT = '2723abbca9e83347d82b056d5b239c6604f786df'
TARGET = Path('compliance/provenance/supplemental/ring-0.17.14-linux-amd64-target.json')
TARGET_SHA256 = 'dad19ddfdd538ef971c275a61101fab798b23ec7edcdcf1494cd4401383f1d80'
RETIRED_ARCHIVE = Path('compliance/provenance/supplemental/ring-0.17.14-linux-amd64-retired.zip')


def object_hash(kind, content):
    return hashlib.sha1(kind.encode() + b' ' + str(len(content)).encode() + b'\0' + content).hexdigest()


def checked_path(root, relative):
    path = PurePosixPath(relative)
    if path.is_absolute() or '..' in path.parts or str(path) != relative:
        raise ValueError(f'unsafe material path: {relative}')
    result = root / relative
    if not result.resolve().is_relative_to(root.resolve()) or result.is_symlink():
        raise ValueError(f'material escaped source tree: {relative}')
    return result


def check_content(content, record):
    if len(content) != record['bytes'] or hashlib.sha256(content).hexdigest() != record['sha256']:
        raise ValueError(f'target source hash/size mismatch: {record["path"]}')


def read_retired_archive(content, record, deleted):
    """Read only authenticated, bounded historical bytes; never extract files."""
    check_content(content, record)
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        members = archive.infolist()
        if (len(members) != record['members'] or len(members) != len(deleted)
                or {member.filename for member in members} != set(deleted)):
            raise ValueError('retired archive member inventory changed')
        if sum(member.file_size for member in members) != record['uncompressed_bytes']:
            raise ValueError('retired archive total uncompressed size changed')
        result = {}
        for member in members:
            path = PurePosixPath(member.filename)
            if (path.is_absolute() or '..' in path.parts or str(path) != member.filename
                    or '\\' in member.filename
                    or member.is_dir() or member.external_attr >> 16 != 0o100644
                    or member.flag_bits & 1 or member.compress_type != zipfile.ZIP_DEFLATED):
                raise ValueError(f'unsafe retired archive member: {member.filename}')
            expected = deleted[member.filename]
            if member.file_size != expected['bytes']:
                raise ValueError(f'retired archive member size changed: {member.filename}')
            with archive.open(member) as stream:
                data = stream.read(expected['bytes'] + 1)
                if stream.read(1):
                    raise ValueError(f'retired archive member exceeds limit: {member.filename}')
            check_content(data, expected)
            if object_hash('blob', data) != expected['blob']:
                raise ValueError(f'retired archive Git blob changed: {member.filename}')
            result[member.filename] = data
    return result


def target_sources(root):
    """Check current sources first; return a narrowly scoped historical view."""
    content = checked_path(root, str(TARGET)).read_bytes()
    if hashlib.sha256(content).hexdigest() != TARGET_SHA256:
        raise ValueError('immutable target-pruning proof changed')
    record = json.loads(content)
    if (record['schema_version'] != 1 or record['archive']['path'] != str(RETIRED_ARCHIVE)
            or record['archive']['members'] != 116
            or record['archive']['uncompressed_bytes'] != 3845917):
        raise ValueError('unexpected target-pruning identity')
    for path, digest in record['historical_proofs'].items():
        if hashlib.sha256(checked_path(root, path).read_bytes()).hexdigest() != digest:
            raise ValueError(f'immutable historical proof changed: {path}')
    deleted = {entry['path']: entry for entry in record['deleted']}
    retained = {entry['path']: entry for entry in record['retained']}
    if (len(deleted) != 116 or len(deleted) != len(record['deleted'])
            or len(retained) != 331 or len(retained) != len(record['retained'])
            or set(deleted) & set(retained)):
        raise ValueError('unexpected target-pruning source inventory')
    source_root = checked_path(root, str(RING))
    paths = list(source_root.rglob('*'))
    if any(path.is_symlink() for path in paths):
        raise ValueError('symlink in current ring source tree')
    actual = {path.relative_to(source_root).as_posix() for path in paths if path.is_file()}
    if actual != set(retained) | {'build.rs'}:
        raise ValueError('current ring source inventory changed')
    for path, entry in retained.items():
        check_content(checked_path(source_root, path).read_bytes(), entry)
    for path in deleted:
        if checked_path(source_root, path).exists():
            raise ValueError(f'retired source reappeared: {path}')
    patch = record['build_patch']
    before = checked_path(source_root, 'build.rs').read_bytes()
    check_content(before, patch['after'])
    for replacement in reversed(patch['replacements']):
        old, new = replacement['old'].encode(), replacement['new'].encode()
        if not new or before.count(new) != 1:
            raise ValueError('target build patch is not uniquely reversible')
        before = before.replace(new, old)
    check_content(before, patch['before'])
    archive = checked_path(root, str(RETIRED_ARCHIVE))
    # Bound compressed input before allocation as well as each decompressed member.
    if archive.stat().st_size != record['archive']['bytes']:
        raise ValueError('retired archive compressed size changed')
    retired = read_retired_archive(archive.read_bytes(), record['archive'], deleted)
    retired['build.rs'] = before
    patches = record['source_patches']
    if (len(patches) != 3 or {patch['path'] for patch in patches}
            != {'Cargo.toml', 'src/lib.rs', 'src/rand.rs'}):
        raise ValueError('unexpected target metadata patch source set')
    for patch in patches:
        path = patch['path']
        before = checked_path(source_root, path).read_bytes()
        check_content(before, patch['after'])
        if object_hash('blob', before) != patch['after']['blob']:
            raise ValueError(f'target metadata after blob changed: {path}')
        for replacement in reversed(patch['replacements']):
            old, new = replacement['old'].encode(), replacement['new'].encode()
            if not new or before.count(new) != 1:
                raise ValueError(f'target metadata patch is not uniquely reversible: {path}')
            before = before.replace(new, old)
        check_content(before, patch['before'])
        if object_hash('blob', before) != patch['before']['blob']:
            raise ValueError(f'target metadata before blob changed: {path}')
        retired[path] = before
    return {str(RING / path): data for path, data in retired.items()}


def historical_sources(root):
    """Authenticate current style edits, then reconstruct the immutable baseline."""
    record = json.loads((root / STYLE).read_text())
    allowed = {
        'tests/hmac_tests.rs': ('    assert_eq!("Key { algorithm: SHA256 }", format!("{:?}", &key));\n\n    let ctx = hmac::Context::with_key(&key);\n    assert_eq!("Context { algorithm: SHA256 }", format!("{:?}", &ctx));', '    assert_eq!("Key { algorithm: SHA256 }", format!("{:?}", key));\n\n    let ctx = hmac::Context::with_key(&key);\n    assert_eq!("Context { algorithm: SHA256 }", format!("{:?}", ctx));'),
        'cavp/tests/shavs.rs': ('msg.truncate(0);', 'msg.clear();'),
        'tests/aead_tests.rs': ('in_out.truncate(0);', 'in_out.clear();'),
        'tests/agreement_tests.rs': ('format!("{:?}", &private_key)', 'format!("{:?}", private_key)'),
        'src/testutil.rs': ('current_section.truncate(0);', 'current_section.clear();'),
        'src/aead/aes/ffi.rs': ('    ///      that corresponds to `f`.', '    ///     that corresponds to `f`.'),
    }
    if record['schema_version'] != 1 or record['upstream_commit'] != COMMIT:
        raise ValueError('unexpected style patch identity')
    if set(record['historical_proofs']) != {str(PROOF), str(OVERLAP)}:
        raise ValueError('style patch historical proof set changed')
    for path, digest in record['historical_proofs'].items():
        if hashlib.sha256(checked_path(root, path).read_bytes()).hexdigest() != digest:
            raise ValueError('immutable historical proof changed')
    patches = record['patches']
    if len(patches) != len(allowed) or {p['path'] for p in patches} != set(allowed):
        raise ValueError('style patch source set changed')
    baseline = {}
    for patch in patches:
        path = patch['path']
        if (patch['old'], patch['new']) != allowed[path]:
            raise ValueError(f'unapproved style transformation: {path}')
        current = checked_path(root / RING, path).read_bytes()
        old, new = patch['old'].encode(), patch['new'].encode()
        if current.count(new) != 1:
            raise ValueError(f'style replacement is not unique: {path}')
        before = current.replace(new, old)
        if before.count(old) != 1 or before.replace(old, new) != current:
            raise ValueError(f'style patch is not reversible: {path}')
        for label, content in [('before', before), ('after', current)]:
            if (len(content) != patch[label + '_bytes']
                    or hashlib.sha256(content).hexdigest() != patch[label + '_sha256']
                    or object_hash('blob', content) != patch[label + '_blob']):
                raise ValueError(f'style patch {label} hash mismatch: {path}')
        baseline[str(RING / path)] = before

    # This view is only for historical evidence, never for the current build.
    # target_sources first authenticates all live files and rejects missing ones.
    retired = target_sources(root)

    def read(relative):
        path = str(relative)
        if path in baseline:
            return baseline[path]
        if path in retired:
            return retired[path]
        return checked_path(root, path).read_bytes()
    return read


def verify(root=ROOT):
    historical = historical_sources(root)
    proof = json.loads((root / PROOF).read_text())
    if proof['repository'] != 'https://github.com/briansmith/ring' or proof['commit'] != COMMIT:
        raise ValueError('unexpected upstream repository or commit')
    commit = proof['commit_object'].encode()
    if object_hash('commit', commit) != COMMIT or not commit.startswith(f"tree {proof['tree']}\n".encode()):
        raise ValueError('commit object does not authenticate the recorded tree')
    entries = {entry['path']: entry for entry in proof['git_tree_entries']}
    if len(entries) != len(proof['git_tree_entries']):
        raise ValueError('duplicate Git tree entries')
    for path in entries:
        checked_path(root / RING, path)
    for parent in [''] + [path for path, entry in entries.items() if entry['type'] == 'tree']:
        children = [entry for path, entry in entries.items()
                    if (str(PurePosixPath(path).parent) if '/' in path else '') == parent]
        content = b''
        for entry in sorted(children, key=lambda e: PurePosixPath(e['path']).name + ('/' if e['type'] == 'tree' else '')):
            mode = '40000' if entry['type'] == 'tree' else entry['mode']
            content += mode.encode() + b' ' + PurePosixPath(entry['path']).name.encode() + b'\0' + bytes.fromhex(entry['sha'])
        expected = entries[parent]['sha'] if parent else proof['tree']
        if object_hash('tree', content) != expected:
            raise ValueError(f'Git tree mismatch: {parent}')
    seen = set()
    for entry in proof['materials']:
        path = entry['path']
        if path in seen:
            raise ValueError(f'duplicate material: {path}')
        seen.add(path)
        destination = entry.get('destination', (RING / path).as_posix())
        content = historical(destination)
        expected_url = f'https://github.com/briansmith/ring/blob/{COMMIT}/{path}'
        if (entry['commit'] != COMMIT or entry['origin_url'] != expected_url
                or len(content) != entry['bytes'] or len(content) != entries[path]['size']
                or hashlib.sha256(content).hexdigest() != entry['sha256']
                or object_hash('blob', content) != entry['blob'] or entry['blob'] != entries[path]['sha']):
            raise ValueError(f'upstream material mismatch: {path}')
        if not entry['associated_sources']:
            raise ValueError(f'material has no associated test source: {path}')
        for source in entry['associated_sources']:
            content = historical(RING / source['path'])
            if hashlib.sha256(content).hexdigest() != source['sha256']:
                raise ValueError(f'associated test source mismatch: {source["path"]}')
    originals = json.loads((root / 'compliance/provenance/packages/ring-0.17.14/cargo-checksum.json.txt').read_text())['files']
    audit = json.loads((root / OVERLAP).read_text())
    if audit['commit'] != COMMIT or {row['path'] for row in audit['records']} != set(originals):
        raise ValueError('original package overlap inventory changed')
    for row in audit['records']:
        path = row['path']
        content = (row['original_content'].encode() if 'original_content' in row
                   else historical(row['original_path']))
        if (hashlib.sha256(content).hexdigest() != originals[path]
                or row['recorded_sha256'] != originals[path]
                or object_hash('blob', content) != row['original_blob']):
            raise ValueError(f'original package hash mismatch: {path}')
        upstream_path = 'Cargo.toml' if path == 'Cargo.toml.orig' else path
        upstream = entries.get(upstream_path)
        if row['upstream_blob'] != (upstream['sha'] if upstream else None):
            raise ValueError(f'original overlap tree mismatch: {path}')
        if upstream and path != 'Cargo.toml' and row['original_blob'] != upstream['sha']:
            raise ValueError(f'unexplained original/upstream mismatch: {path}')
    for source in proof['test_sources']:
        content = historical(RING / source['path'])
        if (hashlib.sha256(content).hexdigest() != source['sha256']
                or source['sha256'] != originals[source['path']]
                or object_hash('blob', content) != source['blob'] or source['blob'] != entries[source['path']]['sha']):
            raise ValueError(f'original integration source mismatch: {source["path"]}')
    for source in proof['implementation_sources']:
        content = historical(RING / source['path'])
        if hashlib.sha256(content).hexdigest() != source['sha256']:
            raise ValueError(f'restoration changed an existing implementation: {source["path"]}')
    manifest = tomllib.loads((root / RING / 'Cargo.toml').read_text())
    wanted = {source['path'] for source in proof['test_sources']} | {'cavp/tests/shavs.rs'}
    actual = [entry['path'] for entry in manifest['test']]
    if set(actual) != wanted or len(actual) != len(wanted):
        raise ValueError('upstream integration test entry set changed')
    if len(seen) != 69:
        raise ValueError('restoration material set changed')
    workspace = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']
    if str(RING) not in workspace['members'] or str(RING) in workspace['default-members']:
        raise ValueError('ring must be an explicit non-default verification member')
    print(f'Verified {len(seen)} upstream materials and {len(wanted)} integration test entries.', flush=True)
    return proof


def run_matrix(root, mode, target_dir):
    # Ring is selected explicitly; default workspace members do not enable this suite.
    for profile in ('debug', 'release'):
        for features in ('default', 'std'):
            run_case(root, mode, target_dir, profile, features)
    run_case(root, mode, target_dir, 'release', 'std,slow_tests')


def run_case(root, mode, target_dir, profile, features):
    verify(root)
    command = ['cargo', 'test', '--locked', '--offline', '-j2', '-p', 'ring']
    if profile == 'release':
        command.append('--release')
    if features != 'default':
        command += ['--features', features]
    env = dict(os.environ, CARGO_TARGET_DIR=str(target_dir / mode))
    print(f'RING CASE {mode}/{profile}/{features}: {" ".join(command)}', flush=True)
    subprocess.run(command, cwd=root, env=env, check=True)
    if mode == 'localgen':
        verify_generated(root, target_dir / mode, profile)
    print(f'RING PASS {mode}/{profile}/{features}', flush=True)


def verify_generated(root, target_dir, profile):
    pregenerated = root / RING / 'pregenerated'
    expected = {path.relative_to(pregenerated) for path in pregenerated.glob('*x86_64*elf.S')}
    expected.update(Path('ring_core_generated') / name for name in (
        'prefix_symbols.h', 'prefix_symbols_asm.h', 'prefix_symbols_nasm.inc',
    ))
    if len(expected) != 15:
        raise ValueError('unexpected GNU amd64 generated-source inventory')
    directories = list(target_dir.glob(f'**/{profile}/build/ring-*/out'))
    if not directories:
        raise ValueError('local-generation mode produced no ring build output')
    for directory in directories:
        for relative in expected:
            generated = directory / relative
            if not generated.is_file() or generated.read_bytes() != (pregenerated / relative).read_bytes():
                raise ValueError(f'local-generation output differs from original package: {generated}')
    print(f'Verified 15 generated assembly/header files in {len(directories)} {profile} build outputs.', flush=True)


def copy_workspace(destination):
    # Includes pending authorized source additions for local validation; excludes
    # ignored build output. Every path comes from Git's source inventory.
    result = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT)
    for relative in sorted(set(p.decode() for p in result.split(b'\0') if p)):
        source = checked_path(ROOT, relative)
        if not source.is_file():
            continue
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    (destination / RING / '.git').mkdir()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--mode', choices=('packaged', 'localgen', 'both'), default='both')
    args = parser.parse_args()
    verify()
    if args.verify_only:
        return
    host = subprocess.check_output(['rustc', '-vV'], text=True)
    if platform.system() != 'Linux' or platform.machine() != 'x86_64' or 'host: x86_64-unknown-linux-gnu' not in host:
        raise SystemExit('Executable coverage requires GNU/Linux amd64; other ABIs are not claimed.')
    if os.environ.get('CARGO_BUILD_TARGET') not in (None, 'x86_64-unknown-linux-gnu'):
        raise SystemExit('A cross-target override would invalidate the GNU amd64 execution matrix.')
    if os.environ.get('RING_PREGENERATE_ASM') is not None:
        raise SystemExit('Unset RING_PREGENERATE_ASM: the Linux amd64 packaging flow is not a test mode.')
    target_dir = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve() / 'ring-regression'
    if args.mode in ('packaged', 'both'):
        if (ROOT / RING / '.git').exists():
            raise SystemExit('Packaged mode requires no ring/.git development marker.')
        run_matrix(ROOT, 'packaged', target_dir)
    if args.mode in ('localgen', 'both'):
        # /tmp may be a small tmpfs. Keep the isolated workspace beside ignored
        # build output on the repository filesystem, never inside source paths.
        target_dir.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='workspace-', dir=target_dir) as temporary:
            isolated = Path(temporary)
            copy_workspace(isolated)
            run_matrix(isolated, 'localgen', target_dir)


if __name__ == '__main__':
    main()
