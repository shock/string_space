# Troubleshooting Proposal: Server Deadlock Investigation
## Updated December 6, 2025

## Investigation Focus
Based on new observations and code analysis, we need to investigate:
1. **Protocol handling flow**: The hang occurs after "Accepting connection..." but before "Request:" is printed
2. **Best Completions Algorithm Bug**: The new `best-completions` feature may have performance issues or infinite loops
3. **Insert Operation Scaling**: The `insert` operation may not handle long sequences of words efficiently
4. **Unsafe memory management** in buffer expansion
5. **Database size thresholds** that may trigger new behavior

## Critical Finding: Hang Location Analysis
The server prints "Accepting connection..." then immediately calls `protocol.handle_client(&mut stream)`. The fact that we don't see "Request:" printed means the hang is happening **inside** `handle_client()` but **before** the request processing begins.

### What Should Happen After "Accepting connection..." (Regardless of Operation):
1. **Stream cloning attempt** (line 228-234) - silent on success, error printed on failure
2. **Enter read loop** - no print until data is received
3. **Blocking read** `reader.read_until(EOT_BYTE, &mut buffer)` (line 240) - hangs here if client doesn't send data
4. **Process request** - prints "Request:" and operation
5. **Create response** - calls `create_response()`
6. **Send response** - prints "Response:" and sends data

Since we don't see stream cloning errors, the hang is most likely at step 3 (blocking read) or inside `create_response()` for certain operations (best-completions or insert).

## Phase 1: Comprehensive Protocol Flow Debugging (Immediate)

### 1.1 Add Protocol Flow Debug Logging
**File**: `src/modules/protocol.rs`

**Add debug macros at top of file**:
```rust
const DEBUG_PROTOCOL_FLOW: bool = std::env::var("STRING_SPACE_PROTOCOL_DEBUG").is_ok();

macro_rules! protocol_debug {
    ($($arg:tt)*) => {
        if DEBUG_PROTOCOL_FLOW {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            println!("[PROTOCOL {}] {}", now, format!($($arg)*));
        }
    };
}
```

**Instrument `handle_client()` with detailed flow tracking**:
```rust
fn handle_client(&mut self, stream: &mut TcpStream) {
    protocol_debug!("ENTER handle_client");
    
    let mut reader = match stream.try_clone() {
        Ok(stream_clone) => {
            protocol_debug!("Stream cloned successfully");
            BufReader::new(stream_clone)
        },
        Err(e) => {
            protocol_debug!("FAILED to clone stream: {}", e);
            eprintln!("Failed to clone stream: {}", e);
            return;
        }
    };

    protocol_debug!("Starting read loop");
    loop {
        let mut buffer = Vec::new();
        protocol_debug!("Waiting for EOT byte (blocking read)...");

        // Read the input from the client until the EOT (End of Text) character is encountered
        match reader.read_until(EOT_BYTE, &mut buffer) {
            Ok(0) => {
                protocol_debug!("Client disconnected (0 bytes read)");
                break;
            }
            Ok(n) => {
                protocol_debug!("Received {} bytes", n);
                if n < 100 {
                    protocol_debug!("Buffer preview: {:?}", String::from_utf8_lossy(&buffer));
                }
            }
            Err(e) => {
                protocol_debug!("Read error: {}", e);
                eprintln!("Failed to read from stream: {}", e);
                break;
            }
        }

        // Remove the EOT character from the buffer
        if let Some(index) = buffer.iter().position(|&b| b == EOT_BYTE) {
            buffer.truncate(index);
            protocol_debug!("EOT found at position {}, buffer length: {}", index, buffer.len());
        } else {
            protocol_debug!("NO EOT found in buffer, client may have disconnected");
            break;
        }

        if buffer.is_empty() {
            protocol_debug!("Empty buffer, continuing loop");
            continue;
        }

        let str_or_err = String::from_utf8(buffer);
        if let Ok(buffer_str) = str_or_err {
            // Split the buffer into a vector of strings using RS_BYTE as the delimiter
            let request_elements: Vec<&str> = buffer_str.split(RS_BYTE_STR).collect();
            protocol_debug!("Parsed request: {} elements", request_elements.len());
            
            if !request_elements.is_empty() {
                protocol_debug!("Operation: {}", request_elements[0]);
                println!("\nRequest:\n{}", request_elements.join("\n"));
            }

            let start_time = std::time::Instant::now();
            let mut response = self.create_response(request_elements[0], request_elements[1..].to_vec());
            let processing_time = start_time.elapsed();
            protocol_debug!("create_response took {:?}", processing_time);
            
            // ... rest of existing code with debug added
        } else {
            protocol_debug!("UTF-8 decode error: {}", str_or_err.unwrap_err());
            // ... error handling
        }
    }
    protocol_debug!("EXIT handle_client");
}
```

