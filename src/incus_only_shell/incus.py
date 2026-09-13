from __future__ import annotations

import json
import os
import shutil
import subprocess
import urllib.parse
from dataclasses import dataclass
from typing import Any


@dataclass
class IncusResult:
    returncode: int
    stdout: str = ''
    stderr: str = ''


class IncusError(RuntimeError):
    pass


class IncusClient:
    def __init__(self, project: str, home: str, incus_conf: str):
        self.project = project
        self.home = home
        self.binary = shutil.which('incus', path='/usr/local/bin:/usr/bin:/bin')
        if self.binary is None:
            raise RuntimeError('incus binary not found')

        self.env = {
            'HOME': home,
            'USER': os.environ.get('USER', ''),
            'LOGNAME': os.environ.get('LOGNAME', ''),
            'PATH': '/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin',
            'LANG': os.environ.get('LANG', 'C.UTF-8'),
            'TERM': os.environ.get('TERM', 'xterm-256color'),
            'INCUS_CONF': incus_conf,
            'INCUS_REMOTE': 'local',
            'INCUS_PROJECT': project,
        }

    def run(self, args: list[str], capture: bool = False) -> IncusResult:
        p = subprocess.run(
            [self.binary, *args],
            cwd=self.home,
            env=self.env,
            shell=False,
            text=True,
            stdout=subprocess.PIPE if capture else None,
            stderr=subprocess.PIPE if capture else None,
        )
        return IncusResult(p.returncode, p.stdout or '', p.stderr or '')

    def check(self, args: list[str], capture: bool = False) -> IncusResult:
        result = self.run(args, capture=capture)
        if result.returncode != 0:
            detail = result.stderr.strip() or f'Incus command failed with status {result.returncode}'
            raise IncusError(detail)
        return result

    def query(self, path: str) -> Any:
        sep = '&' if '?' in path else '?'
        if 'project=' not in path:
            path = f'{path}{sep}project={urllib.parse.quote(self.project, safe="")}'
        result = self.check(['query', path], capture=True)
        data = json.loads(result.stdout)
        if isinstance(data, dict) and 'metadata' in data and 'type' in data:
            return data['metadata']
        return data

    def instances(self) -> list[dict[str, Any]]:
        data = self.query('/1.0/instances?recursion=1')
        if not isinstance(data, list):
            raise IncusError('Unexpected instance-list response')
        return data

    def instance(self, name: str) -> dict[str, Any]:
        escaped = urllib.parse.quote(name, safe='')
        data = self.query(f'/1.0/instances/{escaped}')
        if not isinstance(data, dict):
            raise IncusError('Unexpected instance response')
        return data

    def set_config(self, instance: str, key: str, value: str) -> None:
        self.check(['config', 'set', instance, f'{key}={value}'], capture=True)

    def override_device(self, instance: str, device: str, key: str, value: str) -> None:
        self.check(['config', 'device', 'override', instance, device, f'{key}={value}'], capture=True)
