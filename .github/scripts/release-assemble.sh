#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}"
v="$RELEASE_VERSION"
expected=("zxs-$v-x86_64-unknown-linux-gnu.tar.gz" "zxs-web-$v.tar.gz"
  "zxs-app-$v-x86_64-unknown-linux-gnu.deb"
  "zxs-app-$v-x86_64-pc-windows-msvc-setup.exe"
  "zxs-app-$v-aarch64-apple-darwin.app.tar.gz")
test "$(find release-assets -mindepth 1 -maxdepth 1 -type f | wc -l)" = 5
for asset in "${expected[@]}"; do test -s "release-assets/$asset"; done
cp docs/release.md release-assets/INSTALL.md
{
  echo "version=$v"
  echo "commit=$(git rev-parse HEAD)"
  echo "rust=$(sed -n 's/channel = "\(.*\)"/\1/p' rust-toolchain.toml)"
  echo 'wasm-bindgen-cli=0.2.129'
  echo 'tauri-cli=2.12.2'
  echo "workflow=${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-local}/actions/runs/${GITHUB_RUN_ID:-local}"
  echo 'desktop-signing=unsigned; macOS is not notarized'
  echo 'mobile=not distributed; SDK, project generation, signing and device validation pending'
} > release-assets/BUILD.txt
(cd release-assets && sha256sum -- * > SHA256SUMS && sha256sum --check SHA256SUMS)
