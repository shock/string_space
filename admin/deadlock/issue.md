# Server Deadlock/Hanging Issue

## Problem Description
The string_space server intermittently enters a deadlock or hanging state where:
1. It accepts a new connection
2. Shows "Accepting connection..." in logs
3. Then hangs without processing the request or sending a response
4. The connection remains open (doesn't close)
5. Once this happens, all subsequent clients are unable to access the server

## Observed Behavior
From terminal output:
```
New connection from 127.0.0.1:58666
Accepting connection...

Request:
best-completions
ss
10
Response:
"ssh\nssp\nssl\nsshfs\nssmtp\nssh-keygen\nssh-copy-id\nssl_certificate\nssh_host_rsa_key\nssl_certificate_key\n"
Client disconnected
New connection from 127.0.0.1:62508
Accepting connection...

[NO FURTHER OUTPUT - HANGS HERE]
```

## Timing and Trigger Conditions
- **Primary trigger**: Pasting a large amount of text into a Prompt Toolkit buffer
- **Client context**: New client application using the same Python string space completer package
- **Frequency**: Much less frequent after adding rate limiting to client (spacing out requests)
- **Impact**: Once deadlock occurs, server becomes unresponsive to all clients

## Recent Changes
1. Added "best completions" feature to server
2. Added rate limiting/spacing logic to Python completer package to prevent rapid successive calls
3. The rate limiting helped but didn't completely solve the issue

## Code Analysis Findings

### 1. Server Architecture (CRITICAL FINDING)
- **The server is completely synchronous and single-threaded**
- In `src/modules/protocol.rs`, the `run_server` function uses a simple `for stream in listener.incoming()` loop
- Each connection is processed sequentially by `protocol.handle_client(&mut stream)` (line 334)
- **No concurrency or parallelism**: While processing one client, all other clients wait

### 2. File Locking Mechanism
- **NO file locking found in the codebase**
- The `write_to_file` method in `src/modules/string_space/mod.rs` (line 899) simply calls `File::create(file_path)` which truncates the file
- No `flock`, `fcntl`, or other locking mechanisms are used
- This means concurrent file writes from multiple processes could corrupt the file, but shouldn't cause deadlock within a single server process

### 3. Client Behavior Analysis
- Python client (`string_space_client.py`) sends requests and waits for responses
- When pasting text, `add_words_from_text` is called which sends an "insert" request
- The "insert" operation in the server:
  - Inserts words into memory
  - Calls `write_to_file` to persist changes (line 211 in protocol.rs)
  - This file write could be slow for large datasets

### 4. Potential Deadlock Scenarios Identified

#### Scenario A: Blocking File I/O
- `File::create()` and file writing operations are blocking I/O
- If the filesystem is slow or the file is large, the server could hang during `write_to_file`
- While hanging on file I/O, no other clients can be served

#### Scenario B: "best-completions" Algorithm Complexity
- The "best-completions" feature uses multiple algorithms with dynamic weighting
- Could have performance issues with large datasets
- If a query triggers expensive computation, it could block the server

#### Scenario C: Memory Allocation Issues
- The StringSpace uses custom memory management with 4KB-aligned allocations
- Could have issues with memory fragmentation or allocation failures

#### Scenario D: Client Connection Handling
- The `handle_client` method has a loop that reads until EOT byte
- If a client sends malformed data or doesn't send EOT, the server could wait indefinitely
- The server doesn't have timeouts on read operations

### 5. Critical Issue: No Timeouts
- TCP connections have no read/write timeouts
- `reader.read_until(EOT_BYTE, &mut buffer)` (line 240) blocks indefinitely
- If a client connects but doesn't send data (or sends incomplete data), the server hangs

## Root Cause Hypothesis

Based on the analysis, the most likely scenario is:

**Scenario D + Scenario A Combination**:
1. Client pastes text → triggers `add_words_from_text` → sends "insert" request
2. Server accepts connection and starts processing
3. Server inserts words and calls `write_to_file`
4. File write is slow (large dataset, slow filesystem)
5. Meanwhile, another client connects for "best-completions"
6. Second client waits indefinitely because server is blocked on file I/O
7. Server appears deadlocked from client perspective

**OR**

**Malformed Client Request**:
1. Client sends request but doesn't include EOT byte (`\x04`)
2. Server waits indefinitely in `reader.read_until(EOT_BYTE, &mut buffer)`
3. All subsequent clients queue up behind this hanging connection

## Recommended Fixes

### Immediate (Debugging):
1. Add detailed logging with timestamps
2. Add timeout mechanisms for read/write operations
3. Add non-blocking I/O or async processing

### Short-term:
1. Implement connection timeouts
2. Add request timeouts
3. Make file writes asynchronous

### Long-term:
1. Convert server to async/await using Tokio or async-std
2. Implement proper connection pooling
3. Add file locking for multi-process safety

## Next Steps
1. Add debug logging to trace exact hang location
2. Implement TCP read/write timeouts
3. Test with simulated slow file I/O
4. Add request processing timeouts