### 1.2 Add Operation-Specific Debugging in `create_response()`
**Add to `create_response()` method**:
```rust
fn create_response(&mut self, operation: &str, params: Vec<&str>) -> Vec<u8> {
    protocol_debug!("ENTER create_response: operation='{}', params={:?}", operation, params);
    let start_time = std::time::Instant::now();
    
    let mut response = Vec::new();
    
    if "prefix" == operation {
        protocol_debug!("Processing prefix operation");
        // ... existing code
    }
    else if "best-completions" == operation {
        protocol_debug!("Processing best-completions operation");
        protocol_debug!("Query: '{}', Limit: {:?}", params[0], if params.len() > 1 { Some(params[1]) } else { None });
        
        let query = params[0];
        let limit = if params.len() == 2 {
            match params[1].parse::<usize>() {
                Ok(l) => {
                    protocol_debug!("Parsed limit: {}", l);
                    Some(l)
                },
                Err(_) => {
                    protocol_debug!("Invalid limit parameter: '{}'", params[1]);
                    // ... error handling
                }
            }
        } else {
            protocol_debug!("No limit specified, using default");
            None
        };

        protocol_debug!("Calling space.best_completions()");
        let bc_start = std::time::Instant::now();
        let matches = self.space.best_completions(query, limit);
        let bc_duration = bc_start.elapsed();
        protocol_debug!("best_completions returned {} matches in {:?}", matches.len(), bc_duration);
        
        // ... rest of processing
    }
    else if "insert" == operation {
        protocol_debug!("Processing insert operation with {} params", params.len());
        protocol_debug!("Total param length: {}", params.iter().map(|p| p.len()).sum::<usize>());
        
        let mut counter = 0;
        let mut num_words = 0;
        let insert_start = std::time::Instant::now();
        
        for (i, param) in params.iter().enumerate() {
            if i % 10 == 0 {
                protocol_debug!("Processing param {} of {}", i, params.len());
            }
            
            let string = param.trim();
            // ... existing processing
            
            let words: Vec<&str> = string.split(' ').collect();
            protocol_debug!("Param {} split into {} words", i, words.len());
            
            for (j, word) in words.iter().enumerate() {
                if j % 100 == 0 && j > 0 {
                    protocol_debug!("Inserted {} words so far from param {}", j, i);
                }
                
                let response = self.space.insert_string(word, 1);
                match response {
                    Ok(_) => { counter += 1; },
                    Err(e) => {
                        protocol_debug!("Error inserting word '{}': {}", word, e);
                    }
                }
            }
        }
        
        let insert_duration = insert_start.elapsed();
        protocol_debug!("Insert processing took {:?}, inserted {} of {} words", 
                       insert_duration, counter, num_words);
        
        if counter > 0 {
            protocol_debug!("Writing to file: {}", self.file_path);
            let write_start = std::time::Instant::now();
            if let Err(e) = self.space.write_to_file(&self.file_path) {
                protocol_debug!("File write error: {}", e);
                eprintln!("Failed to write to file {}: {}", self.file_path, e);
            }
            let write_duration = write_start.elapsed();
            protocol_debug!("File write took {:?}", write_duration);
        }
        // ... rest of processing
    }
    // ... other operations
    
    let total_duration = start_time.elapsed();
    protocol_debug!("EXIT create_response: total time {:?}", total_duration);
    response
}
```

### 1.3 Add Best Completions Algorithm Debugging
**File**: `src/modules/string_space/mod.rs`

**Add debug macros**:
```rust
const DEBUG_BEST_COMPLETIONS: bool = std::env::var("DEBUG_BEST_COMPLETIONS").is_ok();
const DEBUG_ALGORITHM_EXECUTION: bool = std::env::var("DEBUG_ALGORITHM_EXECUTION").is_ok();
const DEBUG_INSERT: bool = std::env::var("DEBUG_INSERT").is_ok();
const DEBUG_BUFFER: bool = std::env::var("DEBUG_BUFFER").is_ok();
```

