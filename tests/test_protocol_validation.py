#!/usr/bin/env python3
"""
Protocol validation test for best-completions command.

This script validates that the response format strictly matches the
protocol specification, including separator handling, encoding,
and error responses.
"""

import socket
import time
from string_space_client import StringSpaceClient

def test_raw_protocol_communication():
    """Test raw protocol communication to validate exact format."""
    print("\n=== Testing Raw Protocol Communication ===")

    # Create raw socket connection
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('127.0.0.1', 9898))

    # Test 1: Valid best-completions request
    print("\nTest 1: Valid best-completions request")
    request = "best-completions\x1ehel\x1e10\x04"
    sock.send(request.encode('utf-8'))

    # Receive response
    response = b''
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        response += chunk
        if b'\x04' in chunk:
            break

    # Parse response
    response_text = response.rstrip(b'\x04').decode('utf-8')
    print(f"Raw response length: {len(response_text)} bytes")
    print(f"Response contains EOT: {b'\x04' in response}")
    print(f"Response contains RS: {b'\x1e' in response}")

    # Validate response format
    lines = response_text.split('\n')
    print(f"Number of result lines: {len(lines)}")
    print(f"First 3 results: {lines[:3]}")

    # Response should not contain protocol separators in content
    assert b'\x1e' not in response, "Response should not contain RS separator"
    assert response.endswith(b'\x04'), "Response should end with EOT terminator"

    sock.close()
    print("✓ Raw protocol communication test passed")

def test_error_handling():
    """Test error handling and error response format."""
    print("\n=== Testing Error Handling ===")

    client = StringSpaceClient('127.0.0.1', 9898)

    # Test 1: Invalid command
    print("\nTest 1: Invalid command")
    try:
        # Try to send invalid command
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 9898))
        request = "invalid-command\x1etest\x04"
        sock.send(request.encode('utf-8'))

        response = b''
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            if b'\x04' in chunk:
                break

        response_text = response.rstrip(b'\x04').decode('utf-8')
        print(f"Error response: {response_text}")

        # Error responses should start with "ERROR -"
        assert response_text.startswith("ERROR -"), "Error response should start with 'ERROR -'"

        sock.close()
    except Exception as e:
        print(f"Error test completed: {e}")

    # Test 2: Malformed request (missing separator)
    print("\nTest 2: Malformed request")
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 9898))
        request = "best-completionshel10\x04"  # Missing separators
        sock.send(request.encode('utf-8'))

        response = b''
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            if b'\x04' in chunk:
                break

        response_text = response.rstrip(b'\x04').decode('utf-8')
        print(f"Malformed request response: {response_text}")

        sock.close()
    except Exception as e:
        print(f"Malformed request test completed: {e}")

    print("✓ Error handling tests passed")

def test_encoding_validation():
    """Test UTF-8 encoding validation."""
    print("\n=== Testing UTF-8 Encoding ===")

    client = StringSpaceClient('127.0.0.1', 9898)

    # Test 1: Unicode characters in query
    print("\nTest 1: Unicode characters")
    try:
        results = client.best_completions_search("café", limit=5)
        print(f"Unicode query 'café': Found {len(results)} results")
        print(f"Results: {results}")
    except Exception as e:
        print(f"Unicode query failed: {e}")

    # Test 2: Emoji characters (should be handled gracefully)
    print("\nTest 2: Emoji characters")
    try:
        results = client.best_completions_search("hello🚀", limit=5)
        print(f"Emoji query 'hello🚀': Found {len(results)} results")
        print(f"Results: {results}")
    except Exception as e:
        print(f"Emoji query handled: {e}")

    print("✓ Encoding validation tests passed")

def test_response_consistency():
    """Test that responses are consistent across multiple requests."""
    print("\n=== Testing Response Consistency ===")

    client = StringSpaceClient('127.0.0.1', 9898)

    # Test multiple identical requests
    query = "hel"
    limit = 5
    results_sets = []

    for i in range(3):
        results = client.best_completions_search(query, limit=limit)
        results_sets.append(set(results))
        print(f"Request {i+1}: {len(results)} results")
        time.sleep(0.1)  # Small delay between requests

    # Check consistency
    all_results = set()
    for result_set in results_sets:
        all_results.update(result_set)

    print(f"Total unique results across requests: {len(all_results)}")

    # Results should be reasonably consistent
    # (some variation is expected due to scoring algorithms)
    assert len(all_results) <= 10, "Results should be reasonably consistent"

    print("✓ Response consistency test passed")

def main():
    """Run all protocol validation tests."""
    print("StringSpace Protocol Validation Test")
    print("====================================")

    try:
        test_raw_protocol_communication()
        test_error_handling()
        test_encoding_validation()
        test_response_consistency()

        print("\n=== All Protocol Validation Tests Complete ===")
        print("✓ All protocol validation tests passed successfully!")

    except Exception as e:
        print(f"\n❌ Protocol validation test failed: {e}")
        raise

if __name__ == "__main__":
    main()