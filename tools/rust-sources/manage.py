#!/usr/bin/env python3
"""Verify the single-workspace source tree and immutable upstream provenance."""
import argparse
from collections import deque
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parent))
import external_tests
import source_relocation

ROOT = Path(__file__).resolve().parents[2]
CATALOG = "compliance/rust-sources.json"
ORIGINS = "compliance/provenance/origins.json"
DOC = "docs/THIRD_PARTY_RUST_SOURCES.md"
POLICY = "tools/rust-sources/workspaces.json"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
NATIVE_TARGET = "x86_64-unknown-linux-gnu"
WEB_TARGET = "wasm32-unknown-unknown"
HEX = re.compile(r"[0-9a-f]{64}")
DESCRIPTION_PATHS = (
    ("tools/zxs-bindgen/PROVENANCE.json", "compliance/provenance/packages/zxs-bindgen-0.2.129/PROVENANCE.json"),
    ("tools/zxs-bindgen", "tools/web-bindings"),
    ("crates/zxs-naming/README.md", "docs/components/naming.md"),
    ("crates/zxs-naming", "src/naming"),
    ("vendor/web/zxs-web-core", "src/interface"),
    ("vendor/web/zxs-web-macro", "src/macros"),
    ("vendor/web/dependencies/patches.json", "compliance/provenance/shared/vendor/web/dependencies/patches.json"),
    ("vendor/web/README.md", "docs/components/interface.md"),
    ("web/runtime", "src/interface"),
    ("web/dist", "build/web"),
    ("web/src/", "src/web/src/"),
)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def files(directory):
    """Inventory regular files only; never follow symlinks or read special files."""
    if directory.is_symlink():
        raise ValueError(f"source directory symlink: {directory}")
    if not directory.is_dir():
        raise ValueError(f"missing source directory: {directory}")
    result = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"source symlink: {path}")
        if path.is_file():
            result[path.relative_to(directory).as_posix()] = digest(path.read_bytes())
        elif not path.is_dir():
            raise ValueError(f"source special file: {path}")
    return result


def safe_path(root, relative):
    if (not isinstance(relative, str) or not relative or "\\" in relative
            or any(part in ("", ".", "..") or ":" in part for part in relative.split("/"))):
        raise ValueError(f"unsafe inventory path: {relative}")
    path = Path(relative)
    if path.is_absolute():
        raise ValueError(f"unsafe inventory path: {relative}")
    target = root / path
    if not target.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"inventory escapes repository: {relative}")
    current = root
    for part in path.parts:
        current /= part
        if current.is_symlink():
            raise ValueError(f"inventory path symlink: {relative}")
    return target


def project_manifests(root):
    result = {"Cargo.toml"}
    for base in ("src", "tools", "examples", "test"):
        for path in (root / base).rglob("Cargo.toml"):
            result.add(path.relative_to(root).as_posix())
    return result


def manifests(root):
    return {path for path in project_manifests(root)
            if "workspace" in tomllib.loads(safe_path(root, path).read_text(encoding="utf-8"))}


def structure(root, policy):
    """No nested Git roots, Cargo workspaces, registry edges, or package locks."""
    if [workspace["manifest"] for workspace in policy["workspaces"]] != ["Cargo.toml"]:
        raise ValueError("exactly one root Cargo workspace is required")
    verify_target_policy(policy)
    if any(path.startswith("test/") for path in project_manifests(root)):
        raise ValueError("Cargo package manifests are forbidden beneath test/")
    if manifests(root) != {"Cargo.toml"}:
        raise ValueError("nested/independent Cargo workspace")
    skipped = {".git", "target", "build", "runtime", "zxs-data", "xcs-data", ".agents", ".aws", ".codex"}
    for directory, children, names in os.walk(root, followlinks=False):
        relative = Path(directory).relative_to(root)
        if relative == Path("."):
            children[:] = [name for name in children if name not in skipped]
        if relative != Path(".") and "Cargo.toml" in names:
            # Existing generated example distributions and Cargo caches are
            # outside the maintained source boundary. A cc/src/target source
            # directory is not a package root and remains fully checked.
            children[:] = [name for name in children if name not in {"target", "dist", ".dist-staging", ".dist-backup"}]
        if relative != Path(".") and (".git" in children or ".git" in names):
            raise ValueError(f"nested Git repository: {relative}")
        if "Cargo.lock" in names and relative != Path("."):
            raise ValueError(f"nested Cargo.lock: {relative}")
        if relative.parts and relative.parts[0] == "test":
            for name in children + names:
                if (root / relative / name).is_symlink():
                    raise ValueError(f"test source symlink: {relative / name}")
        if relative.parts and relative.parts[0] in ("src", "tools", "test", "tests", "deploy", "examples"):
            for name in names:
                if Path(name).suffix.lower() in (".md", ".rst", ".adoc"):
                    raise ValueError(f"scattered documentation outside docs/: {relative / name}")
                if re.fullmatch(r"(?:LICEN[CS]E|COPYING|COPYRIGHT|NOTICES?|AUTHORS)(?:[._-].*)?", name, re.IGNORECASE):
                    raise ValueError(f"scattered legal material outside compliance/: {relative / name}")
        if (root / relative / "Cargo.toml").is_file() and relative != Path("."):
            scaffold = set(names) & {".gitignore", ".gitattributes", ".editorconfig", ".rustfmt.toml", "rustfmt.toml", "clippy.toml", ".clippy.toml", "Makefile.toml", "justfile", "flake.nix", "flake.lock", ".pre-commit-config.yaml", "deny.toml"}
            scaffold.update(set(children) & {".github", ".gitlab", ".vscode", ".idea", ".cargo", "LICENSES"})
            if scaffold:
                raise ValueError(f"nested package development/legal scaffolding: {relative} {sorted(scaffold)}")
    if (root / ".gitmodules").exists():
        raise ValueError("Git submodules are not permitted in the monorepo")
    config = tomllib.loads((root / ".cargo/config.toml").read_text(encoding="utf-8"))
    if config.get("net", {}).get("offline") is not True:
        raise ValueError("Cargo must default to offline operation")
    sources = config.get("source", {})
    replacement = sources.get("crates-io", {}).get("replace-with")
    directory = sources.get(replacement, {}).get("directory")
    if not directory:
        raise ValueError("missing empty registry fallback")
    registry = safe_path(root, directory)
    if not registry.is_dir() or any(path.is_dir() or path.is_symlink() for path in registry.iterdir()):
        raise ValueError("registry fallback must contain no source packages or links")
    root_manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    workspace = root_manifest["workspace"]
    verification_members = policy.get("verification_members", [])
    if ((verification_members and not workspace.get("default-members"))
            or len(set(verification_members)) != len(verification_members)
            or set(verification_members) != {
                path for path in workspace.get("members", []) if path.startswith("src/foundation/")}
            or any(not path.startswith("src/foundation/") or path not in workspace.get("members", [])
                   or path in workspace.get("default-members", []) for path in verification_members)):
        raise ValueError("foundation verification members must be explicit non-default root members")
    workspace_dependencies = root_manifest["workspace"].get("dependencies", {})
    for relative in project_manifests(root):
        manifest_path = safe_path(root, relative)
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        sections = [manifest, *manifest.get("target", {}).values()]
        if relative == "Cargo.toml":
            sections.append(manifest["workspace"])
        for section in sections:
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for name, dependency in section.get(kind, {}).items():
                    if not isinstance(dependency, dict):
                        raise ValueError(f"non-path dependency: {relative} {name}")
                    base = manifest_path.parent
                    if dependency.get("workspace") is True:
                        dependency = workspace_dependencies.get(name)
                        base = root
                    if (not isinstance(dependency, dict) or "path" not in dependency
                            or any(key in dependency for key in ("git", "registry", "branch", "tag", "rev"))):
                        raise ValueError(f"non-path dependency: {relative} {name}")
                    path = base / dependency["path"]
                    if not path.resolve().is_relative_to(root.resolve()):
                        raise ValueError(f"external dependency path: {relative} {name}")
                    normalized = Path(os.path.normpath(path))
                    target = safe_path(root, normalized.relative_to(root).as_posix())
                    if not (target / "Cargo.toml").is_file():
                        raise ValueError(f"missing local dependency: {relative} {name}")
    verify_feature_edges(root)


