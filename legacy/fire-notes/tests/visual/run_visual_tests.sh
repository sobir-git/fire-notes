#!/bin/bash
# Smoke tests for Fire Notes — 5 existence-proof tests via Xvfb + xdotool.
# Tests are NOT correctness checks; they verify the app launches, renders,
# and doesn't crash under basic interaction.
#
# Usage:
#   ./run_visual_tests.sh                  # compare against baselines
#   ./run_visual_tests.sh --update-snapshots  # capture new baselines
#
# Requires: xvfb-run (or an active DISPLAY), xdotool, imagemagick
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SNAPSHOT_DIR="$PROJECT_DIR/tests/snapshots"
TEMP_DIR="/tmp/fire-notes-visual-tests"
UPDATE_SNAPSHOTS=false
XVFB_PID=""

[ "$1" == "--update-snapshots" ] && UPDATE_SNAPSHOTS=true

mkdir -p "$TEMP_DIR" "$SNAPSHOT_DIR"

# ── Auto-launch Xvfb if no DISPLAY ───────────────────────────────────────────
if [ -z "$DISPLAY" ]; then
    echo "No DISPLAY — launching Xvfb on :99"
    Xvfb :99 -screen 0 1280x800x24 &
    XVFB_PID=$!
    export DISPLAY=:99
    sleep 1
fi

cleanup() {
    kill -9 "$APP_PID" 2>/dev/null || true
    [ -n "$XVFB_PID" ] && kill -9 "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

# ── Build ─────────────────────────────────────────────────────────────────────
echo "Building..."
cd "$PROJECT_DIR"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release 2>/dev/null

# ── Launch app ────────────────────────────────────────────────────────────────
pkill -9 -f "fire-notes" 2>/dev/null || true
sleep 0.3
"$PROJECT_DIR/target/release/fire-notes" &
APP_PID=$!
sleep 2

WINDOW_ID=$(xdotool search --name "Fire Notes" 2>/dev/null | head -1)
if [ -z "$WINDOW_ID" ]; then
    echo "FAIL: window not found"
    exit 1
fi
xdotool windowactivate "$WINDOW_ID" 2>/dev/null || true
sleep 0.3

PASSED=0
FAILED=0

# ── Helper ────────────────────────────────────────────────────────────────────
run_test() {
    local name="$1"; shift
    printf "%-30s" "  $name"

    "$@" 2>/dev/null; sleep 0.3
    xdotool windowactivate "$WINDOW_ID" 2>/dev/null || true; sleep 0.15

    local shot="$TEMP_DIR/${name}.png"
    import -window "$WINDOW_ID" "$shot" 2>/dev/null

    if [ "$UPDATE_SNAPSHOTS" == "true" ]; then
        cp "$shot" "$SNAPSHOT_DIR/${name}.png"
        echo "  [saved]"
        PASSED=$((PASSED+1))
    elif [ ! -f "$SNAPSHOT_DIR/${name}.png" ]; then
        echo "  [no baseline — run --update-snapshots]"
        FAILED=$((FAILED+1))
    else
        DIFF=$(compare -metric AE "$shot" "$SNAPSHOT_DIR/${name}.png" /dev/null 2>&1 || echo "99999")
        if [ "$DIFF" -lt 1000 ] 2>/dev/null; then
            echo "  ok (diff $DIFF px)"
            PASSED=$((PASSED+1))
        else
            echo "  FAIL (diff $DIFF px)"
            FAILED=$((FAILED+1))
        fi
    fi
}

# ── 5 smoke tests ─────────────────────────────────────────────────────────────
# 1. Empty editor — app launched and rendered its initial state
run_test "01_empty"         true

# 2. Typing — text appears in the content area
run_test "02_typing"        xdotool type --delay 30 "Hello World"

# 3. New tab + switch — tab bar updates correctly
run_test "03_new_tab"       bash -c "xdotool key ctrl+n; sleep 0.2; xdotool type --delay 30 'Tab 2'; xdotool key ctrl+Tab"

# 4. Selection — highlight renders over typed text
run_test "04_selection"     bash -c "xdotool key ctrl+Tab; xdotool key Home; xdotool key shift+End"

# 5. Notes picker — overlay renders without crash
run_test "05_notes_picker"  bash -c "xdotool key Escape; xdotool key ctrl+o"

# ── Results ───────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASSED passed  $FAILED failed"
[ "$FAILED" -gt 0 ] && exit 1 || exit 0
