# Troubleshooting Proposal: Server Deadlock Investigation

## Investigation Focus
Given the user's additional information about unsafe Rust code and database growth, we need to focus on:
1. **Unsafe memory management** in buffer expansion
2. **Database size thresholds** that may trigger new behavior
3. **Buffer expansion logging** to track when and how often it occurs

## Phase 1: Instrumentation and Data Collection (Immediate)

### 1.1 Add Buffer Size and Expansion Logging
**File**: `src/modules/string_space/mod.rs`

**Add to StringSpace struct**:
```rust
// Add debug flag
const DEBUG_BUFFER: bool = std::env::var("STRING_SPACE_BUFFER_DEBUG").is_ok();

macro_rules! buffer_debug {
    ($($arg:tt)*) => {
        if DEBUG_BUFFER {
            println!("[BUFFER] {}", format!($($arg)*));
        }
    };
}
```

**Add to `grow_buffer()` method** (line 793):
```rust
fn grow_buffer(&mut self, required_size: usize) {
    buffer_debug!("START grow_buffer: capacity={}, used={}, required={}", 
        self.capacity, self.used_bytes, required_size);
    
    let start_time = std::time::Instant::now();
    let new_capacity = self.capacity + std::cmp::max(self.capacity, required_size);
    
    buffer_debug!("Calculated new_capacity={} ({}% increase)", 
        new_capacity, ((new_capacity as f64 / self.capacity as f64) * 100.0) - 100.0);
    
    let new_layout = Layout::from_size_align(new_capacity, ALIGNMENT).unwrap();
    buffer_debug!("Allocating {} bytes with alignment {}", new_capacity, ALIGNMENT);
    
    let new_buffer = unsafe { alloc(new_layout) };
    buffer_debug!("Allocation successful, new_buffer={:?}", new_buffer);
    
    let mut new_used_bytes = 0;
    buffer_debug!("Starting copy of {} strings", self.string_refs.len());
    
    for (i, ref_info) in self.string_refs.iter_mut().enumerate() {
        let old_slice = unsafe {
            std::slice::from_raw_parts(self.buffer.add(ref_info.pointer), ref_info.length)
        };
        unsafe {
            ptr::copy_nonoverlapping(
                old_slice.as_ptr(),
                new_buffer.add(new_used_bytes),
                ref_info.length
            );
        }
        ref_info.pointer = new_used_bytes;
        new_used_bytes += ref_info.length;
        
        if i % 1000 == 0 && i > 0 {
            buffer_debug!("Copied {} strings, {} bytes", i, new_used_bytes);
        }
    }
    
    buffer_debug!("Copy completed, total bytes copied: {}", new_used_bytes);
    
    unsafe {
        dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
    }
    buffer_debug!("Old buffer deallocated");
    
    self.buffer = new_buffer;
    self.capacity = new_capacity;
    self.used_bytes = new_used_bytes;
    self.all_strings_cache = None;
    
    let duration = start_time.elapsed();
    buffer_debug!("COMPLETE grow_buffer: new capacity={}, took {:?}", 
        self.capacity, duration);
}
```

**Add to `insert()` method** (around line 680):
```rust
if self.capacity - self.used_bytes < length {
    buffer_debug!("Insufficient space: capacity={}, used={}, needed={}, free={}", 
        self.capacity, self.used_bytes, length, self.capacity - self.used_bytes);
    self.grow_buffer(length);
} else {
    buffer_debug!("Adequate space: capacity={}, used={}, needed={}, free={}", 
        self.capacity, self.used_bytes, length, self.capacity - self.used_bytes);
}
```

### 1.2 Add Startup Buffer Logging
**File**: `src/modules/string_space/mod.rs`

**Add to `new()` or initialization method**:
```rust
pub fn new() -> Self {
    let inner = StringSpaceInner::new();
    buffer_debug!("StringSpace created: capacity={}, used={}", 
        inner.capacity, inner.used_bytes);
    StringSpace { inner }
}

// Or add to load_from_file method
pub fn load_from_file(file_path: &str) -> io::Result<Self> {
    let inner = StringSpaceInner::load_from_file(file_path)?;
    buffer_debug!("StringSpace loaded from {}: capacity={}, used={}, strings={}", 
        file_path, inner.capacity, inner.used_bytes, inner.string_refs.len());
    Ok(StringSpace { inner })
}
```

### 1.3 Add Memory Allocation Failure Handling
**File**: `src/modules/string_space/mod.rs`

**Modify `grow_buffer()` allocation**:
```rust
let new_buffer = unsafe { alloc(new_layout) };
if new_buffer.is_null() {
    buffer_debug!("CRITICAL: Memory allocation failed for {} bytes", new_capacity);
    panic!("Failed to allocate {} bytes for StringSpace buffer", new_capacity);
}
buffer_debug!("Allocation successful, new_buffer={:?}", new_buffer);
```

