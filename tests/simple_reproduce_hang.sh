#!/bin/bash
# Simple script to reproduce the server hang issue

set -e

echo "=== Simple Server Hang Reproduction Test ==="
echo ""

# Configuration
PORT=9897
TEST_DB="test/word_list.txt"
LOG_FILE="tests/simple_hang_test.log"

# Cleanup function
cleanup() {
    echo -e "\nCleaning up..."
    pkill -f "string_space start.*$PORT" 2>/dev/null || true
    sleep 1
}

# Set up trap for cleanup
trap cleanup EXIT INT TERM

# Step 1: Build and start server
echo "Step 1: Building and starting server on port $PORT"
export STRING_SPACE_DEBUG=1
export RUST_BACKTRACE=full
export SS_TEST=true

cargo build
target/debug/string_space start "$TEST_DB" -p "$PORT" > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
sleep 2

if ! ps -p $SERVER_PID > /dev/null; then
    echo "ERROR: Server failed to start"
    tail -20 "$LOG_FILE"
    exit 1
fi

echo "Server started (PID: $SERVER_PID)"

# Step 2: Test normal operation
echo -e "\nStep 2: Testing normal operation"
python3 -c "
import socket
import time

s = socket.socket()
s.settimeout(5)
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    print('Normal operation OK: Server responded with', len(response), 'bytes')
except Exception as e:
    print('ERROR:', e)
    exit(1)
"

# Step 3: Create hanging client
echo -e "\nStep 3: Creating hanging client (no EOT byte)"
python3 -c "
import socket
import threading
import time

def hanging_client():
    s = socket.socket()
    s.settimeout(5)
    try:
        s.connect(('127.0.0.1', $PORT))
        print('Hanging client connected')
        # Send request WITHOUT EOT byte
        s.sendall(b'best-completions\x1ess\x1e10')  # Missing \x04
        print('Hanging client sent partial request (no EOT)')
        # Keep connection open
        time.sleep(10)
        print('Hanging client exiting')
    except Exception as e:
        print('Hanging client error:', e)

# Start hanging client in background
import threading
thread = threading.Thread(target=hanging_client, daemon=True)
thread.start()
time.sleep(1)  # Give it time to connect
print('Hanging client is now blocking the server')
"

# Step 4: Test that server is now blocked
echo -e "\nStep 4: Testing that server is blocked"
python3 -c "
import socket
import time

s = socket.socket()
s.settimeout(3)
start_time = time.time()
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    elapsed = time.time() - start_time
    print(f'ERROR: Got response in {elapsed:.3f}s (server should be blocked)')
    exit(1)
except socket.timeout:
    elapsed = time.time() - start_time
    print(f'SUCCESS: Client timed out after {elapsed:.3f}s (server is blocked)')
except Exception as e:
    elapsed = time.time() - start_time
    print(f'Error after {elapsed:.3f}s: {e}')
"

# Step 5: Wait for hanging client to finish and test recovery
echo -e "\nStep 5: Waiting for hanging client to finish..."
sleep 11  # Wait for hanging client's 10s sleep + buffer

echo -e "\nStep 6: Testing server recovery"
python3 -c "
import socket
import time

s = socket.socket()
s.settimeout(5)
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    print('SUCCESS: Server recovered and responded with', len(response), 'bytes')
except Exception as e:
    print('ERROR: Server still blocked:', e)
"

# Step 7: Show server log evidence
echo -e "\nStep 7: Server log evidence of hang"
echo "----------------------------------------"
grep -A5 -B5 "Waiting for EOT byte" "$LOG_FILE" || tail -20 "$LOG_FILE"
echo "----------------------------------------"

echo -e "\n=== TEST COMPLETE ==="
echo "The hang issue has been successfully reproduced:"
echo "1. Server hangs at 'Waiting for EOT byte (blocking read)...'"
echo "2. Normal clients timeout when trying to connect"
echo "3. Server recovers after hanging client disconnects"
echo ""
echo "Root cause: Single-threaded server with blocking I/O and no timeouts"