#!/bin/sh
set -eu

install -d -o root -g root -m 755 /usr/local/lib/incus-only-shell
rm -rf /usr/local/lib/incus-only-shell/incus_only_shell
cp -R src/incus_only_shell /usr/local/lib/incus-only-shell/
chown -R root:root /usr/local/lib/incus-only-shell/incus_only_shell
find /usr/local/lib/incus-only-shell/incus_only_shell -type d -exec chmod 755 {} \;
find /usr/local/lib/incus-only-shell/incus_only_shell -type f -exec chmod 644 {} \;

install -o root -g root -m 755 bin/incus-only-shell /usr/local/sbin/incus-only-shell

install -d -o root -g root -m 755 /etc/incus-only-shell
if [ ! -e /etc/incus-only-shell/config.toml ]; then
    install -o root -g root -m 644 etc/config.toml /etc/incus-only-shell/config.toml
fi
if [ ! -e /etc/incus-only-shell/users.toml ]; then
    install -o root -g root -m 644 etc/users.toml /etc/incus-only-shell/users.toml
fi

/usr/bin/python3 -m compileall -q /usr/local/lib/incus-only-shell/incus_only_shell
/usr/bin/python3 -m py_compile /usr/local/sbin/incus-only-shell

echo 'Installed incus-only-shell v2.1.'
