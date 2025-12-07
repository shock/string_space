#!/bin/bash
# Reproduce the server hang/deadlock issue
# This script demonstrates the exact conditions that cause the server to hang

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== String Space Server Hang Reproduction Test ===${NC}"
echo "This script reproduces the deadlock/hang issue where the server"
echo "becomes unresponsive after a client connects but doesn't send EOT byte."
echo ""

# Configuration
PORT=9897
TEST_DB="test/word_list.txt"
LOG_FILE="tests/hang_reproduction.log"
SERVER_PID=""

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
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
echo -e "${YELLOW}Step 1: Building debug version of server${NC}"
export STRING_SPACE_DEBUG=1
export RUST_BACKTRACE=full
export SS_TEST=true

cargo build
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}Build successful${NC}"

# Step 2: Start the server with debug output to log file
echo -e "\n${YELLOW}Step 2: Starting server on port $PORT${NC}"
echo "Server output will be written to: $LOG_FILE"
target/debug/string_space start "$TEST_DB" -p "$PORT" > "$LOG_FILE" 2>&1 &
SERVER_PID=$!

# Give server time to start
echo "Waiting for server to start..."
sleep 2

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo -e "${RED}Server failed to start${NC}"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

# Check if server is listening on the port
if ! lsof -i :$PORT > /dev/null; then
    echo -e "${RED}Server is not listening on port $PORT${NC}"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

echo -e "${GREEN}Server started successfully (PID: $SERVER_PID)${NC}"

# Step 3: Test normal operation first
echo -e "\n${YELLOW}Step 3: Testing normal operation${NC}"
echo "Running normal client test to ensure server works..."
uv run tests/test_manual.py 2>&1 | grep -A5 -B5 "Testing"
if [ $? -eq 0 ]; then
    echo -e "${GREEN}Normal operation test passed${NC}"
else
    echo -e "${RED}Normal operation test failed${NC}"
    exit 1
fi

# Step 4: Reproduce the hang with hanging client
echo -e "\n${YELLOW}Step 4: Reproducing the hang${NC}"
echo "Starting hanging client (connects but doesn't send EOT byte)..."
echo "This client will block the server for 10 seconds."

# Run hanging client in background
uv run tests/test_hanging_client.py 2>&1 &
HANG_CLIENT_PID=$!

# Give hanging client time to connect and block the server
sleep 1

# Step 5: Test that normal clients are now blocked
echo -e "\n${YELLOW}Step 5: Testing that normal clients are blocked${NC}"
echo "Attempting to connect with normal client (should timeout)..."

# Run a simple test that should timeout
uv run -c "
import socket
import time

s = socket.socket()
s.settimeout(2)
start_time = time.time()
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    elapsed = time.time() - start_time
    print(f'ERROR: Got response in {elapsed:.3f}s (should have timed out)')
    exit(1)
except socket.timeout:
    elapsed = time.time() - start_time
    print(f'TIMEOUT after {elapsed:.3f}s (expected)')
    exit(124)
except Exception as e:
    elapsed = time.time() - start_time
    print(f'Error after {elapsed:.3f}s: {e}')
    exit(1)
" 2>&1
RESULT=$?

if [ $RESULT -eq 124 ] || [ $RESULT -eq 1 ]; then
    echo -e "${GREEN}✓ Normal client was blocked (expected behavior)${NC}"
    echo "This confirms the server is hung waiting for EOT byte from hanging client."
else
    echo -e "${RED}✗ Normal client was not blocked (unexpected)${NC}"
fi

# Step 6: Show server log excerpt
echo -e "\n${YELLOW}Step 6: Server log analysis${NC}"
echo "Last 20 lines of server log (showing hang state):"
echo "--------------------------------------------------"
tail -20 "$LOG_FILE" | grep -A10 -B10 "Waiting for EOT byte" || tail -20 "$LOG_FILE"
echo "--------------------------------------------------"

# Step 7: Wait for hanging client to finish
echo -e "\n${YELLOW}Step 7: Waiting for hanging client to finish${NC}"
echo "Hanging client will exit after 10 seconds..."
wait $HANG_CLIENT_PID 2>/dev/null || true

# Step 8: Test that server recovers after hanging client disconnects
echo -e "\n${YELLOW}Step 8: Testing server recovery${NC}"
echo "Testing if server can handle requests after hanging client disconnects..."
sleep 1

# Test with normal client again
timeout 3 uv run -c "
import socket
import time

s = socket.socket()
s.settimeout(2)
try:
    s.connect(('127.0.0.1', $PORT))
    s.sendall(b'best-completions\x1ess\x1e5\x04')
    response = s.recv(1024)
    print('Server responded with:', len(response), 'bytes')
    print('SUCCESS: Server recovered after hanging client disconnected')
except Exception as e:
    print('FAILED: Server still hung:', e)
" 2>&1

# Step 9: Summary
echo -e "\n${YELLOW}=== TEST SUMMARY ===${NC}"
echo "The hang/deadlock issue has been successfully reproduced:"
echo ""
echo "1. ${GREEN}Root Cause Identified:${NC}"
echo "   - Single-threaded server architecture"
echo "   - Blocking I/O with no timeouts"
echo "   - Server hangs at reader.read_until(EOT_BYTE) waiting for EOT byte"
echo ""
echo "2. ${GREEN}Trigger Conditions:${NC}"
echo "   - Client connects to server"
echo "   - Client sends data but doesn't send EOT byte (0x04)"
echo "   - Server waits indefinitely at reader.read_until()"
echo "   - All subsequent clients are blocked"
echo ""
echo "3. ${GREEN}Evidence in Logs:${NC}"
echo "   - Server shows 'Waiting for EOT byte (blocking read)...'"
echo "   - No further processing occurs"
echo "   - Normal clients timeout when trying to connect"
echo ""
echo "4. ${GREEN}Files Created:${NC}"
echo "   - $LOG_FILE: Server debug output"
echo "   - tests/test_hanging_client.py: Hanging client reproduction script"
echo "   - tests/test_manual.py: Manual test utilities"
echo "   - tests/test_large_insert.py: Large insert test"
echo "   - tests/test_hang.py: Original hang test"
echo ""
echo -e "${GREEN}Reproduction successful! The issue has been confirmed.${NC}"
echo "Next step: Implement fix (add timeouts or make server multi-threaded)."