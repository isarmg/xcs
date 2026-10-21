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

class UnicodeCapabilityGateTests(unittest.TestCase):
    def test_exactly_seven_retired_package_features_are_checked(self):
        self.assertEqual(sum(map(len, GATE.RETIRED_FEATURES.values())), 7)
        for name, retired in GATE.RETIRED_FEATURES.items():
            self.assertFalse(set(retired) & set(GATE.SUPPORTED_FEATURES[name]))
            self.assertIn(('serialization',), GATE.CAPABILITY_PROFILES[name])
            self.assertIn(GATE.SUPPORTED_FEATURES[name], GATE.CAPABILITY_PROFILES[name])

    def test_alias_and_weak_forwarding_cannot_reenable_retired_features(self):
        manifest = {'dependencies': {'unicode': {'package': 'unicode-properties'}},
                    'features': {'hidden': ['unicode?/datagen']}}
        self.assertEqual(GATE.manifest_capability_violations(manifest), ['unicode?/datagen'])

    def test_target_specific_dependency_selections_are_rejected(self):
        manifest = {'target': {'cfg(unix)': {'dependencies': {
            'normalizer': {'package': 'unicode-normalizer', 'features': ['write16']}}}}}
        self.assertEqual(GATE.manifest_capability_violations(manifest), ['normalizer/write16'])

    def test_workspace_alias_and_feature_inheritance_are_scanned(self):
        inherited = {'icu': {'package': 'unicode-properties', 'features': ['unicode_bidi']}}
        manifest = {'dependencies': {'icu': {'workspace': True}},
                    'features': {'hidden': ['icu?/datagen']}}
        self.assertEqual(GATE.manifest_capability_violations(manifest, inherited),
                         ['icu/unicode_bidi', 'icu?/datagen'])

    def test_empty_feature_and_local_forwarding_are_not_compatibility_aliases(self):
        manifest = {'package': {'name': 'unicode-normalizer'},
                    'features': {'utf16_iter': [], 'hidden': ['write16']}}
        self.assertEqual(GATE.manifest_capability_violations(manifest), ['utf16_iter', 'write16'])

    def test_supported_serialization_and_optional_properties_are_kept(self):
        manifest = {'package': {'name': 'unicode-normalizer'},
                    'dependencies': {'unicode-properties': {'optional': True}},
                    'features': {'serialization': ['unicode-properties?/serialization']}}
        self.assertEqual(GATE.manifest_capability_violations(manifest), [])

    def test_unknown_feature_requires_exact_cargo_resolution_diagnostic(self):
        from subprocess import CompletedProcess
        result = CompletedProcess([], 101, stderr='package `unicode-capability-probe` depends on '
            '`unicode-normalizer` with feature `write16` but `unicode-normalizer` does not have that feature.')
        GATE.validate_unknown_feature(result, 'unicode-normalizer', 'write16')
        with self.assertRaisesRegex(AssertionError, 'Cargo resolution'):
            GATE.validate_unknown_feature(result, 'unicode-normalizer', 'utf16_iter')
        with self.assertRaisesRegex(AssertionError, 'Cargo resolution'):
            GATE.validate_unknown_feature(CompletedProcess([], 101, stderr='error[E0432]: unresolved import write16'),
                                         'unicode-normalizer', 'write16')
        with self.assertRaisesRegex(AssertionError, 'compilation'):
            GATE.validate_unknown_feature(CompletedProcess([], 101, stderr=result.stderr + '\nerror[E0432]: import'),
                                         'unicode-normalizer', 'write16')
        with self.assertRaisesRegex(AssertionError, 'Cargo resolution'):
            GATE.validate_unknown_feature(CompletedProcess([], 0, stderr=result.stderr),
                                         'unicode-normalizer', 'write16')

    def test_default_maintained_gate_runs_capabilities_before_family(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'Cargo.lock').write_text('unchanged')
            calls = []
            with patch.object(GATE, 'validate_capability_surface'), \
                 patch.object(GATE, 'check_capabilities', side_effect=lambda *a: calls.append(('capabilities', a[-1]))), \
                 patch.object(GATE, 'check_family', side_effect=lambda *a: calls.append(('family', a[-1]))):
                GATE.check(root)
            self.assertEqual(calls, [('capabilities', 'normalizer'), ('family', 'normalizer'),
                                    ('capabilities', 'properties'), ('family', 'properties')])

    def test_capabilities_only_does_not_claim_behavior_execution(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'Cargo.lock').write_text('unchanged')
            with patch.object(GATE, 'validate_capability_surface'), \
                 patch.object(GATE, 'check_capabilities') as capabilities, \
                 patch.object(GATE, 'check_family') as behavior:
                GATE.check(root, families=('properties',), capabilities_only=True)
            self.assertEqual(capabilities.call_count, 1)
            behavior.assert_not_called()


    def test_target_alias_variants_cannot_hide_a_unicode_forwarder(self):
        manifest = {'target': {
            'cfg(unix)': {'dependencies': {'text': {'package': 'unicode-properties'}}},
            'cfg(windows)': {'dependencies': {'text': {'package': 'unrelated'}}}},
            'features': {'hidden': ['text?/unicode_bidi']}}
        self.assertEqual(GATE.manifest_capability_violations(manifest), ['text?/unicode_bidi'])
