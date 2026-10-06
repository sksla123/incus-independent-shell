# incus-only-shell v0.5.0 (Rust)

Restricted host shell for Incus users.

## Rust compatibility policy

This project does **not** pin a Rust compiler version.

There is intentionally no `rust-toolchain.toml`, and `Cargo.toml` does not contain a `rust-version` field.
The project is intended to build with a normal current Rust stable toolchain.

The Rust edition remains `2021`. Rust edition and compiler release are separate concepts; edition 2021 code is supported by current stable Rust compilers.

Dependency versions are constrained only by API-compatible major versions:

- `rustyline = "18"`
- `shlex = "2"`

This allows Cargo to select the current compatible release in each major series instead of pinning one exact patch release.

## Source layout

- `src/main.rs`
  - Entry point only.

- `src/config.rs`
  - Loads the banner and external command list from `/etc/incus-only-shell` once at startup.
  - Embeds `config/banner.txt` and `config/allowed-commands.txt` as defaults.

- `src/input.rs`
  - `rustyline` multiline validator.
  - Enter continues editing only while a quote is open or a trailing backslash requires continuation.
  - Bracketed paste is handled by `rustyline`.

- `src/parser.rs`
  - Top-level command boundaries.
  - Normal newline outside quotes = separate command.
  - `\\` + newline = continuation of the same command.
  - Newline inside `'...'` or `"..."` remains inside that argument.
  - One top-level pipeline per command is allowed when both external commands are permitted.
  - `&&` conditionally runs the next command only when the previous command succeeds.
  - A trailing `&&` continues input onto the next line.
  - Shell-style word splitting is delegated to `shlex`.

- `src/policy.rs`
  - Configurable external command allowlist.
  - `bash` is always permitted by the code, independently of the external list.
  - Allowed commands receive all arguments unchanged.

- `src/executor.rs`
  - Always-available built-ins: `cd`, `history`, `help`, `shell-version`, `exit`, `logout`.
  - Executes commands directly with `std::process::Command`.
  - Independent commands are never combined into an implicit `sh -c` or `bash -c` invocation.
  - `&&` chains are evaluated by the wrapper itself using child-process exit codes.
  - Each command in an `&&` chain is classified and executed separately.
  - A shell is invoked only when the user explicitly requests `bash`, or when a command such as `incus exec ... -- sh -c ...` is passed to Incus.

- `src/shell.rs`
  - Interactive/non-interactive session handling.
  - Readline/history.

- `src/ui.rs`
  - Banner, help text, prompt, deny message.
  - Help displays the active external command list and the always-available built-ins.

- `config/banner.txt`, `config/allowed-commands.txt`
  - Editable default configuration, also included in the binary at build time.

- `scripts/install-config.sh`
  - Installs missing default configuration files and preserves existing files.

- `tests/behavior.rs`
  - End-to-end non-interactive behavior tests against the compiled binary.
  - Covers multiline execution order, backslash continuation, quoted newlines, pipelines, `&&` success and failure, and combinations of these forms.

## Parsing contract

### Independent commands

Input:

```text
pwd
pwd
pwd
```

Execution:

```text
pwd -> wait until exit
pwd -> wait until exit
pwd -> wait until exit
prompt
```

Independent commands are never wrapped into one implicit shell command.

### Backslash continuation

```bash
incus launch \\
  images:debian/13/cloud \\
  c1
```

is one command.

### Quoted newline

```bash
incus exec c1 -- sh -c '
echo hello
id
ip route
'
```

The newlines inside the quoted argument are preserved and passed as one argument to Incus.

### Pipeline

```bash
incus list | grep c1
```

creates two processes directly and connects stdout to stdin.
It is not converted to `sh -c 'incus list | grep c1'`.

### Conditional AND

```bash
incus stop c1 && incus start c1
```

The wrapper executes `incus stop c1` first. `incus start c1` runs only when the first command exits with status `0`. Each command is parsed, policy-checked, and executed separately.

A trailing `&&` continues onto the next input line:

```bash
incus stop c1 &&
incus start c1
```

## Rust environment

If the host already has a current stable `rustc` and `cargo`, nothing special is required:

```bash
rustc --version
cargo --version
```

