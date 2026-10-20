#!/usr/bin/env bash
# Invoke every suite through the single root workspace and root Cargo.lock.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

case "${1:-}" in
  static)
    # No Rust toolchain, registry, package installer or live browser required.
    python3 .github/scripts/check-workflows.py
    python3 tools/rust-sources/manage.py verify
    python3 -m unittest discover -s tools/rust-sources -p 'test_*.py'
    python3 -m unittest discover -s .github/scripts/tests -p 'test_*.py'
    python3 -m unittest discover -s tools/perf-check -p 'test_*.py'
    for script in .github/scripts/*.sh .github/scripts/tests/*.sh; do
      bash -n "$script"
    done
    bash .github/scripts/tests/release-package.sh
    bash .github/scripts/tests/release-publish.sh
    node .github/scripts/check-evidence-probes.mjs
    ;;
  rust-sources)
    python3 tools/rust-sources/manage.py verify --metadata
    python3 -m unittest discover -s tools/rust-sources -p 'test_*.py'
    python3 -m unittest discover -s .github/scripts/tests -p 'test_offline_builds.py'
    python3 -m unittest discover -s .github/scripts/tests -p 'test_check_suites.py'
    python3 -m unittest discover -s .github/scripts/tests -p 'test_rust_diagnostics.py'
    ;;
  quality)
    python3 -m unittest discover -s .github/scripts/tests -p 'test_rust_diagnostics.py'
    bash .github/scripts/quality.sh
    ;;
  security)
    bash .github/scripts/security.sh
    ;;
  workspace)
    # Root default-members are the native product components. Web, tools, and
    # the Samba SDK module have dedicated suites in this same workspace.
    cargo test --locked --offline
    cargo test --locked --offline -p storage-engine --lib \
      history_package::graph::tests::distributed_history_larger_than_real_reader_limit_roundtrips \
      -- --ignored --exact --test-threads=1
    ;;
  foundation)
    # Focused regressions for maintained foundation changes. Do not restore
    # retired upstream dev dependencies or run unrelated upstream test suites.
    # All preserved Linux ABI features, not only today's enabled dependency set.
    python3 .github/scripts/check-linux-uapi.py
    python3 tools/rust-sources/check_libc_abi.py
    python3 .github/scripts/check-linux-syscalls.py
    python3 .github/scripts/check-linux-system-backends.py
    python3 .github/scripts/check-linux-parking.py
    python3 .github/scripts/check-linux-search-process.py
    python3 .github/scripts/check-linux-random-backends.py
    cargo test --locked --offline --lib -p socket2 -p mio -p parking_lot_core
    python3 tools/rust-sources/check_ring_tests.py
    cargo test --locked --offline --lib -p password-hash
    # An external workspace package cannot receive --features directly. Its
    # actual storage-client consumer enables compiled_data in the shared graph.
    cargo test --locked --offline --lib -p idna_adapter -p storage-client \
      property_masks_preserve_all_unicode_scalars
    cargo test --locked --offline --lib \
      -p num-traits -p num-integer -p num-iter -p num-rational \
      -p bit-set -p bit-vec -p id-arena -p rustc-demangle
    ;;
  web)
    cargo test --locked --offline -p web-app -p web-interface -p view-macros
    ;;
  contracts)
    cargo test --locked --offline -p contract-check
    cargo run --locked --offline -p contract-check -- --root .
    ;;
  build-tools)
    cargo test --locked --offline -p web-bindings -p release-tools -p web-build -p unicode-build -p perf-check
    bash .github/scripts/tests/release-package.sh
    bash .github/scripts/tests/release-publish.sh
    python3 -m unittest discover -s .github/scripts/tests -p 'test_offline_builds.py'
    python3 -m unittest discover -s tools/perf-check -p 'test_*.py'
    cargo run --locked --offline -p unicode-build -- --check
    ;;
  smb-protocol)
    cargo test --locked --offline -p smb-protocol
    ;;
  web-production)
    python3 tools/rust-sources/manage.py verify
    ZXS_WEB_BINDGEN="$(bash .github/scripts/rust-build-tool.sh wasm-bindgen)"
    export ZXS_WEB_BINDGEN
    cargo run --locked --offline -p web-build -- build --product-root .
    test -s build/web/index.html
    test -s build/web/pkg/zxs_bg.wasm
    test -s build/web/pkg/zxs_bg.wasm.gz
    test -s build/web/build-manifest.json
    ;;
  browser-smoke)
    cargo build --locked --offline -p storage-admin
    cargo test --locked --offline -p browser-smoke
    screenshot_args=()
    if [[ -n "${ZXS_SMOKE_SCREENSHOT:-}" ]]; then
      screenshot_args=(--screenshot "$ZXS_SMOKE_SCREENSHOT")
    fi
    cargo run --locked --offline -p browser-smoke -- \
      --cli-bin "$PWD/target/native/debug/zxs" --web-dir "$PWD/build/web" "${screenshot_args[@]}"
    ;;
  *)
    echo 'Usage: check.sh {static|rust-sources|quality|security|workspace|foundation|web|contracts|build-tools|smb-protocol|web-production|browser-smoke}' >&2
    exit 2
    ;;
esac
