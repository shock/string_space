---
--- String Space Lua Client v0.6.7
--- 2026-05-05
---
--- TCP client for the string-space server — an in-memory word database
--- providing multi-algorithm completion.
---
--- Wire format:
---   Request:  <elem1><RS><elem2><RS>...<elemN><EOT>
---   Response: <text><EOT>
---
--- Constants:
---   RS  = 0x1E (record separator)
---   EOT = 0x04 (end of transmission)
---
--- Transport: vim.loop (libuv) TCP — available in Neovim >= 0.9
--- Async model: callbacks  function(response, err)
---
---@module string-space-client

-- ── Protocol constants ──────────────────────────────────────────────
local EOT = string.char(0x04)
local RS = string.char(0x1E)
local CONNECTION_TIMEOUT_MS = 3000
local HEALTH_CHECK_TIMEOUT_MS = 2000
local AVAILABILITY_CACHE_MS = 30000

-- ── Client metatable ────────────────────────────────────────────────
local StringSpaceClient = {}
StringSpaceClient.__index = StringSpaceClient

--- Create a new StringSpaceClient.
---@param host string?  Server hostname (default "127.0.0.1")
---@param port number?  Server port (default 7878)
---@param debug boolean?  Enable debug logging (default false)
---@return StringSpaceClient
function StringSpaceClient.new(host, port, debug)
    local self = setmetatable({}, StringSpaceClient)
    self.host = host or "127.0.0.1"
    self.port = port or 7878
    self.debug = debug or false
    self._queue = {}
    self._cached_available = false
    self._last_check = 0
    return self
end

-- ── Search Methods ──────────────────────────────────────────────────

