#!/usr/bin/env python3
"""
Test that simulates a hanging client that connects but doesn't send EOT byte.
"""

import socket
import time
import threading

def hanging_client(port):
    """Client that connects but doesn't send EOT byte."""
    print(f"Hanging client connecting to port {port}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)
    
    try:
        sock.connect(('127.0.0.1', port))
        print("Hanging client connected (will not send EOT)")
        
        # Send partial request without EOT byte
        partial_request = "best-completions\x1ess\x1e10"  # Missing \x04
        sock.sendall(partial_request.encode('utf-8'))
        print("Hanging client sent partial request (no EOT)")
        
        # Wait indefinitely (simulating hanging client)
        time.sleep(30)
        print("Hanging client exiting")
        
    except Exception as e:
        print(f"Hanging client error: {e}")
    finally:
        sock.close()

def normal_client(port, client_id):
    """Normal client that sends complete request."""
    time.sleep(1)  # Wait for hanging client to connect first
    
    print(f"Normal client {client_id} connecting...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)
    
    try:
        start_time = time.time()
        sock.connect(('127.0.0.1', port))
        connect_time = time.time() - start_time
        print(f"Normal client {client_id} connected in {connect_time:.3f}s")
        
        # Send complete request
        request = "best-completions\x1ess\x1e5\x04"
        sock.sendall(request.encode('utf-8'))
        print(f"Normal client {client_id} sent request")
        
        # Try to receive response
        sock.settimeout(3.0)
        try:
            response = sock.recv(4096)
            elapsed = time.time() - start_time
            print(f"Normal client {client_id} received response in {elapsed:.3f}s: {len(response)} bytes")
            return True
        except socket.timeout:
            elapsed = time.time() - start_time
            print(f"Normal client {client_id} TIMEOUT after {elapsed:.3f}s")
            return False
            
    except Exception as e:
        print(f"Normal client {client_id} error: {e}")
        return False
    finally:
        sock.close()

def test_hanging_scenario(port=7878):
    """Test scenario where one client hangs and blocks others."""
    print(f"=== Testing Hanging Client Scenario (port: {port}) ===")
    
    # Start hanging client in background thread
    hang_thread = threading.Thread(target=hanging_client, args=(port,))
    hang_thread.daemon = True
    hang_thread.start()
    
    # Give hanging client time to connect
    time.sleep(0.5)
    
    # Try multiple normal clients
    results = []
    for i in range(3):
        print(f"\n--- Normal client {i+1} attempt ---")
        success = normal_client(port, i+1)
        results.append(success)
        time.sleep(1)
    
    # Check results
    successful = sum(results)
    print(f"\n=== Results: {successful}/3 normal clients succeeded ===")
    
    if successful < 3:
        print("WARNING: Some clients were blocked by hanging client!")
        print("This confirms the single-threaded server issue.")
    else:
        print("All clients succeeded (unexpected - server might handle this case)")

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
        test_hanging_scenario(port)
    else:
        test_hanging_scenario()