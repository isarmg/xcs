#!/usr/bin/env python3
"""Recreate unmodified registry sources in a separate empty review directory.

The maintained monorepo is never an output. Archives must match the pinned
inventory SHA-256 before tar processing; fetching is an explicit opt-in.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import tarfile
import tempfile
import tomllib
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]
CHECKSUM_COMMENT = (
    "This file only protects against accidental modifications. It is not a "
    "security mechanism and does not protect against malicious changes."
)
MAX_ARCHIVE_BYTES = 256 * 1024 * 1024
MAX_UNPACKED_BYTES = 1024 * 1024 * 1024
MAX_MEMBERS = 200_000


def digest_file(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def checksum_bytes(files, package_hash):
    value = {"$comment": CHECKSUM_COMMENT, "files": files, "package": package_hash}
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def registry_packages(catalog, selected=()):
    """Only the unmodified registry archive area is eligible for re-import."""
    packages = []
    for relative, package in sorted(catalog["packages"].items()):
        if not relative.startswith("vendor/rust/"):
            continue
        name, version = package["name"], package["version"]
        if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or not re.fullmatch(r"[0-9][0-9A-Za-z.+-]*", version):
            raise ValueError(f"unsafe package identity: {name}@{version}")
        directory = f"{name}-{version}"
        if relative != f"vendor/rust/{directory}" or package["directory"] != relative:
            raise ValueError(f"unexpected registry directory: {relative}")
        upstream = package["upstream"]
        if upstream["name"] != name or upstream["version"] != version:
            raise ValueError(f"upstream identity mismatch: {relative}")
        expected_url = f"https://static.crates.io/crates/{name}/{directory}.crate"
        if upstream["archive_url"] != expected_url:
            raise ValueError(f"unexpected archive URL: {relative}")
        if not re.fullmatch(r"[0-9a-f]{64}", upstream["archive_sha256"]):
            raise ValueError(f"invalid archive SHA-256: {relative}")
        if ".cargo-checksum.json" not in package["files_sha256"]:
            raise ValueError(f"missing inventory file receipt: {relative}")
        packages.append(package)
    if not packages:
        raise ValueError("catalog has no original registry receipts; use the immutable pre-monorepo catalog")
    requested = set(selected)
    identities = {f"{p['name']}@{p['version']}" for p in packages}
    if requested - identities:
        raise ValueError(f"unknown registry packages: {sorted(requested - identities)}")
    return [p for p in packages if not requested or f"{p['name']}@{p['version']}" in requested]


def archive_path(package, cache, temporary, fetch):
    filename = f"{package['name']}-{package['version']}.crate"
    candidates = [cache / filename, *sorted(cache.glob(f"*/{filename}"))]
    for candidate in candidates:
        if candidate.is_symlink():
            raise ValueError(f"archive cache symlink: {candidate}")
        if candidate.is_file():
            if candidate.stat().st_size > MAX_ARCHIVE_BYTES:
                raise ValueError(f"archive exceeds size limit: {filename}")
            if digest_file(candidate) != package["upstream"]["archive_sha256"]:
                raise ValueError(f"archive checksum mismatch: {filename}")
            return candidate
    if not fetch:
        raise ValueError(f"archive absent from cache: {filename}; use --fetch to explicitly allow retrieval")
    destination = temporary / filename
    url = package["upstream"]["archive_url"]
    with urlopen(url, timeout=60) as response, destination.open("xb") as sink:
        if response.geturl() != url:
            raise ValueError(f"archive URL redirected: {url}")
        total = 0
        while chunk := response.read(1024 * 1024):
            total += len(chunk)
            if total > MAX_ARCHIVE_BYTES:
                raise ValueError(f"archive exceeds size limit: {filename}")
            sink.write(chunk)
    if digest_file(destination) != package["upstream"]["archive_sha256"]:
        raise ValueError(f"archive checksum mismatch: {filename}")
    return destination


def archive_members(archive, directory):
    """Validate every header before creating anything from an archive.

    No extract/extractall is used: links, devices, duplicate names, alternate
    path spellings and paths outside the exact crate prefix are rejected.
    """
    result, seen, unpacked = [], set(), 0
    for index, member in enumerate(archive):
        if index >= MAX_MEMBERS:
            raise ValueError("archive has too many members")
        raw = member.name.rstrip("/") if member.isdir() else member.name
        parts = raw.split("/")
        if (not raw or raw.startswith("/") or "\\" in raw or "\x00" in raw
                or any(p in ("", ".", "..") or ":" in p for p in parts)
                or parts[0] != directory):
            raise ValueError(f"unsafe archive path: {member.name}")
        path = PurePosixPath(*parts[1:])
        if member.type not in (tarfile.REGTYPE, tarfile.AREGTYPE, tarfile.DIRTYPE):
            raise ValueError(f"archive link or special file: {member.name}")
        if len(parts) == 1:
            if not member.isdir():
                raise ValueError(f"archive prefix is not a directory: {member.name}")
            continue
        if str(path) in seen:
            raise ValueError(f"duplicate archive path: {member.name}")
        seen.add(str(path))
        if str(path) == ".cargo-checksum.json":
            raise ValueError("archive contains reserved Cargo checksum receipt")
        unpacked += member.size
        if member.size < 0 or unpacked > MAX_UNPACKED_BYTES:
            raise ValueError("archive exceeds unpacked size limit")
        result.append((path, member))
    return result


def extract_package(path, package, output):
    directory = f"{package['name']}-{package['version']}"
    # Callers also verify downloaded/cache archives, but this boundary remains
    # safe when used directly by tests or other maintenance tooling.
    if digest_file(path) != package["upstream"]["archive_sha256"]:
        raise ValueError(f"archive checksum mismatch: {directory}")
    expected = dict(package["files_sha256"])
    expected.pop(".cargo-checksum.json")
    with tarfile.open(path, "r:gz") as archive:
        members = archive_members(archive, directory)
        # Cargo vendor drops archived VCS metadata such as .gitignore and
        # .gitattributes. Reproduce only omissions already pinned by the file
        # receipt; no other upstream material may disappear during re-import.
        paths = {str(relative) for relative, member in members if member.isfile()}
        omitted = paths - set(expected)
        if any(not any(part.startswith(".git") for part in PurePosixPath(p).parts) for p in omitted):
            raise ValueError(f"archive file inventory mismatch: {directory}")
        members = [(relative, member) for relative, member in members
                   if not member.isfile() or str(relative) not in omitted]
        paths -= omitted
        if paths != set(expected):
            raise ValueError(f"archive file inventory mismatch: {directory}")
        target = output / directory
        target.mkdir()
        actual = {}
        for relative, member in members:
            destination = target.joinpath(*relative.parts)
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            with archive.extractfile(member) as source, destination.open("xb") as sink:
                shutil.copyfileobj(source, sink)
            destination.chmod(member.mode & 0o777)
            actual[str(relative)] = digest_file(destination)
            if actual[str(relative)] != expected[str(relative)]:
                raise ValueError(f"archive file checksum mismatch: {directory}/{relative}")
        manifest = tomllib.loads((target / "Cargo.toml").read_text())
        if (manifest["package"]["name"], manifest["package"]["version"]) != (package["name"], package["version"]):
            raise ValueError(f"archive manifest identity mismatch: {directory}")
        receipt = checksum_bytes(actual, package["upstream"]["archive_sha256"])
        if hashlib.sha256(receipt).hexdigest() != package["files_sha256"][".cargo-checksum.json"]:
            raise ValueError(f"Cargo checksum receipt mismatch: {directory}")
        (target / ".cargo-checksum.json").write_bytes(receipt)


def restore(catalog_path, output, cache, fetch=False, selected=()):
    catalog_path, output, cache = Path(catalog_path), Path(output), Path(cache)
    vendor_roots = [ROOT]
    if catalog_path.resolve().parent.name == "vendor":
        vendor_roots.append(catalog_path.resolve().parent)
    if output.is_symlink() or any(output.resolve().is_relative_to(v.resolve()) for v in vendor_roots):
        raise ValueError("output must be separate from the maintained repository/vendor tree and cannot be a symlink")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ValueError("output must be a new or empty directory")
    for parent in output.absolute().parents:
        if parent.is_symlink():
            raise ValueError(f"output parent symlink: {parent}")
    catalog = json.loads(catalog_path.read_text())
    packages = registry_packages(catalog, selected)
    # Stage the whole operation beside the requested output. Failure leaves an
    # existing empty output untouched, and never publishes a partial crate tree.
    with tempfile.TemporaryDirectory(prefix=".zxs-source-import-", dir=output.parent) as stage:
        stage = Path(stage)
        downloads = stage / "archives"
        sources = stage / "sources"
        downloads.mkdir()
        sources.mkdir()
        for package in packages:
            archive = archive_path(package, cache, downloads, fetch)
            extract_package(archive, package, sources)
        output.mkdir(exist_ok=True)
        if any(output.iterdir()):
            raise ValueError("output changed during import; refusing to overwrite")
        for package in packages:
            directory = f"{package['name']}-{package['version']}"
            destination = output / directory
            if destination.exists() or destination.is_symlink():
                raise ValueError(f"output package already exists: {directory}")
            (sources / directory).rename(destination)
    return len(packages)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=ROOT / "compliance/provenance/pre-monorepo/vendor/rust-sources.json")
    parser.add_argument("--output", type=Path, required=True)
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    parser.add_argument("--cache", type=Path, default=cargo_home / "registry/cache")
    parser.add_argument("--fetch", action="store_true", help="explicitly allow pinned official archive retrieval")
    parser.add_argument("--package", action="append", default=[], metavar="NAME@VERSION")
    args = parser.parse_args()
    try:
        count = restore(args.catalog, args.output, args.cache, args.fetch, args.package)
    except (ValueError, KeyError, OSError, tarfile.TarError) as error:
        parser.exit(1, f"Rust source re-import failed: {error}\n")
    print(f"Re-imported {count} pinned registry packages into {args.output}; maintained sources untouched")


if __name__ == "__main__":
    main()
