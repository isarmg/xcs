#!/usr/bin/env bash
# Every distribution carries the same centralized legal and operating material.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}"
test -s target/native/release/zxs
test -s build/web/index.html
test -s build/web/build-manifest.json
test -s compliance/NOTICE.md
material=(compliance README.md docs)
for path in "${material[@]}"; do test -e "$path"; done
mkdir -p release-assets
tar -czf "release-assets/zxs-${RELEASE_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
  -C target/native/release zxs -C "$PWD" "${material[@]}"
tar -czf "release-assets/web-interface-${RELEASE_VERSION}.tar.gz" \
  --exclude=./compliance -C build/web . -C "$PWD" "${material[@]}"
