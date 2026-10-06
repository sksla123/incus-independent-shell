#!/bin/sh

set -eu

if [ "$#" -gt 1 ]; then
    printf '%s\n' 'Usage: install-config.sh [config-directory]' >&2
    exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
DEST_DIR=${1:-/etc/incus-only-shell}

install -d -m 0755 "$DEST_DIR"

for name in banner.txt allowed-commands.txt; do
    destination="$DEST_DIR/$name"
    if [ -e "$destination" ] || [ -L "$destination" ]; then
        printf 'Keeping existing %s\n' "$destination"
    else
        install -m 0644 "$REPO_ROOT/config/$name" "$destination"
        printf 'Installed %s\n' "$destination"
    fi
done
