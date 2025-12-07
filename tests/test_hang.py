#!/usr/bin/env python3
"""
Test script to reproduce the hang issue with best-completions operation.
"""

import time
import sys
from string_space_client import StringSpaceClient

def test_best_completions_hang():
    """Test best-completions operation to reproduce hang."""
    print("=== Testing Best-Completions Hang Reproduction ===")
    
    # Connect to server
    client = StringSpaceClient('127.0.0.1', 7878)
    
    # Test 1: Simple best-completions query
    print("\nTest 1: Simple best-completions query")
    start_time = time.time()
    try:
        results = client.best_completions_search("ss", limit=10)
        elapsed = time.time() - start_time
        print(f"Query 'ss' with limit 10: Found {len(results)} results in {elapsed:.3f}s")
        print(f"Results: {results[:5]}...")
    except Exception as e:
        elapsed = time.time() - start_time
        print(f"ERROR after {elapsed:.3f}s: {e}")
    
    # Test 2: Multiple rapid queries (simulating client behavior)
    print("\nTest 2: Multiple rapid queries")
    queries = ["ss", "hel", "test", "rust", "py", "java", "go", "cpp", "sql", "web"]
    
    for i, query in enumerate(queries):
        start_time = time.time()
        try:
            results = client.best_completions_search(query, limit=10)
            elapsed = time.time() - start_time
            print(f"  Query {i+1}: '{query}' - {len(results)} results in {elapsed:.3f}s")
            if elapsed > 1.0:
                print(f"    WARNING: Slow response ({elapsed:.3f}s)")
        except Exception as e:
            elapsed = time.time() - start_time
            print(f"  Query {i+1}: '{query}' - ERROR after {elapsed:.3f}s: {e}")
    
    # Test 3: Insert operation with multiple words (another suspected trigger)
    print("\nTest 3: Insert operation with multiple words")
    # Create a long text to insert (simulating pasting text)
    long_text = " ".join([f"test_word_{i}" for i in range(100)])
    words_to_insert = [long_text]  # Single parameter with many words
    
    start_time = time.time()
    try:
        result = client.insert(words_to_insert)
        elapsed = time.time() - start_time
        print(f"Insert operation: {result} in {elapsed:.3f}s")
        if elapsed > 2.0:
            print(f"  WARNING: Slow insert ({elapsed:.3f}s)")
    except Exception as e:
        elapsed = time.time() - start_time
        print(f"Insert operation ERROR after {elapsed:.3f}s: {e}")
    
    print("\n=== Test Complete ===")

if __name__ == "__main__":
    test_best_completions_hang()