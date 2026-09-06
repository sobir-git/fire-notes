#!/bin/bash
# Dev workflow: build (debug) and restart fire-notes.
# Usage:
#   ./dev.sh          — build once and (re)launch
#   ./dev.sh --watch  — rebuild + relaunch on every src/ change (requires inotifywait or cargo-watch)
#   ./dev.sh --test   — run all tests

set -e

BINARY="./target/debug/fire-notes"

build_and_run() {
    echo "🔨 Building..."
    cargo build 2>&1
    echo "🛑 Stopping previous instance..."
    pkill -x fire-notes 2>/dev/null || true
    sleep 0.2
    echo "🚀 Launching..."
    "$BINARY" &
    echo "   PID $!"
}

case "${1:-}" in
    --test)
        cargo test
        ;;
    --watch)
        # Try cargo-watch first, fall back to a manual loop
        if cargo watch --version &>/dev/null 2>&1; then
            cargo watch -x 'build' -s "pkill -x fire-notes 2>/dev/null || true; sleep 0.2; ./target/debug/fire-notes &"
        elif command -v inotifywait &>/dev/null; then
            build_and_run
            echo "👁  Watching src/ for changes (inotifywait)..."
            while inotifywait -r -e modify,create,delete,move src/ Cargo.toml 2>/dev/null; do
                build_and_run
            done
        else
            echo "ℹ  No file watcher found. Install cargo-watch for --watch mode:"
            echo "     cargo install cargo-watch"
            echo ""
            echo "Falling back to single build+run."
            build_and_run
        fi
        ;;
    *)
        build_and_run
        ;;
esac
