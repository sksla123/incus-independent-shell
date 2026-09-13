from __future__ import annotations

import os
import shutil
import subprocess


def run_host(argv: list[str], home: str) -> int | None:
    allowed = {
        ('ps',): ['/usr/bin/ps'],
        ('ps', '-ef'): ['/usr/bin/ps', '-ef'],
        ('ps', 'aux'): ['/usr/bin/ps', 'aux'],
    }
    key = tuple(argv)
    if key in allowed:
        return subprocess.run(allowed[key], cwd=home, shell=False).returncode

    if argv in (['top'], ['nvidia-smi'], ['gpu-stat']):
        binary = shutil.which(argv[0], path='/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin')
        if binary is None:
            print(f'{argv[0]} is not installed.')
            return 127
        return subprocess.run([binary], cwd=home, shell=False).returncode

    return None
