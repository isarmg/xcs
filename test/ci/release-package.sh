#!/usr/bin/env bash
# Validate real archive contents without compiling or publishing.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/.github/scripts" "$fixture/target/native/release" \
  "$fixture/build/web/pkg" "$fixture/compliance/licenses/upstream/sample" \
  "$fixture/compliance/provenance" "$fixture/docs/components" "$fixture/docs/templates"
cp .github/scripts/release-package.sh .github/scripts/release-assemble.sh "$fixture/.github/scripts/"
cp rust-toolchain.toml "$fixture/"
for name in zxs zxs-rescue; do
  printf '#!/bin/sh\nexit 0\n' > "$fixture/target/native/release/$name"
  chmod +x "$fixture/target/native/release/$name"
done
# Validation tools must never leak into either product archive.
printf 'test-only engine fixture\n' > "$fixture/target/native/release/browser-smoke"
printf 'unshipped tool\n' > "$fixture/target/native/release/zxs-unpublished-tool"
printf '<html>fixture</html>\n' > "$fixture/build/web/index.html"
printf '{"runtime":"web-interface"}\n' > "$fixture/build/web/build-manifest.json"
printf 'wasm fixture\n' > "$fixture/build/web/pkg/zxs_bg.wasm"
printf 'complete legal notice\n' > "$fixture/compliance/NOTICE.md"
printf 'copyright fixture\n' > "$fixture/compliance/licenses/upstream/sample/LICENSE"
printf '{"upstream":"sample"}\n' > "$fixture/compliance/provenance/origins.json"
for path in README.md docs/README.md docs/release.md docs/25-operation-manual.md docs/THIRD_PARTY_RUST_SOURCES.md; do
  printf 'documentation fixture\n' > "$fixture/$path"
done
printf '[SMB integration](components/smb.md)\n' >> "$fixture/docs/release.md"
printf '[Operations](../25-operation-manual.md)\n' > "$fixture/docs/components/smb.md"
cp docs/templates/release-install.md "$fixture/docs/templates/"
python3 test/ci/web_dist_fixture.py "$fixture"
RELEASE_VERSION=0.1.0 bash "$fixture/.github/scripts/release-package.sh"
for archive in "$fixture"/release-assets/*.tar.gz; do
  output=$(mktemp -d "$fixture/extract.XXXXXX")
  tar -xzf "$archive" -C "$output"
  test -z "$(find "$output" -name browser-smoke -print -quit)"
  test -z "$(find "$output" -name zxs-unpublished-tool -print -quit)"
  test ! -d "$output/test"
  test ! -d "$output/tools"
  for path in compliance/NOTICE.md compliance/licenses/upstream/sample/LICENSE \
    compliance/provenance/origins.json docs/25-operation-manual.md \
    docs/components/smb.md docs/templates/release-install.md; do
    cmp "$fixture/$path" "$output/$path"
  done
  python3 - "$output" <<'PY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
for document in (root / "docs/release.md", root / "docs/components/smb.md"):
    for relative in re.findall(r"\]\(([^)]+)\)", document.read_text()):
        target = (document.parent / relative).resolve()
        assert target.is_relative_to(root.resolve()) and target.is_file(), target
PY
  if [[ "$archive" == *-unknown-linux-gnu.tar.gz ]]; then
    for name in zxs zxs-rescue; do
      cmp "$fixture/target/native/release/$name" "$output/$name"
    done
  else
    cmp "$fixture/build/web/pkg/zxs_bg.wasm" "$output/pkg/zxs_bg.wasm"
  fi
done
# Assemble the same two real archives with a fixed source identity, without
# creating a Git repository or contacting a publishing service.
mkdir "$fixture/bin"
cat > "$fixture/bin/git" <<'SH'
#!/bin/sh
test "$*" = 'rev-parse HEAD' || exit 2
printf '0123456789012345678901234567890123456789\n'
SH
chmod +x "$fixture/bin/git"
PATH="$fixture/bin:$PATH" RELEASE_VERSION=0.1.0 bash "$fixture/.github/scripts/release-assemble.sh"
test "$(head -1 "$fixture/release-assets/INSTALL.md")" = '# ZXS 0.1.0 安装'
tail -n +3 "$fixture/release-assets/INSTALL.md" | cmp "$fixture/docs/templates/release-install.md" -
python3 - "$fixture/release-assets" <<'PY'
from pathlib import Path
import os
import re
import subprocess
import sys

assets = Path(sys.argv[1])
commands = re.search(r"```sh\n(.*?)\n```", (assets / "INSTALL.md").read_text(), re.S).group(1)
server = assets / "server-0.1.0"
web = assets / "web-0.1.0"

def install():
    # No inherited errexit, shell startup file, aliases or exported functions.
    return subprocess.run(
        ["bash", "--noprofile", "--norc", "-c", commands], cwd=assets,
        env={"PATH": os.defpath, "LC_ALL": "C"}, capture_output=True,
    )

archive = assets / "web-interface-0.1.0.tar.gz"
original = archive.read_bytes()
archive.write_bytes(original + b"damaged download\n")
assert install().returncode != 0
assert not server.exists() and not web.exists(), "checksum failure must precede extraction"
archive.write_bytes(original)
result = install()
assert result.returncode == 0, result.stderr.decode()
assert (server / "docs/components/smb.md").is_file()
assert (web / "pkg/zxs_bg.wasm").read_bytes() == b"wasm fixture\n"
# A second copy-and-paste must stop before extracting over existing files.
(server / "zxs").write_bytes(b"existing installation\n")
retry = install()
assert retry.returncode != 0
assert (server / "zxs").read_bytes() == b"existing installation\n"
PY
echo 'PASS release archives retain complete documentation and assembly emits standalone installation instructions'