def verify_target_policy(policy):
    """The maintained native platform is Linux amd64; WASM is Web-only."""
    workspace_targets = policy["workspaces"][0]["targets"]
    if (not workspace_targets or len(set(workspace_targets)) != len(workspace_targets)
            or set(workspace_targets) - {NATIVE_TARGET, WEB_TARGET}):
        raise ValueError("unsupported workspace target: only Linux amd64 and Web WASM are maintained")
    selected = set()
    for profile in policy.get("profiles", []):
        expected = WEB_TARGET if profile["id"] == "web" else NATIVE_TARGET
        if profile["targets"] != [expected]:
            raise ValueError(f"unsupported profile target: {profile['id']} requires {expected}")
        selected.add(expected)
    if policy.get("profiles") and selected != set(workspace_targets):
        raise ValueError("workspace/profile target coverage drift")


def verify_feature_edges(root):
    """Validate dormant feature edges too, not just Cargo's selected features."""
    documents = {relative: tomllib.loads(safe_path(root, relative).read_text(encoding="utf-8"))
                 for relative in project_manifests(root)}
    workspace = documents["Cargo.toml"]["workspace"].get("dependencies", {})
    dependencies = {}
    for relative, manifest in documents.items():
        edges = {}
        sections = [manifest, *manifest.get("target", {}).values()]
        if relative == "Cargo.toml":
            sections.append(manifest["workspace"])
        for section in sections:
            for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
                for alias, dependency in section.get(kind, {}).items():
                    base = root / Path(relative).parent
                    if dependency.get("workspace") is True:
                        dependency, base = workspace[alias], root
                    target = (base / dependency["path"] / "Cargo.toml").resolve().relative_to(root.resolve()).as_posix()
                    edges.setdefault(alias, []).append((dependency, target))
        dependencies[relative] = edges
    features = {}
    for relative, manifest in documents.items():
        declared = manifest.get("features", {})
        hidden = {value[4:] for values in declared.values() for value in values if value.startswith("dep:")}
        features[relative] = set(declared) | {alias for alias, edges in dependencies[relative].items()
                                            if alias not in hidden and any(edge.get("optional") for edge, _ in edges)}
    for relative, manifest in documents.items():
        edges = dependencies[relative]
        for name, values in manifest.get("features", {}).items():
            for value in values:
                if value.startswith("dep:"):
                    alias = value[4:]
                    valid = alias in edges and any(edge.get("optional") for edge, _ in edges[alias])
                elif "/" in value:
                    alias, feature = value.split("/", 1)
                    weak = alias.endswith("?")
                    alias = alias.removesuffix("?")
                    valid = (alias in edges and all(feature in features[target] for _, target in edges[alias])
                             and (not weak or any(edge.get("optional") for edge, _ in edges[alias])))
                else:
                    valid = value in features[relative]
                if not valid:
                    raise ValueError(f"dangling feature edge: {relative} {name} -> {value}")
        for alias, targets in edges.items():
            for dependency, target in targets:
                for feature in dependency.get("features", []):
                    if feature not in features[target]:
                        raise ValueError(f"dangling dependency feature: {relative} {alias}/{feature}")


def cargo_metadata(root, manifest="Cargo.toml", target=None, cargo_home=None):
    if manifest != "Cargo.toml":
        raise ValueError("metadata must resolve the single root workspace")
    command = ["cargo", "metadata", "--locked", "--offline", "--format-version", "1", "--manifest-path", str(root / manifest)]
    if target:
        command += ["--filter-platform", target]
    environment = os.environ.copy()
    if cargo_home:
        environment["CARGO_HOME"] = str(cargo_home)
    result = subprocess.run(command, cwd=root, env=environment, capture_output=True, text=True, encoding="utf-8")
    if result.returncode:
        raise ValueError(f"offline metadata failed: {manifest} {target or 'all targets'}\n{result.stderr}")
    return json.loads(result.stdout)


