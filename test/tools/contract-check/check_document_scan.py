#!/usr/bin/env python3
"""Replay documentation discovery against the hash-verified retired walker."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import resource
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    catalog = json.loads((ROOT / 'compliance/rust-sources.json').read_text())
    with tempfile.TemporaryDirectory(prefix='document-scanner-oracle-') as directory:
        temporary = Path(directory)
        for name, version in [('same-file', '1.0.6'), ('walkdir', '2.5.0')]:
            owner = 'src/foundation/' + name
            archived = json.loads(gzip.decompress((ROOT / f'compliance/{name}-{version}-retirement.json.gz').read_bytes()))
            current = dict(catalog['retired'][owner])
            retirement = current.pop('retirement')
            require(archived['package'] == current, f'archived package receipt drift: {owner}')
            require(archived['external_tests'] == retirement['external_tests'],
                    f'archived external-test receipt drift: {owner}')
            expected = {owner + '/' + path: digest for path, digest in current['files_sha256'].items()}
            expected.update({row['path']: row['sha256'] for row in archived['external_tests']['files']})
            require(set(archived['files']) == set(expected), f'archived file inventory drift: {owner}')
            for path, encoded in archived['files'].items():
                data = base64.b64decode(encoded, validate=True)
                require(hashlib.sha256(data).hexdigest() == expected[path], f'archived file digest drift: {path}')
                target = temporary / path
                require(target.resolve().is_relative_to(temporary), f'archive path escapes temporary directory: {path}')
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            crate = name.replace('-', '_')
            command = ['rustc', '--edition=2018', '--crate-name', crate, '--crate-type=rlib',
                       str(temporary / owner / 'src/lib.rs'), '-L', str(temporary), '-o', str(temporary / f'lib{crate}.rlib')]
            if name == 'walkdir':
                command += ['--extern', f'same_file={temporary}/libsame_file.rlib']
            subprocess.run(command, check=True, cwd=ROOT)
        binary = temporary / 'document-scan-test'
        subprocess.run(['rustc', '--edition=2024', '--test', '--cfg', 'walker_oracle',
                        str(ROOT / 'test/tools/contract-check/document_scan.rs'),
                        '--extern', f'walkdir={temporary}/libwalkdir.rlib', '-L', str(temporary), '-o', str(binary)], check=True, cwd=ROOT)
        def limit_handles():
            _, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
            limit = 32 if hard == resource.RLIM_INFINITY else min(32, hard)
            resource.setrlimit(resource.RLIMIT_NOFILE, (limit, hard))
        subprocess.run([str(binary), '--test-threads=1'], check=True, cwd=ROOT, preexec_fn=limit_handles)


if __name__ == '__main__':
    main()
