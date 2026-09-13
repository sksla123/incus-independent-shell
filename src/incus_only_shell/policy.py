from __future__ import annotations


HOST_DENIED_MESSAGE = """This command is not permitted on the host.
This is an Incus-only shell.
Run general workloads and shell commands inside an Incus container."""

INCUS_ADMIN_DENIED_MESSAGE = """This Incus operation is administrator-only.
Please contact the host administrator if you need this configuration changed."""

INCUS_POLICY_DENIED_MESSAGE = """This Incus operation is not permitted by the user policy.
Please contact the host administrator if you need this operation."""


def valid_name(value: str) -> bool:
    if not value or value.startswith('-'):
        return False

    allowed = set(
        'abcdefghijklmnopqrstuvwxyz'
        'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
        '0123456789._-'
    )

    return all(
        ch in allowed
        for ch in value
    )


def valid_image(value: str) -> bool:
    if not value or value.startswith('-'):
        return False

    forbidden = set(
        " \t\r\n;&|`$><(){}[]\\'\""
    )

    return not any(
        ch in forbidden
        for ch in value
    )