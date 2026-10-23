#!/usr/bin/env python3
"""Test the binding parser without SDKs; --clang adds actual libclang tokenization."""
if not __debug__:
    raise RuntimeError('Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE).')

import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def check(clang=False):
    with tempfile.TemporaryDirectory(prefix="binding-expressions-") as temporary:
        work = Path(temporary)
        executable = work / "parser-tests"
        subprocess.run(["rustc", "--edition=2021", "--test",
                        str(ROOT / "test/ci/fixtures/binding-expressions.rs"),
                        "-o", str(executable)], cwd=ROOT, check=True)
        subprocess.run([str(executable)], cwd=ROOT, check=True)
        if clang:
            (work / "Cargo.toml").write_text(
                '[package]\nname="binding-expression-clang-tests"\nversion="0.0.0"\n'
                'edition="2021"\n[workspace]\n[dependencies]\n'
                f'clang-sys={{path="{ROOT / "src/foundation/tooling/clang-sys"}",features=["clang_11_0"]}}\n'
                '[[test]]\nname="clang"\n'
                f'path="{ROOT / "test/foundation/bindgen/cexpr/tests/clang.rs"}"\n')
            common = ["--offline", "--manifest-path", str(work / "Cargo.toml")]
            subprocess.run(["cargo", "generate-lockfile", *common], cwd=ROOT, check=True)
            # Cargo runs integration tests at the temporary package root. The
            # historical header fixtures use repository-relative paths.
            subprocess.run(["cargo", "test", "--locked", "--no-run", *common,
                            "--target-dir", str(work / "target")], cwd=ROOT, check=True)
            executables = [p for p in (work / "target/debug/deps").glob("clang-*")
                           if p.is_file() and os.access(p, os.X_OK)]
            assert len(executables) == 1, executables
            subprocess.run([str(executables[0])], cwd=ROOT, check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clang", action="store_true")
    check(parser.parse_args().clang)
