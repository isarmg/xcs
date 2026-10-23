"""Discover maintained assertion-based gates and prove optimization fails closed."""
import ast
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
MESSAGE = "Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE)."


def gates():
    # Current validation entrypoints only, not upstream source, historical
    # benchmark evidence, test fixtures, or the assertion-free perf comparator.
    for directory in (ROOT / ".github/scripts", ROOT / "tools/rust-sources"):
        for path in sorted(directory.glob("*.py")):
            module = ast.parse(path.read_text(encoding="utf-8"))
            if any(isinstance(node, ast.Assert) for node in ast.walk(module)):
                yield path, module


def first_work(module):
    body = list(module.body)
    if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) \
            and isinstance(body[0].value.value, str):
        body.pop(0)
    while body and isinstance(body[0], ast.ImportFrom) and body[0].module == "__future__":
        body.pop(0)
    return body[0]


class PythonGateOptimization(unittest.TestCase):
    def test_assertion_based_gates_reject_optimization_before_any_work(self):
        discovered = list(gates())
        self.assertTrue(discovered, "the discovery guard must not silently lose its maintained scope")
        expected = ast.parse(f"if not __debug__:\n    raise RuntimeError({MESSAGE!r})").body[0]
        # Fail structurally before executing a future unguarded entrypoint.
        # This also proves there cannot be a side-effecting import or command
        # before the runtime rejection, without mocking its real dependencies.
        for path, module in discovered:
            self.assertEqual(ast.dump(first_work(module)), ast.dump(expected), str(path))
        with tempfile.TemporaryDirectory() as folder:
            environment = os.environ | {"PATH": folder, "PYTHONDONTWRITEBYTECODE": "1"}
            for path, _ in discovered:
                for flag, optimized in ((["-O"], ""), ([], "2")):
                    with self.subTest(path=path.relative_to(ROOT), flag=flag, optimize=optimized):
                        result = subprocess.run([sys.executable, *flag, str(path)], cwd=folder,
                                                env=environment | {"PYTHONOPTIMIZE": optimized},
                                                capture_output=True, text=True, timeout=10)
                        self.assertNotEqual(result.returncode, 0)
                        self.assertIn("RuntimeError: " + MESSAGE, result.stderr)
                        self.assertEqual(result.stdout, "")
                        self.assertEqual(list(Path(folder).iterdir()), [])


if __name__ == "__main__":
    unittest.main()
