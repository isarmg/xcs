"""Keep maintained entrypoint and SQLite examples aligned with supported APIs."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MaintainedDocs(unittest.TestCase):
    def test_engineering_entrypoints_and_test_layout(self):
        text = (ROOT / 'docs/20-engineering.md').read_text()
        self.assertNotRegex(text, r'\bzxs offline-(?:check|export)\b')
        for command in ('offline-check', 'offline-export'):
            self.assertIn(f'`zxs-rescue {command} ', text)
        self.assertNotIn('`tests/`', text)
        self.assertNotIn('模块测试靠近实现', text)
        self.assertIn('根 `test/`', text)
        self.assertNotIn('admin/', text)
        self.assertIn('rescue/', text)
        rescue = (ROOT / 'src/rescue/src/main.rs').read_text()
        for variant in ('OfflineCheck', 'OfflineExport'):
            self.assertIn(f'    {variant} {{', rescue)
        self.assertNotIn('本机 zxs → 服务启动 / 初始化', text)

    def test_sqlite_type_examples_are_supported_and_compile_checked(self):
        text = (ROOT / 'src/foundation/data-storage/sqlite-access/src/query_engine/types/mod.rs').read_text()
        for removed in ('query!', 'query_as!', 'PgHasArrayType', '#[sqlx(',
                        '#[derive(sqlite_access::Type)]', 'PostgreSQL', 'postgres'):
            self.assertNotIn(removed, text)
        self.assertNotRegex(text, r'```rust,(?:ignore|text)')
        for trait in ('Type<Sqlite>', "Encode<'q, Sqlite>", "Decode<'r, Sqlite>"):
            self.assertIn(trait + ' for UserId', text)
        self.assertIn('sqlite_access::query_scalar("SELECT ?")', text)
        self.assertIn('.bind(UserId(42))', text)


if __name__ == '__main__':
    unittest.main()
