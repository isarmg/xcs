"""Consumer index must distinguish declaration edges from selected closures."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('architecture', Path(__file__).with_name('architecture.py'))
architecture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(architecture)


class ConsumerEdges(unittest.TestCase):
    def test_inherited_alias_target_optional_and_dev_edges_remain_visible(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            documents = {
                'Cargo.toml': {'workspace': {'dependencies': {'renamed': {'path': 'src/foundation/mini', 'package': 'mini'}}}},
                'src/app/Cargo.toml': {'dependencies': {'renamed': {'workspace': True}},
                    'dev-dependencies': {'mini': {'path': '../foundation/mini'}},
                    'target': {'cfg(unix)': {'build-dependencies': {'mini': {'path': '../foundation/mini', 'optional': True}}}}},
            }
            edges = architecture.consumer_edges(root, documents)['src/foundation/mini']
            self.assertEqual(edges, ['src/app [build, optional, cfg(unix)]', 'src/app [dev]', 'src/app [normal] alias=renamed'])
            self.assertFalse(any('Cargo.toml' in edge for edge in edges))


if __name__ == '__main__':
    unittest.main()
