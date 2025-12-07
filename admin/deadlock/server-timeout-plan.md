# Server Timeout Implementation Plan

## Problem Statement
The server hangs indefinitely at `reader.read_until(EOT_BYTE, &mut buffer)` when clients:
1. Send data without EOT byte (0x04)
2. Keep TCP connection open
3. Server blocks waiting for EOT, preventing all other clients from being served

## Root Cause
- Single-threaded server architecture
- Blocking I/O with no timeouts
- `reader.read_until()` waits forever for EOT byte

## Implementation Plan

### Phase 1: Add TCP Stream Timeouts (Immediate Fix)
**File**: `src/modules/protocol.rs`
**Function**: `handle_client()` (around line 240)

**Changes Required**:
1. Import `std::time::Duration`
2. Define configurable timeout constant with 3-second default
3. Set read/write timeouts on TCP stream after cloning using the constant
4. Handle timeout errors gracefully

**Code Implementation**:
```rust
use std::time::Duration;

// Add near top of protocol.rs, after other constants
const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 3; // 3 second default timeout

// In handle_client() method, after stream cloning:
let timeout_duration = Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT_SECS);
stream.set_read_timeout(Some(timeout_duration))?;
stream.set_write_timeout(Some(timeout_duration))?;
```

**Error Handling**:
- Catch `std::io::ErrorKind::WouldBlock` or `TimedOut`
- Log timeout events for debugging
- Close connection and continue to next client

### Phase 2: Test the Fix
**Primary Testing Method**:
- Run the existing test suite using `make test`
- This will build the server and run comprehensive tests

**Test Procedure**:
1. Build server with timeout implementation: `cargo build`
2. Run full test suite: `make test`
3. Verify all tests pass with timeout implementation
4. If additional testing is needed, create a simple test script to verify timeout behavior:
   ```bash
   # Create test_timeout.sh
   #!/bin/bash
   # Start server with timeout
   # Connect client that doesn't send EOT byte
   # Verify server times out after ~3 seconds
   # Clean up
   ```

### Phase 3: Validation Testing
**Success Criteria**:
- [ ] Server no longer hangs indefinitely
- [ ] Hanging clients timeout after ~3 seconds (default timeout)
- [ ] Normal operation unaffected
- [ ] All existing tests pass (`make test`)
- [ ] Timeout behavior can be demonstrated with test script

**Extended Testing**:
1. Run `make test` to ensure no regressions
2. If needed, create manual test to verify timeout behavior
3. Test with normal client operations to ensure they complete within timeout
4. Verify error handling for timeout events

## Implementation Details

### Timeout Values
- **Default timeout**: 3 seconds (configurable via `DEFAULT_CONNECTION_TIMEOUT_SECS` constant)
- **Shared value**: Same timeout used for both read and write operations
- **Rationale**: 3 seconds is sufficient for normal operations while preventing indefinite blocking
- **Configurability**: Constant can be easily adjusted if needed

### Error Handling Strategy
```rust
match reader.read_until(EOT_BYTE, &mut buffer) {
    Ok(0) => { /* Client disconnected */ }
    Ok(n) => { /* Process request */ }
    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
        eprintln!("Read timeout - closing connection");
        break;
    }
    Err(e) => {
        eprintln!("Read error: {}", e);
        break;
    }
}
```

### Logging
- Add timeout event logging: "Connection timeout after {} seconds" (using the constant value)
- Include client IP address for debugging
- Log to stderr for visibility

## Files to Modify
**Primary**: `src/modules/protocol.rs`
- Add `use std::time::Duration;` at top
- Modify `handle_client()` method
- Add timeout error handling in read loop

**No other files need modification** - this is a targeted fix.

## Rollback Plan
If issues arise:
1. Revert timeout implementation
2. Server returns to original behavior (can hang indefinitely)
3. Document findings for alternative approach

## Timeline
**Phase 1 (Implementation)**: 1-2 hours
**Phase 2 (Testing)**: 1-2 hours
**Phase 3 (Validation)**: 2-3 hours
**Total**: 4-7 hours

## Dependencies
- None - uses standard Rust library features
- No external crates required
- Compatible with existing codebase

## Risks and Mitigations
- **Risk**: Timeout too short for legitimate operations
  - **Mitigation**: Start with 3 seconds (configurable constant), monitor logs, adjust constant if needed
- **Risk**: Timeout errors not handled properly
  - **Mitigation**: Comprehensive error handling with logging
- **Risk**: Performance impact
  - **Mitigation**: Minimal - only adds system calls for timeout setting

## Success Metrics
1. **No indefinite hangs** - server times out after ~3 seconds
2. **Graceful timeout handling** with proper logging
3. **Zero regression** in existing functionality (all `make test` tests pass)
4. **Server recovers automatically** after timeout events

## Next Steps After Implementation
1. Run `make test` to verify no regressions
2. Deploy to production environment
3. Monitor for timeout events in logs
4. Adjust `DEFAULT_CONNECTION_TIMEOUT_SECS` constant if needed based on real usage
5. Consider additional improvements (async I/O, connection pooling)

## Ready to Begin
All investigation work is complete. The codebase is ready for timeout implementation. Start by modifying `src/modules/protocol.rs` to add read/write timeouts to the TCP stream in the `handle_client()` method.