## Phase 2: Enhanced Debug Logging

### 2.1 Add Comprehensive Server Debugging
**File**: `src/modules/protocol.rs`

**Add debug macros**:
```rust
const DEBUG_SERVER: bool = std::env::var("STRING_SPACE_SERVER_DEBUG").is_ok();

macro_rules! server_debug {
    ($($arg:tt)*) => {
        if DEBUG_SERVER {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            println!("[SERVER {}] {}", now, format!($($arg)*));
        }
    };
}
```

**Instrument `handle_client()`**:
```rust
fn handle_client(&mut self, stream: &mut TcpStream) {
    server_debug!("START handle_client");
    
    let mut reader = match stream.try_clone() {
        Ok(stream_clone) => {
            server_debug!("Stream cloned successfully");
            BufReader::new(stream_clone)
        },
        Err(e) => {
            server_debug!("Failed to clone stream: {}", e);
            return;
        }
    };

    loop {
        let mut buffer = Vec::new();
        server_debug!("Waiting for EOT byte...");
        
        match reader.read_until(EOT_BYTE, &mut buffer) {
            Ok(0) => {
                server_debug!("Client disconnected (0 bytes)");
                break;
            }
            Ok(n) => {
                server_debug!("Received {} bytes", n);
                if n < 100 {
                    server_debug!("Buffer content: {:?}", String::from_utf8_lossy(&buffer));
                }
            }
            Err(e) => {
                server_debug!("Read error: {}", e);
                break;
            }
        }
        // ... rest of processing
    }
    server_debug!("END handle_client");
}
```

### 2.2 Add Operation Timing
**File**: `src/modules/protocol.rs`

**Instrument `create_response()`**:
```rust
fn create_response(&mut self, operation: &str, params: Vec<&str>) -> Vec<u8> {
    server_debug!("START create_response: operation={}, params={:?}", operation, params);
    let start_time = std::time::Instant::now();
    
    let response = match operation {
        "insert" => {
            server_debug!("Processing insert with {} params", params.len());
            // ... existing code with timing
            let result = self.handle_insert(params);
            server_debug!("Insert completed in {:?}", start_time.elapsed());
            result
        }
        "best-completions" => {
            server_debug!("Processing best-completions: query='{}'", params[0]);
            let result = self.handle_best_completions(params);
            server_debug!("Best-completions completed in {:?}", start_time.elapsed());
            result
        }
        // ... other operations
    };
    
    server_debug!("END create_response: total time {:?}", start_time.elapsed());
    response
}
```

## Phase 3: Timeout Implementation

### 3.1 Add TCP Timeouts
**File**: `src/modules/protocol.rs`

**Modify `handle_client()`**:
```rust
use std::time::Duration;

fn handle_client(&mut self, stream: &mut TcpStream) {
    // Set timeouts
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    
    server_debug!("Timeouts set: read=30s, write=30s");
    // ... rest of function
}
```

### 3.2 Add Read Timeout with Progress Reporting
**Enhanced read loop**:
```rust
let mut total_wait_time = Duration::from_secs(0);
let timeout = Duration::from_secs(30);
let check_interval = Duration::from_secs(5);

loop {
    let mut buffer = Vec::new();
    server_debug!("Waiting for EOT byte (waited {:?} so far)", total_wait_time);
    
    // Simulate timeout by checking periodically
    let read_start = std::time::Instant::now();
    match reader.read_until(EOT_BYTE, &mut buffer) {
        Ok(0) => {
            server_debug!("Client disconnected (0 bytes) after {:?}", read_start.elapsed());
            break;
        }
        Ok(n) => {
            server_debug!("Received {} bytes after {:?}", n, read_start.elapsed());
            // Process request
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
                total_wait_time += read_start.elapsed();
                if total_wait_time >= timeout {
                    server_debug!("TIMEOUT: No data received for {:?}", total_wait_time);
                    break;
                }
                continue;
            }
            server_debug!("Read error: {}", e);
            break;
        }
    }
}
```

## Phase 4: Reproduction and Testing

### 4.1 Test Script for Buffer Expansion
**File**: `test_buffer_expansion.rs`

```rust
use std::env;
use std::process::Command;

fn main() {
    // Set debug environment variables
    env::set_var("STRING_SPACE_BUFFER_DEBUG", "1");
    env::set_var("STRING_SPACE_SERVER_DEBUG", "1");
    env::set_var("RUST_BACKTRACE", "1");
    
    println!("Starting server with buffer debug...");
    
    // Start server
    let mut server = Command::new("cargo")
        .args(["run", "--", "start", "data.txt"])
        .spawn()
        .expect("Failed to start server");
    
    // Wait for server to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Send insert requests to trigger buffer expansion
    println!("Sending insert requests...");
    for i in 0..100 {
        let words: Vec<String> = (0..100).map(|j| format!("testword{}_{}", i, j)).collect();
        let request = format!("insert\n{}\x04", words.join(" "));
        
        // Send request via netcat or custom client
        // This is a simplified example
        println!("Insert batch {}", i);
        
        if i % 10 == 0 {
            // Check server responsiveness
            println!("Checking server responsiveness...");
        }
    }
    
    // Cleanup
    server.kill().expect("Failed to kill server");
}
```