**Add to `best_completions()` method**:
```rust
fn best_completions(&mut self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
    if DEBUG_BEST_COMPLETIONS {
        println!("[BEST-COMPLETIONS] ENTER: query='{}', limit={:?}", query, limit);
        println!("[BEST-COMPLETIONS] Database stats: strings={}, capacity={}, used={}", 
                self.string_refs.len(), self.capacity, self.used_bytes);
    }
    
    let limit = limit.unwrap_or(15);
    
    // Early return for empty database
    if self.empty() {
        if DEBUG_BEST_COMPLETIONS { println!("[BEST-COMPLETIONS] Empty database, returning empty"); }
        return Vec::new();
    }
    
    // Validate query
    if let Err(err) = validate_query(query) {
        if DEBUG_BEST_COMPLETIONS { println!("[BEST-COMPLETIONS] Query validation failed: {}", err); }
        return Vec::new();
    }
    
    // Handle very short queries
    if query.len() == 1 {
        if DEBUG_BEST_COMPLETIONS { println!("[BEST-COMPLETIONS] Single character query, using prefix search"); }
        return self.handle_single_character_query(query, limit);
    }
    
    if DEBUG_BEST_COMPLETIONS { println!("[BEST-COMPLETIONS] Starting progressive algorithm execution"); }
    let algo_start = std::time::Instant::now();
    
    // Use progressive algorithm execution to get initial candidates
    let all_candidates = self.progressive_algorithm_execution(query, limit);
    
    let algo_duration = algo_start.elapsed();
    if DEBUG_BEST_COMPLETIONS { 
        println!("[BEST-COMPLETIONS] progressive_algorithm_execution took {:?}, found {} candidates", 
                algo_duration, all_candidates.len());
    }
    
    // ... rest of method with debug statements
}
```

**Add to `progressive_algorithm_execution()`**:
```rust
fn progressive_algorithm_execution(
    &mut self,
    query: &str,
    limit: usize
) -> Vec<ScoreCandidate> {
    if DEBUG_ALGORITHM_EXECUTION {
        println!("[ALGO-EXEC] ENTER: query='{}', limit={}", query, limit);
    }
    
    let mut all_candidates = Vec::new();
    let mut seen_strings = std::collections::HashSet::new();
    
    // 1. Fast prefix search first (O(log n))
    if DEBUG_ALGORITHM_EXECUTION { println!("[ALGO-EXEC] Starting prefix search"); }
    let prefix_start = std::time::Instant::now();
    let prefix_candidates = self.scored_prefix_search(query).into_iter()
        .take(limit)
        .collect::<Vec<_>>();
    let prefix_duration = prefix_start.elapsed();
    
    if DEBUG_ALGORITHM_EXECUTION { 
        println!("[ALGO-EXEC] Prefix search took {:?}, found {} candidates", 
                prefix_duration, prefix_candidates.len());
    }
    
    // Add unique candidates
    // ... existing code
    
    // 2. Fuzzy subsequence with early termination (O(n) with early exit)
    if DEBUG_ALGORITHM_EXECUTION { println!("[ALGO-EXEC] Starting fuzzy subsequence search"); }
    let fuzzy_start = std::time::Instant::now();
    let fuzzy_candidates = self.fuzzy_subsequence_full_database(
        query,
        limit,
        0.0 // score threshold
    );
    let fuzzy_duration = fuzzy_start.elapsed();
    
    if DEBUG_ALGORITHM_EXECUTION { 
        println!("[ALGO-EXEC] Fuzzy search took {:?}, found {} candidates", 
                fuzzy_duration, fuzzy_candidates.len());
    }
    
    // ... rest of method
}
```

