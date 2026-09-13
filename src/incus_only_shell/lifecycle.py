from __future__ import annotations

from .incus import (
    IncusClient,
    IncusError,
)
from .slots import (
    SlotManager,
    SlotError,
)


class Lifecycle:
    def __init__(
        self,
        client: IncusClient,
        slots: SlotManager,
        nat_device: str,
    ):
        self.client = client
        self.slots = slots
        self.nat_device = nat_device

    def _show_assignment(
        self,
        slot: int,
        ip: str,
    ) -> None:
        print(
            f'Assigned container slot: {slot}',
            flush=True,
        )

        print(
            f'Assigned NAT IPv4:      {ip}',
            flush=True,
        )

        print(
            'Assigned service ports: '
            + ', '.join(
                map(
                    str,
                    self.slots.plan.ports_for_slot(
                        slot
                    ),
                )
            ),
            flush=True,
        )

    def create(
        self,
        mode: str,
        image: str,
        name: str,
    ) -> int:
        start = (
            mode == 'launch'
        )

        with self.slots.locked():
            slot = self.slots.allocate()

            created = False

            try:
                print(
                    f'Creating instance {name!r} '
                    f'from {image!r}...',
                    flush=True,
                )

                #
                # Always create stopped first.
                #
                # Slot metadata and deterministic IP are applied
                # before starting the instance.
                #
                self.client.check(
                    [
                        'init',
                        image,
                        name,
                    ],
                )

                created = True

                print(
                    f'Assigning container slot {slot}...',
                    flush=True,
                )

                ip = self.slots.apply(
                    name,
                    slot,
                    self.nat_device,
                )

            except Exception:
                if created:
                    print(
                        f'Creation failed; removing '
                        f'{name!r}...',
                        flush=True,
                    )

                    #
                    # Do not capture rollback output.
                    # The user should see cleanup failures too.
                    #
                    self.client.run(
                        [
                            'delete',
                            name,
                            '--force',
                        ],
                    )

                raise

            self._show_assignment(
                slot,
                ip,
            )

            if start:
                print(
                    f'Starting instance {name!r}...',
                    flush=True,
                )

                result = self.client.run(
                    [
                        'start',
                        name,
                    ],
                )

                if result.returncode != 0:
                    print(
                        'Instance was created and assigned a slot, '
                        'but failed to start.',
                        flush=True,
                    )

                    return result.returncode

            return 0

    def import_backup(
        self,
        source: str,
        name: str,
    ) -> int:
        with self.slots.locked():
            slot = self.slots.allocate()

            imported = False

            try:
                print(
                    f'Importing backup {source!r} '
                    f'as {name!r}...',
                    flush=True,
                )

                self.client.check(
                    [
                        'import',
                        source,
                        name,
                    ],
                )

                imported = True

                print(
                    f'Assigning container slot {slot}...',
                    flush=True,
                )

                ip = self.slots.apply(
                    name,
                    slot,
                    self.nat_device,
                )

            except Exception:
                if imported:
                    print(
                        f'Import failed; removing '
                        f'{name!r}...',
                        flush=True,
                    )

                    self.client.run(
                        [
                            'delete',
                            name,
                            '--force',
                        ],
                    )

                raise

            self._show_assignment(
                slot,
                ip,
            )

            return 0

    def restore(
        self,
        instance: str,
        snapshot: str,
    ) -> int:
        with self.slots.locked():
            slot = self.slots.slot_of(
                instance
            )

            print(
                f'Restoring snapshot '
                f'{instance!r}/{snapshot!r}...',
                flush=True,
            )

            result = self.client.run(
                [
                    'snapshot',
                    'restore',
                    instance,
                    snapshot,
                ],
            )

            if result.returncode != 0:
                return result.returncode

            #
            # A restored snapshot may contain an older instance
            # configuration. Reassert the platform-owned slot,
            # management ID, and deterministic NAT address.
            #
            print(
                'Reapplying platform-managed '
                'slot and NAT configuration...',
                flush=True,
            )

            ip = self.slots.apply(
                instance,
                slot,
                self.nat_device,
            )

            self._show_assignment(
                slot,
                ip,
            )

            return 0