# Next Steps for Deadlock Fix

## Current State
- **Investigation Complete**: Root cause identified and verified
- **Reproduction Scripts**: Created and tested
- **Debug Instrumentation**: Already in place in `protocol.rs`
- **Status Document**: Fully updated with findings

## Exact Problem
The server hangs at line 240 in `src/modules/protocol.rs`:
```rust
match reader.read_until(EOT_BYTE, &mut buffer) {
```
When a client:
1. Connects to the server
2. Sends data WITHOUT the EOT byte (0x04)
3. Keeps the TCP connection open
4. Server waits indefinitely for EOT byte

## Root Cause
- Single-threaded server architecture
- Blocking I/O with no timeouts
- Sequential connection processing
- No read timeout on `reader.read_until()`

## Files to Modify
**Primary file**: `src/modules/protocol.rs`
- Function: `handle_client()` (around line 240)
- Need to add timeout to `reader.read_until()`

## Test Scripts Available
1. **`tests/reproduce_deadlock.sh`** - Main reproduction script
2. **`tests/test_real_hang.py`** - Python script that reproduces hang
3. **`tests/test_hanging_client.py`** - Test client utilities

## Reproduction Steps
```bash
cd /Users/billdoughty/src/wdd/rust/string_space
tests/reproduce_deadlock.sh
```

## Fix Implementation Plan

### Option 1: Add Read Timeout (Recommended)
Add timeout to the TCP stream before creating the BufReader:
```rust
use std::time::Duration;

// In handle_client() method, after stream cloning:
stream.set_read_timeout(Some(Duration::from_secs(30)))?;
stream.set_write_timeout(Some(Duration::from_secs(30)))?;
```

### Option 2: Use Non-blocking I/O
Make the server non-blocking or use async I/O.

### Option 3: Thread Pool
Implement thread pool to handle multiple connections concurrently.

### Immediate Action (Recommended)
Implement **Option 1** (read timeout) as it's:
1. Simple to implement
2. Prevents indefinite blocking
3. Maintains current architecture
4. Easy to test

## Testing the Fix
After implementing the fix:
1. Run reproduction script: `tests/reproduce_deadlock.sh`
2. The hanging client should timeout after 30 seconds
3. Normal clients should still work
4. Server should recover automatically

## Success Criteria
- [ ] Server no longer hangs indefinitely
- [ ] Hanging clients timeout after reasonable period (30 seconds)
- [ ] Normal operation unaffected
- [ ] All existing tests pass
- [ ] Reproduction script shows timeout instead of hang

## Ready to Begin
All investigation work is complete. The codebase is ready for fix implementation. Start by modifying `src/modules/protocol.rs` to add read timeouts to the TCP stream in the `handle_client()` method.