### 1.4 Add Insert Operation Debugging
**Add to `insert_string()` method**:
```rust
pub fn insert_string(&mut self, string: &str, frequency: TFreq) -> Result<(), String> {
    if DEBUG_INSERT {
        println!("[INSERT] ENTER: string='{}', frequency={}, current strings={}, capacity={}, used={}", 
                string, frequency, self.string_refs.len(), self.capacity, self.used_bytes);
    }
    
    // Validate input
    if string.is_empty() {
        if DEBUG_INSERT { println!("[INSERT] Empty string, returning error"); }
        return Err("Cannot insert empty string".to_string());
    }
    
    // Check if we need to grow buffer
    let length = string.len();
    if self.capacity - self.used_bytes < length {
        if DEBUG_INSERT { 
            println!("[INSERT] Insufficient space: needed={}, free={}, triggering grow_buffer", 
                    length, self.capacity - self.used_bytes);
        }
        let grow_start = std::time::Instant::now();
        self.grow_buffer(length);
        let grow_duration = grow_start.elapsed();
        if DEBUG_INSERT { println!("[INSERT] grow_buffer took {:?}", grow_duration); }
    } else if DEBUG_INSERT {
        println!("[INSERT] Adequate space: needed={}, free={}", 
                length, self.capacity - self.used_bytes);
    }
    
    // ... rest of insert logic with debug
    
    if DEBUG_INSERT { println!("[INSERT] EXIT: success, new strings={}", self.string_refs.len()); }
    Ok(())
}
```

### 1.5 Add Buffer Debugging (Enhanced)
**Add to `grow_buffer()` method** (line 793):
```rust
fn grow_buffer(&mut self, required_size: usize) {
    if DEBUG_BUFFER {
        println!("[BUFFER] START grow_buffer: capacity={}, used={}, required={}, strings={}", 
                self.capacity, self.used_bytes, required_size, self.string_refs.len());
    }
    
    let start_time = std::time::Instant::now();
    let new_capacity = self.capacity + std::cmp::max(self.capacity, required_size);
    
    if DEBUG_BUFFER {
        println!("[BUFFER] Calculated new_capacity={} ({}% increase)", 
                new_capacity, ((new_capacity as f64 / self.capacity as f64) * 100.0) - 100.0);
    }
    
    let new_layout = Layout::from_size_align(new_capacity, ALIGNMENT).unwrap();
    if DEBUG_BUFFER { println!("[BUFFER] Allocating {} bytes with alignment {}", new_capacity, ALIGNMENT); }
    
    let new_buffer = unsafe { alloc(new_layout) };
    if DEBUG_BUFFER { println!("[BUFFER] Allocation successful, new_buffer={:?}", new_buffer); }
    
    let mut new_used_bytes = 0;
    if DEBUG_BUFFER { println!("[BUFFER] Starting copy of {} strings", self.string_refs.len()); }
    
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
        
        if i % 1000 == 0 && i > 0 && DEBUG_BUFFER {
            println!("[BUFFER] Copied {} strings, {} bytes", i, new_used_bytes);
        }
    }
    
    if DEBUG_BUFFER { println!("[BUFFER] Copy completed, total bytes copied: {}", new_used_bytes); }
    
    unsafe {
        dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
    }
    if DEBUG_BUFFER { println!("[BUFFER] Old buffer deallocated"); }
    
    self.buffer = new_buffer;
    self.capacity = new_capacity;
    self.used_bytes = new_used_bytes;
    self.all_strings_cache = None;
    
    let duration = start_time.elapsed();
    if DEBUG_BUFFER {
        println!("[BUFFER] COMPLETE grow_buffer: new capacity={}, took {:?}", 
            self.capacity, duration);
    }
}
```

### 1.6 Add Test Execution Plan

#### 1.6.1 Enable All Debugging
```bash
export STRING_SPACE_PROTOCOL_DEBUG=1
export DEBUG_BEST_COMPLETIONS=1
export DEBUG_ALGORITHM_EXECUTION=1
export DEBUG_INSERT=1
export DEBUG_BUFFER=1
export RUST_BACKTRACE=1
```

#### 1.6.2 Reproduce the Issue
1. Start server with debug enabled
2. Run client that triggers the hang (either best-completions or insert with long text)
3. Capture all debug output
4. Look for the last debug message before hang

#### 1.6.3 Expected Debug Output Sequence
```
[PROTOCOL 1765079510123] ENTER handle_client
[PROTOCOL 1765079510124] Stream cloned successfully
[PROTOCOL 1765079510125] Starting read loop
[PROTOCOL 1765079510126] Waiting for EOT byte (blocking read)...
[PROTOCOL 1765079510456] Received 45 bytes
[PROTOCOL 1765079510457] EOT found at position 44, buffer length: 44
[PROTOCOL 1765079510458] Parsed request: 3 elements
[PROTOCOL 1765079510459] Operation: best-completions
[PROTOCOL 1765079510460] ENTER create_response: operation='best-completions', params=["ss", "10"]
[BEST-COMPLETIONS] ENTER: query='ss', limit=Some(10)
[BEST-COMPLETIONS] Database stats: strings=12345, capacity=1048576, used=789432
[ALGO-EXEC] ENTER: query='ss', limit=10
[ALGO-EXEC] Starting prefix search
[ALGO-EXEC] Prefix search took 2.345ms, found 8 candidates
[ALGO-EXEC] Starting fuzzy subsequence search
[ALGO-EXEC] Fuzzy search took 1.234s, found 15 candidates
```

