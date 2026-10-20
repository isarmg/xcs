#!/usr/bin/env bash
# Offline integration checks: gh is a temporary mock, never the real executable.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/.github/scripts" "$root/release-assets" "$root/bin" "$root/remote"
cp .github/scripts/release-publish.sh "$root/.github/scripts/"
git -C "$root" init -q
git -C "$root" -c user.name=test -c user.email=test@example.invalid commit -qm fixture --allow-empty
export MOCK_SHA="$(git -C "$root" rev-parse HEAD)" MOCK_ROOT="$root"
printf 'actual fixture bytes\n' > "$root/release-assets/test.txt"
(cd "$root/release-assets" && sha256sum test.txt > SHA256SUMS)
cat > "$root/bin/gh" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  'api --include')
    case "$MODE" in
      network) echo 'HTTP/2.0 503 Service Unavailable' ;;
      forbidden) echo 'HTTP/2.0 403 Forbidden' ;;
      *) echo 'HTTP/2.0 404 Not Found' ;;
    esac
    exit 1 ;;
  'api repos/'*) echo "$MOCK_SHA" ;;
  'release view')
    if [[ -f "$MOCK_ROOT/published" ]]; then
      echo false
    elif [[ "$MODE" == public ]]; then
      echo '{"isDraft":false,"body":""}'
    elif [[ "$MODE" == foreign ]]; then
      echo '{"isDraft":true,"body":"<!-- zxs-release-source:another-source -->"}'
    elif [[ "$MODE" == malformed ]]; then
      echo '{"isDraft":true,"body":null}'
    elif [[ "$MODE" == resume ]]; then
      printf '{"isDraft":true,"body":"<!-- zxs-release-source:%s -->"}\n' "$MOCK_SHA"
    else exit 1; fi ;;
  'release create') touch "$MOCK_ROOT/created" ;;
  'release upload') cp "$MOCK_ROOT"/release-assets/* "$MOCK_ROOT/remote/" ;;
  'release download')
    cp "$MOCK_ROOT"/remote/* "$5/"
    if [[ "$MODE" == corrupt ]]; then echo bad >> "$5/test.txt"; fi ;;
  'release edit') touch "$MOCK_ROOT/published" ;;
  *) echo "Unexpected mock call: $*" >&2; exit 3 ;;
esac
MOCK
chmod +x "$root/bin/gh"
export PATH="$root/bin:$PATH" RELEASE_TAG=v0.1.0 RELEASE_PRERELEASE=false GH_REPO=test/repo
for MODE in new resume network forbidden public foreign malformed corrupt; do
  export MODE
  rm -f "$root/created" "$root/published"
  status=0
  bash "$root/.github/scripts/release-publish.sh" > "$root/log" 2>&1 || status=$?
  case "$MODE" in
    new|resume)
      if [[ "$status" != 0 || ! -f "$root/published" ]]; then cat "$root/log"; exit 1; fi ;;
    *)
      if [[ "$status" == 0 || -f "$root/published" ]]; then cat "$root/log"; exit 1; fi ;;
  esac
  case "$MODE" in
    network|forbidden|public|foreign|malformed) test ! -f "$root/created" ;;
  esac
  echo "PASS mock release publication: $MODE"
done
