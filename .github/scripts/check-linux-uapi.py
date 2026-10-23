#!/usr/bin/env python3
"""Compile and test every retained Linux amd64 UAPI in std and no_std modes."""
from pathlib import Path
import os
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-unknown-linux-gnu"


def check(root):
    directory = root / "src/foundation/platform/linux-raw-sys"
    # Direct rustc does not provide Cargo's per-package compile-time context.
    environment = os.environ | {"CARGO_MANIFEST_DIR": str(directory.resolve())}
    manifest = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))
    if any(manifest.get(kind) for kind in ("dependencies", "build-dependencies", "dev-dependencies", "target")):
        raise ValueError("direct UAPI regression requires a dependency-free library")
    features = sorted(manifest["features"])
    common = [feature for feature in features if feature not in {"default", "std", "no_std", "rustc-dep-of-std"}]
    feature_cfg = 'cfg(feature, values(' + ', '.join(f'"{feature}"' for feature in features) + '))'
    with tempfile.TemporaryDirectory(prefix="zxs-linux-uapi-") as temporary:
        for mode in ("std", "no_std"):
            executable = str(Path(temporary) / mode)
            command = ["rustc", "--edition=" + manifest["package"]["edition"],
                       "--target", TARGET, "--crate-name", "linux_raw_sys", "--test",
                       str(directory / "src/lib.rs"), "-o", executable,
                       "--deny", "warnings", "--check-cfg", "cfg(test)", "--check-cfg", feature_cfg]
            for feature in [*common, mode]:
                command += ["--cfg", f'feature="{feature}"']
            print(f"Checking every retained Linux amd64 UAPI with {mode}", flush=True)
            subprocess.run(command, cwd=root, env=environment, check=True)
            subprocess.run([executable], cwd=root, env=environment, check=True)


if __name__ == "__main__":
    check(ROOT)
