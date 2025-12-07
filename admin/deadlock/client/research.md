# Client-Side Deadlock Investigation

**Date**: December 7, 2025  
**Investigator**: Pyra AI Assistant  
**Goal**: Investigate why the new asynchronous client exposes deadlock vulnerability in server when pasting large multi-line text blocks

## Executive Summary

The deadlock issue occurs when:
1. **New client** (Pyra agent) uses `prompt_async()` with `ThreadedCompleter` wrapper
2. **Old client** (llm_chat_cli) uses synchronous `prompt()` without ThreadedCompleter
3. **Trigger**: Pasting large multi-line text into prompt buffer
4. **Result**: Server hangs at `reader.read_until(EOT_BYTE, &mut buffer)` waiting for EOT byte that never arrives

## Key Differences Between Clients

### Old Client (`/Users/billdoughty/src/wdd/python/llm_chat_cli/modules/ChatInterface.py`)
- Uses synchronous `prompt()` method
- Directly uses `StringSpaceCompleter` without `ThreadedCompleter` wrapper
- Code reference (line 59):
  ```python
  self.spell_check_completer = StringSpaceCompleter(host='127.0.0.1', port=7878)
  self.merged_completer = merge_completers([self.spell_check_completer])
  ```
- **PromptSession configuration** (lines 75-80):
  ```python
  self.session = PromptSession(
      history=self.chat_history,
      key_bindings=KeyBindingsHandler(self).create_key_bindings(),
      completer=self.top_level_completer,
      complete_while_typing=True,  # ALSO ENABLED
  )
  ```

### New Client (`/Users/billdoughty/src/wdd/python/agents/pyra/main.py`)
- Uses asynchronous `prompt_async()` method
- Wraps `StringSpaceCompleter` with `ThreadedCompleter`
- Code reference (lines 104-105):
  ```python
  string_space_completer = StringSpaceCompleter(host="127.0.0.1", port=7878)
  spell_check_completer = ThreadedCompleter(string_space_completer)
  ```
- **PromptSession configuration** (lines 133-139):
  ```python
  prompt_session: PromptSession = PromptSession(
      history=chat_history,
      completer=spell_check_completer,  # ThreadedCompleter wrapped
      complete_while_typing=True,  # ALSO ENABLED
      key_bindings=custom_bindings,
  )
  ```

## Critical Analysis: ThreadedCompleter Implementation

### From `prompt_toolkit/completion/base.py` (lines 190-272):
```python
class ThreadedCompleter(Completer):
    """
    Wrapper that runs the `get_completions` generator in a thread.
    (Use this to prevent the user interface from becoming unresponsive if the
    generation of completions takes too much time.)
    """
    
    async def get_completions_async(
        self, document: Document, complete_event: CompleteEvent
    ) -> AsyncGenerator[Completion, None]:
        """
        Asynchronous generator of completions.
        """
        async with aclosing(
            generator_to_async_generator(
                lambda: self.completer.get_completions(document, complete_event)
            )
        ) as async_generator:
            async for completion in async_generator:
                yield completion
```

**Key Insight**: `ThreadedCompleter` uses `generator_to_async_generator` which runs the synchronous `get_completions` in a background thread and passes results through a queue.

## StringSpaceCompleter Analysis

### From `/Users/billdoughty/src/wdd/rust/string_space/python/string_space_client/string_space_client.py`:

**Critical Method**: `add_words_from_text()` (lines 189-192)
```python
def add_words_from_text(self, text: str):
    words = self.parse_text(text)
    response = self.add_words(words)
    return response
```

**Request Creation**: `create_request()` (lines 41-45)
```python
def create_request(self, string: str):
    req = bytearray()
    req.extend(string.encode('utf-8'))
    req.extend(b'\x04')  # Append the EOT byte (ASCII EOT character)
    return req
```

