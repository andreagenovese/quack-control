#!/usr/bin/env bash
# Install or upgrade quack-control on the duck, over ssh.
#
# Usage: install-on-duck.sh [--dry-run] <user@host> [binary]
#
# It runs from either of two places, and finds its files by its own:
#   - an unpacked release package (quack-control-<version>/, see
#     scripts/package.sh): bin/quack-control sits next to this script, and
#     so do systemd/ and quack-control.example.toml;
#   - a checkout of the repository (scripts/install-on-duck.sh): the binary
#     defaults to target/aarch64-unknown-linux-gnu/release/quack-control
#     (scripts/cross-build.sh), the rest is systemd/ and
#     quack-control.example.toml.
# A [binary] given on the command line wins over both.
#
# --dry-run prints every command it would run — the ssh and scp lines and
# the script it would run on the duck — and connects to nothing.
# Extra ssh/scp options, e.g. a port:
#   SSH_OPTS="-p 2222" ./install-on-duck.sh microduck@192.168.1.42
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

USAGE="usage: install-on-duck.sh [--dry-run] <user@host> [binary]"
DRY_RUN=0
if [ "${1:-}" = "--dry-run" ]; then DRY_RUN=1; shift; fi
HOST="${1:?$USAGE}"

HERE="$(cd "$(dirname "$0")" && pwd)"
if [ -f "$HERE/bin/quack-control" ]; then
    # The release package's layout.
    DEFAULT_BIN="$HERE/bin/quack-control"
    UNIT="$HERE/systemd/quack-control.service"
    SYSUSERS="$HERE/systemd/sysusers.d/quack-control.conf"
    CONFIG="$HERE/quack-control.example.toml"
    BUILD_HINT="the package is incomplete: unpack it again"
else
    # The repository's layout (this script in scripts/).
    ROOT="$(cd "$HERE/.." && pwd)"
    DEFAULT_BIN="$ROOT/target/aarch64-unknown-linux-gnu/release/quack-control"
    UNIT="$ROOT/systemd/quack-control.service"
    SYSUSERS="$ROOT/systemd/sysusers.d/quack-control.conf"
    CONFIG="$ROOT/quack-control.example.toml"
    BUILD_HINT="run scripts/cross-build.sh first"
fi
BIN="${2:-$DEFAULT_BIN}"
# ${A[@]+"${A[@]}"} below: macOS bash 3.2 calls an empty array unbound under set -u.
read -ra OPTS <<< "${SSH_OPTS:-}"

[ -f "$BIN" ] || { echo "missing $BIN — $BUILD_HINT" >&2; exit 1; }
for f in "$UNIT" "$SYSUSERS" "$CONFIG"; do
    [ -f "$f" ] || { echo "missing $f — $BUILD_HINT" >&2; exit 1; }
done
if command -v file >/dev/null && ! file "$BIN" | grep -q 'ARM aarch64'; then
    echo "$BIN is not an aarch64 binary: $(file -b "$BIN")" >&2
    exit 1
fi

SCP_OPTS=()
for ((i = 0; i < ${#OPTS[@]}; i++)); do
    # scp spells ssh's -p (port) as -P.
    if [ "${OPTS[i]}" = "-p" ]; then SCP_OPTS+=("-P"); else SCP_OPTS+=("${OPTS[i]}"); fi
done

# Run a local command, or with --dry-run only print it.
step() {
    if [ "$DRY_RUN" = 1 ]; then
        printf '+'; printf ' %q' "$@"; printf '\n'
    else
        "$@"
    fi
}

if [ "$DRY_RUN" = 1 ]; then
    echo "dry run: nothing is copied and nothing runs on $HOST"
    STAGE=/tmp/quack-control-install.XXXXXX
    step ssh ${OPTS[@]+"${OPTS[@]}"} "$HOST" mktemp -d /tmp/quack-control-install.XXXXXX
else
    STAGE=$(ssh ${OPTS[@]+"${OPTS[@]}"} "$HOST" mktemp -d /tmp/quack-control-install.XXXXXX)
fi
echo "copying to $HOST:$STAGE"
step scp ${SCP_OPTS[@]+"${SCP_OPTS[@]}"} -q "$BIN" "$HOST:$STAGE/quack-control"
step scp ${SCP_OPTS[@]+"${SCP_OPTS[@]}"} -q "$UNIT" "$HOST:$STAGE/quack-control.service"
step scp ${SCP_OPTS[@]+"${SCP_OPTS[@]}"} -q "$SYSUSERS" "$HOST:$STAGE/sysusers.conf"
step scp ${SCP_OPTS[@]+"${SCP_OPTS[@]}"} -q "$CONFIG" "$HOST:$STAGE/quack-control.toml"

# shellcheck disable=SC2087 # $STAGE is expanded here on purpose
REMOTE=$(cat <<REMOTE
set -euo pipefail
cd "$STAGE"
run() { echo "+ \$*"; sudo "\$@"; }

run install -m 755 quack-control /usr/local/bin/quack-control
run install -m 644 quack-control.service /etc/systemd/system/quack-control.service
run install -D -m 644 sysusers.conf /etc/sysusers.d/quack-control.conf
run systemd-sysusers /etc/sysusers.d/quack-control.conf
getent group robot >/dev/null \\
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
)

if [ "$DRY_RUN" = 1 ]; then
    step ssh ${OPTS[@]+"${OPTS[@]}"} "$HOST" bash -s
    echo "  with this script on its standard input:"
    printf '%s\n' "$REMOTE" | sed 's/^/  | /'
    exit 0
fi
ssh ${OPTS[@]+"${OPTS[@]}"} "$HOST" bash -s <<< "$REMOTE"

echo "installed on $HOST — the page: http://${HOST#*@}:8090/ (journal: ssh $HOST journalctl -u quack-control -f)"
