from __future__ import annotations

from pathlib import Path

from .incus import IncusClient, IncusError
from .lifecycle import Lifecycle
from .policy import INCUS_ADMIN_DENIED, INCUS_POLICY_DENIED, valid_image, valid_name


class CommandDispatcher:
    def __init__(self, client: IncusClient, lifecycle: Lifecycle, home: str):
        self.client = client
        self.lifecycle = lifecycle
        self.home = Path(home).resolve()

    def _deny_admin(self, message: str | None = None) -> int:
        if message:
            print(message)
        print(INCUS_ADMIN_DENIED)
        return 126

    def _deny_policy(self, message: str | None = None) -> int:
        if message:
            print(message)
        print(INCUS_POLICY_DENIED)
        return 126

    def _home_path(self, value: str, must_exist: bool) -> str | None:
        if not value or value == '-':
            return None
        p = Path(value)
        if not p.is_absolute():
            p = self.home / p
        try:
            resolved = p.resolve(strict=must_exist)
            resolved.relative_to(self.home)
        except (ValueError, FileNotFoundError, RuntimeError):
            return None
        return str(resolved)

    def dispatch(self, args: list[str]) -> int:
        if not args:
            return self._deny_policy()

        if args == ['list']:
            return self.client.run(['list']).returncode
        if len(args) == 2 and args[0] == 'list' and valid_name(args[1]):
            return self.client.run(args).returncode
        if len(args) == 2 and args[0] == 'info' and valid_name(args[1]):
            return self.client.run(args).returncode
        if args in (['top'], ['version']):
            return self.client.run(args).returncode

        if args == ['image', 'list']:
            return self.client.run(args).returncode
        if len(args) == 3 and args[:2] == ['image', 'list'] and args[2] in ('images:', 'local:'):
            return self.client.run(args).returncode
        if len(args) == 3 and args[:2] == ['image', 'info'] and valid_image(args[2]):
            return self.client.run(args).returncode

        if len(args) == 3 and args[0] in ('launch', 'init', 'create'):
            if not valid_image(args[1]) or not valid_name(args[2]):
                return self._deny_policy('Invalid image or instance name.')
            try:
                return self.lifecycle.create(args[0], args[1], args[2])
            except (IncusError, RuntimeError, ValueError) as exc:
                return self._deny_policy(str(exc))

        if len(args) == 2 and args[0] in ('start', 'stop', 'restart', 'delete', 'remove') and valid_name(args[1]):
            return self.client.run(args).returncode
        if len(args) == 3 and args[0] == 'rename' and valid_name(args[1]) and valid_name(args[2]):
            return self.client.run(args).returncode

        if len(args) >= 4 and args[0] == 'exec' and valid_name(args[1]) and args[2] == '--':
            return self.client.run(args).returncode
        if args and args[0] == 'exec':
            return self._deny_policy('Usage: incus exec INSTANCE -- COMMAND [ARGS...]')

        if len(args) in (3, 4) and args[:2] == ['snapshot', 'create'] and valid_name(args[2]) and (len(args) == 3 or valid_name(args[3])):
            return self.client.run(args).returncode
        if len(args) == 3 and args[:2] == ['snapshot', 'list'] and valid_name(args[2]):
            return self.client.run(args).returncode
        if len(args) == 4 and args[:2] == ['snapshot', 'delete'] and valid_name(args[2]) and valid_name(args[3]):
            return self.client.run(args).returncode
        if len(args) == 4 and args[:2] == ['snapshot', 'restore'] and valid_name(args[2]) and valid_name(args[3]):
            try:
                return self.lifecycle.restore(args[2], args[3])
            except (IncusError, RuntimeError, ValueError) as exc:
                return self._deny_policy(str(exc))
        if args and args[0] == 'snapshot':
            return self._deny_policy('Snapshot operation or arguments are not permitted.')

        if len(args) in (2, 3) and args[0] == 'export' and valid_name(args[1]):
            if len(args) == 2:
                return self.client.run(args).returncode
            target = self._home_path(args[2], must_exist=False)
            if target is None:
                return self._deny_policy('Export target must be inside your home directory.')
            return self.client.run(['export', args[1], target]).returncode

        # Require an explicit new instance name so slot assignment is transactional and deterministic.
        if len(args) == 3 and args[0] == 'import' and valid_name(args[2]):
            source = self._home_path(args[1], must_exist=True)
            if source is None:
                return self._deny_policy('Import source must be an existing file inside your home directory.')
            try:
                return self.lifecycle.import_backup(source, args[2])
            except (IncusError, RuntimeError, ValueError) as exc:
                return self._deny_policy(str(exc))
        if args and args[0] == 'import':
            return self._deny_policy('Usage: incus import FILE INSTANCE')

        # Direct config/profile/project/network/storage/remote/query access remains admin-only.
        return self._deny_admin()
