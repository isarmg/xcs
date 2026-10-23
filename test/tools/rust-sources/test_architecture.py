"""Consumer index must distinguish declaration edges from selected closures."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('architecture', (Path(__file__).resolve().parents[3] / "tools/rust-sources" / 'architecture.py'))
architecture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(architecture)


class ConsumerEdges(unittest.TestCase):
    def test_generated_product_roles_use_the_current_rescue_entrypoint(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers = []\n', encoding='utf-8')
            document = architecture.document(root, {'packages': {}}, {'profiles': []})
            self.assertIn('engine / server / client / rescue]', document)
            self.assertNotIn('client / admin]', document)

    def test_inherited_alias_target_optional_and_dev_edges_remain_visible(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            documents = {
                'Cargo.toml': {'workspace': {'dependencies': {'renamed': {'path': 'src/foundation/mini', 'package': 'mini'}}}},
                'src/app/Cargo.toml': {'dependencies': {'renamed': {'workspace': True, 'optional': True}},
                    'dev-dependencies': {'mini': {'path': '../foundation/mini'}},
                    'target': {'cfg(unix)': {'build-dependencies': {'mini': {'path': '../foundation/mini', 'optional': True}}}}},
            }
            edges = architecture.consumer_edges(root, documents)['src/foundation/mini']
            self.assertEqual(edges, ['src/app [build, optional, cfg(unix)]', 'src/app [dev]', 'src/app [normal, optional] alias=renamed'])
            self.assertFalse(any('Cargo.toml' in edge for edge in edges))


if __name__ == '__main__':
    unittest.main()
