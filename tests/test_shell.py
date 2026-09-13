import unittest
from types import SimpleNamespace
from unittest.mock import patch

from incus_only_shell.shell import Shell


class FakeCommands:
    def __init__(self):
        self.calls = []

    def dispatch(self, argv):
        self.calls.append(argv)
        return 0


class ShellRoutingTests(unittest.TestCase):
    def make_shell(self):
        shell = Shell.__new__(Shell)
        shell.identity = SimpleNamespace(username='incus-test', uid=1001, home='/tmp')
        shell.commands = FakeCommands()
        return shell

    def test_incus_prefix_is_stripped_once(self):
        shell = self.make_shell()
        with patch('incus_only_shell.shell.run_host', return_value=None):
            rc = shell.dispatch_command('incus list')
        self.assertEqual(rc, 0)
        self.assertEqual(shell.commands.calls, [['list']])

    def test_exit_is_builtin(self):
        shell = self.make_shell()
        with self.assertRaises(EOFError):
            shell.dispatch_command('exit')

    def test_logout_is_builtin(self):
        shell = self.make_shell()
        with self.assertRaises(EOFError):
            shell.dispatch_command('logout')
