from __future__ import annotations

import ipaddress
from dataclasses import dataclass


@dataclass(frozen=True)
class AddressPlan:
    management_id: int
    max_containers: int
    service_slots: int
    ipv4_prefix: str

    def validate_slot(self, slot: int) -> None:
        if not (1 <= slot <= self.max_containers):
            raise ValueError(f'container slot must be in 1..{self.max_containers}')

    def validate_service(self, service: int) -> None:
        if not (0 <= service < self.service_slots):
            raise ValueError(f'service slot must be in 0..{self.service_slots - 1}')

    def ipv4_for_slot(self, slot: int) -> str:
        self.validate_slot(slot)
        hundreds = self.management_id // 100
        tail = self.management_id % 100
        third = slot * 10 + hundreds
        addr = ipaddress.ip_address(f'{self.ipv4_prefix}.{third}.{tail}')
        if not isinstance(addr, ipaddress.IPv4Address):
            raise ValueError('computed address is not IPv4')
        return str(addr)

    def port_for(self, slot: int, service: int) -> int:
        self.validate_slot(slot)
        self.validate_service(service)
        port = slot * 10000 + service * 1000 + self.management_id
        if not (1 <= port <= 65535):
            raise ValueError('computed port is outside TCP/UDP port range')
        return port

    def ports_for_slot(self, slot: int) -> list[int]:
        return [self.port_for(slot, service) for service in range(self.service_slots)]
