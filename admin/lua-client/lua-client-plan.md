# String-Space Lua Client: Implementation Plan

**Created:** 2026-05-05
**Status:** Planning — implementation not yet started
**Parent context:** `admin/lua-client/context.md`

---

## Goal

Create a Lua TCP client for the string-space server, co-located with the existing Python, TypeScript, and Rust components. The client will be consumed by a Neovim nvim-cmp completion source (built separately in the Neovim config project).

## Context

String-space is a Rust TCP server with an in-memory word database providing multi-algorithm completion. Existing clients:

- **Python**: `python/string_space_client/string_space_client.py` (~280 lines) — synchronous socket, `threading.RLock` serialization
- **TypeScript**: `typescript/string-space-client.ts` (~270 lines) — async TCP via Node `net`, promise-chain serialization

This plan creates the Lua equivalent, using Neovim's `vim.loop` (libuv) for TCP transport.

**Consumers**: A nvim-cmp custom source will `require('string-space-client')` and call `best_completions()` to provide as-you-type word completion. The module lives on Neovim's runtimepath (symlinked from the Neovim config project).

---

## Development Dependencies

| Dependency | Status | Purpose |
|------------|--------|---------|
| **Neovim ≥ 0.9** | ✅ Installed (v0.11.6) | Runtime: provides `vim.loop` (libuv TCP), `vim.uv` in newer versions |
| **`nvim -l`** | ✅ Available | Test runner: executes Lua scripts headlessly |
| **Lua/LuaJIT** | ✅ Available (`/opt/homebrew/bin/lua`, `/opt/homebrew/bin/luajit`) | Not required — all testing uses `nvim -l` since `vim.loop` is the transport |
| **Lua test framework** | ❌ Not needed | Using simple assert-based tests (same approach as TS client) |
| **Luarocks packages** | ❌ Not needed | No external Lua dependencies — `vim.loop` is built into Neovim |
| **Rust server** | ✅ Buildable | Integration tests require a running `string_space` daemon |
| **Test data** | ✅ Existing | `test/word_list.txt` (~10k words) |

**Summary: Zero new development dependencies required.** Everything needed is already on this machine.

---

## Protocol Specification

Same RS/EOT wire protocol as Python and TypeScript clients.

### Wire Format

**Request:**
```
<element_1><0x1E><element_2><0x1E>...<element_N><0x04>
```

**Response:**
```
<response_text><0x04>
```

### Constants

| Constant | Value | Notes |
|----------|-------|-------|
| EOT byte | `0x04` | Terminates request and response |
| RS byte | `0x1E` | Separates request elements |
| Connection timeout | 3 seconds | Server-side read/write timeout |
| Min word length | 3 chars | Server rejects shorter |
| Max word length | 50 chars | Server rejects longer |

### Commands Used by This Client

| Command | Params | Response |
|---------|--------|----------|
| `insert` | `<word1>\n<word2>\n...` | `OK\nInserted N of M words` |
| `best-completions` | `<query><RS><limit>` | newline-separated matches |
| `prefix` | `<prefix>` | newline-separated matches |
| `substring` | `<substring>` | newline-separated matches |
| `similar` | `<word><RS><threshold>` | newline-separated matches |
| `fuzzy-subsequence` | `<query>` | newline-separated matches |
| `data-file` | (none) | file path string |

---

## Reference Implementation: TypeScript Client

**File**: `typescript/string-space-client.ts` (~270 lines)

### Key Patterns to Port

| TypeScript Pattern | Lua Equivalent |
|---|---|
| `net.createConnection()` | `vim.loop.new_tcp()` |
| `Promise<string>` resolve/reject | Callback: `function(response, err)` |
| Promise-chain queue (`this.queue`) | Coroutines or manual queue table |
| `Buffer.concat([response, chunk])` | Table accumulation + `table.concat()` |
| `chunk.includes(EOT)` byte search | `string.find(data, string.char(0x04))` |
| `socket.setTimeout()` | `vim.loop.timer()` |
| `socket.destroy()` | `tcp:close()` |
| `this.debug` flag | `self.debug` flag + `print()` |
| `async/await` | Callbacks throughout |

### Connection Pattern (to replicate)

```
connect → send → accumulate response chunks until EOT → parse → close → callback(results, err)
```

---

## Implementation

### File Layout

```
lua/
├── string-space-client/
│   └── init.lua          -- The client module
└── test_client.lua       -- Standalone integration test script
```

