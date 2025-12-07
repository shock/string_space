# Client-Side Deadlock Investigation - Summary

## Investigation Complete: December 7, 2025

## Executive Summary

The deadlock issue in the string_space server is triggered by the new Pyra agent client due to thread safety violations in `StringSpaceClient` when used with `ThreadedCompleter`. The asynchronous nature of the new client exposes a race condition that causes missing or garbled EOT bytes, leading the server to hang indefinitely.

## Root Cause Analysis

### Primary Cause: Thread Safety Violation
- **`StringSpaceClient` is not thread-safe** - instance variables (`self.sock`, `self.connected`) accessed concurrently
- **`ThreadedCompleter` enables concurrent access** from multiple background threads
- **Pyra agent architecture** allows main thread and background threads to access same client instance simultaneously

### Secondary Contributing Factors:
1. **Server has no read timeouts** - hangs indefinitely at `reader.read_until(EOT_BYTE, &mut buffer)`
2. **Flawed rate limiting** in `StringSpaceCompleter.get_completions()` - updates timestamp before network call completes
3. **No rate limiting** on `add_words_from_text()` method
4. **Single-threaded server architecture** - one hanging client blocks all others

## Evidence

### 1. Architectural Differences Between Clients
| Aspect | Old Client (llm_chat_cli) | New Client (Pyra agent) |
|--------|---------------------------|-------------------------|
| Prompt method | Synchronous `prompt()` | Asynchronous `prompt_async()` |
| Completer | Direct `StringSpaceCompleter` | `ThreadedCompleter` wrapper |
| Threading | Single-threaded, sequential | Multi-threaded, concurrent |
| `complete_while_typing` | True | True (same) |

### 2. Reproduction Evidence
- Test `test_real_hang.py` demonstrates exact failure mode
- Server hangs at `reader.read_until(EOT_BYTE, &mut buffer)` when EOT byte missing
- Client can send partial data without EOT and keep connection open

### 3. Code Analysis Evidence
- `StringSpaceClient.request()` method has race conditions on `self.connected` flag
- No locking or synchronization in client code
- `ThreadedCompleter` documentation warns about thread safety of wrapped completers

## Failure Scenario

1. **User pastes large text** into Pyra agent prompt
2. **`add_words_from_text()` called** in main thread → sends "insert" request
3. **While insert processing**, user continues typing
4. **`get_completions_async()` called** via `ThreadedCompleter` → creates background thread
5. **Background thread** calls `StringSpaceCompleter.get_completions()`
6. **Concurrent access** to same `StringSpaceClient` instance
7. **Race condition** causes socket data interleaving or corruption
8. **EOT byte lost or garbled** in transmission
9. **Server waits forever** for EOT byte, deadlocking

## Recommended Fixes

### Priority 1: Server-side (Immediate)
1. **Add read timeouts** to `reader.read_until()` operations
2. **Implement connection timeouts** to prevent indefinite blocking
3. **Add request validation** to reject malformed data early

### Priority 2: Client-side (Essential)
1. **Make `StringSpaceClient` thread-safe**:
   - Add `threading.Lock` around critical sections
   - Implement connection pooling or thread-local storage
   - Proper error handling for concurrent access
2. **Fix rate limiting logic**:
   - Update `last_completion_time` AFTER network call completes
   - Add rate limiting to `add_words_from_text()`
3. **Consider architectural changes**:
   - Use separate client instances per thread
   - Implement request queuing for concurrent access

### Priority 3: Testing
1. **Create thread safety tests** for `StringSpaceClient`
2. **Reproduce issue** with instrumented client to confirm hypothesis
3. **Test fixes** with both old and new clients

## Conclusion

The deadlock issue is a classic concurrency problem exposed by the architectural differences between the old synchronous client and new asynchronous client. The `ThreadedCompleter` wrapper, while solving UI responsiveness issues, introduces thread safety requirements that `StringSpaceClient` doesn't meet.

**Key Insight**: The issue isn't in the server's core logic, but in the client-server protocol handling when clients behave incorrectly. However, the server should be resilient to malformed clients by implementing timeouts.

**Recommendation**: Implement server-side timeouts immediately as a safety measure, then fix client thread safety issues to prevent the deadlock from occurring.

---
**Investigation Status**: COMPLETE  
**Root Cause**: IDENTIFIED  
**Next Steps**: IMPLEMENT FIXES