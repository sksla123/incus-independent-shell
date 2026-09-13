from __future__ import annotations

import ipaddress
import os
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
class UserConfig:
    username: str
    uid: int
    management_id: int


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
    p = pwd.getpwuid(os.getuid())
    return Identity(
        username=p.pw_name,
        uid=p.pw_uid,
        gid=p.pw_gid,
        home=p.pw_dir,
    )


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

    parts = cfg.ipv4_prefix.split('.')
    if len(parts) != 2:
        raise ValueError('ipv4_prefix must contain exactly two IPv4 octets')
    try:
        octets = [int(part) for part in parts]
    except ValueError as exc:
        raise ValueError('ipv4_prefix must contain numeric IPv4 octets') from exc
    if any(not (0 <= octet <= 255) for octet in octets):
        raise ValueError('ipv4_prefix octets must be in 0..255')

    probe = ipaddress.ip_address(f'{cfg.ipv4_prefix}.0.1')
    if not isinstance(probe, ipaddress.IPv4Address):
        raise ValueError('ipv4_prefix must be the first two octets of an IPv4 address')

    return cfg


def load_user_config(identity: Identity, users_file: Path) -> UserConfig:
    with users_file.open('rb') as f:
        data = tomllib.load(f)

    users = data.get('users', {})
    if not isinstance(users, dict):
        raise ValueError('[users] must be a TOML table')

    seen_usernames: dict[str, str] = {}
    seen_management_ids: dict[int, str] = {}

    for uid_key, entry in users.items():
        try:
            uid = int(uid_key)
        except (TypeError, ValueError) as exc:
            raise ValueError(f'Invalid host UID key {uid_key!r}') from exc
        if uid < 0:
            raise ValueError(f'Host UID must not be negative: {uid}')
        if not isinstance(entry, dict):
            raise ValueError(f'User configuration for UID {uid} must be a TOML table')

        username = str(entry.get('username', '')).strip()
        if not username:
            raise ValueError(f'username is missing for host UID {uid}')

        try:
            management_id = int(entry['management_id'])
        except KeyError as exc:
            raise ValueError(f'management_id is missing for host UID {uid}') from exc
        except (TypeError, ValueError) as exc:
            raise ValueError(f'management_id for host UID {uid} must be an integer') from exc

        if not (0 <= management_id <= 999):
            raise ValueError(f'Management ID for host UID {uid} must be in 000..999')

        previous_uid = seen_usernames.get(username)
        if previous_uid is not None and previous_uid != uid_key:
            raise ValueError(
                f'Duplicate username {username!r}: UID {previous_uid} and UID {uid_key}'
            )

        previous_uid = seen_management_ids.get(management_id)
        if previous_uid is not None and previous_uid != uid_key:
            raise ValueError(
                f'Duplicate management ID {management_id:03d}: '
                f'UID {previous_uid} and UID {uid_key}'
            )

        seen_usernames[username] = uid_key
        seen_management_ids[management_id] = uid_key

    uid_key = str(identity.uid)
    if uid_key not in users:
        raise KeyError(f'No user configuration for host UID {identity.uid}')

    entry = users[uid_key]
    configured_username = str(entry['username']).strip()
    if configured_username != identity.username:
        raise ValueError(
            f'Host UID {identity.uid} resolves to {identity.username!r}, '
            f'but users.toml specifies {configured_username!r}'
        )

    management_id = int(entry['management_id'])
    return UserConfig(
        username=identity.username,
        uid=identity.uid,
        management_id=management_id,
    )
