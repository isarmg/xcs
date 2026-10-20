#!/usr/bin/env python3
"""Review missing upstream notices at pinned commits, outside the maintained tree."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
from urllib.error import HTTPError, URLError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]
ORIGINS = ROOT / "compliance/provenance/origins.json"
NAMES = ("LICENSE-MIT", "LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING", "LICENSE-APACHE", "LICENSE-APACHE-2.0", "LICENSE-MIT.txt", "LICENSE-MIT.md", "LICENSE-APACHE.txt")
MAX_NOTICE_BYTES = 4 * 1024 * 1024


def collect(record, destination, fetch=False):
    if record.get("central_license_materials"):
        return None
    upstream = record["upstream"]
    result = dict(origin_directory=record["origin_directory"], name=upstream["name"], version=upstream["version"],
                  status="requires-manual-notice-review", repository=record.get("repository", ""))
    repository = re.fullmatch(r"https://github.com/([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)/?", result["repository"])
    vcs = record.get("vcs") or {}
    commit = vcs.get("git", {}).get("sha1", "")
    if not repository or not re.fullmatch(r"[a-f0-9]{40}", commit) or not fetch:
        return result
    repository = repository[1].removesuffix(".git")
    result.update(repository=f"https://github.com/{repository}", commit=commit)
    locations = [""]
    parent = Path(vcs.get("path_in_vcs", ""))
    if parent.is_absolute() or ".." in parent.parts or "\\" in str(parent):
        raise ValueError("unsafe historical VCS path")
    while str(parent) not in (".", ""):
        locations.append(parent.as_posix() + "/")
        parent = parent.parent
    for prefix in locations:
        for name in NAMES:
            url = f"https://raw.githubusercontent.com/{repository}/{commit}/{prefix}{name}"
            content = None
            for attempt in range(3):
                try:
                    with urlopen(url, timeout=20) as response:
                        if response.geturl() != url:
                            raise ValueError(f"notice URL redirected: {url}")
                        content = response.read(MAX_NOTICE_BYTES + 1)
                    if len(content) > MAX_NOTICE_BYTES:
                        raise ValueError(f"notice exceeds size limit: {url}")
                    break
                except HTTPError as error:
                    if error.code == 404:
                        break
                    if attempt == 2:
                        return dict(result, error=str(error))
                except (URLError, TimeoutError, OSError) as error:
                    if attempt == 2:
                        return dict(result, error=str(error))
            if content is None or len(content) < 150 or b"<html" in content[:100].lower():
                continue
            checksum = hashlib.sha256(content).hexdigest()
            filename = f"{checksum}.txt"
            target = destination / filename
            if target.exists() and target.read_bytes() != content:
                raise ValueError("conflicting downloaded notice bytes")
            target.write_bytes(content)
            return dict(result, status="upstream-notice-retained-for-review", url=url, file=filename, sha256=checksum)
    return result


def review(output, fetch=False):
    output = Path(output)
    if output.is_symlink() or output.resolve().is_relative_to(ROOT.resolve()):
        raise ValueError("notice review output must be outside the maintained repository")
    for parent in output.absolute().parents:
        if parent.is_symlink():
            raise ValueError(f"output parent symlink: {parent}")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ValueError("notice review output must be a new or empty directory")
    output.mkdir(parents=True, exist_ok=True)
    records = json.loads(ORIGINS.read_text(encoding="utf-8"))["packages"]
    with ThreadPoolExecutor(max_workers=8) as pool:
        results = [result for result in pool.map(lambda record: collect(record, output, fetch), records) if result]
    (output / "provenance.json").write_text(json.dumps(results, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--fetch", action="store_true", help="explicitly allow retrieval from recorded upstream commits")
    arguments = parser.parse_args()
    try:
        results = review(arguments.output, arguments.fetch)
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f"Notice review failed: {error}\n")
    retained = sum(result["status"] == "upstream-notice-retained-for-review" for result in results)
    print(f"Reviewed {len(results)} notice candidates; {retained} downloaded for review. Active provenance and sources were not changed.")


if __name__ == "__main__":
    main()
