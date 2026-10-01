#!/usr/bin/env bash
# Cross-build quack-control for the duck's board: Radxa Zero 3 (RK3566,
# aarch64) running Armbian with the Debian 13 (Trixie) userland — the same
# way quack-nav's scripts/cross-build.sh builds quack-navd.
#
# Usage: scripts/cross-build.sh [extra cargo arguments]
# Output: target/aarch64-unknown-linux-gnu/release/quack-control
#         (under $CARGO_TARGET_DIR when it is set)
#
# Needs rustup with the aarch64-unknown-linux-gnu target, zig and
# cargo-zigbuild (on a Mac: brew install rustup zig cargo-zigbuild, then
# rustup toolchain install stable --target aarch64-unknown-linux-gnu).
# No Docker: `zig cc` is the cross linker and brings the glibc stubs. The
# `.2.31` suffix pins the glibc floor, as microduck's `cargo board` does.
set -euo pipefail

TARGET=aarch64-unknown-linux-gnu
GLIBC_FLOOR=2.31

cd "$(dirname "$0")/.."

has_target() {
    [ -d "$(rustc --print sysroot 2>/dev/null)/lib/rustlib/$TARGET" ]
}

# Homebrew's `rust` has no std for other targets; Homebrew's rustup is
# keg-only, so its cargo may not be first on PATH.
if ! has_target && [ -x /opt/homebrew/opt/rustup/bin/rustup ]; then
    PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi
if ! has_target; then
    echo "no Rust std for $TARGET on this PATH (rustc: $(command -v rustc || echo none))" >&2
    echo "  rustup target add $TARGET" >&2
    exit 1
fi
for tool in zig cargo-zigbuild; do
    command -v "$tool" >/dev/null || { echo "$tool is not installed (brew install zig cargo-zigbuild)" >&2; exit 1; }
done

echo "building quack-control for $TARGET, glibc >= $GLIBC_FLOOR ($(rustc --version))"
cargo zigbuild --release --bin quack-control --target "$TARGET.$GLIBC_FLOOR" "$@"

BIN="${CARGO_TARGET_DIR:-target}/$TARGET/release/quack-control"
command -v file >/dev/null && file "$BIN"
if command -v objdump >/dev/null; then
    echo "glibc required: $(objdump -T "$BIN" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)"
fi
echo "built $BIN ($(wc -c < "$BIN" | tr -d ' ') bytes)"
