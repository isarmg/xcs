#!/usr/bin/env bash
# Runner contexts are unavailable in workflow/job env. Initialize after checkout.
set -euo pipefail

: "${RUNNER_TEMP:?RUNNER_TEMP must identify the current runner temporary directory}"
: "${GITHUB_ENV:?GITHUB_ENV must identify the current step environment file}"
cargo_home="$(mktemp -d "$RUNNER_TEMP/zxs-cargo-home.XXXXXX")"
printf 'CARGO_HOME=%s\n' "$cargo_home" >> "$GITHUB_ENV"
