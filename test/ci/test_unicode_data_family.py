"""Keep Unicode family isolation and immutable table guards fail-closed."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    'unicode_data_family', Path(__file__).resolve().parents[2] /
    '.github/scripts/check-unicode-data-family.py')
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class UnicodeDataGateTests(unittest.TestCase):
    def graph(self, compiled=False):
        return {'packages': [{'id': 'runtime', 'name': 'unicode-normalizer'},
                             {'id': 'provider', 'name': 'icu_provider'}],
                'resolve': {'nodes': [{'id': 'runtime', 'features': ['compiled_data'] if compiled else []},
                                      {'id': 'provider', 'features': ['baked'] if compiled else []}]}}

    def test_minimal_graphs_do_not_need_product_feature_unification(self):
        for compiled in (False, True):
            GATE.validate_graph(self.graph(compiled), 'unicode-normalizer', compiled, '')

    def test_baked_feature_cannot_leak_into_provider_only_graph(self):
        graph = self.graph()
        graph['resolve']['nodes'][1]['features'] = ['baked']
        with self.assertRaisesRegex(AssertionError, 'provider-only'):
            GATE.validate_graph(graph, 'unicode-normalizer', False, '')

    def test_actual_optional_properties_edge_is_rejected(self):
        with self.assertRaisesRegex(AssertionError, 'optional properties'):
            GATE.validate_graph(self.graph(True), 'unicode-normalizer', True, 'unicode-properties v2.3.0')

    def test_absorbed_package_is_rejected(self):
        graph = self.graph(True)
        graph['packages'].append({'id': 'data', 'name': 'icu_normalizer_data'})
        graph['resolve']['nodes'].append({'id': 'data', 'features': []})
        with self.assertRaisesRegex(AssertionError, 'absorbed'):
            GATE.validate_graph(graph, 'unicode-normalizer', True, '')

    def test_missing_table_receipt_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'compliance').mkdir()
            (root / 'compliance/rust-sources.json').write_text(json.dumps({'consolidations': {}}))
            with self.assertRaisesRegex(AssertionError, 'receipt'):
                GATE.validate_table_inputs(root, 'normalizer')
