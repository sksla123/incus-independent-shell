# incus-only-shell v2.1

Modular restricted login shell for a multi-user Incus host.

## v2.1 fixes

- Restores the original shell routing contract: `shell.py` handles `help`, `clear`, `exit`, `logout`, host observability commands, SFTP, and strips the leading `incus` token before `CommandDispatcher`.
- Uses host UID as the primary key in `users.toml`.
- Validates UID-to-username consistency and management-ID uniqueness.
- Keeps compatibility aliases for the temporary `*_MESSAGE` policy names and `IncusOnlyShell` class name.
- Adds regression tests for `incus list` routing and `exit/logout` handling.
- Installer replaces the installed Python package directory to avoid stale modules from older revisions.

## Policy model

- Per-user confined project: `user-<unix uid>`
- Management ID: independent `000..999`, stored in root-owned `users.toml`
- Maximum containers: 3 (also enforce with Incus `limits.containers=3`)
- Container slots: `a = 1..3`
- Service slots: `b = 0..9`
- NAT IPv4: `10.100.ax.xx` where `xxx` is the management ID
- Host port: decimal `abxxx`
- Slot metadata is stored as `user.incus-only.slot` on the instance
- Slot is released implicitly when the instance is deleted

## users.toml

```toml
[users."1001"]
username = "incus-test"
management_id = 123
```

Unix/NAS UID identifies the account and idmap. `management_id` independently controls deterministic IP/port allocation.

## Source layout

- `config.py`: site config and UID-keyed management-ID loading
- `addressing.py`: deterministic IP/port formula
- `incus.py`: Incus CLI/API wrapper
- `slots.py`: slot inventory/allocation and metadata
- `lifecycle.py`: transactional create/import/restore handling
- `commands.py`: Incus command whitelist (receives argv without leading `incus`)
- `hostcmd.py`: read-only host commands
- `parser.py`: safe multiline parser
- `shell.py`: interactive/SSH/SFTP shell and top-level command routing

The Python source contains no hard-coded host bridge name or host address. Site-specific values belong in `/etc/incus-only-shell/config.toml`.
