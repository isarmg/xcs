#!/usr/bin/env python3
"""Exercise every rustix API build and both backends via the real locked consumer."""
import json
import runpy
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
matrix_environment = runpy.run_path(str(Path(__file__).with_name("backend_environment.py")))["matrix_environment"]
TARGET = "x86_64-unknown-linux-gnu"


def libraries(output):
    """Use Cargo's exact artifacts, never a potentially stale filename glob."""
    result = {}
    for line in output.splitlines():
        item = json.loads(line)
        if item.get("reason") == "compiler-message" and item["message"]["level"] in {"warning", "error"}:
            raise ValueError(item["message"].get("rendered") or item["message"]["message"])
        if item.get("reason") != "compiler-artifact":
            continue
        name = item["target"]["name"]
        if name in {"rustix", "libc"} and "lib" in item["target"]["kind"]:
            paths = [Path(path) for path in item["filenames"] if path.endswith(".rlib")]
            if len(paths) != 1 or name in result:
                raise ValueError(f"ambiguous Cargo artifact for {name}")
            result[name] = paths[0]
    if set(result) != {"rustix", "libc"}:
        raise ValueError("Cargo did not report both rustix and libc library artifacts")
    return result


def check(root):
    environment = matrix_environment()
    with tempfile.TemporaryDirectory(prefix="zxs-linux-syscalls-") as temporary:
        guard_test = str(Path(temporary) / "target-guard")
        subprocess.run([
            "rustc", "--edition=2021", "--test",
            str(root / "src/foundation/rustix/build.rs"),
            "--cfg", 'feature="std"', "--deny", "warnings", "-o", guard_test,
        ], cwd=root, check=True)
        subprocess.run([guard_test], cwd=root, check=True)
        for mode, features in (
            ("linux-raw", "rustix/all-apis"),
            ("libc", "rustix/all-apis,rustix/use-libc"),
        ):
            print(f"Checking every rustix API and Linux amd64 {mode} syscall/ABI behavior", flush=True)
            # Features are enabled through an existing consumer. This uses the
            # root manifest and lock, and neither creates a workspace nor adds
            # an unregistered dependency or upstream test-only package.
            try:
                built = subprocess.run(
                    ["cargo", "build", "--locked", "--offline", "--target", TARGET,
                     "-p", "platform-io", "--features", features, "--message-format=json"],
                    cwd=root, env=environment, check=True, text=True, stdout=subprocess.PIPE,
                )
            except subprocess.CalledProcessError as error:
                # Do not hide Cargo's structured diagnostics on a failed build.
                print(error.stdout or "", file=sys.stderr)
                raise
            artifacts = libraries(built.stdout)
            executable = str(Path(temporary) / mode)
            command = [
                "rustc", "--edition=2021", "--target", TARGET, "--test",
                str(root / "test/tools/rust-sources/fixtures/linux_syscalls.rs"),
                "--deny", "warnings", "-o", executable,
            ]
            for name, path in sorted(artifacts.items()):
                command += ["--extern", f"{name}={path}"]
            for directory in sorted({path.parent for path in artifacts.values()}):
                command += ["-L", f"dependency={directory}"]
            subprocess.run(command, cwd=root, check=True)
            subprocess.run([executable], cwd=root, check=True)


if __name__ == "__main__":
    check(ROOT)