def closure(metadata, initial_role="runtime", roots=None):
    """Traverse a conservative workspace-feature graph; skip direct dev edges.

    Cargo unifies workspace features before roots are selected below. Another
    member or dev dependency can therefore activate normal/build edges here.
    Roles describe this superset, not an isolated consumer or linked artifact.
    """
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    if initial_role not in ("runtime", "compile"):
        raise ValueError(f"invalid closure role: {initial_role}")
    queue = deque((package, initial_role) for package in (roots if roots is not None else metadata["workspace_members"]))
    roles = {}
    while queue:
        key, role = queue.popleft()
        if any("proc-macro" in target["kind"] for target in packages[key]["targets"]):
            role = "compile"
        if role in roles.setdefault(key, set()):
            continue
        roles[key].add(role)
        for dependency in nodes[key]["deps"]:
            for kind in dependency["dep_kinds"]:
                if kind["kind"] != "dev":
                    queue.append((dependency["pkg"], "compile" if role == "compile" or kind["kind"] == "build" else "runtime"))
    return {key: dict(roles=sorted(value), features=sorted(nodes[key]["features"])) for key, value in roles.items()}


def workspace_role(manifest):
    return "compile" if manifest.startswith("tools/") else "runtime"


def entries(policy):
    profiles = policy.get("profiles")
    if not profiles:
        profiles = [dict(id="workspace", targets=policy["workspaces"][0]["targets"], role="runtime")]
    names = set()
    for profile in profiles:
        if profile["id"] in names:
            raise ValueError("duplicate source closure profile")
        names.add(profile["id"])
        for target in profile["targets"]:
            yield dict(profile, manifest="Cargo.toml", target=target)


def selected_sources(root, metadata, initial_role="runtime", extracted_sources=(), profile=None):
    roots = None
    if profile and (profile.get("packages") or profile.get("manifests")):
        names, paths = set(profile.get("packages", [])), set(profile.get("manifests", []))
        roots = [package["id"] for package in metadata["packages"]
                 if package["id"] in metadata["workspace_members"]
                 and (package["name"] in names or Path(package["manifest_path"]).relative_to(root).as_posix() in paths)]
        found_names = {package["name"] for package in metadata["packages"] if package["id"] in roots}
        found_paths = {Path(package["manifest_path"]).relative_to(root).as_posix() for package in metadata["packages"] if package["id"] in roots}
        if names - found_names or paths - found_paths:
            raise ValueError(f"profile roots missing from root workspace: {profile['id']}")
    used = closure(metadata, initial_role, roots)
    selected = {}
    for package in metadata["packages"]:
        relative = Path(package["manifest_path"]).parent.relative_to(root).as_posix()
        if package["id"] in used and (relative.startswith("src/foundation/") or relative in extracted_sources):
            selected[relative] = used[package["id"]]
    return dict(sorted(selected.items()))


def verify_source_paths(root, metadata):
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"])
        if not manifest.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"external Rust source: {package['id']} {manifest}")
        safe_path(root, manifest.relative_to(root).as_posix())
        if package.get("source") is not None:
            raise ValueError(f"registry/Git Rust source is forbidden: {package['id']}")


def source_directories(root, policy):
    result = set(policy.get("integrated_sources", ["src/macros", "tools/web-bindings"]))
    foundation = root / "src/foundation"
    if foundation.exists():
        for path in foundation.iterdir():
            if path.is_symlink() or not path.is_dir() or not (path / "Cargo.toml").is_file():
                raise ValueError(f"invalid foundation source directory: {path}")
            result.add(path.relative_to(root).as_posix())
    return result


def upstream_identity(upstream):
    name, version, checksum = (upstream[key] for key in ("name", "version", "archive_sha256"))
    if (not re.fullmatch(r"[A-Za-z0-9_-]+", name)
            or not re.fullmatch(r"[0-9][0-9A-Za-z.+-]*", version) or not HEX.fullmatch(checksum)
            or upstream["archive_url"] != f"https://static.crates.io/crates/{name}/{name}-{version}.crate"):
        raise ValueError(f"invalid upstream archive identity: {name}@{version}")
    return name, version, checksum


def origins(root):
    records = read_json(root / ORIGINS)["packages"]
    result = {}
    for record in records:
        key = record["origin_directory"]
        if key in result:
            raise ValueError(f"duplicate historical origin: {key}")
        upstream_identity(record["upstream"])
        if not record.get("original_files_sha256") or any(not HEX.fullmatch(value) for value in record["original_files_sha256"].values()):
            raise ValueError(f"missing/invalid original file receipts: {key}")
        for material in (*record.get("central_license_materials", []), *record.get("central_original_metadata", []), *record.get("central_original_documentation", [])):
            if not safe_path(root, material).is_file():
                raise ValueError(f"missing centralized origin material: {material}")
        result[key] = record
    return result


def origin_hashes(records):
    return {key: digest(json.dumps(record, sort_keys=True, ensure_ascii=False).encode()) for key, record in sorted(records.items())}


def match_origin(package, records):
    key = package.get("origin_directory")
    if key is not None:
        record = records.get(key)
        if record is None or record["upstream"] != package["upstream"]:
            raise ValueError(f"upstream identity drift: {key}")
        return record
    identity = upstream_identity(package["upstream"])
    matches = [record for record in records.values() if upstream_identity(record["upstream"]) == identity]
    if len(matches) != 1:
        raise ValueError(f"ambiguous/missing historical origin: {package['directory']}")
    return matches[0]


def provenance(root):
    bases = ["compliance/provenance", "compliance/licenses"]
    if (root / "docs/upstream").exists():
        bases.append("docs/upstream")
    result = {f"{base}/{path}": value for base in bases for path, value in files(root / base).items()}
    migration = root / "compliance/provenance/document-migration.json"
    if migration.exists():
        for record in read_json(migration)["files"]:
            central = record["central_path"]
            if central.startswith(("compliance/", "docs/upstream/")):
                if digest(safe_path(root, central).read_bytes()) != record["sha256"]:
                    raise ValueError(f"original archive material checksum drift: {central}")
    return result


