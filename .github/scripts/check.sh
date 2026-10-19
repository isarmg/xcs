#!/usr/bin/env bash
# Invoke from any directory; each excluded workspace is checked explicitly.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

case "${1:-}" in
  workspace)
    cargo test --locked --workspace
    ;;
  apps)
    cargo test --locked --manifest-path apps/Cargo.toml --workspace
    ;;
  web)
    cargo test --locked --manifest-path web/Cargo.toml --workspace
    ;;
  contracts)
    cargo test --locked --manifest-path tools/contract-check/Cargo.toml
    cargo run --locked --manifest-path tools/contract-check/Cargo.toml -- --root .
    ;;
  build-tools)
    cargo test --locked --manifest-path tools/release-tools/Cargo.toml
    bash .github/scripts/tests/release-publish.sh
    cargo test --locked --manifest-path tools/web-build/Cargo.toml
    cargo test --locked --manifest-path tools/unicode-build/Cargo.toml
    cargo run --locked --manifest-path tools/unicode-build/Cargo.toml -- --check
    ;;
  demo)
    cargo test --locked --manifest-path demos/branch-system/Cargo.toml
    ;;
  smb-protocol)
    # Do not build the VFS module without its configured Samba 4.23.0 SDK.
    cargo test --locked --manifest-path integrations/smb/Cargo.toml --package xcs-smb
    ;;
  web-production)
    # The production builder deliberately invokes Cargo offline. A clean CI
    # runner must fetch the locked Web dependencies before calling it.
    cargo fetch --locked --manifest-path web/Cargo.toml
    bindgen_root="$PWD/target/ci-tools"
    export XCS_WEB_BINDGEN="$bindgen_root/bin/wasm-bindgen"
    if [[ ! -x "$XCS_WEB_BINDGEN" ]] || \
       [[ "$("$XCS_WEB_BINDGEN" --version)" != 'wasm-bindgen 0.2.129' ]]; then
      cargo install wasm-bindgen-cli --version '=0.2.129' --locked --root "$bindgen_root" --force
    fi
    cargo run --locked --manifest-path tools/web-build/Cargo.toml -- build --product-root .
    test -s web/dist/index.html
    test -s web/dist/pkg/xcs_bg.wasm
    test -s web/dist/pkg/xcs_bg.wasm.gz
    test -s web/dist/build-manifest.json
    ;;
  desktop-host)
    test -s web/dist/build-manifest.json
    cargo run --locked --manifest-path tools/web-build/Cargo.toml -- icons --product-root .
    cargo check --locked --manifest-path apps/host/Cargo.toml
    ;;
  browser-smoke)
    cargo build --locked --package xcs-cli
    cargo test --locked --manifest-path tools/browser-smoke/Cargo.toml
    screenshot_args=()
    if [[ -n "${XCS_SMOKE_SCREENSHOT:-}" ]]; then
      screenshot_args=(--screenshot "$XCS_SMOKE_SCREENSHOT")
    fi
    cargo run --locked --manifest-path tools/browser-smoke/Cargo.toml -- \
      --xcs "$PWD/target/native/debug/xcs" --web-dir "$PWD/web/dist" "${screenshot_args[@]}"
    ;;
  *)
    echo 'Usage: check.sh {workspace|apps|web|contracts|build-tools|demo|smb-protocol|web-production|desktop-host|browser-smoke}' >&2
    exit 2
    ;;
esac
