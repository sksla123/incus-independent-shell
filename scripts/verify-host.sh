#!/bin/sh
set -eu

printf '%s\n' '== Rust environment =='
rustc --version --verbose
cargo --version
if command -v rustup >/dev/null 2>&1; then
    rustup show active-toolchain || true
fi

if cargo fmt --version >/dev/null 2>&1; then
    printf '%s\n' '== formatting =='
    cargo fmt -- --check
else
    printf '%s\n' '== formatting: skipped (rustfmt not installed) =='
fi

if cargo clippy --version >/dev/null 2>&1; then
    printf '%s\n' '== clippy =='
    cargo clippy --all-targets -- -D warnings
else
    printf '%s\n' '== clippy: skipped (clippy not installed) =='
fi

printf '%s\n' '== tests =='
cargo test --all-targets

printf '%s\n' '== release build =='
cargo build --release

printf '%s\n' '== binary =='
file target/release/incus-only-shell
ls -lh target/release/incus-only-shell

printf '%s\n' '== non-interactive smoke tests =='
printf 'pwd\npwd\npwd\n' | target/release/incus-only-shell
printf 'ls \\\n-d \\\n.\n' | target/release/incus-only-shell
printf 'pwd && pwd\n' | target/release/incus-only-shell

printf '%s\n' 'verification completed'
