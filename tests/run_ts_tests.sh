#!/bin/bash
set -e

# Run TypeScript integration tests against a live string-space server.
# Builds the server, starts a daemon, runs all ts_*.ts scripts, then stops.
#
# Usage: tests/run_ts_tests.sh
# Requires: bun on $PATH, cargo

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

# Run all TypeScript test scripts
for script in tests/ts_*.ts; do
    echo ""
    echo "=== Running $script ==="
    bun run "$script" $PORT
    echo "✓ $script passed"
done

echo ""
echo "=== All TypeScript tests passed ==="
