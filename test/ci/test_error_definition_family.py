import copy
from pathlib import Path
import runpy
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
CHECK=runpy.run_path(str(ROOT/'.github/scripts/check-error-definition-family.py'))
class ErrorDefinitionFamily(unittest.TestCase):
    def manifests(self):
        return {name:tomllib.loads((foundation_path(ROOT, name)/'Cargo.toml').read_text()) for name in CHECK['NAMES']}
    def test_current_family(self):
        CHECK['check_private_family'](ROOT)
    def test_version_skew_rejected(self):
        for name in CHECK['NAMES']:
            for version in ('2.0.22','2.1.21','3.0.21'):
                manifests=self.manifests();manifests[name]['package']['version']=version
                with self.assertRaisesRegex(AssertionError,'versions must match'):
                    CHECK['validate_private_family'](manifests)
    def test_unpinned_or_aliased_or_nonlocal_dependency_rejected(self):
        for key,value in [('version','2.0.21'),('version','=2.0.20'),('package','thiserror-impl'),('path','../thiserror-impl')]:
            manifests=self.manifests();manifests['error-definition']['dependencies']['error-definition-macros'][key]=value
            with self.assertRaises(AssertionError):CHECK['validate_private_family'](manifests)
    def test_stable_hidden_support(self):
        for path in ('error-definition/build.rs','error-definition-macros/src/lib.rs'):
            text=(foundation_path(ROOT, path)).read_text()
            self.assertIn('__derive_support',text)
            self.assertNotIn('CARGO_PKG_VERSION_PATCH',text)
            self.assertNotIn('__private',text)
if __name__=='__main__':unittest.main()
