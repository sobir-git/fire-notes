#!/bin/bash
# Visual regression test runner for Fire Notes (v2 - more robust)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SNAPSHOT_DIR="$PROJECT_DIR/tests/snapshots"
TEMP_DIR="/tmp/fire-notes-visual-tests"
UPDATE_SNAPSHOTS=false

[ "$1" == "--update-snapshots" ] && UPDATE_SNAPSHOTS=true && echo "📸 Mode: Update snapshots" || echo "🧪 Mode: Compare"

mkdir -p "$TEMP_DIR" "$SNAPSHOT_DIR"

# Build
echo "🔨 Building..."
cd "$PROJECT_DIR"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release 2>/dev/null

# Kill existing
pkill -9 -f "fire-notes" 2>/dev/null || true
sleep 0.5

# Start app
echo "🚀 Starting app..."
"$PROJECT_DIR/target/release/fire-notes" &
APP_PID=$!
sleep 2

# Find window
WINDOW_ID=$(xdotool search --name "Fire Notes" 2>/dev/null | head -1)
if [ -z "$WINDOW_ID" ]; then
    echo "❌ Window not found"
    kill -9 $APP_PID 2>/dev/null || true
    exit 1
fi
echo "✅ Window: $WINDOW_ID"

# Focus
xdotool windowactivate "$WINDOW_ID" 2>/dev/null || true
sleep 0.5

PASSED=0
FAILED=0

run_test() {
    local name="$1"
    shift
    echo "━━━ Test: $name ━━━"
    
    # Run commands
    "$@"
    sleep 0.3
    
    # Screenshot
    local shot="$TEMP_DIR/${name}.png"
    local baseline="$SNAPSHOT_DIR/${name}.png"
    
    xdotool windowactivate "$WINDOW_ID" 2>/dev/null || true
    sleep 0.2
    
    # Use import from ImageMagick instead of scrot (more reliable)
    import -window "$WINDOW_ID" "$shot" 2>/dev/null
    
    if [ "$UPDATE_SNAPSHOTS" == "true" ]; then
        cp "$shot" "$baseline"
        echo "📸 Saved: $baseline"
        PASSED=$((PASSED + 1))
    else
        if [ ! -f "$baseline" ]; then
            echo "⚠️  No baseline - run with --update-snapshots first"
            FAILED=$((FAILED + 1))
        else
            # Simple pixel comparison
            DIFF=$(compare -metric AE "$shot" "$baseline" /dev/null 2>&1 || echo "99999")
            if [ "$DIFF" -lt 1000 ] 2>/dev/null; then
                echo "✅ Pass (diff: $DIFF pixels)"
                PASSED=$((PASSED + 1))
            else
                echo "❌ Fail (diff: $DIFF pixels)"
                FAILED=$((FAILED + 1))
            fi
        fi
    fi
}

# Tests
# Tests
run_test "01_empty" true
run_test "02_hello" xdotool type --delay 30 "Hello World"
run_test "03_newline" bash -c "xdotool key Return; xdotool type --delay 30 'Line 2'"
run_test "04_newtab" xdotool key ctrl+n
run_test "05_tabtwo" xdotool type --delay 30 "Tab 2"
run_test "06_switch" xdotool key ctrl+Tab
run_test "07_home" xdotool key Home
run_test "08_select_shift" xdotool key shift+Right shift+Right shift+Right
run_test "09_copy_paste" bash -c "xdotool key ctrl+c; xdotool key End; xdotool key Return; xdotool key ctrl+v"
run_test "10_select_all" xdotool key ctrl+a

run_test "11_double_click" xdotool click --repeat 2 --delay 50 1

# Cleanup
echo "🧹 Cleanup..."
kill -9 $APP_PID 2>/dev/null || true

echo ""
echo "━━━ Results: ✅ $PASSED  ❌ $FAILED ━━━"
[ "$FAILED" -gt 0 ] && exit 1 || exit 0
