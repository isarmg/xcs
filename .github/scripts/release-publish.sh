#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?}" "${RELEASE_PRERELEASE:?}" "${GH_REPO:?}"
# Never create a missing Git tag or silently release a tag moved during the build.
sha=$(git rev-parse HEAD)
remote_sha=$(gh api "repos/$GH_REPO/commits/$RELEASE_TAG" --jq .sha)
test "$sha" = "$remote_sha"
(cd release-assets && sha256sum --check SHA256SUMS)
marker="<!-- zxs-release-source:$sha -->"
notes=$(mktemp)
existing=$(mktemp)
trap 'rm -f "$notes" "$existing"' EXIT
cat > "$notes" <<EOF
$marker
ZXS $RELEASE_TAG

Portable tests, source/security checks, contract checks and production WASM/browser smoke passed for $sha.

- Linux x86_64: direct zxs daemon and minimal zxs-rescue offline utility; normal setup and administration use Web.
- Production Web static assets, including the build manifest.
- SMB uses the separately built Samba VFS module and the pinned native Samba SDK.

Read INSTALL.md and BUILD.txt; verify SHA256SUMS before installation. These checks do not replace physical-disk and live Samba acceptance or promise bit-for-bit reproducibility.
EOF
if gh release view "$RELEASE_TAG" --json isDraft,body > "$existing"; then
  # Retry only our own draft for this exact source; never mutate a published release.
  python3 - "$existing" "$marker" <<'PY'
import json
from pathlib import Path
import sys

release = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
body = release.get("body")
if release.get("isDraft") is not True or not isinstance(body, str) or sys.argv[2] not in body:
    raise SystemExit("Refusing to resume a release outside this source draft")
PY
else
  # Only a confirmed HTTP 404 permits creation. Auth/network/server errors stop.
  gh api --include "repos/$GH_REPO/releases/tags/$RELEASE_TAG" > "$existing" || true
  if ! grep -Eq '^HTTP/[0-9.]+ 404([[:space:]]|$)' "$existing"; then
    echo "Cannot confirm release absence; refusing to create" >&2
    exit 1
  fi
  flags=()
  if [[ "$RELEASE_PRERELEASE" == true ]]; then flags+=(--prerelease); fi
  gh release create "$RELEASE_TAG" --verify-tag --draft --title "ZXS $RELEASE_TAG" \
    --notes-file "$notes" "${flags[@]}"
fi
gh release upload "$RELEASE_TAG" release-assets/* --clobber
# Verify the server has exactly this release set, including its checksum manifest.
# Downloading checks bytes, rather than assuming a successful upload means complete delivery.
download=$(mktemp -d)
trap 'rm -f "$notes" "$existing"; rm -rf "$download"' EXIT
gh release download "$RELEASE_TAG" --dir "$download"
test "$(find "$download" -maxdepth 1 -type f | wc -l)" = "$(find release-assets -maxdepth 1 -type f | wc -l)"
cmp release-assets/SHA256SUMS "$download/SHA256SUMS"
(cd "$download" && sha256sum --check SHA256SUMS)
# Recheck the tag after upload. Until this succeeds the release remains a draft.
test "$sha" = "$(gh api "repos/$GH_REPO/commits/$RELEASE_TAG" --jq .sha)"
flags=()
if [[ "$RELEASE_PRERELEASE" == true ]]; then flags+=(--prerelease --latest=false); else flags+=(--prerelease=false); fi
gh release edit "$RELEASE_TAG" --draft=false "${flags[@]}"
test "$(gh release view "$RELEASE_TAG" --json isDraft --jq .isDraft)" = false