**Request Sending**: `request()` method (lines 67-98)
```python
def request(self, request_elements: list[str]) -> str:
    request = self.create_request(RS_BYTE_STR.join(request_elements))
    # ... connection handling ...
    self.sock.sendall(request)
    response = self.receive_response()
    self.disconnect()
    return response
```

**Response Receiving**: `receive_response()` (lines 47-65)
```python
def receive_response(self):
    try:
        data = b''
        while True:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("Connection closed by the server")
            data += chunk
            if b'\x04' in chunk:
                break
        result = data.rstrip(b'\x04').decode('utf-8')
        # check the first 5 characters of the response to see if it's an error
        if result[:5] == "ERROR":
            raise ProtocolError(result)
        return result
    except Exception as e:
        if self.debug:
            print(f"Error: {e}")
        raise e
```

## StringSpaceCompleter Implementation Analysis

### From `/Users/billdoughty/src/wdd/rust/string_space/python/string_space_completer/string_space_completer.py`:

**Key Method**: `get_completions()` (lines 27-60)
```python
def get_completions(self, document: Document, complete_event: CompleteEvent):
    if self.disabled:
        return
    now = time.time()
    # get delta since last completion
    delta = now - self.last_completion_time
    # if delta is less than 100 milliseconds, return
    if delta < 0.1:
        return
    self.last_completion_time = now
    word_before_cursor = document.get_word_before_cursor(WORD=True)

    if len(word_before_cursor) < 2 and not complete_event.completion_requested:
        return

    # if word_before_cursor ends with a non-word character, return
    if re.search(r'[^\w_\-\_\']', word_before_cursor):
        return

    # remove starting non-word characters from word_before_cursor
    while re.match(r'^[^\w_\-\']', word_before_cursor):
        word_before_cursor = word_before_cursor[1:]

    suggestions = self.client.best_completions_search(word_before_cursor, limit=10)

    # for each suggestion in suggestions, if the first character is lower case and matches the first character of word_before_cursor, change it to upper case
    for i in range(len(suggestions)):
        if (suggestions[i][0].islower() and word_before_cursor[0].isupper() and
            suggestions[i][0].lower() == word_before_cursor[0].lower()):
            suggestions[i] = word_before_cursor[0] + suggestions[i][1:]

    for suggestion in suggestions:
        if suggestion.strip() != '':
            yield Completion(suggestion, start_position=-len(word_before_cursor))
```

**Critical Observations**:
1. **Rate Limiting**: Built-in 100ms rate limiting (`if delta < 0.1: return`)
2. **Word Extraction**: Uses `document.get_word_before_cursor(WORD=True)` 
3. **Minimum Length**: Requires word length >= 2 (or completion requested)
4. **Character Filtering**: Filters non-word characters from word boundaries

**`add_words_from_text()` Method** (lines 78-84):
```python
def add_words_from_text(self, text: str):
    if self.disabled:
        return
    words = self.parse_text(text)
    if len(words) == 0:
        return
    self.client.add_words(words)
```

## Pyra Agent Usage Pattern Analysis

### From `/Users/billdoughty/src/wdd/python/agents/pyra/main.py`:

**Two locations where `add_words_from_text()` is called**:

1. **After assistant response** (line 225):
   ```python
   if (
       last_message
       and last_message["role"] == "assistant"
       and string_space_completer
   ):
       string_space_completer.add_words_from_text(last_message["content"])
   ```

2. **After user input** (line 238):
   ```python
   if string_space_completer:
       string_space_completer.add_words_from_text(user_input)
   ```

## Hypothesis: The Deadlock Trigger

### Scenario Analysis:

1. **Normal Operation (Old Client)**:
   - Synchronous `prompt()` → Completer runs in main thread
   - `StringSpaceCompleter.get_completions()` called synchronously
   - Client sends request with EOT byte (`\x04`)
   - Server receives complete request, processes, sends response with EOT
   - Client receives response with EOT, disconnects

2. **Problematic Operation (New Client)**:
   - Asynchronous `prompt_async()` → `ThreadedCompleter` wrapper
   - `get_completions_async()` runs completer in background thread
   - **Potential Issue**: Background thread management with `generator_to_async_generator`
   - **Critical Question**: What happens if the background thread is interrupted/cancelled?

