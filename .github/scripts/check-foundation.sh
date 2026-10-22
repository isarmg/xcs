#!/usr/bin/env bash
# Functional foundation shards share the same root workspace and lockfile.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

case "${1:-}" in
  foundation)
    for suite in foundation-platform foundation-crypto foundation-data foundation-language foundation-async; do
      bash .github/scripts/check-foundation.sh "$suite"
    done
    ;;
  foundation-platform)
    python3 .github/scripts/check-linux-uapi.py
    python3 tools/rust-sources/check_libc_abi.py
    python3 .github/scripts/check-linux-syscalls.py
    python3 .github/scripts/check-linux-system-backends.py
    python3 .github/scripts/check-linux-parking.py
    python3 .github/scripts/check-synchronization.py
    python3 .github/scripts/check-linux-search-process.py
    cargo test --locked --offline --lib -p socket2 -p mio -p synchronization
    cargo test --locked --offline --lib -p bit-set -p bit-vec -p id-arena -p rustc-demangle
    ;;
  foundation-crypto)
    python3 .github/scripts/check-crypto-consolidation.py
    python3 .github/scripts/check-linux-random-backends.py
    python3 .github/scripts/check-entropy-consumers.py
    python3 test/entropy/check.py
    python3 tools/rust-sources/check_ring_tests.py
    cargo test --locked --offline --lib -p password-hash
    ;;
  foundation-data)
    python3 .github/scripts/check-sqlite-scope.py
    cargo test --locked --offline --lib -p sqlite-access
    python3 .github/scripts/check-sqlite-family.py
    python3 .github/scripts/check-numeric-consolidation.py
    python3 .github/scripts/check-numeric-precision.py
    cargo test --locked --offline --tests -p numeric
    python3 .github/scripts/check-compression-family.py
    python3 .github/scripts/check-date-time-family.py
    ;;
  foundation-language)
    python3 .github/scripts/check-foundation-consumers.py
    python3 .github/scripts/check-serialization-family.py
    python3 .github/scripts/check-serialization-consolidation.py
    python3 .github/scripts/check-error-definition-family.py
    python3 .github/scripts/check-command-line-family.py
    python3 .github/scripts/check-toml-consolidation.py
    python3 .github/scripts/check-pattern-matching.py
    python3 .github/scripts/check-web-bindgen-scope.py
    python3 .github/scripts/check-wasm-binding-macros.py
    python3 .github/scripts/check-unicode-data-family.py
    python3 .github/scripts/check-url-encoding.py
    python3 .github/scripts/check-terminal-style.py
    ;;
  foundation-async)
    python3 .github/scripts/check-tracing-family.py
    python3 .github/scripts/check-runtime-join-map.py
    python3 .github/scripts/check-futures-consolidation.py
    python3 .github/scripts/check-http-body-consolidation.py
    python3 .github/scripts/check-concurrent-collections.py
    python3 .github/scripts/check-service-stack.py
    python3 .github/scripts/check-http-routing.py
    python3 .github/scripts/check-http-transport.py
    ;;
  *)
    echo 'Usage: check-foundation.sh {foundation|foundation-platform|foundation-crypto|foundation-data|foundation-language|foundation-async}' >&2
    exit 2
    ;;
esac
