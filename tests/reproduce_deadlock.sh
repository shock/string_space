#!/bin/bash
# Reproduce the server deadlock/hang issue
# This script demonstrates the exact conditions that cause the server to hang

set -e

echo "=== String Space Server Deadlock Reproduction ==="
echo "This reproduces the issue where server hangs when client sends data"
echo "without EOT byte and keeps connection open."
echo ""

# Configuration
PORT=9897
TEST_DB="test/word_list.txt"
LOG_FILE="tests/deadlock_reproduction.log"
SERVER_PID=""

# Cleanup function
cleanup() {
    echo -e "\nCleaning up..."
    if [ ! -z "$SERVER_PID" ]; then
        echo "Killing server process $SERVER_PID"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    # Clean up any other server instances
    pkill -f "string_space start.*$PORT" 2>/dev/null || true
}

# Set up trap for cleanup
trap cleanup EXIT INT TERM

# Step 1: Build the debug version
echo "Step 1: Building debug version of server"
export STRING_SPACE_DEBUG=1
export RUST_BACKTRACE=full
export SS_TEST=true

cargo build
if [ $? -ne 0 ]; then
    echo "Build failed"
    exit 1
fi
echo "Build successful"

# Step 2: Start the server with debug output to log file
echo -e "\nStep 2: Starting server on port $PORT"
echo "Server output will be written to: $LOG_FILE"

# Kill any existing server on this port
pkill -f "string_space start.*$PORT" 2>/dev/null || true
sleep 1

target/debug/string_space start "$TEST_DB" -p "$PORT" > "$LOG_FILE" 2>&1 &
SERVER_PID=$!

# Give server time to start
echo "Waiting for server to start..."
sleep 2

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo "Server failed to start"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

echo "Server started successfully (PID: $SERVER_PID)"

# Step 3: Test normal operation first
echo -e "\nStep 3: Testing normal operation"
python3 -c "
import socket
import time

s = socket.socket()
s.settimeout(5)
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    print('✓ Normal operation: Server responded with', len(response), 'bytes')
except Exception as e:
    print('✗ Normal operation failed:', e)
    exit(1)
"

# Step 4: Start hanging client (sends data without EOT, keeps connection open)
echo -e "\nStep 4: Starting hanging client"
echo "This client will:"
echo "  1. Connect to server"
echo "  2. Send 'best-completions' request WITHOUT EOT byte"
echo "  3. Keep connection open for 30 seconds"
echo ""
echo "The server should hang at 'Waiting for EOT byte (blocking read)...'"

# Run the actual hang reproduction script
uv run tests/test_real_hang.py $PORT 2>&1 &
HANG_TEST_PID=$!

# Give it time to run
sleep 5

# Step 5: Check server log for hang evidence
echo -e "\nStep 5: Checking server log for hang evidence"
echo "Looking for 'Waiting for EOT byte (blocking read)...' in logs:"
echo "------------------------------------------------------------"
if grep -q "Waiting for EOT byte" "$LOG_FILE"; then
    echo "✓ Found: Server is waiting for EOT byte"
    grep -A2 -B2 "Waiting for EOT byte" "$LOG_FILE"
else
    echo "✗ Not found: Server may not be hanging as expected"
    echo "Last 10 lines of log:"
    tail -10 "$LOG_FILE"
fi
echo "------------------------------------------------------------"

# Step 6: Wait for test to complete
echo -e "\nStep 6: Waiting for test to complete (30 seconds)..."
wait $HANG_TEST_PID 2>/dev/null || true

# Step 7: Final summary
echo -e "\n=== REPRODUCTION COMPLETE ==="
echo ""
echo "Summary of findings:"
echo "1. Server hang location: reader.read_until(EOT_BYTE, &mut buffer)"
echo "   - File: src/modules/protocol.rs"
echo "   - Line: 240 (in handle_client() method)"
echo ""
echo "2. Trigger conditions:"
echo "   - Client sends data without EOT byte (0x04)"
echo "   - Client keeps TCP connection open"
echo "   - Server waits indefinitely for EOT byte"
echo ""
echo "3. Root cause:"
echo "   - Single-threaded server architecture"
echo "   - Blocking I/O with no timeouts"
echo "   - Sequential connection processing"
echo ""
echo "4. Impact:"
echo "   - One hanging client blocks all subsequent clients"
echo "   - Server becomes unresponsive"
echo "   - Requires server restart to recover"
echo ""
echo "5. Files created:"
echo "   - $LOG_FILE: Server debug output"
echo "   - tests/test_real_hang.py: Hang reproduction script"
echo "   - tests/test_hanging_client.py: Test client utilities"
echo ""
echo "Next step: Implement fix by adding read timeouts in handle_client() method."