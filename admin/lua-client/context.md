# Lua String-Space Client — Context

## Goal

Build a Lua TCP client for the string-space server. This client will be used as a **custom nvim-cmp completion source** inside Neovim, providing intelligent multi-algorithm word completion as-you-type.

The full integration has two parts — this task is **Part 1 only** (the client library). Part 2 (the nvim-cmp source) will be built separately in the Neovim config project.

## What to Build

Create `lua/string-space-client/init.lua` — a self-contained Lua module that mirrors the API surface of the existing Python and TypeScript clients. It should feel like a natural third sibling to those clients, adapted for Lua/Neovim conventions.

### Required API

```lua
local client = StringSpaceClient.new("127.0.0.1", 7878)

client:is_available()                           -- → boolean (positive result cached for 30s, may block up to 2s)
client:best_completions(query, limit, cb)       -- → string[] (async, callback-based)
client:prefix_search(prefix, cb)                -- → string[] (async)
client:substring_search(substring, cb)           -- → string[] (async)
client:similar_search(word, threshold, cb)       -- → string[] (async)
client:fuzzy_subsequence_search(query, cb)       -- → string[] (async)
client:add_words_from_text(text, cb)             -- → inserts words (async)
client:insert(words, cb)                         -- → inserts word list (async)
client:data_file(cb)                             -- → string (async)
```

### Key Technical Decisions

1. **Transport: `vim.loop.new_tcp()`** (libuv TCP handle) — not `vim.fn.jobstart`. This gives direct TCP control matching the Python/TS socket approach. Available in Neovim >= 0.9.

2. **Async model: callbacks** — Lua has no async/await. Use callback-style API:
   ```lua
   client:best_completions("hel", 10, function(results, err)
     -- results is string[], err is nil or string
   end)
   ```

3. **Connection lifecycle: connect → send → receive → close per request** — same pattern as Python and TypeScript clients. Don't hold connections open.

4. **Request serialization** — the server is single-client. If multiple requests arrive concurrently, they must be queued and sent one at a time.

5. **Retry with exponential backoff** — up to 2 retries on connection errors (ECONNREFUSED, ECONNRESET, ETIMEDOUT), same as the TypeScript client.

### Wire Protocol

Same RS/EOT protocol as Python and TypeScript clients:
- **Request**: `<elem1>\x1E<elem2>\x1E...\x04`
- **Response**: `<text>\x04`
- Key command: `best-completions\x1E<query>\x1E<limit>\x04`

### Word Extraction for `add_words_from_text`

Same rules as Python/TS clients:
- Split on non-word characters (keeps apostrophes, hyphens, underscores)
- Filter: length 3–50, no leading apostrophe, deduplicated

### Availability Check

Same caching strategy as TypeScript client:
- Cache result for 30 seconds
- Simple connect-attempt probe (connect then immediately close)

## Testing

Create `lua/test_client.lua` — a standalone test script runnable with:

```bash
nvim -l lua/test_client.lua
```

Requires a running string-space server (e.g. `string_space start -d /tmp/test_words.txt`).

Tests should verify:
- `is_available()` returns true/false correctly
- `best_completions()` returns results for a known query
- `add_words_from_text()` inserts words that are then retrievable
- Graceful error when server is not running

Add a `lua-test` target to the project Makefile:

```makefile
lua-test: cargo-build
	tests/run_lua_tests.sh
```

This mirrors the `ts-test` target: builds the server, starts a daemon, runs the tests, then stops the server.

## File Layout

```
lua/
├── string-space-client/
│   └── init.lua          -- The client module
└── test_client.lua       -- Standalone test script
```

## Runtime Context

This client will be used inside Neovim, so `vim.loop` (libuv) is always available. It does **not** need to work outside Neovim — `nvim -l` is sufficient for testing.

## What Comes After (not your task)

The Neovim config project will:
1. Symlink `lua/string-space-client/` into nvim's runtimepath
2. Build a nvim-cmp custom source in `lua/plugins/cmp-string-space.lua` that calls `client:best_completions()`
3. Wire it into the existing nvim-cmp setup as `{ name = "string_space", priority = 40 }` with `[Words]` label

Your client just needs to provide a clean, reliable API for that source to call.
