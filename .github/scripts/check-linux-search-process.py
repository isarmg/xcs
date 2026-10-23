#!/usr/bin/env python3
"""Check retained byte-search, Linux process, and Unix IPC consumer APIs."""
from pathlib import Path
import json
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-unknown-linux-gnu"
CONSUMER = (ROOT / "test/ci/fixtures/linux-search-process.rs").read_text()


def check(root):
    root = Path(root).resolve()
    temporary_root = root / "target" / "scope-consumers"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="search-process-", dir=temporary_root) as temporary:
        work = Path(temporary)
        (work / "src").mkdir()
        manifest = [
            '[package]',
            'name = "retained-search-process-consumer"',
            'version = "0.0.0"',
            'edition = "2021"',
            '[workspace]',
            '[dependencies]',
            'memchr = { path = ' + json.dumps(str(root / "src/foundation/text-unicode/memchr")) + ' }',
            'tokio = { path = ' + json.dumps(str(root / "src/foundation/concurrency/tokio")) + ', features = ["full"] }',
        ]
        (work / "Cargo.toml").write_text("\n".join(manifest) + "\n", encoding="utf-8")
        (work / "src/lib.rs").write_text(CONSUMER, encoding="utf-8")
        common = ["--manifest-path", str(work / "Cargo.toml"), "--offline"]
        # Create only this isolated consumer's lockfile, then prohibit resolution changes.
        subprocess.run(["cargo", "generate-lockfile", *common], cwd=root, check=True, timeout=120)
        print("Checking memchr boundaries and Tokio Linux process / Unix IPC APIs", flush=True)
        diagnostics_type = runpy.run_path(
            str(root / ".github/scripts/check-rust-diagnostics.py")
        )["CargoDiagnostics"]
        diagnostics = diagnostics_type(sys.stderr)
        executables = []
        command = [
            "cargo", "test", *common, "--locked", "--no-run", "--message-format=json",
            "--jobs", "2", "--target", TARGET,
            "--target-dir", str(root / "target" / "search-process-consumer"),
        ]
        with subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE,
                              text=True, encoding="utf-8") as process:
            for number, line in enumerate(process.stdout, 1):
                diagnostics.consume(line, number)
                try:
                    event = json.loads(line)
                except ValueError:
                    continue  # CargoDiagnostics rejects malformed JSON.
                if (isinstance(event, dict) and event.get("reason") == "compiler-artifact"
                        and event.get("target", {}).get("name") == "retained_search_process_consumer"
                        and event.get("profile", {}).get("test") and event.get("executable")):
                    executables.append(event["executable"])
            if diagnostics.result(process.wait()):
                raise RuntimeError("Cargo diagnostics rejected the search/process consumer build")
        if len(executables) != 1:
            raise AssertionError("Expected exactly one search/process consumer test executable")
        subprocess.run([executables[0]], cwd=root, check=True, timeout=60)



if __name__ == "__main__":
    check(ROOT)
