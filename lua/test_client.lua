---
--- Integration tests for the String Space Lua client.
---
--- Usage: nvim -l lua/test_client.lua <port>
--- Requires a running string-space server with test/word_list.txt loaded.
---

-- Get port from CLI args
local port = tonumber(arg[1])
if not port then
    print("Usage: nvim -l lua/test_client.lua <port>")
    os.exit(1)
end

-- Add project root to package.path so we can require the module
package.path = "./lua/?/init.lua;" .. package.path

local StringSpaceClient = require("string-space-client")

local passed = 0
local failed = 0

local function assert_test(condition, message)
    if condition then
        print("  \u{2713} " .. message)
        passed = passed + 1
    else
        print("  \u{2717} FAIL: " .. message)
        failed = failed + 1
    end
end

--- Helper: run an async call and block until the callback fires.
---@param fn function  function(done) — call done when callback resolves
---@param timeout number?  Max wait in ms (default 3000)
local function sync(fn, timeout)
    local done = false
    fn(function()
        done = true
    end)
    vim.wait(timeout or 3000, function()
        return done
    end, 50)
    if not done then
        error("sync() timed out waiting for callback")
    end
end

-- ── Tests ───────────────────────────────────────────────────────────

print("=== Lua Client Integration Tests (port " .. port .. ") ===")
print("")

local client = StringSpaceClient.new("127.0.0.1", port)

-- is_available()
print("is_available:")
local available = client:is_available()
assert_test(available == true, "returns true when server is running")

-- best_completions
print("")
print("best_completions:")
local results = nil
sync(function(done)
    client:best_completions("hel", 5, function(r, err)
        results = r
        done()
    end)
end)
assert_test(results ~= nil and #results > 0, "'hel' returns results")
assert_test(#results <= 5, "respects limit (" .. #results .. " results)")

-- prefix_search
print("")
print("prefix_search:")
results = nil
sync(function(done)
    client:prefix_search("hel", function(r, err)
        results = r
        done()
    end)
end)
assert_test(results ~= nil and #results > 0, "'hel' returns results")

-- substring_search
print("")
print("substring_search:")
results = nil
sync(function(done)
    client:substring_search("ing", function(r, err)
        results = r
        done()
    end)
end)
assert_test(results ~= nil and #results > 0, "'ing' returns results")

-- similar_search
print("")
print("similar_search:")
results = nil
sync(function(done)
    client:similar_search("helo", 0.8, function(r, err)
        results = r
        done()
    end)
end)
assert_test(results ~= nil, "'helo' threshold 0.8 returns without error")

-- fuzzy_subsequence_search
print("")
print("fuzzy_subsequence_search:")
results = nil
sync(function(done)
    client:fuzzy_subsequence_search("hlo", function(r, err)
        results = r
        done()
    end)
end)
assert_test(results ~= nil, "'hlo' returns without error")

-- insert
print("")
print("insert:")
local insert_result = nil
sync(function(done)
    client:insert({ "testword1", "testword2" }, function(r, err)
        insert_result = r
        done()
    end)
end)
assert_test(insert_result and insert_result:find("OK"), "inserts two words: " .. tostring(insert_result))

-- insert dedup (same word again — server accepts and updates frequency)
print("")
print("insert dedup:")
sync(function(done)
    client:insert({ "testword1" }, function(r, err)
        insert_result = r
        done()
    end)
end)
assert_test(insert_result and insert_result:find("OK"), "re-inserts duplicate: " .. tostring(insert_result))

-- add_words_from_text
print("")
print("add_words_from_text:")
local add_result = nil
sync(function(done)
    client:add_words_from_text("The quick brown fox's jump", function(r, err)
        add_result = r
        done()
    end)
end)
assert_test(add_result and add_result:find("OK"), "extracts and inserts words: " .. tostring(add_result))

-- data_file
print("")
print("data_file:")
local data_file_result = nil
sync(function(done)
    client:data_file(function(r, err)
        data_file_result = r
        done()
    end)
end)
assert_test(data_file_result and #data_file_result > 0, "returns path: " .. tostring(data_file_result))

-- Server unavailable (bad port)
print("")
print("is_available on bad port:")
local bad_client = StringSpaceClient.new("127.0.0.1", 19999)
local not_available = bad_client:is_available()
assert_test(not_available == false, "returns false when server not listening")

-- Error callback on search against bad port
print("")
print("error callback on bad port:")
local err_received = nil
sync(function(done)
    bad_client:best_completions("hel", 5, function(r, err)
        err_received = err
        done()
    end)
end, 10000) -- longer timeout to allow for retries
assert_test(err_received ~= nil, "receives error: " .. tostring(err_received))

-- ── Summary ─────────────────────────────────────────────────────────

print("")
if failed > 0 then
    print("=== " .. failed .. " TEST(S) FAILED ===")
    os.exit(1)
else
    print("=== All " .. passed .. " Lua tests passed ===")
end
