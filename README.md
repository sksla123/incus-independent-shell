# incus-only-shell v4.2 (Rust)

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

- `src/input.rs`
  - `rustyline` multiline validator.
  - Enter continues editing only while a quote is open or a trailing backslash requires continuation.
  - Bracketed paste is handled by `rustyline`.

- `src/parser.rs`
  - Top-level command boundaries.
  - Normal newline outside quotes = separate command.
  - `\\` + newline = continuation of the same command.
  - Newline inside `'...'` or `"..."` remains inside that argument.
  - One top-level pipeline per command is allowed only when the final command is `grep`.
  - `&&` conditionally runs the next command only when the previous command succeeds.
  - A trailing `&&` continues input onto the next line.
  - Shell-style word splitting is delegated to `shlex`.

- `src/policy.rs`
  - Host allowlist and argument validation for commands that need additional restrictions.
  - `lspci` is allowed so users can identify GPU PCI addresses.

- `src/executor.rs`
  - Built-ins: `cd`, `history`, `help`, `exit`.
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

- `tests/behavior.rs`
  - End-to-end non-interactive behavior tests against the compiled binary.

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
