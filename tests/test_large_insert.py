#!/usr/bin/env python3
"""
Test large insert operation to reproduce hang.
"""

import socket
import time
import random
import string

def generate_large_text(num_words=1000, word_length=10):
    """Generate large text with many words."""
    words = []
    for i in range(num_words):
        # Generate random word
        word = ''.join(random.choices(string.ascii_lowercase, k=random.randint(3, word_length)))
        words.append(word)
    return " ".join(words)

def send_large_insert(port, text):
    """Send large insert request to server."""
    print(f"Sending insert request with {len(text.split())} words ({len(text)} chars)")
    
    # Protocol format: insert + RS byte + text + EOT byte
    request = f"insert\x1e{text}\x04"
    
    # Create socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(30.0)  # 30 second timeout for large insert
    
    try:
        # Connect to server
        start_time = time.time()
        sock.connect(('127.0.0.1', port))
        connect_time = time.time() - start_time
        print(f"Connected in {connect_time:.3f}s")
        
        # Send request
        send_start = time.time()
        sock.sendall(request.encode('utf-8'))
        send_time = time.time() - send_start
        print(f"Request sent in {send_time:.3f}s")
        
        # Try to receive response
        sock.settimeout(10.0)  # 10 second timeout for response
        try:
            response = sock.recv(65536)  # Larger buffer for possible large response
            elapsed = time.time() - start_time
            print(f"Response received at {elapsed:.3f}s")
            print(f"Response preview: {repr(response[:100])}...")
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

def test_progressive_inserts():
    """Test progressively larger inserts to find threshold."""
    print("=== Testing Progressive Inserts ===")
    
    sizes = [10, 50, 100, 500, 1000, 2000]
    
    for size in sizes:
        print(f"\n--- Testing insert with {size} words ---")
        text = generate_large_text(size)
        response = send_large_insert(7878, text)
        
        if response:
            print(f"Success: {response[:50]}...")
        else:
            print(f"FAILED: No response for {size} words")
            break
        
        # Small delay between tests
        time.sleep(0.5)

def test_concurrent_connections():
    """Test multiple concurrent connections."""
    print("\n=== Testing Concurrent Connections ===")
    
    # Create 5 connections simultaneously
    sockets = []
    for i in range(5):
        print(f"Creating connection {i+1}...")
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5.0)
        try:
            sock.connect(('127.0.0.1', 7878))
            sockets.append(sock)
            print(f"  Connection {i+1} established")
        except Exception as e:
            print(f"  Connection {i+1} failed: {e}")
    
    # Send requests from all connections
    print("\nSending requests from all connections...")
    for i, sock in enumerate(sockets):
        try:
            request = f"best-completions\x1ess\x1e5\x04"
            sock.sendall(request.encode('utf-8'))
            print(f"  Sent request from connection {i+1}")
        except Exception as e:
            print(f"  Failed to send from connection {i+1}: {e}")
    
    # Try to receive responses
    print("\nWaiting for responses...")
    for i, sock in enumerate(sockets):
        sock.settimeout(2.0)
        try:
            response = sock.recv(4096)
            print(f"  Received response from connection {i+1}: {len(response)} bytes")
        except socket.timeout:
            print(f"  Timeout from connection {i+1}")
        except Exception as e:
            print(f"  Error from connection {i+1}: {e}")
    
    # Close all sockets
    for sock in sockets:
        sock.close()
    print("All connections closed")

if __name__ == "__main__":
    # First test progressive inserts
    test_progressive_inserts()
    
    # Then test concurrent connections
    test_concurrent_connections()