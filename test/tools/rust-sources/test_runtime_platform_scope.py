"""Guard retired runtime implementations and required future/fallback surfaces."""
import hashlib
import json
from pathlib import Path
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools/rust-sources"))
from historical_test_sources import historical_reader

ROOT = Path(__file__).resolve().parents[3]
PROOF = ROOT / 'docs/validation/evidence/foundation/linux-amd64-runtime-final-20261020.json'


class RuntimePlatformScope(unittest.TestCase):
    def test_retired_and_retained_source_inventory(self):
        proof = json.loads(PROOF.read_text())
        read = historical_reader(ROOT)
        self.assertEqual(len(proof['retired_files']), 60)
        self.assertEqual(sum(v['bytes'] for v in proof['retired_files'].values()), 223354)
        for path in proof['retired_files']:
            self.assertFalse((ROOT / path).exists(), path)
        for path, expected in proof['retained_files_sha256'].items():
            self.assertEqual(hashlib.sha256(read(path)).hexdigest(), expected, path)
        for path, evidence in proof['changed_files'].items():
            self.assertEqual(hashlib.sha256(read(path)).hexdigest(), evidence['after_sha256'], path)

    def test_retired_random_overrides_fail_closed(self):
        # Removed 0.2/0.3 implementations stay authenticated by the pinned proof above.
        crate = ROOT / 'src/foundation/system-entropy'
        source = (crate / 'src/backends.rs').read_text()
        self.assertIn('this getrandom backend was retired', source)
        for backend in ('rndr', 'efi_rng', 'windows_legacy'):
            self.assertIn(f'system_entropy_backend = "{backend}"', source)
            self.assertNotIn(f'mod {backend};', source)
        self.assertIn('all(target_os = "linux", target_env = "")', source)
        for backend in ('linux_raw', 'linux_android_with_fallback', 'getrandom', 'custom', 'wasm_js', 'rdrand', 'unsupported', 'extern_impl'):
            self.assertIn(f'mod {backend};', source)
        self.assertNotIn('feature(uefi_std)', (crate / 'src/lib.rs').read_text())

    def test_required_future_and_documentation_surfaces_remain(self):
        paths = ('mio/src/sys/unix/selector/poll.rs', 'mio/src/sys/unix/waker/pipe.rs',
                 'mio/src/sys/unix/pipe.rs', 'mio/src/sys/shell/mod.rs',
                 'parking_lot_core/src/thread_parker/linux.rs', 'parking_lot_core/src/thread_parker/unix.rs',
                 'parking_lot_core/src/thread_parker/generic.rs', 'parking_lot_core/src/thread_parker/wasm.rs',
                 'parking_lot_core/src/thread_parker/wasm_atomic.rs', 'tokio/src/net/windows/named_pipe.rs',
                 'tokio/src/signal/windows/stub.rs', 'tokio/src/process/unix/mod.rs',
                 'memchr/src/arch/generic/mod.rs', 'memchr/src/arch/x86_64/mod.rs', 'memchr/src/arch/wasm32/mod.rs')
        import proof_routing
        view = proof_routing.load(ROOT)
        for path in paths:
            current = view.current_path('src/foundation/' + path)
            self.assertTrue((ROOT / current).is_file(), path)


if __name__ == '__main__':
    unittest.main()