When Rust is managed with `rustup`, update and select the stable channel without specifying a numeric compiler version:

```bash
rustup update stable
rustup default stable
rustup component add rustfmt clippy   # optional: formatting/lint checks
```

The project itself does not select or override the host toolchain.

## Verify everything

```bash
./scripts/verify-host.sh
```

This runs:

```text
rustc/cargo environment display
cargo fmt -- --check                  # when rustfmt is installed
cargo clippy --all-targets -- -D warnings # when clippy is installed
cargo test --all-targets
cargo build --release
non-interactive smoke tests
```

## Build only

```bash
cargo build --release
```

Binary:

```text
target/release/incus-only-shell
```

## Install

```bash
sudo install \\
  -o root \\
  -g root \\
  -m 0755 \\
  target/release/incus-only-shell \\
  /usr/local/sbin/incus-only-shell
```

Existing users whose login shell is `/usr/local/sbin/incus-only-shell` use the new binary on their next login.

Install the editable default configuration:

```bash
sudo ./scripts/install-config.sh
```

The script preserves existing files, so upgrading the binary and running the script again keeps local configuration changes.
To prepare files in another directory, pass its path as the script's only argument.

## Configuration

| File | Behavior |
| --- | --- |
| `/etc/incus-only-shell/banner.txt` | Replaces the main banner. Missing file uses the embedded default; an empty file hides the main banner. |
| `/etc/incus-only-shell/allowed-commands.txt` | Replaces the configurable external command list. Missing file uses the embedded default; an empty or comment-only file permits built-ins and `bash`. |
| `/etc/incus-only-shell/banner-before.txt` | Optional text printed before the main banner. |
| `/etc/incus-only-shell/banner-after.txt` | Optional text printed after the main banner. |

Edit the banner as plain UTF-8 text:

```bash
sudoedit /etc/incus-only-shell/banner.txt
```

Edit the external command list:

```bash
sudoedit /etc/incus-only-shell/allowed-commands.txt
```

Write one executable name per line, without a path or arguments. Blank lines, duplicate names, and `#` comments are supported:

```text
# This is the complete list for this example.
incus
ls
pwd
grep
cp # Allow cp with any arguments.
```

The supplied default contains the existing configurable external command names, including `ip`, `hostname`, `resolvectl`, `nvidia-smi`, `top`, and `htop`.
Every listed command runs with the arguments entered by the user. For example, allowing `ip` also allows `ip link set ...`; `top` and `htop` run with the user's options.
Executables are resolved through the shell's system `PATH`.

`cd`, `history`, `help`, `shell-version`, `exit`, and `logout` are shell built-ins and remain available regardless of the external list. Listing them in the file has no effect.
`bash` is also always allowed by the code. The external file cannot enable or disable it, and listing `bash` in that file has no effect. Its arguments are passed through unchanged.
`cp` is an external command: it is absent from the supplied default and can be enabled by adding `cp` to the file.

Settings are read once when the shell starts. Log in again or start a new shell after editing a file; rebuilding the binary is unnecessary.
The same command policy applies to interactive sessions, stdin input, `-c`, `&&` chains, and both sides of a pipeline. Either pipeline command can be `bash` or a name in the external list.
`help` displays the configured external command list, always-available built-ins, and `bash`.
Its final note explains that working directly on the host is not recommended, but users can use `bash` if they want to work there.

If an existing command list cannot be read or contains an invalid entry, startup reports the file and error and exits with status `126`.
An unreadable banner reports an error and uses the embedded banner.

## Manual interactive checks

Paste these three lines together:

```text
pwd
pwd
pwd
```

Expected behavior: all three commands execute in order and only then does the next `[incus-shell]` prompt appear.

Continuation:

```bash
incus launch \\
  images:debian/13/cloud \\
  c1
```

Expected behavior: one `incus launch` invocation.

Quoted multiline argument:

```bash
incus exec c1 -- sh -c '
echo hello
id
ip route
'
```

Expected behavior: one `incus exec` invocation whose `sh -c` argument contains the embedded newlines.

## GPU PCI address

```bash
lspci -Dnnk | grep -EA3 'VGA|3D|Display'
```

Example PCI address:

```text
0000:01:00.0
```
