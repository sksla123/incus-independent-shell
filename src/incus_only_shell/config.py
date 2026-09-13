from __future__ import annotations

import ipaddress
import pwd
import tomllib
from dataclasses import dataclass
from pathlib import Path

DEFAULT_CONFIG = Path('/etc/incus-only-shell/config.toml')
DEFAULT_USERS = Path('/etc/incus-only-shell/users.toml')


@dataclass(frozen=True)
class Identity:
    username: str
    uid: int
    gid: int
    home: str


@dataclass(frozen=True)
class PlatformConfig:
    project_prefix: str
    max_containers: int
    slot_key: str
    management_key: str
    nat_device: str
    ipv4_prefix: str
    service_slots: int
    users_file: Path
    incus_conf: Path


def current_identity() -> Identity:
    p = pwd.getpwuid(__import__('os').getuid())
    return Identity(p.pw_name, p.pw_uid, p.pw_gid, p.pw_dir)


def load_platform(path: Path = DEFAULT_CONFIG) -> PlatformConfig:
    with path.open('rb') as f:
        data = tomllib.load(f)

    platform = data.get('platform', {})
    network = data.get('network', {})

    cfg = PlatformConfig(
        project_prefix=str(platform.get('project_prefix', 'user-')),
        max_containers=int(platform.get('max_containers', 3)),
        slot_key=str(platform.get('slot_key', 'user.incus-only.slot')),
        management_key=str(platform.get('management_key', 'user.incus-only.management_id')),
        nat_device=str(network.get('nat_device', 'eth0')),
        ipv4_prefix=str(network.get('ipv4_prefix', '10.100')),
        service_slots=int(network.get('service_slots', 10)),
        users_file=Path(platform.get('users_file', str(DEFAULT_USERS))),
        incus_conf=Path(platform.get('incus_conf', '/etc/incus-only-shell/client')),
    )

    if not (1 <= cfg.max_containers <= 9):
        raise ValueError('max_containers must be in 1..9')
    if not (1 <= cfg.service_slots <= 10):
        raise ValueError('service_slots must be in 1..10')

    # Validate the two-octet prefix without baking a specific network into code.
    probe = ipaddress.ip_address(f'{cfg.ipv4_prefix}.0.1')
    if not isinstance(probe, ipaddress.IPv4Address):
        raise ValueError('ipv4_prefix must be the first two octets of an IPv4 address')

    return cfg


def load_management_id(username: str, users_file: Path) -> int:
    with users_file.open('rb') as f:
        data = tomllib.load(f)

    users = data.get('users', {})
    if username not in users:
        raise KeyError(f'No management ID configured for user {username!r}')

    value = int(users[username])
    if not (0 <= value <= 999):
        raise ValueError(f'Management ID for {username!r} must be in 000..999')
    return value