### API Surface

```lua
local StringSpaceClient = {}
StringSpaceClient.__index = StringSpaceClient

-- Constructor
function StringSpaceClient.new(host, port, debug)
    -- host defaults to "127.0.0.1", port defaults to 7878
end

-- Search (all callback-based: callback(results, err))
function StringSpaceClient:best_completions(query, limit, callback) end
function StringSpaceClient:prefix_search(prefix, callback) end
function StringSpaceClient:substring_search(substring, callback) end
function StringSpaceClient:similar_search(word, threshold, callback) end
function StringSpaceClient:fuzzy_subsequence_search(query, callback) end

-- Mutation
function StringSpaceClient:insert(words, callback) end
function StringSpaceClient:add_words_from_text(text, callback) end

-- Health
function StringSpaceClient:is_available() end  -- synchronous, positive result cached for 30s

-- Utility
function StringSpaceClient:data_file(callback) end
```

**Why callbacks, not coroutines**: The nvim-cmp source will call `best_completions` from a completion callback and provide its own callback to receive results. The libuv event loop handles the async I/O naturally. Coroutines (`coroutine.wrap`) would add complexity without benefit for the Neovim consumer.

### Default Values

- `host`: `'127.0.0.1'`
- `port`: `7878`
- `debug`: `false`

### Internal: Transport Layer

```lua
function StringSpaceClient:_request(elements, callback)
    local tcp = vim.loop.new_tcp()

    local request_str = table.concat(elements, string.char(0x1E)) .. string.char(0x04)
    local response_parts = {}
    local settled = false

    local function settle(fn)
        if settled then return end
        settled = true
        fn()
    end

    -- Client-side timeout (matches TS client's socket.setTimeout(3000))
    local timer = vim.loop.new_timer()
    timer:start(3000, 0, function()
        settle(function()
            tcp:close()
            timer:close()
            callback(nil, "Connection timeout")
        end)
    end)

    tcp:connect(self.host, self.port, function(err)
        if err then
            timer:close()
            settle(function() tcp:close(); callback(nil, err) end)
            return
        end

        tcp:write(request_str, function(write_err)
            if write_err then
                timer:close()
                settle(function() tcp:close(); callback(nil, write_err) end)
                return
            end
        end)

        -- read_start is registered at connect level (fires as data arrives)
        tcp:read_start(function(read_err, data)
            if read_err then
                timer:close()
                settle(function() tcp:close(); callback(nil, read_err) end)
                return
            end
            if not data then
                timer:close()
                settle(function() tcp:close(); callback(nil, "Connection closed by server") end)
                return
            end
            table.insert(response_parts, data)
            local full = table.concat(response_parts)
            if full:find(string.char(0x04), 1, true) then
                timer:close()
                settle(function()
                    tcp:close()
                    local text = full:gsub(string.char(0x04) .. "+$", "")
                    if text:find("ERROR", 1, true) == 1 then
                        callback(nil, text)
                    else
                        callback(text, nil)
                    end
                end)
            end
        end)
    end)
end
```

### Internal: Request Serialization (Queue)

The server handles one client at a time. Concurrent requests must be queued.

```lua
-- Queue: array of { elements, callback }
-- Process one at a time, dequeue next on completion

function StringSpaceClient:_enqueue(elements, callback)
    table.insert(self._queue, { elements = elements, callback = callback })
    if #self._queue == 1 then
        self:_process_queue()
    end
end

function StringSpaceClient:_process_queue()
    if #self._queue == 0 then return end
    local item = self._queue[1]
    self:_request_with_retry(item.elements, function(response, err)
        local cb = item.callback
        table.remove(self._queue, 1)
        cb(response, err)
        self:_process_queue()
    end)
end
```

### Internal: Auto-Retry

```lua
function StringSpaceClient:_request_with_retry(elements, callback, attempt)
    attempt = attempt or 0
    self:_request(elements, function(response, err)
        if err and attempt < 2 and self:_is_connection_error(err) then
            -- Exponential backoff: 1s, 2s
            vim.defer_fn(function()
                self:_request_with_retry(elements, callback, attempt + 1)
            end, 1000 * math.pow(2, attempt))
        else
            callback(response, err)
        end
    end)
end

function StringSpaceClient:_is_connection_error(err)
    if type(err) ~= "string" then return false end
    return err:find("ECONNREFUSED") or err:find("ECONNRESET") or
           err:find("ETIMEDOUT") or err:find("timeout") or
           err:find("EPIPE") or err:find("Connection closed")
end
```

