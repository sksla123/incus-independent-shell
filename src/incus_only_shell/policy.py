from __future__ import annotations

HOST_DENIED = """This command is not permitted on the host.
This is an Incus-only shell.
Run general workloads and shell commands inside an Incus container."""

INCUS_ADMIN_DENIED = """This Incus operation is administrator-only.
Please contact the host administrator if you need this configuration changed."""

INCUS_POLICY_DENIED = """This Incus operation is not permitted by the user policy.
Please contact the host administrator if you need this operation."""

# Compatibility aliases for code that used the later *_MESSAGE names.
HOST_DENIED_MESSAGE = HOST_DENIED
INCUS_ADMIN_DENIED_MESSAGE = INCUS_ADMIN_DENIED
INCUS_POLICY_DENIED_MESSAGE = INCUS_POLICY_DENIED


def valid_name(value: str) -> bool:
    if not value or value.startswith('-'):
        return False
    allowed = set('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-')
    return all(ch in allowed for ch in value)


def valid_image(value: str) -> bool:
    if not value or value.startswith('-'):
        return False
    forbidden = set(" \t\r\n;&|`$><(){}[]\\'\"")
    return not any(ch in forbidden for ch in value)
