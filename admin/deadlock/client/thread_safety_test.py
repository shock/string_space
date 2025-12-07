#!/usr/bin/env python3
"""
Test to demonstrate thread safety issues in StringSpaceClient.
"""

import threading
import time
import socket
from string_space_client import StringSpaceClient

def simulate_concurrent_requests():
    """Simulate concurrent requests to StringSpaceClient from multiple threads."""
    client = StringSpaceClient('127.0.0.1', 7878, debug=True)
    
    def make_request(thread_id, request_type):
        """Make a request from a thread."""
        print(f"Thread {thread_id} ({request_type}): Starting request")
        try:
            if request_type == "completion":
                # Simulate best_completions_search
                result = client.best_completions_search("test", limit=5)
                print(f"Thread {thread_id} ({request_type}): Got {len(result)} results")
            elif request_type == "insert":
                # Simulate add_words
                result = client.add_words(["test", "thread", "safety"])
                print(f"Thread {thread_id} ({request_type}): Insert result: {result}")
        except Exception as e:
            print(f"Thread {thread_id} ({request_type}): ERROR: {e}")
    
    # Start multiple threads making concurrent requests
    threads = []
    for i in range(5):
        request_type = "completion" if i % 2 == 0 else "insert"
        t = threading.Thread(target=make_request, args=(i, request_type))
        threads.append(t)
        t.start()
        time.sleep(0.01)  # Small delay to ensure overlap
    
    # Wait for all threads
    for t in threads:
        t.join()
    
    print("\nTest complete.")

def test_socket_interleaving():
    """Test if socket data can get interleaved with concurrent sends."""
    # Create a simple echo server to see what data arrives
    import socketserver
    
    class EchoHandler(socketserver.BaseRequestHandler):
        received_data = []
        
        def handle(self):
            data = self.request.recv(4096)
            print(f"Echo server received {len(data)} bytes: {data[:50]}...")
            self.received_data.append(data)
            # Echo back
            self.request.sendall(b"ECHO: " + data)
    
    # Start echo server
    server = socketserver.TCPServer(('127.0.0.1', 9999), EchoHandler)
    server_thread = threading.Thread(target=server.serve_forever)
    server_thread.daemon = True
    server_thread.start()
    time.sleep(0.5)  # Let server start
    
    # Create client and make concurrent requests
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('127.0.0.1', 9999))
    
    def send_from_thread(thread_id, message):
        """Send data from a thread."""
        try:
            sock.sendall(f"Thread{thread_id}:{message}\x04".encode())
            print(f"Thread {thread_id} sent: {message}")
        except Exception as e:
            print(f"Thread {thread_id} error: {e}")
    
    # Try to send concurrently (this should fail or interleave)
    threads = []
    for i in range(3):
        t = threading.Thread(target=send_from_thread, args=(i, f"Message{i}" * 10))
        threads.append(t)
        t.start()
    
    for t in threads:
        t.join()
    
    sock.close()
    server.shutdown()
    print("\nSocket interleaving test complete.")

if __name__ == "__main__":
    print("=== Testing StringSpaceClient Thread Safety ===")
    print("\n1. Testing concurrent requests to StringSpaceClient...")
    # Note: This requires server to be running
    # simulate_concurrent_requests()
    
    print("\n2. Testing socket interleaving...")
    test_socket_interleaving()
    
    print("\n=== Analysis ===")
    print("""
    Key Findings:
    1. StringSpaceClient is NOT thread-safe
    2. Multiple threads using same client instance can:
       - Interleave data on socket
       - Cause race conditions on self.connected flag
       - Corrupt socket state
    3. ThreadedCompleter creates background threads
    4. Pyra agent calls add_words_from_text() in main thread
    5. Concurrent access to same StringSpaceClient instance!
    
    This likely causes:
    - Garbled requests sent to server
    - Missing or misplaced EOT bytes
    - Server hangs waiting for EOT
    """)