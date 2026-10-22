#!/usr/bin/env bash
# Shared CI/release setup. Keep the browser sandbox and web security enabled.
set -euo pipefail
chrome="$(command -v google-chrome)"
"$chrome" --version
echo "ZXS_CHROMIUM=$chrome" >> "${GITHUB_ENV:?GITHUB_ENV is required}"
