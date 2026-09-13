import tempfile
import unittest
from pathlib import Path

from incus_only_shell.config import Identity, load_user_config


class UserConfigTests(unittest.TestCase):
    def write_users(self, text: str) -> Path:
        td = tempfile.TemporaryDirectory()
        self.addCleanup(td.cleanup)
        path = Path(td.name) / 'users.toml'
        path.write_text(text, encoding='utf-8')
        return path

    def test_uid_keyed_lookup(self):
        path = self.write_users('''
[users."1001"]
username = "incus-test"
management_id = 123
''')
        ident = Identity('incus-test', 1001, 1001, '/home/incus-test')
        cfg = load_user_config(ident, path)
        self.assertEqual(cfg.management_id, 123)
        self.assertEqual(cfg.uid, 1001)

    def test_username_mismatch_rejected(self):
        path = self.write_users('''
[users."1001"]
username = "someone-else"
management_id = 123
''')
        ident = Identity('incus-test', 1001, 1001, '/home/incus-test')
        with self.assertRaises(ValueError):
            load_user_config(ident, path)

    def test_duplicate_management_id_rejected(self):
        path = self.write_users('''
[users."1001"]
username = "incus-test"
management_id = 123

[users."1002"]
username = "alice"
management_id = 123
''')
        ident = Identity('incus-test', 1001, 1001, '/home/incus-test')
        with self.assertRaises(ValueError):
            load_user_config(ident, path)
