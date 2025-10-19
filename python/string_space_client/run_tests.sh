#!/bin/bash
# Test runner for StringSpaceClient tests

echo "Running StringSpaceClient tests..."
cd "$(dirname "$0")"

# Run the tests
python test_string_space_client.py -v

# Check exit code
if [ $? -eq 0 ]; then
    echo "✅ All tests passed!"
else
    echo "❌ Some tests failed!"
    exit 1
fi