### 4.2 Monitor Script
**File**: `monitor_buffer.sh`

```bash
#!/bin/bash

# Monitor buffer expansion events
echo "Starting buffer expansion monitor..."
echo "Press Ctrl+C to stop"

# Tail server logs and filter for buffer events
tail -f server.log | grep -E "\[BUFFER\]|grow_buffer|capacity=|CRITICAL:" | while read line; do
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] $line"
    
    # Alert on potential issues
    if echo "$line" | grep -q "CRITICAL:"; then
        echo "ALERT: Critical buffer issue detected!"
    fi
    
    if echo "$line" | grep -q "grow_buffer"; then
        echo "INFO: Buffer expansion occurred"
    fi
done
```

## Phase 5: Analysis Plan

### 5.1 Data to Collect
1. **Buffer expansion frequency**: How often does `grow_buffer()` get called?
2. **Expansion size**: What are the capacity increases?
3. **Execution time**: How long does each expansion take?
4. **Memory patterns**: Are there memory allocation failures?
5. **Correlation with hangs**: Do hangs occur during or after buffer expansion?

### 5.2 Analysis Questions
1. Is the hang occurring inside `grow_buffer()`?
2. Is memory allocation failing for large buffers?
3. Are pointer calculations correct for large datasets?
4. Does the copy loop have any edge cases with large numbers of strings?
5. Is the deallocation of old buffer causing issues?

### 5.3 Expected Debug Output
```
[BUFFER] StringSpace loaded from data.txt: capacity=1048576, used=789432, strings=12345
[SERVER 1701901234567] START handle_client
[SERVER 1701901234568] Received 45 bytes
[SERVER 1701901234569] START create_response: operation=insert, params=["word1", "word2", ...]
[BUFFER] Insufficient space: capacity=1048576, used=1048000, needed=1000, free=576
[BUFFER] START grow_buffer: capacity=1048576, used=1048000, required=1000
[BUFFER] Calculated new_capacity=2097152 (100% increase)
[BUFFER] Allocating 2097152 bytes with alignment 4096
[BUFFER] Allocation successful, new_buffer=0x7f8a5c000000
[BUFFER] Starting copy of 12345 strings
[BUFFER] Copied 1000 strings, 45678 bytes
[BUFFER] Copied 2000 strings, 91234 bytes
...
[BUFFER] Copy completed, total bytes copied: 1049000
[BUFFER] Old buffer deallocated
[BUFFER] COMPLETE grow_buffer: new capacity=2097152, took 2.345s
[SERVER 1701901234570] Insert completed in 2.567s
[SERVER 1701901234571] END handle_client
```

## Phase 6: Implementation Timeline

### Day 1: Instrumentation
1. Add buffer debug logging to `string_space/mod.rs`
2. Add server debug logging to `protocol.rs`
3. Test debug output with simple requests

### Day 2: Timeout Implementation
1. Add TCP timeouts to `handle_client()`
2. Implement progress reporting for long operations
3. Test timeout behavior

### Day 3: Reproduction Testing
1. Create test script to trigger buffer expansion
2. Run with current database to reproduce issue
3. Collect debug logs

### Day 4: Analysis
1. Analyze logs for patterns
2. Identify exact hang location
3. Determine if issue is in buffer expansion, file I/O, or elsewhere

### Day 5: Fix Implementation
1. Implement fix based on findings
2. Add additional safety checks if needed
3. Test fix thoroughly

## Success Criteria
1. **Identify exact location** of hang (file + line number + function)
2. **Understand trigger conditions** (buffer size threshold, specific operations)
3. **Capture performance metrics** for buffer expansion operations
4. **Implement fix** that prevents hang
5. **Verify fix** with stress testing at current database size

## Risks and Mitigations
- **Risk**: Debug logging affects performance
  - **Mitigation**: Use environment variables to enable/disable
- **Risk**: Can't reproduce the hang
  - **Mitigation**: Use current database and simulate paste operations
- **Risk**: Buffer expansion is correct but extremely slow
  - **Mitigation**: Optimize copy loop or implement incremental expansion
- **Risk**: Memory allocation failures
  - **Mitigation**: Add proper error handling and fallback strategies

## Immediate Next Actions
1. **Approve this proposal**
2. **Implement Phase 1** (buffer debug logging - 2 hours)
3. **Test with current database** to establish baseline
4. **Reproduce the issue** with debug enabled
5. **Analyze results** and proceed to Phase 2