def current_description(text):
    """Migrate path tokens once, without matching inside an already moved path."""
    # Repair descriptions produced by the old unbounded replacement. This is
    # descriptive metadata only, never the immutable upstream file inventory.
    text = re.sub(r"(?:compliance/provenance/shared/){2,}(?=vendor/web/dependencies/patches\.json)",
                  "compliance/provenance/shared/", text)
    for before, after in DESCRIPTION_PATHS:
        text = re.sub(description_path_pattern(before), lambda _: after, text)
    return text


def description_path_pattern(path):
    end = "" if path.endswith("/") else r"(?![A-Za-z0-9_-]|\.[A-Za-z0-9])"
    return rf"(?<![A-Za-z0-9_./-]){re.escape(path)}{end}"


def verify_descriptions(root, package):
    for key in ("changes", "merged_into"):
        text = package.get(key, "")
        if current_description(text) != text:
            raise ValueError(f"outdated source description: {package['directory']} {key}")
        # Build output need not exist in a clean checkout. Migrated references
        # to retained source, documentation and provenance must still resolve.
        for target in {after for _, after in DESCRIPTION_PATHS if after != "build/web"}:
            if re.search(description_path_pattern(target), text):
                if not safe_path(root, target.rstrip("/")).exists():
                    raise ValueError(f"missing source description path: {target}")


def resolutions(root, policy, merged_sources=()):
    result, metadata = [], {}
    with tempfile.TemporaryDirectory(prefix="zxs-empty-cargo-") as home:
        for entry in entries(policy):
            target = entry["target"]
            if target not in metadata:
                metadata[target] = cargo_metadata(root, "Cargo.toml", target, home)
                verify_source_paths(root, metadata[target])
            selected = selected_sources(root, metadata[target], entry.get("role", "runtime"), source_directories(root, policy) | set(merged_sources), entry)
            result.append(dict(manifest="Cargo.toml", profile=entry["id"], target=target, packages=selected))
    return result


def retired_sources(root, catalog, records):
    """Retired implementations keep their last receipt, never an active path."""
    retired = catalog.get("retired", {})
    if not isinstance(retired, dict):
        raise ValueError("invalid retired source inventory")
    for relative, package in retired.items():
        if (relative != package["directory"] or not relative.startswith("src/foundation/")
                or relative in catalog["packages"]):
            raise ValueError(f"invalid retired source identity: {relative}")
        path = safe_path(root, relative)
        if path.exists():
            raise ValueError(f"retired source has returned: {relative}")
        match_origin(package, records)
        retirement = package.get("retirement", {})
        if (not retirement.get("reason") or not retirement.get("category")
                or not retirement.get("targets") or not package.get("files_sha256")
                or retirement.get("file_count") != len(package["files_sha256"])
                or not isinstance(retirement.get("bytes"), int) or retirement["bytes"] < 0
                or any(not HEX.fullmatch(value) for value in package["files_sha256"].values())):
            raise ValueError(f"incomplete retired source receipt: {relative}")
    expected = catalog.get("retired_records_sha256", {})
    actual = origin_hashes(retired)
    if actual != expected:
        raise ValueError("immutable retired source receipt drift")
    return retired


def retirement_plan(root, previous, plan, records, policy):
    """Only an explicit, reviewed removal plan may retire registered sources."""
    retired = dict(retired_sources(root, previous, records))
    packages = dict(previous["packages"])
    if plan is None:
        return packages, retired
    if not isinstance(plan, dict) or plan.get("schema") != 1 or not isinstance(plan.get("packages"), list):
        raise ValueError("invalid explicit source retirement plan")
    targets = sorted({entry["target"] for entry in entries(policy)})
    seen = set()
    for item in plan["packages"]:
        relative = item["directory"]
        if relative in seen or relative in retired:
            raise ValueError(f"duplicate/already retired source: {relative}")
        seen.add(relative)
        before = packages.get(relative)
        if before is None or item.get("package") != before:
            raise ValueError(f"retirement plan does not match last source receipt: {relative}")
        if not relative.startswith("src/foundation/") or safe_path(root, relative).exists():
            raise ValueError(f"retirement requires a removed foundation source: {relative}")
        match_origin(before, records)
        package = dict(before)
        package["retirement"] = dict(reason=item["reason"], category=item["category"],
                                     bytes=item["bytes"], file_count=item["file_count"], targets=targets)
        retired[relative] = package
        del packages[relative]
    return packages, retired


def upgrade_source_prefixes(root, item):
    prefixes = item.get("source_prefixes")
    if (not isinstance(prefixes, list) or not prefixes
            or any(not isinstance(prefix, str) or not prefix.endswith("/") for prefix in prefixes)
            or len(set(prefixes)) != len(prefixes)):
        raise ValueError("source upgrade requires explicit nonempty source_prefixes")
    for prefix in prefixes:
        safe_path(root, prefix[:-1])
        if any(prefix != other and prefix.startswith(other) for other in prefixes):
            raise ValueError("overlapping source upgrade prefixes")
    return tuple(prefixes)


