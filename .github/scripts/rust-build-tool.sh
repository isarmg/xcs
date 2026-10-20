#!/usr/bin/env bash
# Compile release helpers as root workspace members with the root Cargo.lock.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
case "${1:-}" in
  wasm-bindgen)
    version=0.2.129
    package=web-bindings
    binary=wasm-bindgen
    expected="wasm-bindgen $version"
    ;;
  *) echo 'Usage: rust-build-tool.sh wasm-bindgen' >&2; exit 2 ;;
esac
test -f Cargo.toml
test -f Cargo.lock
tool_root="$PWD/target/rust-build-tools"
# Re-enter Cargo even when the version is unchanged: maintained local source
# edits must rebuild the helper. Cargo reuses unchanged compilation artifacts.
python3 .github/scripts/check-rust-diagnostics.py -- \
  cargo build --locked --offline --release --package "$package" --bin "$binary" \
  --target-dir "$tool_root" --message-format=json
tool="$tool_root/release/$binary"
if [[ ! -x "$tool" && -x "$tool.exe" ]]; then tool="$tool.exe"; fi
test -x "$tool"
actual="$("$tool" --version)"
if [[ "$actual" != "$expected" ]]; then
  printf 'Expected %s, got %s\n' "$expected" "$actual" >&2
  exit 1
fi
printf '%s\n' "$tool"
