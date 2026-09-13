from __future__ import annotations

import atexit
import os
import readline
import shutil
import sys
from pathlib import Path

from .addressing import AddressPlan
from .commands import CommandDispatcher
from .config import current_identity, load_management_id, load_platform
from .hostcmd import run_host
from .incus import IncusClient
from .lifecycle import Lifecycle
from .parser import parse_command, split_commands
from .policy import HOST_DENIED
from .slots import SlotManager


def discover_sftp_servers() -> set[str]:
    candidates = (
        '/usr/lib/openssh/sftp-server',
        '/usr/libexec/openssh/sftp-server',
        '/usr/lib/ssh/sftp-server',
        '/usr/libexec/sftp-server',
    )
    return {p for p in candidates if os.path.isfile(p) and os.access(p, os.X_OK)}


class Shell:
    def __init__(self):
        self.identity = current_identity()
        self.cfg = load_platform()
        self.management_id = load_management_id(self.identity.username, self.cfg.users_file)
        self.project = f'{self.cfg.project_prefix}{self.identity.uid}'
        self.plan = AddressPlan(
            management_id=self.management_id,
            max_containers=self.cfg.max_containers,
            service_slots=self.cfg.service_slots,
            ipv4_prefix=self.cfg.ipv4_prefix,
        )
        self.client = IncusClient(self.project, self.identity.home, str(self.cfg.incus_conf))
        self.slots = SlotManager(
            self.client,
            self.plan,
            self.cfg.slot_key,
            self.cfg.management_key,
            self.identity.home,
        )
        self.lifecycle = Lifecycle(self.client, self.slots, self.cfg.nat_device)
        self.commands = CommandDispatcher(self.client, self.lifecycle, self.identity.home)
        self.history = Path(self.identity.home) / '.incus-only-shell_history'
        self.sftp_servers = discover_sftp_servers()

    def help_text(self) -> str:
        return f'''Incus-only shell

User:             {self.identity.username}
Incus project:    {self.project}
Management ID:    {self.management_id:03d}
Container slots:  1..{self.cfg.max_containers}
Service slots:    0..{self.cfg.service_slots - 1}

Allowed Incus commands:
  incus list [INSTANCE]
  incus info INSTANCE
  incus top
  incus version
  incus image list [images:|local:]
  incus image info IMAGE
  incus launch IMAGE INSTANCE
  incus init IMAGE INSTANCE
  incus create IMAGE INSTANCE
  incus start|stop|restart|delete|remove INSTANCE
  incus rename OLD NEW
  incus exec INSTANCE -- COMMAND [ARGS...]
  incus snapshot create INSTANCE [SNAPSHOT]
  incus snapshot list INSTANCE
  incus snapshot delete INSTANCE SNAPSHOT
  incus snapshot restore INSTANCE SNAPSHOT
  incus export INSTANCE [FILE]
  incus import FILE INSTANCE

Container slots are allocated automatically and released when the
container is deleted. NAT IPv4 and the ten service-port numbers are
computed from the container slot and your management ID.

Read-only host commands:
  ps
  ps -ef
  ps aux
  top
  nvidia-smi
  gpu-stat

Shell commands:
  help
  clear
  exit
  logout

General host shell commands are disabled.
Administrator-owned Incus configuration changes must be requested from
the host administrator.
'''

    def setup_readline(self) -> None:
        readline.parse_and_bind('set editing-mode emacs')
        readline.parse_and_bind('set enable-bracketed-paste on')
        readline.set_history_length(1000)
        try:
            readline.read_history_file(self.history)
        except (FileNotFoundError, OSError):
            pass

    def save_history(self) -> None:
        try:
            readline.write_history_file(self.history)
            os.chmod(self.history, 0o600)
        except OSError:
            pass

    def dispatch_command(self, command: str) -> int:
        try:
            argv = parse_command(command)
        except ValueError as exc:
            print(f'Parse error: {exc}', file=sys.stderr)
            return 2
        if not argv:
            return 0
        if argv == ['help']:
            print(self.help_text())
            return 0
        if argv == ['clear']:
            print('\033[2J\033[H', end='')
            return 0
        if argv in (['exit'], ['logout']):
            raise EOFError

        host_result = run_host(argv, self.identity.home)
        if host_result is not None:
            return host_result
        if argv[0] == 'incus':
            return self.commands.dispatch(argv[1:])

        print(HOST_DENIED, file=sys.stderr)
        return 126

    def dispatch_text(self, text: str) -> int:
        result = 0
        for command in split_commands(text):
            result = self.dispatch_command(command)
        return result

    def interactive(self) -> int:
        self.setup_readline()
        atexit.register(self.save_history)
        print(f'Incus-only shell for {self.identity.username} ({self.project})')
        print('Type "help" for available commands.')
        while True:
            try:
                self.dispatch_text(input(f'{self.identity.username}@incus> '))
            except EOFError:
                print()
                return 0
            except KeyboardInterrupt:
                print()

    def noninteractive(self, argv: list[str]) -> int:
        if len(argv) != 2 or argv[0] != '-c':
            print(HOST_DENIED, file=sys.stderr)
            return 126
        command = argv[1]
        stripped = command.strip()
        if stripped in self.sftp_servers:
            os.execve(
                stripped,
                [stripped],
                {
                    'HOME': self.identity.home,
                    'USER': self.identity.username,
                    'LOGNAME': self.identity.username,
                    'PATH': '/usr/bin:/bin',
                    'LANG': os.environ.get('LANG', 'C.UTF-8'),
                },
            )
        try:
            return self.dispatch_text(command)
        except EOFError:
            return 0


def main() -> int:
    shell = Shell()
    os.chdir(shell.identity.home)
    if len(sys.argv) == 1:
        return shell.interactive()
    return shell.noninteractive(sys.argv[1:])