def superseded_sources(root, catalog, records, check_manifests=True):
    """Preserve prior source receipts and immutable same-path upgrade chains."""
    superseded = catalog.get("superseded", {})
    if (not isinstance(superseded, dict)
            or (superseded and catalog.get("schema", 0) < 4)
            or (catalog.get("schema", 0) >= 4
                and not {"superseded", "superseded_records_sha256"} <= catalog.keys())):
        raise ValueError("invalid superseded source inventory")
    if origin_hashes(superseded) != catalog.get("superseded_records_sha256", {}):
        raise ValueError("immutable superseded source receipt drift")
    successors = set()
    for origin, receipt in superseded.items():
        package = receipt["package"]
        relative = package["directory"]
        successor = receipt["successor_origin_directory"]
        if (origin != package["origin_directory"] or not relative.startswith("src/foundation/")
                or not isinstance(receipt.get("reason"), str) or not receipt["reason"].strip()
                or not package.get("files_sha256")
                or any(not HEX.fullmatch(value) for value in package["files_sha256"].values())):
            raise ValueError(f"incomplete superseded source receipt: {origin}")
        safe_path(root, relative)
        upgrade_source_prefixes(root, receipt)
        match_origin(package, records)
        record = records.get(successor)
        if (record is None or successor == origin or successor in successors
                or record["upstream"]["name"] != package["upstream"]["name"]
                or record["upstream"]["version"] == package["upstream"]["version"]):
            raise ValueError(f"invalid superseded source successor: {origin}")
        successors.add(successor)
        # Direct links never change on later upgrades. Follow the chain to the
        # current receipt, including an explicit eventual retirement.
        seen = {origin}
        while successor in superseded:
            if successor in seen:
                raise ValueError(f"cyclic superseded source history: {origin}")
            seen.add(successor)
            next_receipt = superseded[successor]
            if next_receipt["package"]["directory"] != relative:
                raise ValueError(f"superseded source path drift: {origin}")
            successor = next_receipt["successor_origin_directory"]
        current = catalog["packages"].get(relative) or catalog.get("retired", {}).get(relative)
        if current is None or current["origin_directory"] != successor:
            raise ValueError(f"superseded source has no current successor: {origin}")
        match_origin(current, records)
        if current["version"] != current["upstream"]["version"]:
            raise ValueError(f"upgraded source version drift: {relative}")
        if check_manifests and relative in catalog["packages"]:
            manifest = tomllib.loads((safe_path(root, relative) / "Cargo.toml").read_text(encoding="utf-8"))["package"]
            if manifest["name"] != current["name"] or manifest["version"] != current["version"]:
                raise ValueError(f"upgraded source manifest identity drift: {relative}")
    return superseded


def upgrade_plan(root, previous, plan, records):
    """Only an exact reviewed plan may change an active upstream identity."""
    packages = dict(previous["packages"])
    superseded = dict(superseded_sources(root, previous, records, check_manifests=False))
    if plan is None:
        return packages, superseded
    if not isinstance(plan, dict) or plan.get("schema") != 1 or not isinstance(plan.get("packages"), list):
        raise ValueError("invalid explicit source upgrade plan")
    seen = set()
    for item in plan["packages"]:
        relative = item["directory"]
        if relative in seen:
            raise ValueError(f"duplicate source upgrade: {relative}")
        seen.add(relative)
        before = packages.get(relative)
        if before is None or item.get("package") != before:
            raise ValueError(f"upgrade plan does not match last source receipt: {relative}")
        prior_origin = match_origin(before, records)["origin_directory"]
        successor = item["successor_origin_directory"]
        record = records.get(successor)
        if (not relative.startswith("src/foundation/") or prior_origin in superseded
                or record is None or successor in previous.get("origin_records_sha256", {})
                or record["upstream"]["name"] != before["upstream"]["name"]
                or record["upstream"]["version"] == before["upstream"]["version"]):
            raise ValueError(f"upgrade requires a new same-package origin: {relative}")
        if not isinstance(item.get("reason"), str) or not item["reason"].strip():
            raise ValueError(f"source upgrade requires a reason: {relative}")
        manifest = tomllib.loads((safe_path(root, relative) / "Cargo.toml").read_text(encoding="utf-8"))["package"]
        if manifest["name"] != before["name"] or manifest["version"] != record["upstream"]["version"]:
            raise ValueError(f"upgrade manifest does not match successor identity: {relative}")
        prefixes = upgrade_source_prefixes(root, item)
        expected = {path: value for path, value in record["original_files_sha256"].items()
                    if path.startswith(prefixes)}
        actual = {path: value for path, value in files(safe_path(root, relative)).items()
                  if path.startswith(prefixes)}
        if (any(not any(path.startswith(prefix) for path in expected) for prefix in prefixes)
                or actual != expected):
            raise ValueError(f"upgraded source differs from successor release: {relative}")
        superseded[prior_origin] = dict(package=before, successor_origin_directory=successor,
                                       reason=item["reason"], source_prefixes=list(prefixes))
        package = dict(before, origin_directory=successor, upstream=record["upstream"])
        for key in ("vcs", "license", "repository"):
            package.pop(key, None)
            if key in record:
                package[key] = record[key]
        package["changes"] = before.get("changes", "") + "；上游发布包升级：" + item["reason"]
        packages[relative] = package
    return packages, superseded


