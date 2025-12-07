# Client-Side Deadlock Investigation

## Overview

This directory contains the investigation into why the new Pyra agent client exposes a deadlock vulnerability in the string_space server when pasting large multi-line text blocks.

## Investigation Files

### Core Documents
1. **`research.md`** - Complete investigation log with analysis, hypotheses, and findings
2. **`summary.md`** - Executive summary of root cause and recommendations  
3. **`code_issues.md`** - Detailed code analysis with specific issues and proposed fixes

### Supporting Files
4. **`thread_safety_test.py`** - Test to demonstrate thread safety issues in StringSpaceClient

## Key Findings

### Root Cause
The deadlock is caused by **thread safety violations in `StringSpaceClient`** when used with `ThreadedCompleter` in the new asynchronous Pyra agent client.

### Contributing Factors
1. **`StringSpaceClient` is not thread-safe** - race conditions on socket and connection state
2. **Flawed rate limiting** in `StringSpaceCompleter.get_completions()` - updates timestamp before network call completes
3. **No rate limiting** on `add_words_from_text()` method
4. **Server has no read timeouts** - hangs indefinitely waiting for EOT byte
5. **Single-threaded server architecture** - one hanging client blocks all others

### Why Old Client Doesn't Have This Issue
- Uses synchronous `prompt()` method (single-threaded)
- Doesn't use `ThreadedCompleter` wrapper
- Sequential access to `StringSpaceClient` (no concurrency)

### Why New Client Exposes the Issue
- Uses asynchronous `prompt_async()` with `ThreadedCompleter`
- Creates background threads for completion requests
- Concurrent access to non-thread-safe `StringSpaceClient`
- Results in garbled/missing EOT bytes

## Failure Scenario

1. User pastes large text into Pyra agent
2. `add_words_from_text()` called in main thread → sends "insert" request
3. While server processes insert (slow with large database), user continues typing
4. `get_completions_async()` called via `ThreadedCompleter` → background thread
5. Concurrent access to same `StringSpaceClient` instance
6. Race condition causes socket data interleaving/corruption
7. EOT byte lost or garbled in transmission
8. Server hangs forever waiting for EOT byte

## Recommended Fixes

### Priority 1: Server-side (Immediate)
1. **Add read timeouts** to `reader.read_until()` operations
2. **Implement connection timeouts** to prevent indefinite blocking
3. **Add request validation** to reject malformed data early

### Priority 2: Client-side (Essential)
1. **Make `StringSpaceClient` thread-safe** with proper locking
2. **Fix rate limiting logic** in `StringSpaceCompleter.get_completions()`
3. **Add rate limiting** to `add_words_from_text()`

### Priority 3: Testing
1. **Create thread safety tests** for `StringSpaceClient`
2. **Reproduce issue** with instrumented client
3. **Test fixes** with both old and new clients

## Investigation Status

✅ **COMPLETE** - Root cause identified and documented

**Next Steps**: Implement server-side timeouts immediately, then fix client thread safety issues.

---
**Investigator**: Pyra AI Assistant  
**Date**: December 7, 2025  
**Location**: `/Users/billdoughty/src/wdd/rust/string_space/admin/deadlock/client/`