#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}"
v="$RELEASE_VERSION"
# Reject unexpected entries, including hidden files, directories and symlinks,
# before writing metadata or checksums into the release set.
python3 - "$v" <<'PYVALIDATE'
from pathlib import Path
import re
import stat
import sys

version = sys.argv[1]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+([-+][A-Za-z0-9.+-]+)?", version):
    raise SystemExit("RELEASE_VERSION must be a safe version filename")
assets = Path("release-assets")
if not stat.S_ISDIR(assets.lstat().st_mode):
    raise SystemExit("release-assets must be a real directory")
expected = {f"zxs-{version}-x86_64-unknown-linux-gnu.tar.gz", f"web-interface-{version}.tar.gz"}
if {path.name for path in assets.iterdir()} != expected:
    raise SystemExit("Release directory must contain exactly the two expected archives")
for name in expected:
    info = (assets / name).lstat()
    if not stat.S_ISREG(info.st_mode) or not info.st_size:
        raise SystemExit(f"Release archive must be a nonempty regular file: {name}")
PYVALIDATE
{
  printf '# ZXS %s 安装\n\n' "$v"
  cat docs/templates/release-install.md
} > release-assets/INSTALL.md
{
  echo "version=$v"
  echo "commit=$(git rev-parse HEAD)"
  echo "rust=$(sed -n 's/channel = "\(.*\)"/\1/p' rust-toolchain.toml)"
  echo 'web-bindings=0.2.129; derived Web-only wasm-bindgen command'
  echo "workflow=${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-local}/actions/runs/${GITHUB_RUN_ID:-local}"
  echo 'access=Web and SMB; Samba VFS is built separately against its pinned native SDK'
} > release-assets/BUILD.txt
(cd release-assets && sha256sum -- * > SHA256SUMS && sha256sum --check SHA256SUMS)
