from __future__ import annotations

import atexit
import os
import readline
import sys

from .addressing import AddressPlan
from .commands import CommandDispatcher
from .config import (
    current_identity,
    load_platform,
    load_user_config,
)
from .incus import IncusClient
from .lifecycle import Lifecycle
from .parser import split_commands
from .policy import HOST_DENIED_MESSAGE
from .slots import SlotManager


class IncusOnlyShell:
    def __init__(self) -> None:
        self.identity = current_identity()
        self.cfg = load_platform()

        self.user_cfg = load_user_config(
            self.identity,
            self.cfg.users_file,
        )

        self.management_id = self.user_cfg.management_id
        self.project = f'{self.cfg.project_prefix}{self.identity.uid}'

        self.history_file = os.path.join(
            self.identity.home,
            '.incus-only-shell_history',
        )

        self.incus = IncusClient(
            project=self.project,
            home=self.identity.home,
            incus_conf=str(self.cfg.incus_conf),
        )

        self.address_plan = AddressPlan(
            management_id=self.management_id,
            max_containers=self.cfg.max_containers,
            service_slots=self.cfg.service_slots,
            ipv4_prefix=self.cfg.ipv4_prefix,
        )

        self.slots = SlotManager(
            client=self.incus,
            plan=self.address_plan,
            slot_key=self.cfg.slot_key,
            management_key=self.cfg.management_key,
            home=self.identity.home,
        )

        self.lifecycle = Lifecycle(
            client=self.incus,
            slots=self.slots,
            nat_device=self.cfg.nat_device,
        )

        self.dispatcher = CommandDispatcher(
            client=self.incus,
            lifecycle=self.lifecycle,
            home=self.identity.home,
        )

    def setup_readline(self) -> None:
        readline.parse_and_bind(
            'set editing-mode emacs'
        )
        readline.parse_and_bind(
            'set enable-bracketed-paste on'
        )
        readline.set_history_length(1000)

        try:
            readline.read_history_file(
                self.history_file
            )
        except (FileNotFoundError, OSError):
            pass

    def save_history(self) -> None:
        try:
            readline.set_history_length(1000)
            readline.write_history_file(
                self.history_file
            )
            os.chmod(
                self.history_file,
                0o600,
            )
        except OSError:
            pass

    def banner(self) -> str:
        return (
            f'Incus-only shell for '
            f'{self.identity.username} '
            f'({self.project})\n'
            f'Management ID: '
            f'{self.management_id:03d}\n'
            f'Type "help" for available commands.'
        )

    def prompt(self) -> str:
        return (
            f'{self.identity.username}'
            '@incus> '
        )

    def dispatch_text(
        self,
        text: str,
    ) -> int:
        commands = split_commands(text)

        result = 0

        for command in commands:
            result = self.dispatcher.dispatch(
                command
            )

        return result

    def interactive(self) -> int:
        self.setup_readline()
        atexit.register(
            self.save_history
        )

        print(
            self.banner()
        )

        while True:
            try:
                text = input(
                    self.prompt()
                )

                self.dispatch_text(
                    text
                )

            except EOFError:
                print()
                return 0

            except KeyboardInterrupt:
                print()

    def noninteractive(
        self,
        command: str,
    ) -> int:
        try:
            return self.dispatch_text(
                command
            )
        except EOFError:
            return 0


def main() -> int:
    try:
        shell = IncusOnlyShell()

    except Exception as exc:
        print(
            f'Incus-only shell initialization failed: {exc}',
            file=sys.stderr,
        )
        return 1

    try:
        os.chdir(
            shell.identity.home
        )

    except OSError as exc:
        print(
            f'Unable to enter home directory: {exc}',
            file=sys.stderr,
        )
        return 1

    if len(sys.argv) == 1:
        return shell.interactive()

    if (
        len(sys.argv) == 3
        and sys.argv[1] == '-c'
    ):
        return shell.noninteractive(
            sys.argv[2]
        )

    print(
        HOST_DENIED_MESSAGE,
        file=sys.stderr,
    )

    return 126


if __name__ == '__main__':
    raise SystemExit(
        main()
    )