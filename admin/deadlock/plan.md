# Deadlock Investigation Plan

## Issue Summary

The string_space server intermittently enters a deadlock/hanging state where:
1. It accepts a new connection and prints "Accepting connection..."
2. Then hangs without processing the request or sending a response
3. The connection remains open (doesn't close)
4. Once this happens, all subsequent clients are unable to access the server

**Critical Observation**: The hang occurs after "Accepting connection..." but before "Request:" is printed. This means the hang is happening inside `handle_client()` but before request processing begins.

## Phase 1: Identify Exact Hang Location (Priority #1)

### 1.1 Focus on Protocol Flow Debugging

We need to pepper debug statements throughout the protocol handling flow to verify where we're hanging. The key insight from the issue document is section "### 4. What Should Happen After 'Accepting connection...'" - this is where we should start.

**What should happen after "Accepting connection..."**:
1. Stream cloning attempt (silent on success, error printed on failure)
2. Enter read loop (no print until data is received)
3. Blocking read `reader.read_until(EOT_BYTE, &mut buffer)` - hangs here if client doesn't send data
4. Process request - prints "Request:" and operation
5. Create response - calls `create_response()`
6. Send response - prints "Response:" and sends data

Since we don't see stream cloning errors, the hang is most likely at step 3 (blocking read) or inside `create_response()` for certain operations (best-completions or insert).

### 1.2 Debug Version Execution Protocol

**CRITICAL**: From now on until we find the bug, we must:
1. **Always run the debug version of the executable** (`cargo build` or `cargo run`)
2. **Set Rust Backtrace environment variable** to get detailed information:
   - `export RUST_BACKTRACE=1` (shows basic backtrace)
   - `export RUST_BACKTRACE=full` (shows full backtrace with all details)
   - **Recommendation**: Use `RUST_BACKTRACE=full` for maximum detail

### 1.3 Minimal Debug Instrumentation Strategy

Instead of the extensive logging proposed in the resolution document, we'll start with minimal, targeted debug statements to identify the exact hang location:

**File: `src/modules/protocol.rs` - `handle_client()` method**:
1. Add debug print immediately after entering `handle_client()`
2. Add debug print after successful stream cloning
3. Add debug print before `reader.read_until()` call
4. Add debug print after successful read
5. Add debug print before calling `create_response()`
6. Add debug print after `create_response()` returns

**File: `src/modules/protocol.rs` - `create_response()` method**:
1. Add debug print at entry for each operation type (best-completions, insert, prefix)
2. Add debug print before and after algorithm calls
3. Add timing measurements for each major operation

### 1.4 Execution Plan

1. **Build debug version**: `cargo build`
2. **Set environment variables**:
   ```bash
   export RUST_BACKTRACE=full
   export STRING_SPACE_DEBUG=1  # Simple flag for our debug prints
   ```
3. **Start server** with debug enabled
4. **Run client** that triggers the hang (either best-completions or insert with long text)
5. **Capture output** and look for the last debug message before hang
6. **Analyze** to identify exact hang location

### 1.5 Expected Outcomes

We should be able to answer:
1. **Does the hang occur in `reader.read_until()`?** (blocking read)
2. **Does the hang occur inside `create_response()`?**
3. **If inside `create_response()`, which operation causes it?**
4. **If inside an operation, which specific algorithm or function hangs?**

## Phase 2: Root Cause Analysis

Once we identify the exact hang location, we'll:
1. **Analyze the specific function** that's hanging
2. **Check for infinite loops** or blocking operations
3. **Examine memory management** in that area
4. **Review algorithm complexity** for large datasets

## Phase 3: Fix Implementation

Based on findings from Phase 1 and 2:
1. **Implement targeted fix** for the identified issue
2. **Add appropriate timeouts** or early termination
3. **Optimize performance** if needed
4. **Add preventive measures** to avoid similar issues

## Status Tracking

We will maintain a status document called `status.md` in this directory to track our progress through this plan. The status document will be updated after each investigation step.

## Success Criteria

1. **Identify exact location** of hang (file + line number + function)
2. **Understand trigger conditions** (specific operations, query patterns, data size)
3. **Implement fix** that prevents the hang
4. **Verify fix** with stress testing at current database size

## Immediate Next Actions

1. **Implement Phase 1 debug instrumentation** (targeted, minimal)
2. **Test with current database** to reproduce issue
3. **Capture and analyze debug output**
4. **Update status.md** with findings