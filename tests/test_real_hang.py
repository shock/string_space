#!/usr/bin/env python3
"""
Test that actually reproduces the hang by keeping connection open.
"""

import socket
import time
import threading

def real_hanging_client(port):
    """Client that connects, sends data without EOT, and keeps connection open."""
    print(f"Real hanging client connecting to port {port}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    
    try:
        sock.connect(('127.0.0.1', port))
        print("Real hanging client connected")
        
        # Send partial request without EOT byte
        partial_request = "best-completions\x1ess\x1e10"  # Missing \x04
        sock.sendall(partial_request.encode('utf-8'))
        print("Real hanging client sent partial request (no EOT)")
        print("Client will keep connection open indefinitely...")
        
        # Keep connection open by periodically sending small data
        # This prevents the connection from being closed
        for i in range(60):  # Keep alive for 60 seconds
            try:
                # Send a small keep-alive byte (not EOT)
                sock.sendall(b" ")
                time.sleep(1)
            except:
                break
                
        print("Real hanging client exiting")
        
    except Exception as e:
        print(f"Real hanging client error: {e}")
    finally:
        sock.close()

def test_real_hang(port=9897):
    """Test the real hang scenario."""
    print(f"=== Testing Real Hang Scenario (port: {port}) ===")
    
    # Start real hanging client in background
    hang_thread = threading.Thread(target=real_hanging_client, args=(port,))
    hang_thread.daemon = True
    hang_thread.start()
    
    # Give it time to connect
    time.sleep(1)
    
    # Now try to connect with normal client
    print("\nAttempting to connect with normal client (should hang)...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(3)
    
    try:
        start_time = time.time()
        sock.connect(('127.0.0.1', port))
        connect_time = time.time() - start_time
        print(f"Normal client connected in {connect_time:.3f}s")
        
        # Send complete request
        request = "best-completions\x1ess\x1e5\x04"
        sock.sendall(request.encode('utf-8'))
        print("Normal client sent request")
        
        # Try to receive response (should timeout)
        sock.settimeout(2)
        try:
            response = sock.recv(1024)
            elapsed = time.time() - start_time
            print(f"ERROR: Got response in {elapsed:.3f}s (should have hung)")
            return False
        except socket.timeout:
            elapsed = time.time() - start_time
            print(f"SUCCESS: Normal client timed out after {elapsed:.3f}s")
            print("This confirms server is hung waiting for EOT from hanging client")
            return True
            
    except socket.timeout:
        print("Normal client connection timed out")
        return True
    except Exception as e:
        print(f"Normal client error: {e}")
        return False
    finally:
        sock.close()

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    else:
        port = 7878
    
    success = test_real_hang()
    if success:
        print("\n✓ Real hang reproduced successfully!")
        print("Server is hung at reader.read_until() waiting for EOT byte")
    else:
        print("\n✗ Failed to reproduce real hang")