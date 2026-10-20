#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}"
v="$RELEASE_VERSION"
expected=("zxs-$v-x86_64-unknown-linux-gnu.tar.gz" "web-interface-$v.tar.gz")
test "$(find release-assets -mindepth 1 -maxdepth 1 -type f | wc -l)" = 2
for asset in "${expected[@]}"; do test -s "release-assets/$asset"; done
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
