#!/usr/bin/env bash
# Install or upgrade quack-control on the duck, over ssh.
#
# Usage: scripts/install-on-duck.sh <user@host> [binary]
#   binary defaults to target/aarch64-unknown-linux-gnu/release/quack-control
#   (scripts/cross-build.sh). Extra ssh/scp options, e.g. a port:
#   SSH_OPTS="-p 2222" scripts/install-on-duck.sh microduck@192.168.1.42
#
# Idempotent, and it replaces only what it owns: the binary
# (/usr/local/bin/quack-control), the unit (/etc/systemd/system/
# quack-control.service) and the service account (/etc/sysusers.d/
# quack-control.conf, user `quackctl`). The config
# (/etc/robot/quack-control.toml) is installed only when there is none, and
# so is the token's file (/etc/robot/quack-control.env, root, mode 600,
# QC_TOKEN commented out). Then the service is enabled and (re)started and
# the page's address printed. The user needs sudo on the duck; quack-nav
# (quack-navd and the `robot` group) should be installed first.
set -euo pipefail

HOST="${1:?usage: install-on-duck.sh <user@host> [binary]}"
cd "$(dirname "$0")/.."
BIN="${2:-target/aarch64-unknown-linux-gnu/release/quack-control}"
read -ra OPTS <<< "${SSH_OPTS:-}"

[ -f "$BIN" ] || { echo "missing $BIN — run scripts/cross-build.sh first" >&2; exit 1; }
if command -v file >/dev/null && ! file "$BIN" | grep -q 'ARM aarch64'; then
    echo "$BIN is not an aarch64 binary: $(file -b "$BIN")" >&2
    exit 1
fi

SCP_OPTS=()
for ((i = 0; i < ${#OPTS[@]}; i++)); do
    # scp spells ssh's -p (port) as -P.
    if [ "${OPTS[i]}" = "-p" ]; then SCP_OPTS+=("-P"); else SCP_OPTS+=("${OPTS[i]}"); fi
done

STAGE=$(ssh "${OPTS[@]}" "$HOST" 'mktemp -d /tmp/quack-control-install.XXXXXX')
echo "copying to $HOST:$STAGE"
scp "${SCP_OPTS[@]}" -q "$BIN" "$HOST:$STAGE/quack-control"
scp "${SCP_OPTS[@]}" -q systemd/quack-control.service "$HOST:$STAGE/quack-control.service"
scp "${SCP_OPTS[@]}" -q systemd/sysusers.d/quack-control.conf "$HOST:$STAGE/sysusers.conf"
scp "${SCP_OPTS[@]}" -q quack-control.example.toml "$HOST:$STAGE/quack-control.toml"

# shellcheck disable=SC2087 # $STAGE is expanded here on purpose
ssh "${OPTS[@]}" "$HOST" bash -s <<REMOTE
set -euo pipefail
cd "$STAGE"
run() { echo "+ \$*"; sudo "\$@"; }

run install -m 755 quack-control /usr/local/bin/quack-control
run install -m 644 quack-control.service /etc/systemd/system/quack-control.service
run install -D -m 644 sysusers.conf /etc/sysusers.d/quack-control.conf
run systemd-sysusers /etc/sysusers.d/quack-control.conf
getent group robot >/dev/null \
    || echo "warning: no 'robot' group: quack-navd's sockets are not provisioned here, and the unit's SupplementaryGroups=robot will fail"

if [ -f /etc/robot/quack-control.toml ]; then
    echo "keeping /etc/robot/quack-control.toml"
else
    run install -D -m 644 quack-control.toml /etc/robot/quack-control.toml
fi
if ! sudo test -f /etc/robot/quack-control.env; then
    printf '# QC_TOKEN=a-long-random-string\n' > env
    run install -D -m 600 -o root -g root env /etc/robot/quack-control.env
fi

run systemctl daemon-reload
run systemctl enable quack-control
run systemctl restart quack-control
sleep 2
sudo systemctl --no-pager --lines=5 status quack-control || true
rm -rf "$STAGE"
REMOTE

echo "installed on $HOST — the page: http://${HOST#*@}:8080/ (journal: ssh $HOST journalctl -u quack-control -f)"
