from __future__ import annotations

import atexit
import os
import readline
import sys

from .commands import CommandDispatcher
from .config import (
    current_identity,
    load_platform,
    load_user_config,
)
from .incus import IncusClient
from .parser import split_commands
from .policy import HOST_DENIED_MESSAGE
from .slots import SlotAllocator


class IncusOnlyShell:
    def __init__(self) -> None:
        self.identity = current_identity()

        self.cfg = load_platform()

        self.user_cfg = load_user_config(
            self.identity,
            self.cfg.users_file,
        )

        self.management_id = (
            self.user_cfg.management_id
        )

        self.project = (
            f'{self.cfg.project_prefix}'
            f'{self.identity.uid}'
        )

        self.history_file = os.path.join(
            self.identity.home,
            '.incus-only-shell_history',
        )

        self.env = {
            'HOME': self.identity.home,
            'USER': self.identity.username,
            'LOGNAME': self.identity.username,

            'PATH': (
                '/usr/local/bin:'
                '/usr/bin:'
                '/bin:'
                '/usr/sbin:'
                '/sbin'
            ),

            'LANG': os.environ.get(
                'LANG',
                'C.UTF-8',
            ),

            'TERM': os.environ.get(
                'TERM',
                'xterm-256color',
            ),

            # Ignore user-controlled ~/.config/incus.
            'INCUS_CONF': str(
                self.cfg.incus_conf
            ),

            # User cannot switch remote/project through environment.
            'INCUS_REMOTE': 'local',
            'INCUS_PROJECT': self.project,
        }

        self.incus = IncusClient(
            project=self.project,
            env=self.env,
        )

        self.slots = SlotAllocator(
            client=self.incus,
            cfg=self.cfg,
            management_id=self.management_id,
        )

        self.dispatcher = CommandDispatcher(
            identity=self.identity,
            user_cfg=self.user_cfg,
            cfg=self.cfg,
            client=self.incus,
            slots=self.slots,
            env=self.env,
        )

    def setup_readline(self) -> None:
        readline.parse_and_bind(
            'set editing-mode emacs'
        )

        readline.parse_and_bind(
            'set enable-bracketed-paste on'
        )

        readline.set_history_length(
            1000
        )

        try:
            readline.read_history_file(
                self.history_file
            )
        except (
            FileNotFoundError,
            OSError,
        ):
            pass

    def save_history(self) -> None:
        try:
            readline.set_history_length(
                1000
            )

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
        commands = split_commands(
            text
        )

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

    # sshd normally invokes the user's login shell as:
    #
    #   shell -c 'command'
    #
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