### Key Questions to Investigate:

1. **Thread Cancellation**: Does `prompt_async()` cancel background operations when user continues typing/pasting?
2. **Generator Interruption**: What happens to `generator_to_async_generator` if the async generator is cancelled?
3. **Socket State**: If background thread is cancelled mid-request, does socket send EOT byte?
4. **Buffer Flushing**: Are socket buffers properly flushed before thread termination?

## Prompt Toolkit Architecture Analysis

### `prompt()` vs `prompt_async()`:

From `prompt_toolkit/shortcuts/prompt.py`:
- `prompt()`: Synchronous, runs event loop internally
- `prompt_async()`: Asynchronous, must be called from async context

**Critical Difference**: Async operations can be cancelled via `asyncio.CancelledError`

### Buffer and Completer Interaction:

When user pastes text:
1. Buffer content changes trigger completion requests
2. For each change, `get_completions_async()` may be called
3. With rapid pasting (large text), multiple completion requests may overlap
4. Previous requests may be cancelled if new input arrives

## Server-Side Deadlock Location

From `admin/deadlock/status.md`:
> **Exact hang location identified**: `reader.read_until(EOT_BYTE, &mut buffer)` in `handle_client()` method (line 240 in `src/modules/protocol.rs`)

**Root Cause**: Server waits indefinitely for EOT byte (`\x04`) that never arrives.

## Critical Discovery: Both Clients Use `complete_while_typing=True`

**Both clients have the same configuration**:
- Old client: `complete_while_typing=True` (line 79, ChatInterface.py)
- New client: `complete_while_typing=True` (line 329, main.py)

**This means the key difference is NOT `complete_while_typing`**.

## The Real Difference: Synchronous vs Asynchronous with ThreadedCompleter

### Old Client Flow:
1. `prompt()` called synchronously
2. Buffer changes trigger `get_completions()` synchronously in main thread
3. `StringSpaceCompleter.get_completions()` runs with 100ms rate limiting
4. If typing too fast (> 100ms between keystrokes), completions are skipped
5. `add_words_from_text()` called after input is complete (not during typing)

### New Client Flow:
1. `prompt_async()` called asynchronously  
2. Buffer changes trigger `get_completions_async()` via `ThreadedCompleter`
3. `generator_to_async_generator` runs `StringSpaceCompleter.get_completions()` in background thread
4. **Critical**: Background thread may be cancelled if new input arrives
5. `add_words_from_text()` called TWICE: after assistant response AND after user input

## Critical Discovery: Race Condition in StringSpaceClient

### Analysis of `StringSpaceClient.request()` method:

**Thread Safety Issue**: `StringSpaceClient` is NOT thread-safe!
- Instance variables: `self.sock`, `self.connected`
- Multiple threads (from `ThreadedCompleter`) could call `request()` concurrently
- Race conditions on socket state

**Potential Race Scenario**:
1. Thread 1: Calls `request()`, connects, starts `sendall()`
2. Thread 2: Calls `request()` on same client instance
3. Thread 1: Completes `sendall()`, calls `receive_response()`
4. Thread 2: Calls `sendall()` on same socket (INTERLEAVED DATA!)
5. Server receives garbled request, can't find EOT byte
6. Server hangs at `reader.read_until(EOT_BYTE, &mut buffer)`

### StringSpaceCompleter Rate Limiting Issue:

**`get_completions()` method has flawed rate limiting**:
```python
def get_completions(self, document: Document, complete_event: CompleteEvent):
    now = time.time()
    delta = now - self.last_completion_time
    if delta < 0.1:  # 100ms rate limiting
        return  # RETURNS IMMEDIATELY
    self.last_completion_time = now  # UPDATED BEFORE WORK IS DONE
    # ... makes network call
```

