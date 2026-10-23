"""Keep task dispatch's active surface small and its runtime checks wired into CI."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class TaskDispatchContracts(unittest.TestCase):
    def test_only_supported_backends_are_advertised(self):
        manifest = tomllib.loads((ROOT / 'src/foundation/concurrency/task-dispatch/Cargo.toml').read_text())
        self.assertEqual(set(manifest['features']), {'tokio', 'wasm-bindgen', 'futures-executor'})
        self.assertEqual(manifest['dependencies']['futures']['features'], ['channel', 'std'])
        self.assertEqual(manifest['features']['futures-executor'], ['futures/thread-pool'])
        source = (ROOT / 'src/foundation/concurrency/task-dispatch/src/lib.rs').read_text()
        for retired in ('CustomExecutor', 'init_glib', 'init_async_executor', 'no_op_spawn', 'debug_assertions'):
            self.assertNotIn(retired, source)

    def test_runtime_tests_are_connected(self):
        script = (ROOT / '.github/scripts/check-task-dispatch.py').read_text()
        for contract in ('uninitialized', 'futures', 'tokio', '--release', 'wasm32-unknown-unknown'):
            self.assertIn(contract, script)
        self.assertIn('check-task-dispatch.py', (ROOT / '.github/scripts/check-foundation.sh').read_text())
        browser = (ROOT / '.github/scripts/check-web-bindgen-scope.py').read_text()
        self.assertIn('test/task-dispatch/browser.rs', browser)
        self.assertIn('len(checks) == len(expected) and set(checks) == expected', browser)
        self.assertIn('task dispatch Send and local futures', browser)
        self.assertIn('task dispatch initialization and tick', browser)
        self.assertIn('await api.task_dispatch_contract()', (ROOT / 'test/wasm-binding-macros/browser-runtime.html').read_text())


if __name__ == '__main__':
    unittest.main()
