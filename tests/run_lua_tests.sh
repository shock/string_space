#!/bin/bash
set -e

# Run Lua integration tests against a live string-space server.
# Builds the server, starts a daemon, runs lua test scripts, then stops.
#
# Usage: tests/run_lua_tests.sh
# Requires: nvim on $PATH, cargo

cargo build
EXECUTABLE=target/debug/string_space
PORT=9898

# Clean up any existing server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null || true

# Start daemon
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p $PORT -d
echo "Server started on port $PORT"

# Wait for server to be ready
sleep 1

# Trap ensures cleanup on exit
trap 'RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null' EXIT

# Run all Lua test scripts
for script in lua/test_*.lua; do
    echo ""
    echo "=== Running $script ==="
    nvim -l "$script" $PORT
    echo "✓ $script passed"
done

echo ""
echo "=== All Lua tests passed ==="
