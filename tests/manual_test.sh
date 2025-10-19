#!/bin/bash
# manual_test.sh - Comprehensive manual testing for fuzzy-subsequence search

echo "=== Manual Testing: Fuzzy-Subsequence Search ==="

# Build the project
echo "Building project..."
cargo build --release

# Start server in background
echo "Starting server..."
./target/release/string_space start test/word_list.txt --port 7878 --host 127.0.0.1 &
SERVER_PID=$!
sleep 2

echo "Testing basic functionality..."

# Test 1: Basic fuzzy-subsequence search
echo "Test 1: Basic search"
python3 -c "
from string_space_client import StringSpaceClient
client = StringSpaceClient('127.0.0.1', 7878)

# Insert test data
client.insert(['hello', 'world', 'help', 'helicopter', 'openai/gpt-4o-2024-08-06'])

# Test fuzzy-subsequence search
results = client.fuzzy_subsequence_search('hl')
print('Search for \"hl\":', results)

results = client.fuzzy_subsequence_search('g4')
print('Search for \"g4\":', results)

results = client.fuzzy_subsequence_search('')
print('Search for empty string:', results)
"

# Test 2: Performance with large dataset
echo "Test 2: Performance testing"
./target/release/string_space benchmark test/word_list.txt --count 10000

# Test 3: Protocol error handling
echo "Test 3: Error handling"
python3 -c "
from string_space_client import StringSpaceClient
client = StringSpaceClient('127.0.0.1', 7878)

# Test invalid parameter count (simulate by sending malformed request)
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('127.0.0.1', 7878))
s.send(b'fuzzy-subsequence\x1e\x04')  # Missing query
response = s.recv(1024).decode()
print('Error response:', repr(response))
s.close()
"

# Cleanup
echo "Cleaning up..."
kill $SERVER_PID

echo "=== Manual testing complete ==="