#!/usr/bin/env python3
"""
Comprehensive integration test for best-completions protocol command.

This script tests various query types, limits, and edge cases for the
best-completions functionality between Python client and Rust server.
"""

import time
import threading
from string_space_client import StringSpaceClient

def test_basic_best_completions(client):
    """Test basic best-completions functionality."""
    print("\n=== Testing Basic Best-Completions ===")

    # Test 1: Simple prefix query
    print("\nTest 1: Simple prefix query")
    results = client.best_completions_search("hel")
    print(f"Query 'hel': Found {len(results)} results")
    print(f"First 5 results: {results[:5]}")
    assert len(results) > 0, "Should find results for 'hel'"

    # Test 2: Query with limit
    print("\nTest 2: Query with limit")
    results = client.best_completions_search("hel", limit=5)
    print(f"Query 'hel' with limit 5: Found {len(results)} results")
    print(f"Results: {results}")
    assert len(results) <= 5, "Result limit should be respected"

    # Test 3: Very short query
    print("\nTest 3: Very short query")
    results = client.best_completions_search("h")
    print(f"Query 'h': Found {len(results)} results")
    print(f"First 3 results: {results[:3]}")
    assert len(results) > 0, "Should find results for very short query"

    # Test 4: Empty query
    print("\nTest 4: Empty query")
    results = client.best_completions_search("")
    print(f"Query '': Found {len(results)} results")
    print(f"Results: {results}")
    # Empty query might return all words or none - depends on implementation

    print("✓ Basic best-completions tests passed")

def test_query_length_categories(client):
    """Test queries of different lengths."""
    print("\n=== Testing Query Length Categories ===")

    test_cases = [
        ("h", "Very short (1 char)"),
        ("he", "Very short (2 chars)"),
        ("hel", "Short (3 chars)"),
        ("hell", "Short (4 chars)"),
        ("hello", "Medium (5 chars)"),
        ("helix", "Medium (5 chars)"),
        ("helico", "Long (6 chars)"),
        ("helicop", "Long (7 chars)"),
    ]

    for query, description in test_cases:
        results = client.best_completions_search(query, limit=5)
        print(f"Query '{query}' ({description}): Found {len(results)} results")
        if results:
            print(f"  First result: {results[0]}")
        assert len(results) <= 5, f"Result limit should be respected for {description}"

    print("✓ Query length category tests passed")

def test_edge_cases(client):
    """Test edge cases and error handling."""
    print("\n=== Testing Edge Cases ===")

    # Test 1: Non-existent query
    print("\nTest 1: Non-existent query")
    results = client.best_completions_search("xyz123nonexistent")
    print(f"Query 'xyz123nonexistent': Found {len(results)} results")
    print(f"Results: {results}")
    # Should return empty list or handle gracefully

    # Test 2: Very long query (should be truncated by server)
    print("\nTest 2: Long query")
    long_query = "a" * 50  # Maximum allowed length
    results = client.best_completions_search(long_query)
    print(f"Query length {len(long_query)}: Found {len(results)} results")
    print(f"Results: {results}")

    # Test 3: Special characters
    print("\nTest 3: Special characters")
    results = client.best_completions_search("test-")
    print(f"Query 'test-': Found {len(results)} results")
    print(f"Results: {results}")

    print("✓ Edge case tests passed")

def test_performance(client):
    """Test performance characteristics."""
    print("\n=== Testing Performance ===")

    test_queries = ["h", "he", "hel", "hell", "hello", "helico"]

    for query in test_queries:
        start_time = time.time()
        results = client.best_completions_search(query, limit=10)
        end_time = time.time()

        response_time = (end_time - start_time) * 1000  # Convert to milliseconds
        print(f"Query '{query}': {len(results)} results in {response_time:.2f}ms")

        # Performance expectations
        assert response_time < 1000, f"Query '{query}' took too long: {response_time:.2f}ms"

    print("✓ Performance tests passed")

def test_concurrent_clients():
    """Test concurrent client connections."""
    print("\n=== Testing Concurrent Clients ===")

    def client_worker(client_id, results):
        try:
            client = StringSpaceClient('127.0.0.1', 9898)
            worker_results = client.best_completions_search("hel", limit=3)
            results[client_id] = worker_results
            print(f"Client {client_id}: Found {len(worker_results)} results")
        except Exception as e:
            results[client_id] = f"Error: {e}"

    num_clients = 5
    threads = []
    results = {}

    # Start concurrent clients
    for i in range(num_clients):
        thread = threading.Thread(target=client_worker, args=(i, results))
        threads.append(thread)
        thread.start()

    # Wait for all threads to complete
    for thread in threads:
        thread.join()

    # Verify all clients got results
    for client_id, result in results.items():
        if isinstance(result, list):
            print(f"Client {client_id}: Success with {len(result)} results")
            assert len(result) > 0, f"Client {client_id} should get results"
        else:
            print(f"Client {client_id}: {result}")

    print("✓ Concurrent client tests passed")

def validate_response_format(client):
    """Validate response format matches protocol specification."""
    print("\n=== Validating Response Format ===")

    # Test various queries and validate response format
    test_queries = ["h", "he", "hel", "test", "nonexistent"]

    for query in test_queries:
        results = client.best_completions_search(query)

        # Response should be a list
        assert isinstance(results, list), f"Response for '{query}' should be a list"

        # Each item in the list should be a string
        for item in results:
            assert isinstance(item, str), f"Result item should be string, got {type(item)}"
            # Strings should not contain protocol separators
            assert '\x1e' not in item, f"Result contains RS separator: {item}"
            assert '\x04' not in item, f"Result contains EOT terminator: {item}"

        print(f"Query '{query}': Response format valid ({len(results)} results)")

    print("✓ Response format validation passed")

def main():
    """Run all integration tests."""
    print("StringSpace Best-Completions Integration Test")
    print("=============================================")

    # Create client
    client = StringSpaceClient('127.0.0.1', 9898)

    try:
        # Run all test suites
        test_basic_best_completions(client)
        test_query_length_categories(client)
        test_edge_cases(client)
        test_performance(client)
        validate_response_format(client)
        test_concurrent_clients()

        print("\n=== All Integration Tests Complete ===")
        print("✓ All integration tests passed successfully!")

    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        raise

if __name__ == "__main__":
    main()