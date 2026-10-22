#!/usr/bin/env bash
# Package verified snapshots, never a mutable or stale distribution tree.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}"
# release-tools performs canonical SemVer validation in CI. Independently bound
# the filename here so direct invocations cannot redirect archive output paths.
[[ "$RELEASE_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][A-Za-z0-9.+-]+)?$ ]] || {
  echo 'RELEASE_VERSION must be a safe version filename' >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
python3 .github/scripts/verify-web-dist.py --root "$PWD" --stage "$work/snapshot"
material=(compliance README.md docs)
tar -czf "$work/native.tar.gz" \
  -C "$work/snapshot/native" zxs zxs-rescue -C "$work/snapshot/material" "${material[@]}"
tar -czf "$work/web.tar.gz" \
  --exclude=./compliance -C "$work/snapshot/web" . -C "$work/snapshot/material" "${material[@]}"
[[ ! -L release-assets ]]
mkdir -p release-assets
mv -T -- "$work/native.tar.gz" "release-assets/zxs-${RELEASE_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
mv -T -- "$work/web.tar.gz" "release-assets/web-interface-${RELEASE_VERSION}.tar.gz"