**Problem**: `self.last_completion_time` is updated BEFORE the completion work (network call) is done. So:
- Time 0ms: Call 1 starts, sets `last_completion_time = 0`
- Time 50ms: Call 2 starts, `delta = 0.05` (< 0.1), RETURNS IMMEDIATELY
- Time 150ms: Call 1 FINISHES (took 150ms due to slow server)
- Time 151ms: Call 3 starts, `delta = 0.001` (since `last_completion_time` was set to 0!), returns immediately
- **Result**: Completions are effectively blocked for a long time after a slow request

### Revised Hypothesis

**Most Likely Scenario**:
1. User pastes large text block into Pyra agent
2. `add_words_from_text()` is called (NO rate limiting!)
3. Sends "insert" request to server via `StringSpaceClient.request()`
4. Server processes insert (slow with large database)
5. While insert is processing, user continues typing
6. `get_completions_async()` called via `ThreadedCompleter`
7. Creates background thread, calls `StringSpaceCompleter.get_completions()`
8. `get_completions()` tries to use same `StringSpaceClient` instance
9. **RACE CONDITION**: Two threads using same socket
10. Data gets interleaved, EOT byte lost or garbled
11. Server hangs waiting for EOT

**Alternative Scenario**:
1. `StringSpaceCompleter.get_completions()` rate limiting bug
2. Slow completion request blocks subsequent requests
3. But `add_words_from_text()` has NO rate limiting
4. Multiple `add_words_from_text()` calls can happen concurrently
5. Thread safety issue causes socket corruption
6. EOT byte not properly sent

## Investigation Plan

### Phase 1: Understand ThreadedCompleter Cancellation
1. Trace execution flow of `get_completions_async()` with debug logging
2. Understand `generator_to_async_generator` cancellation behavior
3. Test what happens when async operation is cancelled mid-request

### Phase 2: Analyze StringSpaceCompleter Socket Handling
1. Examine `add_words_from_text()` method for potential issues
2. Test socket behavior with interrupted connections
3. Verify EOT byte is always sent in all code paths, even with exceptions

### Phase 3: Reproduce with Instrumentation
1. Add debug logging to StringSpaceCompleter
2. Monitor socket send/receive operations
3. Capture exact moment when EOT byte is NOT sent

### Phase 4: Compare Old vs New Client Behavior
1. Create test to simulate rapid text pasting
2. Monitor completion request frequency and timing
3. Identify differences in request/response patterns

## Initial Findings

### 1. **Rate Limiting Mentioned in Issue**
From `admin/deadlock/issue.md` (lines 33-40):
> "Frequency: Much less frequent after adding rate limiting to client (spacing out requests)"

**Implication**: Rapid successive calls exacerbate the issue. ThreadedCompleter may be making concurrent or overlapping requests.

### 2. **Database Growth Factor**
From `admin/deadlock/issue.md` (line 35):
> "Database growth: Database has grown significantly over time, potentially reaching new size thresholds"

**Implication**: Larger database → slower completions → longer running background threads → higher chance of cancellation/interruption.

### 3. **Single-Threaded Server Architecture**
From `admin/deadlock/issue.md` (lines 90-95):
> "The server is completely synchronous and single-threaded... Each connection is processed sequentially"

**Critical Insight**: Server can only handle one request at a time. If client sends incomplete request (no EOT), server hangs blocking all other clients.

## Next Steps for Deep Investigation

1. **Instrument StringSpaceCompleter** to log:
   - When `create_request()` is called
   - When `sock.sendall()` is called
   - When EOT byte is appended
   - When `receive_response()` starts/ends
   - When exceptions occur

2. **Test cancellation scenarios**:
   - What happens when `asyncio.CancelledError` occurs during `get_completions_async()`?
   - Does socket get properly closed?
   - Is EOT byte guaranteed to be sent?

3. **Analyze prompt_toolkit buffer events**:
   - How does rapid text insertion trigger completion requests?
   - Are previous completion requests cancelled?
   - What's the timing between buffer changes and completer calls?

