#!/usr/bin/env sh
# Build the release binary and optionally run it.
#   ./build.sh        build only
#   ./build.sh run    build, then launch the app
set -eu
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found. On Arch/CachyOS: sudo pacman -S rust (or rustup)" >&2
    exit 1
fi

cargo build --release
echo "Built: $(pwd)/target/release/traveller_npc"

if [ "${1:-}" = "run" ]; then
    exec ./target/release/traveller_npc
fi
