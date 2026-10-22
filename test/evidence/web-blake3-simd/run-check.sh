#!/usr/bin/env bash
# Optional focused correctness gate, not a browser benchmark or CI entrypoint.
set -euo pipefail
HERE=$(cd -- "$(dirname -- "$0")" && pwd)
SOURCE=$(cd -- "${1:-$HERE/../../..}" && pwd)
# All builds and lockfile generation stay outside the supplied repository.
OUTPUT=$(mktemp -d "${TMPDIR:-/tmp}/web-blake3-simd.XXXXXXXX")
trap 'rm -rf -- "$OUTPUT"' EXIT
mkdir -p "$OUTPUT/src" "$OUTPUT/docs/components"
cp -a "$SOURCE/src/foundation" "$OUTPUT/src/"
cp "$SOURCE/docs/components/cpu-feature-detect.md" "$OUTPUT/docs/components/"
mkdir "$OUTPUT/fixture"
cp "$HERE/fixture/Cargo.toml.in" "$OUTPUT/fixture/Cargo.toml"
cp -a "$HERE/fixture/src" "$OUTPUT/fixture/"
export CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$OUTPUT/target"
MANIFEST="$OUTPUT/fixture/Cargo.toml"
cargo tree --manifest-path "$MANIFEST" --target x86_64-unknown-linux-gnu -e features --offline > "$OUTPUT/native-features.txt"
cargo tree --manifest-path "$MANIFEST" --target wasm32-unknown-unknown -e features --offline > "$OUTPUT/wasm-features.txt"
if grep -q 'blake3 feature "wasm32_simd"' "$OUTPUT/native-features.txt"; then
    echo 'Wasm SIMD feature unexpectedly enabled for the native host' >&2
    exit 1
fi
grep -q 'blake3 feature "wasm32_simd"' "$OUTPUT/wasm-features.txt"
cargo run --manifest-path "$MANIFEST" --release --offline > "$OUTPUT/native.txt"
cargo build --manifest-path "$MANIFEST" --release --lib --target wasm32-unknown-unknown --offline
cp "$OUTPUT/target/wasm32-unknown-unknown/release/web_blake3_simd_check.wasm" "$OUTPUT/simd.wasm"
sed -i 's/, features = \["wasm32_simd"\]//' "$MANIFEST"
cargo build --manifest-path "$MANIFEST" --release --lib --target wasm32-unknown-unknown --offline
cp "$OUTPUT/target/wasm32-unknown-unknown/release/web_blake3_simd_check.wasm" "$OUTPUT/portable.wasm"
node "$HERE/check.mjs" "$OUTPUT/simd.wasm" "$OUTPUT/portable.wasm" "$OUTPUT/native.txt"