### Search Method Implementations

```lua
function StringSpaceClient:best_completions(query, limit, callback)
    self:_enqueue({ "best-completions", query, tostring(limit or 10) }, function(response, err)
        if err then callback(nil, err); return end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

-- prefix_search, substring_search, similar_search, fuzzy_subsequence_search
-- follow the same pattern with their respective command names
```

### Insert Methods

```lua
function StringSpaceClient:insert(words, callback)
    -- Match TS: words joined with \n, NOT RS
    self:_enqueue({ "insert", table.concat(words, "\n") }, callback)
end

function StringSpaceClient:add_words_from_text(text, callback)
    -- Split on non-word chars (keeps apostrophes, hyphens, underscores)
    local seen = {}
    local words = {}
    for w in text:gmatch("[%w_%-%']+") do
        if #w >= 3 and #w <= 50 and not w:find("^'") and not seen[w] then
            seen[w] = true
            table.insert(words, w)
        end
    end
    if #words == 0 then callback("", nil); return end
    self:insert(words, callback)
end
```

**Note on `gmatch` pattern**: Lua's `[%w_%-%']` matches word chars, underscores, hyphens, and apostrophes -- the same character set as the TypeScript client's split pattern `/[^\w_\-']+/`. Python's `\w` is Unicode-broader by default, but this only matters if `add_words_from_text()` is called on non-English text, which is a non-goal for the nvim-cmp use case.

### Health Check

```lua
function StringSpaceClient:is_available()
    local now = vim.loop.now()
    if self._cached_available and (now - self._last_check) < 30000 then
        return self._cached_available
    end

    -- Synchronous probe: try connecting, then immediately close
    -- Using a coroutine to make this synchronous from the caller's perspective
    local available = false
    local done = false

    local tcp = vim.loop.new_tcp()
    tcp:connect(self.host, self.port, function(err)
        if not err then tcp:close() end
        available = (err == nil)
        done = true
    end)

    -- Run the event loop briefly to resolve the connection attempt
    -- In Neovim, vim.wait() pumps the event loop
    vim.wait(2000, function() return done end, 50)

    self._cached_available = available
    self._last_check = vim.loop.now()
    return available
end
```

**Note**: `is_available()` uses `vim.wait()` to appear synchronous to callers. This is the standard Neovim pattern for blocking until a libuv operation completes. The 2000ms timeout matches the TypeScript health check timeout.

**Note on blocking**: `is_available()` is the only blocking method -- it can block for up to 2 seconds when the cache is cold or the server was previously unavailable. All search and mutation methods are fully asynchronous (non-blocking callback-based). The nvim-cmp consumer should call `is_available()` judiciously; in normal operation the first call returns in milliseconds and the cache holds for 30 seconds.

### Pre-Implementation: Verify `nvim -l` TCP Support

Before building the full client, verify that `nvim -l` supports TCP operations and `vim.wait()` in headless mode. Run a quick smoke test against a running server:

```lua
-- test_vim_loop.lua: nvim -l test_vim_loop.lua <port>
local port = tonumber(arg[1]) or 7878
local tcp = vim.loop.new_tcp()
local done = false
tcp:connect("127.0.0.1", port, function(err)
    print("Connect callback: err=" .. tostring(err))
    if not err then tcp:close() end
    done = true
end)
vim.wait(2000, function() return done end, 50)
print(done and "OK: TCP works in nvim -l" or "FAIL: TCP did not work")
```

---

## Testing Strategy

Follows the same pattern as the TypeScript client tests: standalone Lua scripts executed by `nvim -l`, run against a live string-space server.

### Prerequisites

- Neovim ≥ 0.9 installed (✅ v0.11.6)
- Rust server built (`cargo build`) (✅)
- Test data file: `test/word_list.txt` (✅ existing)

**No additional dependencies required.**

### Test Runner

Create `tests/run_lua_tests.sh` — mirrors `tests/run_ts_tests.sh`:

```bash
#!/bin/bash
set -e

# Run Lua integration tests against a live string-space server.
# Builds the server, starts a daemon, runs lua test, then stops.
#
# Usage: tests/run_lua_tests.sh
# Requires: nvim on $PATH, cargo

cargo build
EXECUTABLE=target/debug/string_space
PORT=9898

# Clean up any existing server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null || true

# Start daemon
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p $PORT -d
echo "Server started on port $PORT"

# Wait for server to be ready
sleep 1

# Trap ensures cleanup on exit
trap 'RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null' EXIT

# Run all Lua test scripts
for script in lua/test_*.lua; do
    echo ""
    echo "=== Running $script ==="
    nvim -l "$script" $PORT
    echo "✓ $script passed"
done

echo ""
echo "=== All Lua tests passed ==="
```

### Test File: `lua/test_client.lua`

A single comprehensive test script, mirroring `tests/ts_client.ts`. Uses simple assert-based testing with `print()` output (no framework needed).

```lua
-- Get port from CLI args
local port = tonumber(arg[1])
if not port then
    print("Usage: nvim -l lua/test_client.lua <port>")
    os.exit(1)
end

-- Add project root to package.path so we can require the module
package.path = "./lua/?.lua;" .. package.path
-- Also need to handle the init.lua in a directory
package.path = "./lua/?/init.lua;" .. package.path

local StringSpaceClient = require("string-space-client")

local passed = 0
local failed = 0

local function assert_test(condition, message)
    if condition then
        print("  ✓ " .. message)
        passed = passed + 1
    else
        print("  ✗ FAIL: " .. message)
        failed = failed + 1
    end
end

local client = StringSpaceClient.new("127.0.0.1", port)

-- is_available()
local available = client:is_available()
assert_test(available == true, "is_available: returns true")

-- best_completions (async — uses vim.wait to block until callback fires)
local results = nil
local done = false
client:best_completions("hel", 5, function(r, err)
    results = r
    done = true
end)
vim.wait(3000, function() return done end, 50)
assert_test(results ~= nil and #results > 0, "best_completions 'hel': has results")
assert_test(#results <= 5, "best_completions 'hel': respects limit (" .. #results .. " results)")

-- insert
done = false
local insert_result = nil
client:insert({ "testword1", "testword2" }, function(r, err)
    insert_result = r
    done = true
end)
vim.wait(3000, function() return done end, 50)
assert_test(insert_result and insert_result:find("OK"), "insert: " .. tostring(insert_result))

-- insert dedup
done = false
client:insert({ "testword1" }, function(r, err)
    insert_result = r
    done = true
end)
vim.wait(3000, function() return done end, 50)
assert_test(insert_result and insert_result:find("OK"), "insert dedup: " .. tostring(insert_result))

-- add_words_from_text
done = false
local add_result = nil
client:add_words_from_text("The quick brown fox's jump", function(r, err)
    add_result = r
    done = true
end)
vim.wait(3000, function() return done end, 50)
assert_test(add_result and add_result:find("OK"), "add_words_from_text: " .. tostring(add_result))

-- Server unavailable (bad port)
local bad_client = StringSpaceClient.new("127.0.0.1", 19999)
local not_available = bad_client:is_available()
assert_test(not_available == false, "is_available on bad port: returns false")

-- data_file
done = false
local data_file_result = nil
client:data_file(function(r, err)
    data_file_result = r
    done = true
end)
vim.wait(3000, function() return done end, 50)
assert_test(data_file_result and #data_file_result > 0, "data_file: " .. tostring(data_file_result))

-- Summary
print("")
if failed > 0 then
    print("=== " .. failed .. " TEST(S) FAILED ===")
    os.exit(1)
else
    print("=== All " .. passed .. " Lua tests passed ===")
end
```

### Test Cases Covered

| Test | Method | Asserts |
|------|--------|----------|
| Server available | `is_available()` | Returns `true` |
| Basic search | `best_completions("hel", 5)` | Non-empty, ≤5 results |
| Insert words | `insert({"testword1", "testword2"})` | Response contains "OK" |
| Insert dedup | `insert({"testword1"})` again | Response contains "OK" (server accepts, updates frequency) |
| Add words from text | `add_words_from_text("The quick brown fox's jump")` | Non-empty response |
| Server unavailable | `is_available()` against bad port | Returns `false` |
| Data file | `data_file()` | Non-empty string |

### Edge Cases to Cover (in test or future expansion)

| Edge case | How to test |
|-----------|-------------|
| Server not running | Client against dead port: `is_available()` returns false, search callbacks get error |
| Empty query | `best_completions("")` — graceful error |
| Large batch insert | 100 words via `insert()` |
| Unicode query | `best_completions("café")` — no crash |
| Concurrent requests | Multiple `best_completions` calls before first resolves — queue serializes them |
| EOT split across chunks | Server returns large response (>4096 bytes) — reassembly works |

---

## Makefile Integration

### New target: `lua-test`

```makefile
# Lua integration tests: builds server, starts daemon, runs lua test script, stops server
lua-test: cargo-build
	tests/run_lua_tests.sh
```

### Addition to `.PHONY`

```makefile
.PHONY: all debug release clean test ts-test lua-test client auto-server cargo-build
```

### Addition to `tests/run_tests.sh` (full suite)

Inserted **before** the `# Stop the server after all tests` block (after the TypeScript test loop, still inside the existing daemon lifecycle on port 9898):

```bash
# --- Lua tests (new) ---
echo ""
echo "=== Running lua/test_client.lua ==="
nvim -l lua/test_client.lua 9898
if [ $? -ne 0 ]; then
    echo "lua/test_client.lua failed"
    RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop
    exit 1
fi
echo "✓ lua/test_client.lua passed"
```

---

## Implementation Order

| Step | What | Estimate |
|------|------|----------|
| 1 | Create `lua/string-space-client/init.lua` — module skeleton, constructor, constants | ~20 min |
| 2 | Implement `_request()` transport with libuv TCP | ~30 min |
| 3 | Implement `_enqueue()` queue and `_process_queue()` serialization | ~15 min |
| 4 | Implement `_request_with_retry()` with exponential backoff | ~15 min |
| 5 | Implement all search methods (`best_completions`, `prefix_search`, etc.) | ~15 min |
| 6 | Implement `insert()` and `add_words_from_text()` | ~15 min |
| 7 | Implement `is_available()` health check with cache | ~15 min |
| 8 | Implement `data_file()` utility | ~5 min |
| 9 | Create `lua/test_client.lua` integration tests | ~20 min |
| 10 | Create `tests/run_lua_tests.sh` and Makefile `lua-test` target | ~10 min |
| 11 | Add Lua tests to `tests/run_tests.sh` (full suite) | ~5 min |
| 12 | Wire `lua-test` into `.PHONY` | ~2 min |
| | **Total** | **~2.5 hours** |

---

## Differences from TypeScript Client

| Aspect | TypeScript | Lua |
|--------|-----------|-----|
| Transport | Node `net` module | `vim.loop.new_tcp()` (libuv) |
| Async model | `Promise<string>` | Callbacks `function(response, err)` |
| Serialization lock | Promise-chain (`this.queue`) | Table-based FIFO queue + `_process_queue()` |
| Retry delay | `setTimeout` | `vim.defer_fn` |
| Health check blocking | `await` (async) | `vim.wait()` (pumps event loop) |
| String ops | `Buffer.concat`, `.includes()` | `table.concat`, `string.find()` |
| Byte handling | `Buffer.from` | `string.char()` / `string.byte()` |
| Word splitting | `.split(regex)` | `string.gmatch(pattern)` |
| Dedup | `.indexOf(w) === i` | `seen = {}` table (hash set) |
| Error type | `Error` instances | Strings |

---

## Protocol Gotchas Summary

1. **One-shot connections**: Connect → send → receive → disconnect per request
2. **Insert word joining**: Words joined with `\n` for insert, NOT RS
3. **EOT may be split across TCP chunks**: Must accumulate until EOT found
4. **3-second timeout**: Server read/write timeout; don't exceed on client
5. **No concurrency**: Server handles one connection at a time — must serialize
6. **String length limits**: MIN_CHARS=3, MAX_CHARS=50
7. **Error responses**: Start with `ERROR` — check before parsing as data
8. **Client-side timeout**: The TS client sets a 3-second socket timeout. The Lua `_request()` should use `vim.loop.new_timer()` to enforce a matching client-side timeout, so the client doesn't hang indefinitely if the server or network misbehaves.
9. **`vim.loop` vs `vim.uv`**: Neovim 0.10+ aliases `vim.loop` to `vim.uv`. Use `vim.loop` for backward compatibility (works in 0.9+).

---

## Future: nvim-cmp Source (not this task)

The Neovim config project will:
1. Symlink `lua/string-space-client/` into nvim's runtimepath
2. Create `lua/plugins/cmp-string-space.lua` — a nvim-cmp source that:
   - Calls `client:best_completions(query, limit, callback)` on completion trigger
   - Returns completion items with `{ name = "string_space", label = "[Words]" }`
3. Wire into existing nvim-cmp setup with `priority = 40`
