#!/usr/bin/env bash
# Check first-party workspace members, never reformat immutable upstream snapshots.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

manifests=(Cargo.toml apps/Cargo.toml web/Cargo.toml
  tools/contract-check/Cargo.toml tools/release-tools/Cargo.toml
  tools/web-build/Cargo.toml tools/unicode-build/Cargo.toml
  tools/browser-smoke/Cargo.toml tools/perf-check/Cargo.toml
  demos/branch-system/Cargo.toml integrations/smb/Cargo.toml)

for manifest in "${manifests[@]}"; do
  echo "Checking first-party Rust quality: $manifest"
  # Explicit member selection avoids cargo fmt --all traversing path dependencies
  # into vendor/dependencies and vendor/web/dependencies.
  mapfile -t packages < <(cargo metadata --locked --no-deps --format-version 1 \
    --manifest-path "$manifest" | python3 -c '
import json, sys
m = json.load(sys.stdin)
members = set(m["workspace_members"])
for p in m["packages"]:
    if p["id"] in members:
        print(p["name"])
')
  ((${#packages[@]} > 0))
  fmt_args=()
  for package in "${packages[@]}"; do fmt_args+=(--package "$package"); done
  cargo fmt --manifest-path "$manifest" "${fmt_args[@]}" -- --check
  if [[ "$manifest" == integrations/smb/Cargo.toml ]]; then
    # The VFS module needs the explicitly configured Samba SDK.
    cargo clippy --locked --manifest-path "$manifest" --package zxs-smb --all-targets -- -D warnings
  else
    cargo clippy --locked --manifest-path "$manifest" --workspace --all-targets -- -D warnings
  fi
done
# Formatting does not require the native Tauri system libraries or generated assets.
cargo fmt --manifest-path apps/host/Cargo.toml -- --check

# These two maintained project forks are excluded from the consumer workspace.
# Without --all this formats only the selected crate, not upstream dependencies.
cargo fmt --manifest-path vendor/web/zxs-web-core/Cargo.toml -- --check
cargo fmt --manifest-path vendor/web/zxs-web-macro/Cargo.toml -- --check
cargo clippy --locked --manifest-path web/Cargo.toml -p zxs-web-core -p zxs-web-macro --all-targets -- -D warnings

# Linux native regression harness is compiled in its SDK-enabled CI job.
cargo fmt --manifest-path tools/native-dependency-tests/Cargo.toml -- --check
