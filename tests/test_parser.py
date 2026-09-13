import sys
import unittest

sys.path.insert(0, 'src')
from incus_only_shell.parser import split_commands


class ParserTests(unittest.TestCase):
    def test_multiline_paste(self):
        self.assertEqual(split_commands('help\nincus list\nincus version'), ['help', 'incus list', 'incus version'])

    def test_multiline_quote(self):
        text = "incus exec c1 -- bash -c 'echo one\necho two'"
        self.assertEqual(split_commands(text), [text])


if __name__ == '__main__':
    unittest.main()
