from __future__ import annotations

from pathlib import Path

from .incus import IncusClient, IncusError
from .slots import SlotManager, SlotError


class Lifecycle:
    def __init__(self, client: IncusClient, slots: SlotManager, nat_device: str):
        self.client = client
        self.slots = slots
        self.nat_device = nat_device

    def create(self, mode: str, image: str, name: str) -> int:
        start = mode == 'launch'
        with self.slots.locked():
            slot = self.slots.allocate()
            created = False
            try:
                self.client.check(['init', image, name], capture=True)
                created = True
                ip = self.slots.apply(name, slot, self.nat_device)
            except Exception:
                if created:
                    self.client.run(['delete', name, '--force'], capture=True)
                raise

            print(f'Assigned container slot: {slot}')
            print(f'Assigned NAT IPv4:      {ip}')
            print('Assigned service ports: ' + ', '.join(map(str, self.slots.plan.ports_for_slot(slot))))

            if start:
                result = self.client.run(['start', name])
                if result.returncode != 0:
                    print('Instance was created and assigned a slot, but failed to start.', flush=True)
                    return result.returncode
            return 0

    def import_backup(self, source: str, name: str) -> int:
        with self.slots.locked():
            slot = self.slots.allocate()
            imported = False
            try:
                self.client.check(['import', source, name], capture=True)
                imported = True
                ip = self.slots.apply(name, slot, self.nat_device)
            except Exception:
                if imported:
                    self.client.run(['delete', name, '--force'], capture=True)
                raise

            print(f'Assigned container slot: {slot}')
            print(f'Assigned NAT IPv4:      {ip}')
            print('Assigned service ports: ' + ', '.join(map(str, self.slots.plan.ports_for_slot(slot))))
            return 0

    def restore(self, instance: str, snapshot: str) -> int:
        with self.slots.locked():
            slot = self.slots.slot_of(instance)
            result = self.client.run(['snapshot', 'restore', instance, snapshot])
            if result.returncode != 0:
                return result.returncode
            # A snapshot can carry older instance config. Reassert platform-owned metadata/IP.
            self.slots.apply(instance, slot, self.nat_device)
            return 0
