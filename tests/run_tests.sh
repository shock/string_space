#!/bin/bash

cargo test
if [ $? -ne 0 ]; then
    echo "cargo test failed"
    exit 1
fi

cargo build --release

EXECUTABLE=target/release/string_space

# Stop the server if it's running
SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
fi

# Start the server
SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9898 -d
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE start test/word_list.txt -p 9898 failed"
    exit 1
else
    echo "Server started successfully"
fi

# Run the client
uv run tests/client.py 9898
if [ $? -ne 0 ]; then
    echo "uv run tests/client.py 9898 failed"
    exit 1
else
    echo "Client connected successfully"
fi

# Stop the server
SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
    exit 1
else
    echo "Server stopped successfully"
fi

# Start the server again
SS_TEST=true $EXECUTABLE start test/word_list.txt -p 9898 -d
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE start test/word_list.txt -p 9898 -d failed"
    exit 1
else
    echo "Server started successfully"
fi

# Restart the server
SS_TEST=true $EXECUTABLE restart test/word_list.txt -p 9898
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE restart test/word_list.txt -p 9898 failed"
    exit 1
else
    echo "Server restarted successfully"
fi

# Stop the server
SS_TEST=true $EXECUTABLE stop
if [ $? -ne 0 ]; then
    echo "$EXECUTABLE stop failed"
    exit 1
else
    echo "Server stopped successfully"
fi

echo
echo "ALL TESTS PASSED !!!"
echo
