# Debugging Proposal: Server Deadlock Investigation

## Current Understanding

The server hangs **indefinitely** (5+ minutes), not just slow operations. This suggests:

1. **Deadlock in code** (not I/O)
2. **Infinite loop** 
3. **Blocking on a lock that never releases**
4. **Waiting for condition that never occurs**

## Existing Debug Infrastructure Found

### 1. Python Client Debug Flag
The Python client (`string_space_client.py`) already has a `debug` parameter:
```python
def __init__(self, host, port, debug=False):
    self.debug = debug
    # ... prints debug info when debug=True
```

### 2. Rust Debug Functions
Found in `src/modules/string_space/mod.rs`:
- `print_debug_score_candidates()` - prints scoring debug info (line 1335)
- Multiple `#[derive(Debug)]` attributes on structs
- Commented debug prints in `best_completions` method

### 3. Makefile Debug Target
```bash
make debug  # Builds debug version
```

## Immediate Debugging Strategy

### 1. Enable Rust Backtraces
Run server with:
```bash
RUST_BACKTRACE=1 cargo run -- start data.txt
```

For production/debug builds:
```bash
# Debug build (slower but better debugging)
RUST_BACKTRACE=1 cargo run -- start data.txt

# Release build with debug symbols  
RUST_BACKTRACE=1 cargo run --release -- start data.txt
```

### 2. Add Comprehensive Debug Logging to Server

**Current Server Debug Statements**:
- Basic println! in protocol.rs for requests/responses
- No debug levels or flags in Rust server code
- No timestamps
- Existing `print_debug_score_candidates()` but not called

**Proposed Debug System** (add to `protocol.rs`):
```rust
// Use environment variable for debug control
const DEBUG: bool = std::env::var("STRING_SPACE_DEBUG").is_ok();

macro_rules! debug_print {
    ($($arg:tt)*) => {
        if DEBUG {
            println!($($arg)*);
        }
    };
}

macro_rules! debug_print_with_timestamp {
    ($($arg:tt)*) => {
        if DEBUG {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            println!("[{}] {}", now, format!($($arg)*));
        }
    };
}
```

Run with debug enabled:
```bash
STRING_SPACE_DEBUG=1 RUST_BACKTRACE=1 cargo run -- start data.txt
```

### 3. Critical Debug Points to Instrument

#### 3.1 Connection Acceptance (`protocol.rs:334`)
```rust
debug_print_with_timestamp!("ACCEPT: New connection from {}", addr);
debug_print_with_timestamp!("ACCEPT: Starting handle_client");
```

#### 3.2 Request Reading (`protocol.rs:240`)
```rust
debug_print_with_timestamp!("READ: Starting read_until for EOT byte");
match reader.read_until(EOT_BYTE, &mut buffer) {
    Ok(0) => {
        debug_print_with_timestamp!("READ: Client disconnected (0 bytes)");
        break;
    }
    Ok(n) => {
        debug_print_with_timestamp!("READ: Received {} bytes", n);
        debug_print_with_timestamp!("READ: Buffer content (first 100 chars): {:?}", 
            String::from_utf8_lossy(&buffer[..buffer.len().min(100)]));
    }
    Err(e) => {
        debug_print_with_timestamp!("READ: Error: {}", e);
        break;
    }
}
```

#### 3.3 Request Processing (`protocol.rs:273`)
```rust
debug_print_with_timestamp!("PROCESS: Operation: {}, Params: {:?}", 
    request_elements[0], &request_elements[1..]);

let response = self.create_response(request_elements[0], request_elements[1..].to_vec());
debug_print_with_timestamp!("PROCESS: Response generated, length: {}", response.len());
```