--- Intelligent multi-algorithm completion.
--- Combines prefix, fuzzy-subsequence, Jaro-Winkler similarity, and substring
--- searches with progressive execution and dynamic weighting.
---@param query string  The search query
---@param limit number?  Max results (default 10)
---@param callback function  function(results: string[]|nil, err: string|nil)
function StringSpaceClient:best_completions(query, limit, callback)
    self:_enqueue({ "best-completions", query, tostring(limit or 10) }, function(response, err)
        if err then
            callback(nil, err)
            return
        end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

--- Search by exact prefix match.
---@param prefix string
---@param callback function  function(results: string[]|nil, err: string|nil)
function StringSpaceClient:prefix_search(prefix, callback)
    self:_enqueue({ "prefix", prefix }, function(response, err)
        if err then
            callback(nil, err)
            return
        end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

--- Search by substring occurrence.
---@param substring string
---@param callback function  function(results: string[]|nil, err: string|nil)
function StringSpaceClient:substring_search(substring, callback)
    self:_enqueue({ "substring", substring }, function(response, err)
        if err then
            callback(nil, err)
            return
        end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

--- Fuzzy similarity search using Jaro-Winkler distance.
---@param word string
---@param threshold number  Similarity threshold (0.0–1.0)
---@param callback function  function(results: string[]|nil, err: string|nil)
function StringSpaceClient:similar_search(word, threshold, callback)
    self:_enqueue({ "similar", word, tostring(threshold) }, function(response, err)
        if err then
            callback(nil, err)
            return
        end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

--- Character order-preserving fuzzy subsequence search.
---@param query string
---@param callback function  function(results: string[]|nil, err: string|nil)
function StringSpaceClient:fuzzy_subsequence_search(query, callback)
    self:_enqueue({ "fuzzy-subsequence", query }, function(response, err)
        if err then
            callback(nil, err)
            return
        end
        local results = {}
        for line in response:gmatch("[^\n]+") do
            table.insert(results, line)
        end
        callback(results, nil)
    end)
end

-- ── Mutation Methods ────────────────────────────────────────────────

--- Insert one or more words into the database.
--- Words must be 3–50 characters; the server silently filters invalid ones.
---@param words string[]  List of words to insert
---@param callback function  function(response: string|nil, err: string|nil)
function StringSpaceClient:insert(words, callback)
    -- Match Python/TS clients: words joined with \n, NOT RS
    self:_enqueue({ "insert", table.concat(words, "\n") }, callback)
end

--- Extract words from free-form text and insert them.
--- Applies the same filtering rules as Python/TS clients:
---   - Split on non-word characters (keeps apostrophes, hyphens, underscores)
---   - Filter: length 3–50, no leading apostrophe, deduplicated
---@param text string  Free-form text to extract words from
---@param callback function  function(response: string|nil, err: string|nil)
function StringSpaceClient:add_words_from_text(text, callback)
    local seen = {}
    local words = {}
    for w in text:gmatch("[%w_%-%']+") do
        if #w >= 3 and #w <= 50 and not w:find("^'") and not seen[w] then
            seen[w] = true
            table.insert(words, w)
        end
    end
    if #words == 0 then
        callback("", nil)
        return
    end
    self:insert(words, callback)
end

-- ── Health & Utility ────────────────────────────────────────────────

--- Returns the server's data file path.
---@param callback function  function(path: string|nil, err: string|nil)
function StringSpaceClient:data_file(callback)
    self:_enqueue({ "data-file" }, callback)
end

--- Check if the server is reachable.
--- Results are cached for 30 seconds to avoid hammering the port.
--- This is the only blocking method — may block up to 2 seconds when cache is cold.
---@return boolean
function StringSpaceClient:is_available()
    local now = vim.loop.now()
    if self._cached_available and (now - self._last_check) < AVAILABILITY_CACHE_MS then
        return self._cached_available
    end

    local available = false
    local done = false

    local tcp = vim.loop.new_tcp()
    tcp:connect(self.host, self.port, function(err)
        if not err then
            tcp:close()
        end
        available = (err == nil)
        done = true
    end)

    -- Run the event loop briefly to resolve the connection attempt
    vim.wait(HEALTH_CHECK_TIMEOUT_MS, function()
        return done
    end, 50)

    self._cached_available = available
    self._last_check = vim.loop.now()
    return available
end

-- ── Internal: Request Pipeline ──────────────────────────────────────

--- Enqueue a request for serialized execution.
--- The server is single-client, so concurrent requests must be queued.
function StringSpaceClient:_enqueue(elements, callback)
    table.insert(self._queue, { elements = elements, callback = callback })
    if #self._queue == 1 then
        self:_process_queue()
    end
end

--- Process the next item in the queue.
function StringSpaceClient:_process_queue()
    if #self._queue == 0 then
        return
    end
    local item = self._queue[1]
    self:_request_with_retry(item.elements, function(response, err)
        local cb = item.callback
        table.remove(self._queue, 1)
        cb(response, err)
        self:_process_queue()
    end)
end

--- Retry up to 2 times with exponential backoff on connection-level errors.
function StringSpaceClient:_request_with_retry(elements, callback, attempt)
    attempt = attempt or 0
    self:_request(elements, function(response, err)
        if err and attempt < 2 and self:_is_connection_error(err) then
            if self.debug then
                print("[DEBUG] Retry attempt " .. (attempt + 1) .. " after error: " .. tostring(err))
            end
            -- Exponential backoff: 1s, 2s
            vim.defer_fn(function()
                self:_request_with_retry(elements, callback, attempt + 1)
            end, 1000 * math.pow(2, attempt))
        else
            callback(response, err)
        end
    end)
end

--- Determine whether an error is retryable (connection-level).
function StringSpaceClient:_is_connection_error(err)
    if type(err) ~= "string" then
        return false
    end
    return err:find("ECONNREFUSED") ~= nil
        or err:find("ECONNRESET") ~= nil
        or err:find("ETIMEDOUT") ~= nil
        or err:find("timeout") ~= nil
        or err:find("EPIPE") ~= nil
        or err:find("Connection closed") ~= nil
end

--- Core transport: connect -> send -> accumulate response until EOT -> disconnect.
function StringSpaceClient:_request(elements, callback)
    local tcp = vim.loop.new_tcp()

    local request_str = table.concat(elements, RS) .. EOT
    local response_parts = {}
    local settled = false

    local function settle(fn)
        if settled then
            return
        end
        settled = true
        fn()
    end

    if self.debug then
        print("[DEBUG] Sending: " .. table.concat(elements, RS) .. " (+EOT)")
    end

    -- Client-side timeout
    local timer = vim.loop.new_timer()
    timer:start(CONNECTION_TIMEOUT_MS, 0, function()
        settle(function()
            tcp:close()
            timer:close()
            callback(nil, "Connection timeout")
        end)
    end)

    tcp:connect(self.host, self.port, function(err)
        if err then
            timer:close()
            settle(function()
                tcp:close()
                callback(nil, err)
            end)
            return
        end

        tcp:write(request_str, function(write_err)
            if write_err then
                timer:close()
                settle(function()
                    tcp:close()
                    callback(nil, write_err)
                end)
                return
            end
        end)

        tcp:read_start(function(read_err, data)
            if read_err then
                timer:close()
                settle(function()
                    tcp:close()
                    callback(nil, read_err)
                end)
                return
            end
            if not data then
                timer:close()
                settle(function()
                    tcp:close()
                    callback(nil, "Connection closed by server")
                end)
                return
            end
            table.insert(response_parts, data)
            local full = table.concat(response_parts)
            if full:find(EOT, 1, true) then
                timer:close()
                settle(function()
                    tcp:close()
                    -- Strip trailing EOT bytes
                    local text = full:gsub(EOT .. "+$", "")
                    if self.debug then
                        print("[DEBUG] Response: " .. text)
                    end
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

return StringSpaceClient
