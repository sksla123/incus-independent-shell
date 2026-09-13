# incus-only-shell v2

Modular restricted login shell for a multi-user Incus host.

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

## Source layout

- `config.py`: site config and management-ID loading
- `addressing.py`: deterministic IP/port formula
- `incus.py`: Incus CLI/API wrapper
- `slots.py`: slot inventory/allocation and metadata
- `lifecycle.py`: transactional create/import/restore handling
- `commands.py`: Incus command whitelist
- `hostcmd.py`: read-only host commands
- `parser.py`: safe multiline parser
- `shell.py`: interactive/SSH/SFTP shell

The Python source contains no hard-coded host bridge name or host address. Site-specific values belong in `/etc/incus-only-shell/config.toml`.
