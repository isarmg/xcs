"""Production codegen inputs remain source assets even when unit tests import them."""
import json
import re
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]


class ProductionTemplates(unittest.TestCase):
    def test_bindgen_templates_stay_with_production_codegen(self):
        codegen = ROOT / "src/foundation/bindgen/codegen"
        source = (codegen / "mod.rs").read_text()
        for name in ("bitfield_unit.rs", "bitfield_unit_raw_ref_macros.rs"):
            with self.subTest(template=name):
                self.assertIn(f'include_str!("./{name}")', source)
                self.assertTrue((codegen / name).is_file())
                self.assertIn("pub struct __BindgenBitfieldUnit", (codegen / name).read_text())

    def test_shared_template_is_not_classified_as_external_test_material(self):
        mapping = json.loads((ROOT / "compliance/external-tests/foundation.json").read_text())
        bindgen = next(owner for owner in mapping["owners"]
                       if owner["directory"] == "src/foundation/bindgen")
        originals = {entry["source"] for entry in bindgen["files"]}
        self.assertNotIn("src/foundation/bindgen/codegen/bitfield_unit.rs", originals)
        self.assertIn("src/foundation/bindgen/codegen/bitfield_unit_tests.rs", originals)


class RelocatedFixturePaths(unittest.TestCase):
    def test_retained_fixture_references_resolve_after_test_extraction(self):
        cases = {
            "test/foundation/terminal-style/utf8/unit/src/lib/guarded_benches_94.rs": ["UTF-8-demo.txt"],
            "test/foundation/mime_guess/unit/src/lib/tests_429.rs": ["mime_types.rs"],
            "test/foundation/rustc-demangle/unit/src/v0/tests_1289.rs": ["early-recursion-limit"],
            "test/foundation/wasm-bindgen-cli-support/unit/src/transforms/catch_handler/tests_619.rs": [
                "catch_handler_legacy.wat", "catch_handler_modern.wat"],
        }
        for relative, fixture_names in cases.items():
            source = ROOT / relative
            includes = re.findall(r'include(?:_str|_bytes)?!\s*\(\s*"([^"\n]+)"', source.read_text())
            for name in fixture_names:
                with self.subTest(source=relative, fixture=name):
                    matches = [value for value in includes if Path(value).name == name]
                    self.assertEqual(len(matches), 1)
                    self.assertTrue((source.parent / matches[0]).is_file())


if __name__ == "__main__":
    unittest.main()
