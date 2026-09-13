from __future__ import annotations

import shlex


def split_commands(text: str) -> list[str]:
    commands: list[str] = []
    current: list[str] = []
    single_quote = False
    double_quote = False
    escaped = False

    for ch in text:
        if escaped:
            current.append(ch)
            escaped = False
            continue

        if ch == '\\':
            current.append(ch)
            if not single_quote:
                escaped = True
            continue

        if ch == "'" and not double_quote:
            single_quote = not single_quote
            current.append(ch)
            continue

        if ch == '"' and not single_quote:
            double_quote = not double_quote
            current.append(ch)
            continue

        if ch == '\n' and not single_quote and not double_quote:
            command = ''.join(current).strip()
            if command:
                commands.append(command)
            current = []
            continue

        current.append(ch)

    command = ''.join(current).strip()
    if command:
        commands.append(command)
    return commands


def parse_command(command: str) -> list[str]:
    return shlex.split(command, comments=False, posix=True)
