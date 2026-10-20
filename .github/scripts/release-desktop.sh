#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_VERSION:?}" "${RELEASE_TARGET:?}" "${RELEASE_BUNDLE:?}"
export CARGO_TARGET_DIR="$PWD/target/release-desktop"
if command -v cygpath >/dev/null 2>&1; then
  export CARGO_TARGET_DIR="$(cygpath -m "$CARGO_TARGET_DIR")"
fi
cargo run --locked --manifest-path tools/web-build/Cargo.toml -- icons --product-root .
cargo install tauri-cli --version '=2.12.2' --locked
# Explicit target and output directory avoid dependence on runner architecture
# and the repository's target/native Cargo configuration.
(cd apps/host && cargo tauri build --ci --no-sign --target "$RELEASE_TARGET" --bundles "$RELEASE_BUNDLE" -- --locked)
mkdir -p release-assets
base="zxs-app-${RELEASE_VERSION}-${RELEASE_TARGET}"
bundle="$CARGO_TARGET_DIR/$RELEASE_TARGET/release/bundle"
case "$RELEASE_BUNDLE" in
  deb)
    files=("$bundle"/deb/*.deb)
    test "${#files[@]}" = 1 && test -s "${files[0]}"
    cp "${files[0]}" "release-assets/$base.deb"
    ;;
  nsis)
    files=("$bundle"/nsis/*.exe)
    test "${#files[@]}" = 1 && test -s "${files[0]}"
    cp "${files[0]}" "release-assets/$base-setup.exe"
    ;;
  app)
    test -d "$bundle/macos/zxs.app"
    tar -czf "release-assets/$base.app.tar.gz" -C "$bundle/macos" zxs.app
    ;;
  *) echo 'Unsupported bundle' >&2; exit 2 ;;
esac
