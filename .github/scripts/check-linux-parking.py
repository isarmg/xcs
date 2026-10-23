#!/usr/bin/env python3
"""Check Linux parking timeout and wake-token behavior in a locked consumer."""
from pathlib import Path
import json
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
matrix_environment = runpy.run_path(str(Path(__file__).with_name("backend_environment.py")))["matrix_environment"]
TARGET = "x86_64-unknown-linux-gnu"
CONSUMER = (ROOT / "test/ci/fixtures/linux-parking.rs").read_text()

def check(root):
    environment = matrix_environment()
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='parking-regression-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        foundation = root / 'src/foundation'
        (work / 'Cargo.toml').write_text(f'''[package]
name = "parking-runtime-regression"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
synchronization = {{ path = "{root / 'src/foundation/concurrency/synchronization'}", default-features = false, features = ["parking"] }}
''')
        (work / 'src/lib.rs').write_text(CONSUMER)
        subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path",
                        str(work / "Cargo.toml")], cwd=root, check=True)
        diagnostics_type = runpy.run_path(
            str(root / ".github/scripts/check-rust-diagnostics.py")
        )["CargoDiagnostics"]
        for name, flags in [("parking", "")]:
            print(f"Checking Linux amd64 {name}", flush=True)
            command = ["cargo", "test", "--locked", "--offline", "--no-run",
                       "--message-format=json", "--manifest-path", str(work / "Cargo.toml"),
                       "--target", TARGET, "--target-dir", str(work / "target"), "-j2"]
            diagnostics = diagnostics_type(sys.stderr)
            executables = []
            with subprocess.Popen(command, cwd=root, env=environment | {"RUSTFLAGS": flags},
                                  stdout=subprocess.PIPE, text=True, encoding="utf-8") as process:
                for number, line in enumerate(process.stdout, 1):
                    diagnostics.consume(line, number)
                    try:
                        event = json.loads(line)
                    except ValueError:
                        continue  # The strict diagnostic consumer rejects malformed JSON.
                    if (isinstance(event, dict) and event.get("reason") == "compiler-artifact"
                            and event.get("target", {}).get("name") == "parking_runtime_regression"
                            and event.get("profile", {}).get("test") and event.get("executable")):
                        executables.append(event["executable"])
                if diagnostics.result(process.wait()):
                    raise RuntimeError(f"{name}: Cargo diagnostics rejected the regression build")
            if len(executables) != 1:
                raise AssertionError(f"{name}: expected exactly one consumer test executable")
            subprocess.run([executables[0]], cwd=root, check=True)

if __name__ == "__main__":
    check(ROOT)
