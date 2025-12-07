#!/usr/bin/env python3
"""
Manual test to connect to server and send raw protocol messages.
"""

import socket
import time

def send_request(port, request_data):
    """Send raw request to server and get response."""
    print(f"Sending request: {repr(request_data)}")
    
    # Create socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)  # 5 second timeout
    
    try:
        # Connect to server
        sock.connect(('127.0.0.1', port))
        print("Connected to server")
        
        # Send request
        start_time = time.time()
        sock.sendall(request_data.encode('utf-8'))
        print(f"Request sent at {time.time() - start_time:.3f}s")
        
        # Try to receive response
        sock.settimeout(2.0)  # 2 second timeout for response
        try:
            response = sock.recv(4096)
            elapsed = time.time() - start_time
            print(f"Response received at {elapsed:.3f}s: {repr(response)}")
            return response.decode('utf-8', errors='ignore')
        except socket.timeout:
            elapsed = time.time() - start_time
            print(f"TIMEOUT waiting for response after {elapsed:.3f}s")
            return None
            
    except Exception as e:
        print(f"Error: {e}")
        return None
    finally:
        sock.close()

def test_best_completions():
    """Test best-completions operation manually."""
    print("=== Testing Best-Completions Manually ===")
    
    # Protocol format: operation + RS byte + params + RS byte + ... + EOT byte
    # best-completions operation with query "ss" and limit 10
    request = "best-completions\x1ess\x1e10\x04"
    
    response = send_request(7878, request)
    if response:
        print(f"Full response: {response}")
    else:
        print("No response received")

def test_insert():
    """Test insert operation manually."""
    print("\n=== Testing Insert Operation Manually ===")
    
    # Insert operation with single word
    request = "insert\x1ehello\x04"
    
    response = send_request(7878, request)
    if response:
        print(f"Full response: {response}")
    else:
        print("No response received")

def test_simple_connection():
    """Test simple connection without sending data."""
    print("\n=== Testing Simple Connection ===")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(2.0)
    
    try:
        start_time = time.time()
        sock.connect(('127.0.0.1', 7878))
        connect_time = time.time() - start_time
        print(f"Connected in {connect_time:.3f}s")
        
        # Just wait a bit to see if server sends anything
        time.sleep(1)
        
        # Try to receive (should timeout since we didn't send anything)
        sock.settimeout(1.0)
        try:
            data = sock.recv(1024)
            print(f"Received data without sending: {repr(data)}")
        except socket.timeout:
            print("No data received (expected)")
            
    except Exception as e:
        print(f"Error: {e}")
    finally:
        sock.close()

if __name__ == "__main__":
    test_simple_connection()
    test_best_completions()
    test_insert()