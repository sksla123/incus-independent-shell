#!/bin/sh

# This script must be executed, not sourced.
# When sourced from Bash, return before enabling `set -e` so the login shell
# is not terminated by a later verification failure.
if [ -n "${BASH_VERSION:-}" ] && [ "${BASH_SOURCE:-}" != "$0" ]; then
    printf '%s\n' 'Run this script with: ./scripts/verify-host.sh' >&2
    return 2
fi

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

BIN='target/release/incus-only-shell'

printf '%s\n' '== Rust environment =='
rustc --version --verbose
cargo --version

if command -v rustup >/dev/null 2>&1; then
    rustup show active-toolchain || true
fi

if cargo fmt --version >/dev/null 2>&1; then
    printf '%s\n' '== formatting =='
    cargo fmt --all -- --check
fi

if cargo clippy --version >/dev/null 2>&1; then
    printf '%s\n' '== clippy =='
    cargo clippy --all-targets -- -D warnings
fi

printf '%s\n' '== tests =='
cargo test --all-targets

printf '%s\n' '== release build =='
cargo build --release

printf '%s\n' '== binary =='
test -f "$BIN"
test -x "$BIN"
ls -lh "$BIN"

printf '%s\n' '== non-interactive smoke tests =='
printf 'pwd\npwd\npwd\n' | "$BIN"
printf 'ls \\\n-d \\\n.\n' | "$BIN"
printf 'pwd && pwd\n' | "$BIN"

printf '%s\n' 'verification completed'
