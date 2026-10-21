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
VERSIONS = ("0.2.17", "0.3.4", "0.4.3")
RETIRED = ("rndr", "efi_rng", "windows_legacy")
DIAGNOSTIC = "this getrandom backend was retired"


def consumer(version):
    old = version.startswith("0.2")
    code = (ROOT / "test/ci/fixtures/linux-random-boundaries.rs").read_text().replace("FILL", "getrandom" if old else "fill").replace(
        "UNINIT", "getrandom_uninit" if old else "fill_uninit"
    )
    if old:
        code = re.sub(r'        #\[cfg\(not\(getrandom_backend = "unsupported"\)\)\]\n', "", code)
        code = re.sub(r'        #\[cfg\(getrandom_backend = "unsupported"\)\]\n        assert!\(result.is_err\(\)\);\n', "", code)
    else:
        code += (ROOT / "test/ci/fixtures/linux-random-custom.rs").read_text()
        if version.startswith("0.4"):
            code = code.replace("#[no_mangle]", "#[unsafe(no_mangle)]")
    return code


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
        for version in VERSIONS:
            source = ROOT / "src" / "foundation" / ("getrandom-" + version)
            isolated = Path(directory) / version
            isolated.mkdir()
            manifest = re.sub(
                r'(path|readme) = "([^"]+)"',
                lambda m: m[1] + ' = "' + str((source / m[2]).resolve()) + '"',
                (source / "Cargo.toml").read_text(),
            ).replace('build = "build.rs"', 'build = "' + str(source / "build.rs") + '"')
            manifest += '\n[workspace]\n[[test]]\nname = "isolated_consumer"\npath = "consumer.rs"\n'
            path = isolated / "Cargo.toml"
            path.write_text(manifest)
            (isolated / "consumer.rs").write_text(consumer(version))
            run(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(path)], base_env)
            common = ["--locked", "--offline", "-j2", "--manifest-path", str(path),
                      "--target-dir", str(args.target_dir.resolve()), "--message-format=json"]
            modes = [("default", "", [])]
            if version.startswith("0.2"):
                modes.append(("no-fallback", "", ["--features", "linux_disable_fallback"]))
            else:
                modes.extend([
                    ("fallback", "--cfg getrandom_test_linux_fallback", []),
                    ("syscall", "--cfg getrandom_test_linux_without_fallback", []),
                    *[(backend, '--cfg getrandom_backend="' + backend + '"', [])
                      for backend in ("linux_getrandom", "linux_raw", "custom", "unsupported")],
                ])
            for name, flags, features in modes:
                print(f"{version}: {name}", flush=True)
                env = base_env | {"RUSTFLAGS": base_env["RUSTFLAGS"] + " " + flags}
                run(["cargo", "test", "--no-run", *common, "--test", "isolated_consumer", *features], env)
            feature = "js" if version.startswith("0.2") else "wasm_js"
            print(f"{version}: browser wasm compile", flush=True)
            run(["cargo", "check", *common, "--lib", "--target", "wasm32-unknown-unknown",
                 "--features", feature], base_env)
            if not version.startswith("0.2"):
                for backend in RETIRED:
                    print(f"{version}: reject {backend}", flush=True)
                    env = base_env | {"RUSTFLAGS": base_env["RUSTFLAGS"] +
                                     ' --cfg getrandom_backend="' + backend + '"'}
                    run(["cargo", "check", *common, "--lib"], env, DIAGNOSTIC)
    print("PASS: 16 native component modes, 3 wasm compile checks, 6 retired-backend rejections")
    print("This matrix is not a production closure check or a browser runtime smoke test.")


if __name__ == "__main__":
    main()
