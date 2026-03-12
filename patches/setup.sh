#!/usr/bin/env bash
set -e

PATCHES_DIR="$(cd "$(dirname "$0")" && pwd)"
CARGO_REGISTRY="$HOME/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f"

WINIT_SRC="$CARGO_REGISTRY/winit-0.30.12"
SCTK_SRC="$CARGO_REGISTRY/smithay-client-toolkit-0.19.2"
WINIT_DST="$PATCHES_DIR/winit-0.30.12"
SCTK_DST="$PATCHES_DIR/smithay-client-toolkit-0.19.2"

if [ ! -d "$WINIT_SRC" ]; then
    echo "winit-0.30.12 not in cargo cache. Run: cargo fetch" >&2
    exit 1
fi
if [ ! -d "$SCTK_SRC" ]; then
    echo "smithay-client-toolkit-0.19.2 not in cargo cache. Run: cargo fetch" >&2
    exit 1
fi

echo "Copying winit-0.30.12..."
rm -rf "$WINIT_DST"
cp -r "$WINIT_SRC" "$WINIT_DST"

echo "Copying smithay-client-toolkit-0.19.2..."
rm -rf "$SCTK_DST"
cp -r "$SCTK_SRC" "$SCTK_DST"

echo "Applying patches..."
patch -p1 -d "$WINIT_DST" < "$PATCHES_DIR/winit-cursor-scale.patch"
patch -p1 -d "$SCTK_DST"  < "$PATCHES_DIR/sctk-cursor-explicit-scale.patch"

echo "Done. Run 'cargo build' to verify."
