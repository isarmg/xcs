"""Keep runner-only context and environment setup out of early workflow env."""
import importlib.util
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("workflow_checker", SCRIPTS / "check-workflows.py")
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class WorkflowContexts(unittest.TestCase):
    def test_repository_workflows_use_available_early_contexts(self):
        for path in (SCRIPTS.parent / "workflows").glob("*.y*ml"):
            with self.subTest(path=path.name):
                self.assertEqual(checker.check(path.read_text()), [])

    def test_every_rust_job_initializes_cargo_home_before_toolchain_use(self):
        expected = {
            "ci.yml": {"tests", "web-production", "browser-smoke", "smb-sdk"},
            "release.yml": {"version", "verify", "linux-web"},
        }
        for name, rust_jobs in expected.items():
            source = (SCRIPTS.parent / "workflows" / name).read_text()
            parts = re.split(r"^  ([A-Za-z0-9_-]+):\s*$", source.split("\njobs:\n", 1)[1], flags=re.MULTILINE)
            jobs = dict(zip(parts[1::2], parts[2::2]))
            self.assertIn("bash .github/scripts/check.sh static", jobs["static"])
            self.assertNotRegex(jobs["static"], r"\b(?:cargo|rustup|rustc)\s")
            for job in rust_jobs:
                with self.subTest(workflow=name, job=job):
                    body = jobs[job]
                    self.assertLess(body.index("initialize-cargo-home.sh"), body.index("rustup "))

    def test_workflow_env_rejects_runner_and_other_job_only_contexts(self):
        for expression in ("runner.temp", "Runner['temp']", "toJSON(runner)",
                           "matrix.target", "needs.build.outputs.path", "env.OTHER"):
            with self.subTest(expression=expression):
                errors = checker.check(f"env:\n  CARGO_HOME: ${{{{ {expression} }}}}/cargo\njobs:\n  test:\n")
                self.assertEqual(len(errors), 1)
                self.assertIn("env cannot use", errors[0])

    def test_job_env_rejects_runner_but_allows_job_planning_contexts(self):
        source = "jobs:\n  test:\n    env:\n      VALUE: ${{ %s }}\n    steps:\n      - run: true\n"
        for expression in ("runner.temp", "steps.build.outputs.path", "job.status", "env.OTHER"):
            with self.subTest(expression=expression):
                self.assertEqual(len(checker.check(source % expression)), 1)
        for expression in ("matrix.target", "needs.build.outputs.version", "strategy.job-index",
                           "github.sha", "vars.TARGET", "inputs.target", "secrets.TOKEN"):
            with self.subTest(expression=expression):
                self.assertEqual(checker.check(source % expression), [])

    def test_step_env_run_and_action_inputs_allow_runner_context(self):
        source = """jobs:
  test:
    steps:
      - name: Runner context
        env:
          VALUE: ${{ runner.temp }}
        run: echo "$VALUE"
      - uses: actions/cache@sha
        with:
          key: ${{ runner.os }}
"""
        self.assertEqual(checker.check(source), [])

    def test_inline_env_and_multiline_expressions_are_checked(self):
        self.assertEqual(len(checker.check("env: {VALUE: '${{ runner.temp }}'}\n")), 1)
        source = "jobs:\n  test:\n    env:\n      VALUE: >-\n        ${{\n          runner.temp\n        }}\n"
        self.assertEqual(len(checker.check(source)), 1)

    def test_literals_and_properties_are_not_mistaken_for_context_roots(self):
        source = """# ${{ runner.temp }}
env:
  VALUE: ${{ format('runner.temp {0}', github.runner) }}
  REF: ${{ github['runner'] }}
  LITERAL: runner.temp
  BRANCH: ${{ github.ref }} # runner.temp
"""
        self.assertEqual(checker.check(source), [])

    def test_hash_files_is_not_available_in_early_env(self):
        self.assertIn("cannot use hashFiles", checker.check("env:\n  HASH: ${{ hashFiles('Cargo.lock') }}\n")[0])


class CargoHome(unittest.TestCase):
    def test_initialization_creates_unique_empty_homes_and_preserves_environment_file(self):
        with tempfile.TemporaryDirectory(prefix="runner temp ") as temporary:
            root = Path(temporary)
            environment_file = root / "github env"
            environment_file.write_text("EXISTING=value\n")
            environment = dict(os.environ, RUNNER_TEMP=str(root), GITHUB_ENV=str(environment_file))
            for _ in range(2):
                subprocess.run(["bash", str(SCRIPTS / "initialize-cargo-home.sh")],
                               env=environment, check=True, capture_output=True, text=True)
            lines = environment_file.read_text().splitlines()
            self.assertEqual(lines[0], "EXISTING=value")
            homes = [Path(line.removeprefix("CARGO_HOME=")) for line in lines[1:]]
            self.assertEqual(len(set(homes)), 2)
            for home in homes:
                self.assertEqual(home.parent, root)
                self.assertTrue(home.is_dir())
                self.assertEqual(list(home.iterdir()), [])

    def test_initialization_requires_runner_environment(self):
        for missing in ("RUNNER_TEMP", "GITHUB_ENV"):
            with self.subTest(missing=missing):
                environment = dict(os.environ, RUNNER_TEMP="/tmp", GITHUB_ENV="/dev/null")
                del environment[missing]
                result = subprocess.run(["bash", str(SCRIPTS / "initialize-cargo-home.sh")],
                                        env=environment, check=False, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(missing, result.stderr)


if __name__ == "__main__":
    unittest.main()