## Phase 2: Analysis and Next Steps (No Timeouts Yet - Per User Request)

### 2.1 Diagnostic Questions to Answer
1. **Where exactly does the hang occur?**
   - Is it in `reader.read_until()` (blocking read)?
   - Is it inside `best_completions()` algorithm?
   - Is it during buffer expansion in `grow_buffer()`?
   - Is it during file write in `write_to_file()`?

2. **What triggers the hang?**
   - Specific query patterns in best-completions?
   - Large insert operations?
   - Database size thresholds?

3. **Is it a true deadlock or just extreme slowness?**
   - Check timing logs to see if operations complete eventually
   - Look for patterns in operation duration

### 2.2 Analysis Plan
1. **Collect debug logs** from reproduction run
2. **Identify last log message** before hang
3. **Check operation durations** - are any operations taking unusually long?
4. **Look for memory patterns** - buffer expansions, allocation failures
5. **Check algorithm performance** - fuzzy search times with current database size

### 2.3 Immediate Actions Based on Findings

#### If hang is in blocking read (`reader.read_until()`):
- Client may not be sending EOT byte
- Network issue or client bug
- **Solution**: Add read timeout (after bug is fixed, not before)

#### If hang is in `best_completions()`:
- Algorithm may have infinite loop or extreme complexity
- Fuzzy subsequence search could be O(n) with large database
- **Solution**: Optimize algorithm or add early termination

#### If hang is in `grow_buffer()`:
- Memory allocation failure or extremely slow copy
- Large database makes buffer expansion expensive
- **Solution**: Optimize buffer management or use incremental expansion

#### If hang is in file write:
- Filesystem performance issue
- Large file writes blocking server
- **Solution**: Async file writes or write to temporary file first

## Phase 3: Implementation Timeline (Updated)

### Day 1: Protocol Flow Instrumentation
1. Add protocol debug logging to `protocol.rs` (2 hours)
2. Add operation-specific debugging to `create_response()` (1 hour)
3. Test debug output with simple requests (1 hour)

### Day 2: Algorithm and Insert Debugging
1. Add best-completions algorithm debugging to `string_space/mod.rs` (2 hours)
2. Add insert operation debugging (1 hour)
3. Add enhanced buffer debugging (1 hour)

### Day 3: Reproduction and Data Collection
1. Set up test environment with all debug enabled (1 hour)
2. Reproduce issue with current database (2 hours)
3. Collect and analyze debug logs (3 hours)

### Day 4: Analysis and Root Cause Identification
1. Analyze logs to identify exact hang location (4 hours)
2. Determine trigger conditions and patterns (2 hours)
3. Document findings and propose fix (2 hours)

### Day 5: Fix Implementation
1. Implement fix based on findings (4 hours)
2. Test fix thoroughly (3 hours)
3. Add preventive measures if needed (1 hour)

## Success Criteria
1. **Identify exact location** of hang (file + line number + function)
2. **Understand trigger conditions** (specific operations, query patterns, data size)
3. **Capture performance metrics** for all operations
4. **Implement fix** that prevents hang
5. **Verify fix** with stress testing at current database size

## Risks and Mitigations
- **Risk**: Debug logging affects performance and changes timing
  - **Mitigation**: Use environment variables to enable/disable, test with and without
- **Risk**: Can't reproduce the hang with debug enabled
  - **Mitigation**: Use current production database, simulate exact client behavior
- **Risk**: Hang is intermittent and hard to capture
  - **Mitigation**: Run extended tests, capture all logs to file for analysis
- **Risk**: Issue is in client code, not server
  - **Mitigation**: Test with multiple clients, capture network traffic

## Immediate Next Actions
1. **Approve this updated proposal**
2. **Implement Phase 1** (protocol flow debugging - 4 hours)
3. **Test with current database** to establish baseline
4. **Reproduce the issue** with debug enabled
5. **Analyze results** and proceed to Phase 2
