# Implementation Plan: Fix Server Deadlock Issue

## Phase 1: Immediate Fixes (Critical)

### 1.1 Add TCP Timeouts
**File**: `src/modules/protocol.rs`
**Function**: `handle_client`

**Changes**:
```rust
use std::time::Duration;

fn handle_client(&mut self, stream: &mut TcpStream) {
    // Set timeouts on the stream
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    
    // Clone stream with timeouts
    let mut reader = match stream.try_clone() {
        Ok(stream_clone) => {
            let _ = stream_clone.set_read_timeout(Some(Duration::from_secs(30)));
            BufReader::new(stream_clone)
        },
        Err(e) => {
            eprintln!("Failed to clone stream: {}", e);
            return;
        }
    };
    
    // Rest of the function...
}
```

### 1.2 Add Detailed Logging with Timestamps
**File**: `src/modules/protocol.rs`
**Functions**: `handle_client`, `create_response`

**Changes**:
```rust
use chrono::Local;

fn handle_client(&mut self, stream: &mut TcpStream) {
    println!("[{}] Starting handle_client", Local::now().format("%H:%M:%S%.3f"));
    
    // ... existing code ...
    
    println!("[{}] Reading request...", Local::now().format("%H:%M:%S%.3f"));
    match reader.read_until(EOT_BYTE, &mut buffer) {
        Ok(0) => {
            println!("[{}] Client disconnected (read 0 bytes)", Local::now().format("%H:%M:%S%.3f"));
            break;
        }
        Ok(n) => {
            println!("[{}] Read {} bytes", Local::now().format("%H:%M:%S%.3f"), n);
        }
        Err(e) => {
            eprintln!("[{}] Failed to read from stream: {}", Local::now().format("%H:%M:%S%.3f"), e);
            break;
        }
    }
    
    // ... rest of function ...
}

fn create_response(&mut self, operation: &str, params: Vec<&str>) -> Vec<u8> {
    println!("[{}] create_response: operation='{}', params={:?}", 
        Local::now().format("%H:%M:%S%.3f"), operation, params);
    
    match operation {
        "insert" => {
            println!("[{}] Insert operation with {} params", 
                Local::now().format("%H:%M:%S%.3f"), params.len());
            // ... existing code ...
            if counter > 0 {
                println!("[{}] Writing to file: {}", 
                    Local::now().format("%H:%M:%S%.3f"), self.file_path);
                if let Err(e) = self.space.write_to_file(&self.file_path) {
                    eprintln!("[{}] Failed to write to file {}: {}", 
                        Local::now().format("%H:%M:%S%.3f"), self.file_path, e);
                }
                println!("[{}] File write completed", 
                    Local::now().format("%H:%M:%S%.3f"));
            }
        }
        "best-completions" => {
            println!("[{}] Best-completions operation with query='{}'", 
                Local::now().format("%H:%M:%S%.3f"), params[0]);
            // ... existing code ...
        }
        // ... other operations ...
    }
}
```

### 1.3 Make File Writes Asynchronous (Optional)
**File**: `src/modules/protocol.rs`
**Function**: `create_response` (insert section)

**Alternative approach - defer file writes**:
```rust
else if "insert"  == operation {
    // ... existing insert logic ...
    
    if counter > 0 {
        // Defer file write - don't block response
        let file_path = self.file_path.clone();
        let space = self.space.clone(); // Need to add Clone trait to StringSpace
        
        std::thread::spawn(move || {
            println!("[{}] Starting async file write", Local::now().format("%H:%M:%S%.3f"));
            if let Err(e) = space.write_to_file(&file_path) {
                eprintln!("[{}] Failed to write to file {}: {}", 
                    Local::now().format("%H:%M:%S%.3f"), file_path, e);
            }
            println!("[{}] Async file write completed", Local::now().format("%H:%M:%S%.3f"));
        });
    }
    
    // Return response immediately
    let response_str = format!("OK\nInserted {} of {} words", counter, num_words);
    response.extend_from_slice(response_str.as_bytes());
    return response;
}
```

## Phase 2: Medium-term Improvements

### 2.1 Thread Pool for Connection Handling
**New File**: `src/modules/thread_pool.rs`
**Changes to `run_server`**:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

