#!/usr/bin/env python3
"""Offline component checks for retained entropy paths; not a production closure test."""

import argparse
import json
from pathlib import Path
import re
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
matrix_environment = runpy.run_path(str(Path(__file__).with_name("backend_environment.py")))["matrix_environment"]
PACKAGES = ("system-entropy",)
RETIRED = ("rndr", "efi_rng", "windows_legacy")
DIAGNOSTIC = "this getrandom backend was retired"


def consumer():
    code = (ROOT / "test/ci/fixtures/linux-random-boundaries.rs").read_text()
    return code + (ROOT / "test/ci/fixtures/linux-random-custom.rs").read_text()


def run(command, env, expected_error=None):
    if "--message-format=json" not in command:
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        return
    diagnostics_type = runpy.run_path(
        str(ROOT / ".github/scripts/check-rust-diagnostics.py")
    )["CargoDiagnostics"]
    diagnostics = diagnostics_type(sys.stderr)
    executable = []
    expected_seen = False
    unexpected_errors = []
    with subprocess.Popen(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                          text=True, encoding="utf-8") as process:
        for number, line in enumerate(process.stdout, 1):
            diagnostics.consume(line, number)
            try:
                event = json.loads(line)
            except ValueError:
                continue  # CargoDiagnostics rejects the malformed stream.
            if not isinstance(event, dict):
                continue
            if (event.get("reason") == "compiler-message" and expected_error
                    and expected_error in event.get("message", {}).get("message", "")):
                expected_seen = True
            if (expected_error and event.get("reason") == "compiler-message"
                    and event.get("message", {}).get("level") == "error"
                    and expected_error not in event.get("message", {}).get("message", "")):
                unexpected_errors.append(event["message"]["message"])
            if (event.get("reason") == "compiler-artifact"
                    and event.get("target", {}).get("name") == "isolated_consumer"
                    and event.get("profile", {}).get("test") and event.get("executable")):
                executable.append(event["executable"])
        returncode = process.wait()
    if expected_error:
        if (returncode == 0 or not expected_seen or not diagnostics.finished
                or diagnostics.succeeded or diagnostics.invalid or diagnostics.warnings
                or unexpected_errors):
            raise RuntimeError("retired backend did not fail cleanly with its explicit diagnostic")
    elif diagnostics.result(returncode):
        raise RuntimeError("component build diagnostics rejected")
    if "--no-run" in command:
        if len(executable) != 1:
            raise RuntimeError("expected exactly one isolated consumer executable")
        subprocess.run([executable[0]], cwd=ROOT, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target" / "random-backends")
    args = parser.parse_args()
    base_env = matrix_environment()
    base_env["RUSTFLAGS"] = "--deny=warnings"
    with tempfile.TemporaryDirectory(prefix="zxs-random-backends-") as directory:
        for package in PACKAGES:
            source = ROOT / "src" / "foundation" / package
            isolated = Path(directory) / package
            isolated.mkdir()
            (isolated / "rust-toolchain.toml").write_bytes((ROOT / "rust-toolchain.toml").read_bytes())
            manifest = re.sub(
                r'(path|readme) = "([^"]+)"',
                lambda m: m[1] + ' = "' + str((source / m[2]).resolve()) + '"',
                (source / "Cargo.toml").read_text(),
            ).replace('build = "build.rs"', 'build = "' + str(source / "build.rs") + '"')
            manifest += '\n[workspace]\n[[test]]\nname = "isolated_consumer"\npath = "consumer.rs"\n'
            path = isolated / "Cargo.toml"
            path.write_text(manifest)
            (isolated / "consumer.rs").write_text(consumer())
            run(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(path)], base_env)
            common = ["--locked", "--offline", "-j2", "--manifest-path", str(path),
                      "--target-dir", str(args.target_dir.resolve()), "--message-format=json"]
            modes = [
                ("default", "", []),
                ("fallback", "--cfg system_entropy_test_linux_fallback", []),
                ("syscall", "--cfg system_entropy_test_linux_without_fallback", []),
                *[(backend, '--cfg system_entropy_backend="' + backend + '"', [])
                  for backend in ("linux_getrandom", "linux_raw", "custom", "unsupported")],
            ]
            for name, flags, features in modes:
                print(f"{package}: {name}", flush=True)
                env = base_env | {"RUSTFLAGS": base_env["RUSTFLAGS"] + " " + flags}
                run(["cargo", "test", "--no-run", *common, "--test", "isolated_consumer", *features], env)
            feature = "wasm_js"
            print(f"{package}: browser wasm compile", flush=True)
            run(["cargo", "check", *common, "--lib", "--target", "wasm32-unknown-unknown",
                 "--features", feature], base_env)
            for backend in RETIRED:
                print(f"{package}: reject {backend}", flush=True)
                env = base_env | {"RUSTFLAGS": base_env["RUSTFLAGS"] +
                                 ' --cfg system_entropy_backend="' + backend + '"'}
                run(["cargo", "check", *common, "--lib"], env, DIAGNOSTIC)
    print("PASS: 7 native component modes, 1 wasm compile check, 3 retired-backend rejections")
    print("This matrix is not a production closure check or a browser runtime smoke test.")


if __name__ == "__main__":
    main()