#### 3.4 "best-completions" Algorithm (`string_space/mod.rs`)
Enable existing debug and add more:
```rust
// Uncomment or add near line 1052:
// print_debug_score_candidates(&results);

// Add new debug prints:
debug_print_with_timestamp!("BEST-COMPLETIONS: Starting with query='{}', limit={:?}", query, limit);
// At key points in the algorithm
debug_print_with_timestamp!("BEST-COMPLETIONS: Prefix search results: {}", prefix_results.len());
debug_print_with_timestamp!("BEST-COMPLETIONS: Fuzzy search results: {}", fuzzy_results.len());
debug_print_with_timestamp!("BEST-COMPLETIONS: Jaro-Winkler results: {}", jaro_results.len());
debug_print_with_timestamp!("BEST-COMPLETIONS: Final results: {}", final_results.len());
```

#### 3.5 "insert" Operation (`protocol.rs:182-218`)
```rust
debug_print_with_timestamp!("INSERT: Starting with {} params", params.len());
// Inside the word processing loop
debug_print_with_timestamp!("INSERT: Processing word {} of {}", i, words.len());
// After file write
debug_print_with_timestamp!("INSERT: File write attempt, counter={}", counter);
```

### 4. Investigate Specific Deadlock Scenarios

#### 4.1 Infinite Loop in "best-completions"
The algorithm has complex logic with:
- Multiple nested loops (check lines 1287+ in string_space/mod.rs)
- Sorting and scoring
- Could get stuck with certain data patterns

**Check**: Look for loops without proper termination conditions.

#### 4.2 Custom Memory Allocator Issues
The 4KB-aligned memory management (`string_space/mod.rs`):
- `alloc`/`dealloc` calls with unsafe code
- Pointer arithmetic could cause infinite loops
- Buffer bounds checking issues

#### 4.3 Client Protocol Deadlock
The protocol expects EOT byte (`\x04`):
- Client sends data but no EOT → server waits forever
- Buffer parsing edge cases
- Unicode/UTF-8 boundary issues

### 5. Add Timeouts for Diagnosis (Temporary)

Even though timeouts won't fix root cause, they help identify WHERE it hangs:

```rust
use std::time::Duration;

// In handle_client, before read_until
let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));

// For read_until specifically
let start = std::time::Instant::now();
let timeout = Duration::from_secs(30);

// Simple timeout check before read_until
if start.elapsed() > timeout {
    debug_print_with_timestamp!("READ: Timeout waiting for data");
    break;
}
```

### 6. Reproducing the Issue

#### 6.1 Use Python Client with Debug
```python
from string_space_client import StringSpaceClient

client = StringSpaceClient('127.0.0.1', 7878, debug=True)
client.connect()

# Simulate paste operation
text = "some large pasted text with many words"
response = client.add_words_from_text(text)
print(f"Insert response: {response}")

# Immediately try best-completions
completions = client.best_completions_search("so", limit=10)
print(f"Completions: {completions}")
```

#### 6.2 Stress Test Script
```bash
#!/bin/bash
# stress_test.sh

# Start server with debug
STRING_SPACE_DEBUG=1 RUST_BACKTRACE=1 cargo run -- start test_data.txt &
SERVER_PID=$!
sleep 2

# Make rapid concurrent requests
for i in {1..20}; do
    if (( i % 3 == 0 )); then
        # Insert request
        echo -e "insert\nword${i} test${i} example${i}\x04" | nc localhost 7878 &
    else
        # best-completions request  
        echo -e "best-completions\nwo\n10\x04" | nc localhost 7878 &
    fi
done

# Wait and check if server is responsive
sleep 10
echo -e "best-completions\ntest\n5\x04" | timeout 5 nc localhost 7878
if [ $? -ne 0 ]; then
    echo "SERVER HANG DETECTED"
    # Get thread dump
    kill -3 $SERVER_PID
    sleep 1
    # Capture backtrace
    lldb -p $SERVER_PID --batch -o "thread backtrace all" -o "quit"
fi

kill $SERVER_PID 2>/dev/null
```

### 7. Investigation Priority Order