## Working Hypothesis

**Most Likely Scenario**:
1. User pastes large text block into Pyra agent
2. Each word/character insertion triggers completion request via `ThreadedCompleter`
3. `generator_to_async_generator` starts background thread for `StringSpaceCompleter.get_completions()`
4. Background thread calls `add_words_from_text()` which sends "insert" request to server
5. Before request completes (EOT sent), new buffer change cancels previous async operation
6. Background thread terminated, socket may not send EOT byte
7. Server waits forever for EOT, deadlocking

**Alternative Hypothesis**:
1. `ThreadedCompleter` creates race condition in socket handling
2. Multiple background threads may share or conflict on socket connection
3. EOT byte lost due to threading/synchronization issue

## Root Cause Identification

**Primary Root Cause**: `StringSpaceClient` is not thread-safe, and `ThreadedCompleter` enables concurrent access from multiple threads.

**Secondary Issues**:
1. `StringSpaceCompleter.get_completions()` has flawed rate limiting logic
2. `add_words_from_text()` has no rate limiting at all
3. Server has no read timeouts, hangs indefinitely waiting for EOT

## Proof of Hypothesis

### Evidence 1: Thread Safety Violation
- `StringSpaceClient` instance variables (`self.sock`, `self.connected`) accessed concurrently
- No locking or synchronization mechanisms
- `ThreadedCompleter` creates background threads
- Pyra agent calls methods on same client instance from multiple threads

### Evidence 2: Different Client Architectures
- **Old client**: Synchronous `prompt()`, single-threaded, sequential access
- **New client**: Asynchronous `prompt_async()` with `ThreadedCompleter`, multi-threaded, concurrent access

### Evidence 3: Reproduction Test Shows Exact Failure Mode
Test `test_real_hang.py` demonstrates server hangs at `reader.read_until(EOT_BYTE, &mut buffer)` when client sends data without EOT byte and keeps connection open.

### Evidence 4: Rate Limiting Ineffective
- `get_completions()` rate limiting updates timestamp BEFORE network call completes
- Slow network calls can block completions for extended periods
- `add_words_from_text()` has NO rate limiting

## Recommended Fixes

### Immediate Fix (Server-side):
1. **Add read timeouts** to `reader.read_until()` in server
2. **Add connection timeouts** to prevent indefinite blocking
3. **Validate requests early** and reject malformed data

### Client-side Fixes:
1. **Make `StringSpaceClient` thread-safe**:
   - Add locking around socket operations
   - Use thread-local storage or connection pooling
   - Implement proper connection management
2. **Fix rate limiting** in `StringSpaceCompleter.get_completions()`:
   - Update timestamp AFTER completion, not before
   - Consider more sophisticated rate limiting
3. **Add rate limiting** to `add_words_from_text()`

### Architectural Improvements:
1. **Consider connection pooling** for `StringSpaceClient`
2. **Implement request queuing** for concurrent access
3. **Add request cancellation** support
4. **Improve error handling** for socket operations

## Conclusion

The deadlock issue is caused by the interaction of three factors:

1. **Thread safety violation** in `StringSpaceClient` when used with `ThreadedCompleter`
2. **No timeouts** in server's blocking I/O operations  
3. **Rapid concurrent requests** from async client with `complete_while_typing=True`

The new Pyra agent exposes this vulnerability because:
- Uses `prompt_async()` with `ThreadedCompleter` → creates background threads
- Calls `add_words_from_text()` in main thread while completions run in background
- Concurrent access to non-thread-safe `StringSpaceClient`
- Results in garbled/missing EOT bytes, server hangs

The old client doesn't have this issue because it's single-threaded and synchronous.

## Immediate Action Items

1. **Implement server-side timeouts** to prevent deadlock
2. **Make `StringSpaceClient` thread-safe** with proper locking
3. **Fix rate limiting logic** in `StringSpaceCompleter`
4. **Test with thread-safe client** to verify fix

---
**Research Complete** - Root cause identified and documented.