def refresh(root, retire_plan=None, upgrade_plan_data=None):
    policy = read_json(root / POLICY)
    structure(root, policy)
    previous = read_json(root / CATALOG)
    records = origins(root)
    hashes = origin_hashes(records)
    for key, expected in previous.get("origin_records_sha256", {}).items():
        if hashes.get(key) != expected:
            raise ValueError(f"immutable historical origin drift: {key}")
    material = provenance(root)
    for path, expected in previous.get("provenance_sha256", {}).items():
        # New imports may append original identities and document migration
        # entries; original per-package materials must retain their bytes.
        if path in (ORIGINS, "compliance/provenance/document-migration.json"):
            continue
        if material.get(path) != expected:
            raise ValueError(f"immutable original material drift: {path}")
    active, superseded = upgrade_plan(root, previous, upgrade_plan_data, records)
    active, retired = retirement_plan(root, dict(previous, packages=active), retire_plan, records, policy)
    actual = source_directories(root, policy)
    if actual != set(active):
        raise ValueError(f"unregistered source directories: {sorted(actual ^ set(active))}")
    packages = {}
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"].get("package", {})
    for relative, before in sorted(active.items()):
        directory = safe_path(root, relative)
        record = match_origin(before, records)
        package = dict(before)
        manifest = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))["package"]
        version = manifest["version"]
        if isinstance(version, dict) and version.get("workspace") is True:
            version = workspace["version"]
        package.update(name=manifest["name"], version=version, directory=relative,
                       origin_directory=record["origin_directory"], license_materials=record["central_license_materials"],
                       files_sha256=files(directory))
        for key in ("changes", "merged_into"):
            if key in package:
                package[key] = current_description(package[key])
        packages[relative] = package
    merges = []
    for before in previous.get("merges", []):
        record = match_origin(before, records)
        merged = dict(before)
        merged.update(origin_directory=record["origin_directory"],
                      license_materials=record["central_license_materials"],
                      files_sha256=files(safe_path(root, merged["directory"])))
        for key in ("changes", "merged_into"):
            if key in merged:
                merged[key] = current_description(merged[key])
        merges.append(merged)
    external, external_receipts = external_tests.load(root, dict(packages, **{m["directory"]: m for m in merges}))
    if external:
        if not previous.get("external_tests"):
            baseline_packages = [*previous["packages"].values(), *previous.get("merges", [])]
            baseline = {item["directory"] + "/" + path: value
                        for item in baseline_packages for path, value in item["files_sha256"].items()}
            source_relocation.verify(root, external, safe_path, baseline)
        else:
            source_relocation.read_receipt(root, safe_path)
    external_tests.check_previous(previous.get("external_tests", {}), external)
    catalog = dict(external_tests=external, external_test_mappings_sha256=external_receipts, schema=4, policy_sha256=digest((root / POLICY).read_bytes()),
                   config_sha256=digest((root / ".cargo/config.toml").read_bytes()),
                   manifests_sha256={path: digest(safe_path(root, path).read_bytes()) for path in sorted(project_manifests(root))},
                   lockfiles_sha256={"Cargo.lock": digest((root / "Cargo.lock").read_bytes())},
                   origins_sha256=digest((root / ORIGINS).read_bytes()), origin_records_sha256=hashes,
                   provenance_sha256=material, packages=packages, resolutions=resolutions(root, policy, [merge["directory"] for merge in merges]),
                   upstream_identities=[record["upstream"] for record in records.values()], merges=merges,
                   retired=retired, retired_records_sha256=origin_hashes(retired),
                   superseded=superseded, superseded_records_sha256=origin_hashes(superseded))
    retired_sources(root, catalog, records)
    superseded_sources(root, catalog, records)
    verify_origin_coverage(catalog, records)
    external_tests.check_removed(dict(previous["packages"], **{m["directory"]: m for m in previous.get("merges", [])}), dict(packages, **{m["directory"]: m for m in merges}), external)
    verify_retained_closure(catalog, policy)
    (root / CATALOG).write_text(json.dumps(catalog, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
    (root / DOC).write_text(document(catalog, policy), encoding="utf-8", newline="\n")
    print(f"Recorded {len(packages)} active source trees, {len(retired)} retired and {len(superseded)} superseded sources and {len(catalog['resolutions'])} root-workspace closure profiles.")


def cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


def document(catalog, policy):
    lines = ["# Rust 第三方源码来源与使用清单", "",
             "本仓库只有根 Git 仓库、根 Cargo workspace 与根 Cargo.lock。活动 Rust 依赖全部通过仓库内 path 连接；基础实现位于 src/foundation，界面内核与命名规范化实现分别整合进 src/interface 与 src/naming。没有独立第三方工作区或独立锁文件。", "",
             "精确来源及原始逐文件摘要见 compliance/provenance/origins.json；当前源码逐文件摘要、单锁文件及各目标闭包见 compliance/rust-sources.json；法律材料统一见 compliance/NOTICE.md 和 compliance/licenses。集中管理不改变各上游许可，也不把改造代码全部重新许可为根项目许可。", "",
             "runtime 是统一工作区特性图中 normal 边可达的运行候选；compile 包括 build/proc-macro 子图及维护工具。Cargo 先统一整个 workspace 的 features，再从各登记入口遍历；直接 dev 边虽被跳过，其他成员或开发依赖启用的特性仍可能间接扩大 normal/build 可达集合。这是保守可达超集，不是精确单入口运行时闭包或最终链接证明，不能单独据此判定源码可删除。元数据也不替代实际浏览器/SMB 验收或安全审计。每个活动源码身份必须至少进入一个受支持入口/目标的保守超集，移除身份仍须显式退役并保留来源证据。工具链及既有 Samba/SQLite/系统 SDK 管理边界保持独立。", "",
             "修改后显式执行 `python3 tools/rust-sources/manage.py refresh`，审查源码、来源清单与文档差异，再执行 `python3 tools/rust-sources/manage.py verify --metadata`；verify 不接受新哈希。删除依赖必须另提供经复核的 `refresh --retire-plan PLAN.json`，保存已移除实现的最后源码摘要及原因；普通 refresh 拒绝意外丢失目录或改写退役记录。原始身份与法律材料继续保存，不因退役而删除。原始发布包重建工具只向仓库外独立复核目录输出，不覆盖活动源码。", "",
             "同目录上游升级必须追加新的原始发布包及集中材料，并使用经复核的 `refresh --upgrade-plan PLAN.json`。计划 schema 为 1，packages 每项包含 directory、与上次活动记录完全相同的 package、successor_origin_directory、reason 及非空 source_prefixes（例如 `[\"src/\"]`）。升级时逐文件校验这些完整前缀下的源码与新发布包完全一致，不能仅提升版本号；路径清单、缺失、新增和内容差异均拒绝。schema 4 的 superseded 以旧 origin_directory 为键，保留原 package、后继来源、原因和已校验前缀，superseded_records_sha256 固定其摘要。连续升级保留不可改写的直接后继链，最终指向同目录的当前身份或显式退役记录。升级后有意的本地源码修改仍需常规 refresh，不改写上游原始收据。", "",
             "## 单工作区入口与解析目标", "", "| 入口闭包 | 用途 | 目标 |", "| --- | --- | --- |"]
    if policy.get("verification_members"):
        lines[2] += (" 基础库验证成员 " + "、".join(policy["verification_members"])
                     + " 显式登记在同一个根 workspace，不属于 default-members；这是上游回归对象，不是新业务组件。"
                     + " 追加测试材料的精确来源单独保存于 compliance/provenance/supplemental，不改写原始发布包记录。")
    for profile in policy.get("profiles", policy["workspaces"]):
        lines.append("| " + " | ".join(map(cell, [profile.get("id", "workspace"), profile.get("purpose", "根工作区"), ", ".join(profile["targets"])])) + " |")
    lines += ["", "## 逐项来源、许可与改造", "", "| 原始身份 | 当前组件 / 位置 | 许可材料 | 用途与闭包 | 修改及合并 |", "| --- | --- | --- | --- | --- |"]
    for key, package in sorted(catalog["packages"].items()):
        upstream = package["upstream"]
        roles, consumers = set(), set()
        for resolution in catalog["resolutions"]:
            if key in resolution["packages"]:
                roles.update(resolution["packages"][key]["roles"])
                consumers.add(f"{resolution.get('profile', 'workspace')}@{resolution['target']}")
        use = "+".join(sorted(roles)) or "保留超集"
        legal = ", ".join(package.get("license_materials", [])) or "原始发布包未提供独立许可资料；查中央来源记录的待复核状态"
        origin = f"{upstream['name']}@{upstream['version']}；{upstream['archive_url']}；SHA256 {upstream['archive_sha256']}"
        lines.append("| " + " | ".join(map(cell, [origin, f"{package['name']}@{package['version']}；{key}", f"{package.get('license', '见原始声明')}；{legal}", f"{use}；{', '.join(sorted(consumers))}", f"{package.get('status', '已整合')}；{package.get('changes', '')}；{package.get('merged_into', '')}"])) + " |")
    lines += ["", "## 已合并的上游实现", "", "这些实现保留来源身份与当前文件摘要，已经没有独立 Cargo 包或独立工作区。", ""]
    for merged in catalog.get("merges", []):
        upstream = merged["upstream"]
        usage = [f"{item.get('profile', 'workspace')}@{item['target']} ({'+'.join(item['packages'][merged['directory']]['roles'])})"
                 for item in catalog["resolutions"] if merged["directory"] in item["packages"]]
        lines.append(f"- `{upstream['name']}@{upstream['version']}` → `{merged['directory']}`；{merged.get('changes', '')}；实际闭包：{', '.join(usage) or '未选中'}；法律材料：{', '.join(merged.get('license_materials', []))}。")
    lines += ["", "## 已升级替换的历史来源", "",
              "以下旧发布包已在同一源码目录由后继身份替换，不再作为当前实现参与公告扫描。完整旧活动记录、原始摘要及法律材料保留，不能把历史来源伪装成当前源码或反复改写升级链。当前活动及合并实现仍全部扫描，不设公告豁免。", "",
              "| 原始身份 | 源码目录 | 直接后继来源 | 升级原因 | 导入校验前缀 |", "| --- | --- | --- | --- | --- |"]
    for _, receipt in sorted(catalog.get("superseded", {}).items()):
        package = receipt["package"]
        upstream = package["upstream"]
        lines.append("| " + " | ".join(map(cell, [f"{upstream['name']}@{upstream['version']}", package["directory"],
                     receipt["successor_origin_directory"], receipt["reason"], ", ".join(receipt["source_prefixes"])])) + " |")
    lines += ["", "## 已移除的非目标平台与可选实现", "",
              "以下实现已从活动源码及依赖声明中移除，不参与构建或当前依赖公告扫描。最后源码摘要、移除原因与原始来源仍在 JSON retired 中，原始材料继续保留。重新引入必须经过显式来源登记，不能悄悄恢复目录。", "",
              "| 原始身份 | 原目录 | 移除原因 | 最后文件数 / 字节 |", "| --- | --- | --- | --- |"]
    for relative, package in sorted(catalog.get("retired", {}).items()):
        upstream, retired = package["upstream"], package["retirement"]
        lines.append("| " + " | ".join(map(cell, [f"{upstream['name']}@{upstream['version']}", relative,
                     retired["reason"], f"{retired['file_count']} / {retired['bytes']}"])) + " |")
    external = catalog.get("external_tests", {})
    external_count = sum(item["file_count"] for item in external.values())
    external_bytes = sum(item["bytes"] for item in external.values())
    production_count = sum(len(item["files_sha256"]) for item in catalog["packages"].values()) + sum(len(item["files_sha256"]) for item in catalog.get("merges", []))
    lines += ["", "## 根测试目录的来源核算", "",
              f"包内当前文件 {production_count} 个，另有 {len(external)} 个原始身份所属的外置测试文件 {external_count} 个、{external_bytes} 字节；当前来源文件总计 {production_count + external_count} 个。测试从源码目录迁移至 test/，不是删除、裁剪或空间优化，仍完整计入来源覆盖、许可及安全审计。", "",
              "JSON external_tests 按原包目录和原始身份登记精确根目录、原路径、当前路径及逐文件摘要；external_test_mappings_sha256 固定当前映射文档。refresh 先验证外置清单，不能静默吞入新增文件或因目录迁移缩减来源核算。独立说明见 docs/EXTERNAL_TEST_SOURCES.md。", ""]
    lines += ["", "## 完整性、安全与验收", "",
              f"共 {len(catalog['packages'])} 个活动源码树、{len(catalog.get('retired', {}))} 个已移除来源、{len(catalog.get('superseded', {}))} 个已升级替换来源，{len(catalog['resolutions'])} 个入口/目标解析组合。原始 archive SHA-256 和原始逐文件摘要独立保留；当前修改、裁剪及合并后的文件另存摘要，不再声称其逐字节等同上游发布包。合并记录见 JSON merges，法律复核状态见 compliance/NOTICE.md。", "",
              "`.cargo/config.toml` 默认离线，并用空 registry 兜底拒绝遗漏的 registry 边。CI 检查所有活动 manifest 的本地路径、全部特性转发、单锁文件、工作区/嵌套 Git 边界、中央原始材料及空 Cargo home 的目标闭包。保留源码必须至少进入一个登记入口的保守可达超集；该记录不证明每个入口实际编译或链接了这些源码。安全门禁扫描全部保留和合并实现的原始身份；已移除实现单列历史记录，本地路径或改名不形成豁免。安全扫描、构建、真实浏览器、真实 Samba SDK 和物理盘故障验证分别提供证据。", ""]
    return "\n".join(lines)


def verify_origin_coverage(catalog, records):
    covered = [package["origin_directory"] for package in catalog["packages"].values()]
    covered.extend(merge["origin_directory"] for merge in catalog.get("merges", []))
    covered.extend(package["origin_directory"] for package in catalog.get("retired", {}).values())
    covered.extend(receipt["package"]["origin_directory"] for receipt in catalog.get("superseded", {}).values())
    if len(set(covered)) != len(covered) or set(covered) != set(records):
        raise ValueError("historical active/merged/retired/superseded source coverage drift")


def verify_retained_closure(catalog, policy):
    if policy.get("require_reachable_sources"):
        reachable = {path for resolution in catalog["resolutions"] for path in resolution["packages"]}
        retained = set(catalog["packages"]) | {merge["directory"] for merge in catalog.get("merges", [])}
        if retained - reachable:
            raise ValueError(f"retained source outside supported entry closures: {sorted(retained - reachable)}")


def verify(root, metadata=False):
    catalog, policy = read_json(root / CATALOG), read_json(root / POLICY)
    structure(root, policy)
    for key, relative in (("policy_sha256", POLICY), ("config_sha256", ".cargo/config.toml"), ("origins_sha256", ORIGINS)):
        if catalog[key] != digest(safe_path(root, relative).read_bytes()):
            raise ValueError(f"policy/config/origin content drift: {relative}")
    records = origins(root)
    if origin_hashes(records) != catalog["origin_records_sha256"] or provenance(root) != catalog["provenance_sha256"]:
        raise ValueError("centralized original provenance/license inventory drift")
    external, external_receipts = external_tests.load(root, dict(catalog["packages"], **{m["directory"]: m for m in catalog.get("merges", [])}))
    if external:
        source_relocation.read_receipt(root, safe_path)
    if external != catalog.get("external_tests", {}) or external_receipts != catalog.get("external_test_mappings_sha256", {}):
        raise ValueError("external test source accounting drift")
    current_manifests = project_manifests(root)
    if current_manifests != set(catalog["manifests_sha256"]):
        raise ValueError("manifest inventory drift")
    for relative, expected in catalog["manifests_sha256"].items():
        if digest(safe_path(root, relative).read_bytes()) != expected:
            raise ValueError(f"manifest content drift: {relative}")
    if set(catalog["lockfiles_sha256"]) != {"Cargo.lock"}:
        raise ValueError("exactly one root lockfile receipt is required")
    if digest((root / "Cargo.lock").read_bytes()) != catalog["lockfiles_sha256"]["Cargo.lock"]:
        raise ValueError("root lockfile drift")
    for package in tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))["package"]:
        if package.get("source") is not None or package.get("checksum") is not None:
            raise ValueError(f"root lock contains non-path Rust source: {package['name']}")
    if source_directories(root, policy) != set(catalog["packages"]):
        raise ValueError("source directory inventory drift")
    for relative, package in catalog["packages"].items():
        if relative != package["directory"]:
            raise ValueError(f"source directory identity mismatch: {relative}")
        match_origin(package, records)
        verify_descriptions(root, package)
        if files(safe_path(root, relative)) != package["files_sha256"]:
            raise ValueError(f"source content drift: {relative}")
    for merge in catalog.get("merges", []):
        match_origin(merge, records)
        verify_descriptions(root, merge)
        if files(safe_path(root, merge["directory"])) != merge["files_sha256"]:
            raise ValueError(f"merged source content drift: {merge['directory']}")
    retired_sources(root, catalog, records)
    superseded_sources(root, catalog, records)
    verify_origin_coverage(catalog, records)
    verify_retained_closure(catalog, policy)
    expected = {(entry["manifest"], entry["id"], entry["target"]) for entry in entries(policy)}
    actual = {(item["manifest"], item.get("profile", "workspace"), item["target"]) for item in catalog["resolutions"]}
    if expected != actual or len(actual) != len(catalog["resolutions"]):
        raise ValueError("resolution inventory drift: root profile/target coverage")
    identities = sorted(upstream_identity(record["upstream"]) for record in records.values())
    if sorted(upstream_identity(record) for record in catalog["upstream_identities"]) != identities:
        raise ValueError("upstream advisory identity inventory drift")
    if (root / DOC).read_text(encoding="utf-8") != document(catalog, policy):
        raise ValueError("third-party documentation out of sync")
    if metadata and resolutions(root, policy, [merge["directory"] for merge in catalog.get("merges", [])]) != catalog["resolutions"]:
        raise ValueError("closure resolution/features drift")
    print(f"Verified {len(catalog['packages'])} active source trees, {len(catalog.get('retired', {}))} retired and {len(catalog.get('superseded', {}))} superseded sources, immutable upstream provenance and one root workspace/lock.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("refresh", "verify"))
    parser.add_argument("--metadata", action="store_true")
    parser.add_argument("--retire-plan", type=Path, help="explicit reviewed removal plan; refresh only")
    parser.add_argument("--upgrade-plan", type=Path, help="explicit reviewed upstream replacement plan; refresh only")
    arguments = parser.parse_args()
    try:
        if (arguments.retire_plan or arguments.upgrade_plan) and arguments.command != "refresh":
            raise ValueError("--retire-plan and --upgrade-plan are only accepted by refresh")
        (refresh(ROOT, read_json(arguments.retire_plan) if arguments.retire_plan else None,
                 read_json(arguments.upgrade_plan) if arguments.upgrade_plan else None)
         if arguments.command == "refresh" else verify(ROOT, arguments.metadata))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"Rust source check failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
