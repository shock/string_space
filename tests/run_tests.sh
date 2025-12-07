#!/bin/bash

cargo test
if [ $? -ne 0 ]; then
    echo "cargo test failed"
    exit 1
fi

cargo build

ENV_VARS="STRING_SPACE_DEBUG=1 RUST_BACKTRACE=full SS_TEST=true"
EXECUTABLE="target/debug/string_space"

# Stop the server if it's running
STRING_SPACE_DEBUG=1 RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
fi

# Test foreground mode
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9899 &
FOREGROUND_PID=$!

# Give the server a moment to start
sleep 1

# Check if the foreground server is running
if ps -p $FOREGROUND_PID > /dev/null; then
    echo "Foreground server started successfully"
else
    echo "Foreground server failed to start"
    exit 1
fi

# Run the client against foreground server
uv run tests/client.py 9899
if [ $? -ne 0 ]; then
    echo "uv run tests/client.py 9899 failed"
    kill $FOREGROUND_PID 2>/dev/null
    exit 1
else
    echo "Client connected successfully to foreground server"
fi

# Kill the foreground server
kill $FOREGROUND_PID 2>/dev/null
if [ $? -eq 0 ]; then
    echo "Foreground server stopped successfully"
else
    echo "Failed to stop foreground server"
    exit 1
fi

# Test daemon mode

# Start the server in daemon mode
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9898 -d
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE start test/word_list.txt -p 9898 -d failed"
    exit 1
else
    echo "Daemon server started successfully"
fi

# Run the client
uv run tests/client.py 9898
if [ $? -ne 0 ]; then
    echo "uv run tests/client.py 9898 failed"
    RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
    exit 1
else
    echo "Client connected successfully to daemon server"
fi

# Stop the server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
    exit 1
else
    echo "Daemon server stopped successfully"
fi

# Start the server again in daemon mode
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9898 -d
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE start test/word_list.txt -p 9898 -d failed"
    exit 1
else
    echo "Daemon server started successfully"
fi

# Restart the server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE restart test/word_list.txt -p 9898
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE restart test/word_list.txt -p 9898 failed"
    RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
    exit 1
else
    echo "Daemon server restarted successfully"
fi

# Stop the server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
    exit 1
else
    echo "Daemon server stopped successfully"
fi

# Run the import test
uv run tests/import.py 9898
if [ $? -ne 0 ]; then
    echo "uv run tests/import.py 9898 failed"
    exit 1
else
    echo "Import test ran successfully"
fi

# Start server again for the new integration tests
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9898 -d
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE start test/word_list.txt -p 9898 -d failed"
    exit 1
else
    echo "Daemon server started successfully for integration tests"
fi

# Run the best-completions integration test
uv run tests/test_best_completions_integration.py
if [ $? -ne 0 ]; then
    echo "uv run tests/test_best_completions_integration.py failed"
    RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
    exit 1
else
    echo "Best-completions integration test ran successfully"
fi

# Run the protocol validation test
uv run tests/test_protocol_validation.py
if [ $? -ne 0 ]; then
    echo "uv run tests/test_protocol_validation.py failed"
    RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
    exit 1
else
    echo "Protocol validation test ran successfully"
fi

# Stop the server after all tests
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
    exit 1
else
    echo "Daemon server stopped successfully after all tests"
fi

# Run the manual test for Fuzzy-Subsequence Search
echo "=== Manual Testing: Fuzzy-Subsequence Search ==="
tests/manual_test.sh
if [ $? -ne 0 ]; then
    echo "manual_test.sh failed"
    exit 1
fi

echo
echo "ALL TESTS PASSED !!!"
echo
