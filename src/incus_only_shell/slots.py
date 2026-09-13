from __future__ import annotations

import fcntl
from contextlib import contextmanager
from pathlib import Path

from .addressing import AddressPlan
from .incus import IncusClient, IncusError


class SlotError(RuntimeError):
    pass


class SlotManager:
    def __init__(self, client: IncusClient, plan: AddressPlan, slot_key: str, management_key: str, home: str):
        self.client = client
        self.plan = plan
        self.slot_key = slot_key
        self.management_key = management_key
        self.lock_path = Path(home) / '.incus-only-shell-slot.lock'

    @contextmanager
    def locked(self):
        self.lock_path.touch(mode=0o600, exist_ok=True)
        with self.lock_path.open('r+') as f:
            fcntl.flock(f.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(f.fileno(), fcntl.LOCK_UN)

    def inventory(self) -> tuple[dict[int, str], list[str]]:
        used: dict[int, str] = {}
        unassigned: list[str] = []

        for inst in self.client.instances():
            name = str(inst.get('name', ''))
            if inst.get('type') not in ('container', None):
                continue
            config = inst.get('config') or {}
            raw = config.get(self.slot_key)
            if raw in (None, ''):
                unassigned.append(name)
                continue
            try:
                slot = int(raw)
            except (TypeError, ValueError) as exc:
                raise SlotError(f'Instance {name!r} has invalid slot metadata {raw!r}') from exc
            self.plan.validate_slot(slot)
            if slot in used:
                raise SlotError(f'Duplicate container slot {slot}: {used[slot]!r} and {name!r}')
            used[slot] = name

        return used, unassigned

    def allocate(self) -> int:
        used, unassigned = self.inventory()
        if unassigned:
            names = ', '.join(sorted(unassigned))
            raise SlotError(f'Existing instance(s) have no slot metadata: {names}. Ask the administrator to migrate them first.')

        for slot in range(1, self.plan.max_containers + 1):
            if slot not in used:
                return slot
        raise SlotError(f'Container limit reached: all {self.plan.max_containers} slots are in use')

    def slot_of(self, instance: str) -> int:
        data = self.client.instance(instance)
        config = data.get('config') or {}
        raw = config.get(self.slot_key)
        if raw in (None, ''):
            raise SlotError(f'Instance {instance!r} has no slot metadata')
        slot = int(raw)
        self.plan.validate_slot(slot)
        return slot

    def apply(self, instance: str, slot: int, nat_device: str) -> str:
        self.plan.validate_slot(slot)
        ip = self.plan.ipv4_for_slot(slot)
        self.client.set_config(instance, self.slot_key, str(slot))
        self.client.set_config(instance, self.management_key, f'{self.plan.management_id:03d}')
        self.client.override_device(instance, nat_device, 'ipv4.address', ip)
        return ip