struct ThreadPool {
    workers: Vec<Worker>,
    sender: std::sync::mpsc::Sender<Job>,
}

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        // Create channel for jobs
        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        
        ThreadPool { workers, sender }
    }
    
    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

// Update run_server to use thread pool
pub fn run_server<F>(host: &str, port: u16, protocol: Box<dyn Protocol>, bind_success: Option<F>) -> io::Result<()>
where F: FnMut()
{
    let listener = TcpListener::bind(format!("{}:{}", host, port))?;
    
    if let Some(mut bind_success) = bind_success {
        bind_success();
    }
    
    println!("TCP protocol handler listening on {}:{}", host, port);
    
    // Create thread pool with 4 workers
    let pool = ThreadPool::new(4);
    
    // Wrap protocol in Arc<Mutex<>> for thread safety
    let protocol = Arc::new(Mutex::new(protocol));
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let protocol = Arc::clone(&protocol);
                
                pool.execute(move || {
                    let mut protocol = protocol.lock().unwrap();
                    match stream.peer_addr() {
                        Ok(addr) => {
                            println!("New connection from {}", addr);
                            println!("Accepting connection...");
                            protocol.handle_client(&mut stream);
                        },
                        Err(e) => {
                            eprintln!("Failed to get peer address: {}", e);
                            protocol.handle_client(&mut stream);
                        }
                    }
                });
            },
            Err(e) => { eprintln!("Failed: {}", e); },
        }
    }
    
    Ok(())
}
```

## Phase 3: Testing Strategy

### 3.1 Test Cases to Implement

#### Test 1: Slow File I/O Simulation
```rust
#[test]
fn test_slow_file_io_doesnt_block_server() {
    // Create test with mocked slow file system
    // Verify other requests can be processed while file write is in progress
}
```

#### Test 2: Client Timeout Handling
```rust
#[test]
fn test_client_timeout() {
    // Create client that doesn't send EOT byte
    // Verify server times out after 30 seconds
}
```

#### Test 3: Concurrent Requests
```rust
#[test]
fn test_concurrent_requests() {
    // Simulate multiple clients making requests simultaneously
    // Verify all requests are processed
}
```

### 3.2 Performance Testing
```bash
# Benchmark script
#!/bin/bash
# test_concurrent_requests.sh

# Start server
cargo run -- start test_data.txt &

# Wait for server to start
sleep 2

# Make 10 concurrent requests
for i in {1..10}; do
    echo "Making request $i"
    echo -e "best-completions\nss\n10\x04" | nc localhost 7878 &
done

# Wait for all requests
wait

# Kill server
pkill -f "string_space"
```

## Phase 4: Deployment Plan

### 4.1 Step-by-Step Implementation

1. **Week 1**: Implement TCP timeouts and logging
   - Add `chrono` dependency to Cargo.toml
   - Modify `handle_client` to set timeouts
   - Add detailed logging throughout

2. **Week 2**: Test timeout behavior
   - Create test cases for timeouts
   - Verify server recovers from hung connections
   - Test with malformed client requests

3. **Week 3**: Implement async file writes (optional)
   - Add `Clone` trait to `StringSpace` if needed
   - Spawn threads for file writes
   - Test with large datasets

4. **Week 4**: Implement thread pool
   - Create thread pool module
   - Update `run_server` to use thread pool
   - Test concurrent request handling

### 4.2 Rollback Plan

If issues arise:
1. Revert to previous version (git checkout)
2. Keep logging changes for debugging
3. Disable timeouts if causing connection issues

### 4.3 Monitoring

Add monitoring metrics:
- Request count per second
- Average response time
- Timeout count
- File write duration

## Risks and Mitigations

### Risk 1: Timeouts Too Aggressive
**Mitigation**: Start with 30-second timeout, adjust based on testing

### Risk 2: Thread Safety Issues
**Mitigation**: Use `Arc<Mutex<>>` for shared protocol state

### Risk 3: Performance Overhead
**Mitigation**: Profile before and after changes

### Risk 4: Client Compatibility
**Mitigation**: Maintain existing protocol, only add timeouts on server side

## Success Criteria

1. Server no longer hangs indefinitely
2. Timeouts recover gracefully
3. Concurrent requests are handled properly
4. File writes don't block other requests
5. Performance remains acceptable