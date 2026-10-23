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

提交 $sha 已通过可移植测试、源码与安全检查、契约检查、正式 WASM 构建及浏览器冒烟。

- Linux x86_64：直接运行的 zxs 守护进程与最小离线救援工具 zxs-rescue；正常初始化和管理使用 Web。
- 正式 Web 静态资源，包含构建清单。
- SMB 使用单独构建的 Samba VFS 模块及固定版本的原生 Samba SDK。

安装前请阅读 INSTALL.md 和 BUILD.txt，并完整校验 SHA256SUMS。上述检查不能替代物理盘与真实 Samba 验收，也不承诺位级可复现。
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