#### Phase 1: Instrument and Observe (Today)
1. Add debug macros with timestamps to `protocol.rs`
2. Add debug prints to `string_space/mod.rs` `best_completions`
3. Run with `STRING_SPACE_DEBUG=1 RUST_BACKTRACE=1`
4. Reproduce the hang with paste operation
5. Identify last debug message before hang

#### Phase 2: Add Timeouts and Capture State (Tomorrow)
1. Add TCP timeouts (30 seconds) for diagnosis
2. Add signal handler for thread dumps (`kill -3`)
3. Capture stack traces when hung
4. Log memory state when timeout occurs

#### Phase 3: Analyze Specific Code Paths
Based on Phase 1 results:
- If hangs in `read_until`: Client protocol issue
- If hangs in `best-completions`: Algorithm infinite loop  
- If hangs in `insert`: Memory/file issue
- If hangs between operations: Synchronization issue

#### Phase 4: Fix Root Cause
Implement fix based on findings.

### 8. Expected Output Format

With debug enabled:
```
[1701901234567] ACCEPT: New connection from 127.0.0.1:12345
[1701901234568] ACCEPT: Starting handle_client
[1701901234569] READ: Starting read_until for EOT byte
[1701901234570] READ: Received 25 bytes
[1701901234571] READ: Buffer content: "best-completions\x1Ess\x1E10"
[1701901234572] PROCESS: Operation: best-completions, Params: ["ss", "10"]
[1701901234573] BEST-COMPLETIONS: Starting with query='ss', limit=Some(10)
[1701901234574] BEST-COMPLETIONS: Prefix search results: 15
[1701901234575] BEST-COMPLETIONS: Fuzzy search results: 12
[1701901234576] BEST-COMPLETIONS: Jaro-Winkler results: 8
[1701901234577] BEST-COMPLETIONS: Final results: 10
[1701901234578] PROCESS: Response generated, length: 120
[1701901234579] WRITE: Starting write_all, 120 bytes
[1701901234580] WRITE: Completed successfully
[1701901234581] Client disconnected
```

### 9. Implementation Steps

#### Step 1: Add Debug Infrastructure (1 hour)
1. Add `DEBUG` constant using env var to `protocol.rs`
2. Add debug macros with timestamps
3. Add similar debug to `string_space/mod.rs`

#### Step 2: Instrument Critical Paths (1 hour)
1. Add debug statements at all key points in `protocol.rs`
2. Add debug to `best_completions` method
3. Include buffer contents and counts

#### Step 3: Test Debug Output (30 min)
1. Run server with normal requests
2. Verify debug output looks correct
3. Ensure no performance issues

#### Step 4: Reproduce and Capture (Variable)
1. Reproduce the hang with paste operation
2. Capture last debug message before hang
3. Run with `RUST_BACKTRACE=1`

#### Step 5: Analyze and Fix (Variable)
1. Based on last message, focus investigation
2. Add more detailed debug in that area
3. Identify and fix root cause

### 10. Risks and Mitigations

#### Risk: Debug Overhead
**Mitigation**: Use simple `if DEBUG` checks, minimal formatting

#### Risk: Missing Debug in Critical Area
**Mitigation**: Start with broad instrumentation, add more as needed

#### Risk: Can't Reproduce Hang
**Mitigation**: Use stress test with concurrent insert/search

#### Risk: Debug Changes Behavior
**Mitigation**: Keep debug statements simple, no side effects

### 11. Success Criteria

1. **Identify exact location** of hang (file + line number)
2. **Capture Rust backtrace** when hung
3. **Understand trigger conditions** (specific requests/data)
4. **Implement fix** that prevents hang
5. **Verify fix** with stress testing

### 12. Immediate Next Actions

1. **Approve this proposal**
2. **Implement Phase 1** (debug infrastructure - 2 hours)
3. **Reproduce the issue** with debug enabled
4. **Analyze results** and proceed to Phase 2

This focused approach will identify where the server is hanging indefinitely, which is the critical first step to fixing it.