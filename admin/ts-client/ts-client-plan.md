# String-Space TypeScript Client: Implementation Plan

**Created:** 2026-04-29
**Status:** Planning — implementation not yet started
**Parent plan:** `admin/string-space-acmpl/master-plan.md` *(not yet created)*

---

## Goal

Create a TypeScript TCP client for the string-space server, co-located with the existing Python client and Rust server in this project. The client will be used by a Pi editor extension to provide word auto-completion.

## Context

String-space is a Rust TCP server with an in-memory word database providing multi-algorithm completion. The Python ecosystem already has:

- `python/string_space_client/string_space_client.py` — TCP client (~270 lines)
- `python/string_space_completer/string_space_completer.py` — prompt-toolkit completer (~110 lines)

This plan creates the TypeScript equivalent of the Python client.

**Consumers**: A Pi TUI extension (`word-autocomplete`) will import this client. During development, a raw `.ts` file is imported directly. After finalization, it will be packaged as a GitHub-hosted npm-style package.

---

## Protocol Specification

### Wire Format

**Request:**
```
<element_1><0x1E><element_2><0x1E>...<element_N><0x04>
```
- Elements are UTF-8 strings separated by RS byte (`0x1E`)
- Terminated by EOT byte (`0x04`)

**Response:**
```
<response_text><0x04>
```
- UTF-8 text, newline-separated records for list results
- Terminated by EOT byte (`0x04`)
- Errors start with `ERROR`

### Constants

| Constant | Value | Source |
|----------|-------|--------|
| EOT byte | `0x04` | `src/modules/protocol.rs:31` |
| RS byte | `0x1E` | `src/modules/protocol.rs:32` |
| Connection timeout | 3 seconds | `src/modules/protocol.rs:34` |
| Min word length | 3 chars | `src/modules/string_space/mod.rs:38` |
| Max word length | 50 chars | `src/modules/string_space/mod.rs:39` |

### Commands

| Command | Params | Response |
|---------|--------|----------|
| `insert` | `<word1>\n<word2>\n...` | `OK\nInserted N of M words` |
| `best-completions` | `<query>[<RS><limit>]` | newline-separated matches |
| `prefix` | `<prefix>` | newline-separated matches |
| `substring` | `<substring>` | newline-separated matches |
| `similar` | `<word><RS><threshold>` | newline-separated matches |
| `fuzzy-subsequence` | `<query>` | newline-separated matches |
| `data-file` | (none) | file path string |

> **Not available via TCP:** The `StringSpace` Rust struct also exposes `len`, `empty`, `clear_space`, `get_all_strings`, `capacity`, and `sort` as methods, but these have **no protocol handlers** — they are not reachable over TCP. The TypeScript client does not expose them. If needed in the future, protocol handlers must be added to `src/modules/protocol.rs` first.

### Server Behavior

- **One client at a time** — sequential, not concurrent (no thread pool)
- **3-second read/write timeout** per connection
- Handles persistent connections but **one-shot connections** are recommended
- Reads until EOT byte, responds, then waits for next request on same connection

---

## Reference Implementation: Python Client

**File**: `python/string_space_client/string_space_client.py` (~270 lines)

### Connection Pattern

```python
class StringSpaceClient:
    def __init__(self, host, port, debug=False):
        self.host = host
        self.port = port
        self.lock = threading.RLock()  # Serialize access

    def request(self, request_elements: list[str]) -> str:
        with self.lock:
            # Retry logic: up to 2 retries with exponential backoff
            request = self.create_request(RS_BYTE_STR.join(request_elements))
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(3.0)
            self.sock.connect((self.host, self.port))
            self.sock.sendall(request)
            response = self.receive_response()
            self.disconnect()
            return response
```

Key pattern: **connect → send → receive → disconnect** per request.

### Request Encoding

```python
def create_request(self, string: str):
    req = bytearray()
    req.extend(string.encode('utf-8'))
    req.extend(b'\x04')  # EOT byte
    return req
```

Elements are joined with `\x1E` (RS) before the EOT terminator is appended.

### Response Reading

```python
def receive_response(self):
    data = b''
    while True:
        chunk = self.sock.recv(4096)
        if not chunk:
            raise ConnectionError("Connection closed by the server")
        data += chunk
        if b'\x04' in chunk:
            break
    result = data.rstrip(b'\x04').decode('utf-8')
    if result[:5] == "ERROR":
        raise ProtocolError(result)
    return result
```

**Important**: EOT may arrive split across TCP chunks. Must accumulate chunks until EOT is found.

### Insert Gotcha

```python
def insert(self, strings: list[str]):
    request_elements = ["insert", "\n".join(strings)]  # Words joined by \n, NOT RS
    response = self.request(request_elements)
```

Words are joined with `\n` for insert. The server replaces newlines with spaces, then splits by space.

### Search Methods

```python
def best_completions_search(self, query: str, limit: int = 10) -> list[str]:
    request_elements = ["best-completions", query, str(limit)]
    response = self.request(request_elements)
    return [line for line in response.split('\n') if line]
```

All search methods follow the same pattern: build element list, call `request()`, split response by newline.

### Word Extraction for Insert

```python
def parse_text(self, text: str) -> list[str]:
    words = re.split(r'[^\w_\-\']+', text)
    words = list(set(words))
    words = [word for word in words if len(word) >= 3]
    return words
```

---

## Implementation

### Location

```
typescript/
└── string-space-client.ts
```

A single `.ts` file, no build step. Importable directly by Bun/TypeScript consumers.

### API Surface

```typescript
export class StringSpaceClient {
    constructor(host?: string, port?: number, debug?: boolean);

    // Search
    async bestCompletions(query: string, limit?: number): Promise<string[]>;
    async prefixSearch(prefix: string): Promise<string[]>;
    async substringSearch(substring: string): Promise<string[]>;
    async similarSearch(word: string, threshold: number): Promise<string[]>;
    async fuzzySubsequenceSearch(query: string): Promise<string[]>;

    // Mutation
    async insert(words: string[]): Promise<string>;
    async addWordsFromText(text: string): Promise<string>;

    // Health
    async isAvailable(): Promise<boolean>;

    // Utility
    async dataFile(): Promise<string>;
}
```

### Default Values

- `host`: `'127.0.0.1'`
- `port`: `7878`
- `debug`: `false`

### Internal: Transport Layer

```typescript
import * as net from 'net';

const EOT = 0x04;
const RS = '\x1E';

private async request(elements: string[]): Promise<string> {
    // Serialize with a promise chain (server is one-client-at-a-time)
    return new Promise<string>((resolve, reject) => {
        const serialized = Buffer.from(elements.join(RS) + '\x04', 'utf-8');
        const socket = net.createConnection({ host: this.host, port: this.port });
        socket.setTimeout(3000);

        let response = Buffer.alloc(0);
        let settled = false;

        socket.on('data', (chunk: Buffer) => {
            response = Buffer.concat([response, chunk]);
            if (chunk.includes(EOT)) {
                settled = true;
                socket.destroy();
                const text = response.toString('utf-8').replace(/\x04+$/, '');
                if (text.startsWith('ERROR')) {
                    reject(new Error(text));
                } else {
                    resolve(text);
                }
            }
        });

        socket.on('timeout', () => { settled = true; socket.destroy(); reject(new Error('Connection timeout')); });
        socket.on('error', (err) => { if (!settled) { settled = true; reject(err); } });
        socket.on('close', () => { if (!settled) { settled = true; reject(new Error('Connection closed by server')); } });

        socket.write(serialized);
    });
}
```

### Internal: Request Serialization

The server handles one client at a time. Use a promise-chain lock:

```typescript
private queue: Promise<void> = Promise.resolve();

private async serializedRequest(elements: string[]): Promise<string> {
    let resolver: () => void;
    const waiter = new Promise<void>(r => { resolver = r; });
    const prev = this.queue;
    this.queue = waiter;

    await prev;
    try {
        // Auto-retry: 2 attempts with exponential backoff (1s, 2s)
        return await this.requestWithRetry(elements);
    } finally {
        resolver!();
    }
}
```

### Internal: Auto-Retry

```typescript
private async requestWithRetry(elements: string[], maxRetries = 2): Promise<string> {
    for (let attempt = 0; attempt <= maxRetries; attempt++) {
        try {
            return await this.request(elements);
        } catch (err) {
            if (attempt === maxRetries) throw err;
            // Only retry on connection-level errors (ECONNREFUSED, timeout, etc.)
            if (!this.isConnectionError(err)) throw err;
            await new Promise(r => setTimeout(r, 1000 * Math.pow(2, attempt))); // 1s, 2s
        }
    }
    throw new Error('Unreachable');
}

private isConnectionError(err: unknown): boolean {
    if (err instanceof Error) {
        const msg = err.message;
        return msg.includes('ECONNREFUSED') ||
               msg.includes('ECONNRESET') ||
               msg.includes('ETIMEDOUT') ||
               msg.includes('Connection timeout') ||
               msg.includes('EPIPE');
    }
    return false;
}
```

### Search Method Implementations

```typescript
async bestCompletions(query: string, limit: number = 10): Promise<string[]> {
    // Server default limit is 15 when omitted; client always sends explicitly.
    const response = await this.serializedRequest(['best-completions', query, String(limit)]);
    return response.split('\n').filter(line => line.length > 0);
}

async prefixSearch(prefix: string): Promise<string[]> {
    const response = await this.serializedRequest(['prefix', prefix]);
    return response.split('\n').filter(line => line.length > 0);
}

async similarSearch(word: string, threshold: number): Promise<string[]> {
    const response = await this.serializedRequest(['similar', word, String(threshold)]);
    return response.split('\n').filter(line => line.length > 0);
}

async substringSearch(substring: string): Promise<string[]> {
    const response = await this.serializedRequest(['substring', substring]);
    return response.split('\n').filter(line => line.length > 0);
}

async fuzzySubsequenceSearch(query: string): Promise<string[]> {
    const response = await this.serializedRequest(['fuzzy-subsequence', query]);
    return response.split('\n').filter(line => line.length > 0);
}
```

### Insert Methods

```typescript
async insert(words: string[]): Promise<string> {
    // Match Python: join words with \n, NOT RS
    const response = await this.serializedRequest(['insert', words.join('\n')]);
    return response;
}

async addWordsFromText(text: string): Promise<string> {
    const words = text
        .split(/[^\w_\-']+/) // Matches Python: keeps apostrophes (e.g. "don't")
        .filter(w => w.length >= 3)
        .filter(w => w.length <= 50) // Server rejects words > MAX_CHARS
        .filter(w => !w.startsWith("'")) // Skip bare apostrophe fragments (e.g. "'s", "'t")
        .filter((w, i, arr) => arr.indexOf(w) === i); // unique
    if (words.length === 0) return '';
    return this.insert(words);
}
```

### Data File

```typescript
async dataFile(): Promise<string> {
    return this.serializedRequest(['data-file']);
}
```

### Health Check

```typescript
private lastAvailableCheck: number = 0;
private cachedAvailable: boolean = false;
private readonly RETRY_INTERVAL_MS = 30_000;

async isAvailable(): Promise<boolean> {
    const now = Date.now();
    if (this.cachedAvailable && now - this.lastAvailableCheck < this.RETRY_INTERVAL_MS) {
        return true;
    }

    try {
        await new Promise<void>((resolve, reject) => {
            const socket = net.createConnection({ host: this.host, port: this.port });
            socket.setTimeout(2000);
            socket.on('connect', () => { socket.destroy(); resolve(); });
            socket.on('timeout', () => { socket.destroy(); reject(new Error('timeout')); });
            socket.on('error', reject);
        });
        this.cachedAvailable = true;
        this.lastAvailableCheck = now;
        return true;
    } catch {
        this.cachedAvailable = false;
        this.lastAvailableCheck = now;
        return false;
    }
}
```

---

## Testing Strategy

Follows the same pattern as the Python client tests: standalone TypeScript scripts executed by `bun run`, run against a live string-space server as part of `make test`.

### Prerequisites

- `bun` installed and on `$PATH`
- Rust server built (`cargo build`)
- Test data file: `test/word_list.txt` (existing, ~10k words)

### Test Files

All TypeScript test scripts live alongside the Python test scripts:

```
tests/
├── ts_client.ts              # Smoke test: every API method
tests/
├── ts_best_completions.ts    # Comprehensive best-completions tests
tests/
├── ts_add_words.ts           # addWordsFromText extraction + batch insert
tests/
├── ts_protocol.ts            # Raw protocol validation (wire format, encoding, errors)
```

Each script:
- Takes a port number as its first CLI argument (e.g., `bun run tests/ts_client.ts 9898`)
- Creates a `StringSpaceClient` connecting to `127.0.0.1:<port>`
- Runs assertions with descriptive output
- Exits 0 on success, non-zero on failure (uncaught exception or explicit `process.exit(1)`)
- Uses `console.log` for progress output, prefixed with section headers

### Test Scripts

#### `tests/ts_client.ts` — Smoke Test

Mirrors `tests/client.py`. Exercises every public API method once against a server loaded with `test/word_list.txt`.

| Test | Method | Asserts |
|------|--------|----------|
| Prefix search | `prefixSearch('hel')` | Returns non-empty array |
| Substring search | `substringSearch('lo')` | Returns non-empty array |
| Similar search | `similarSearch('hello', 0.6)` | Returns non-empty array |
| Fuzzy-subsequence search | `fuzzySubsequenceSearch('hl')` | Returns non-empty array |
| Best completions | `bestCompletions('hel', 5)` | Returns ≤5 results |
| Insert | `insert(['testword1', 'testword2'])` | Returns string containing "OK" |
| Insert dedup | `insert(['testword1'])` again | Returns "Inserted 0 of 1" |
| Add words from text | `addWordsFromText(...)` | Returns non-empty string |
| Data file | `dataFile()` | Returns non-empty string |
| Availability | `isAvailable()` | Returns `true` |

```typescript
import { StringSpaceClient } from '../typescript/string-space-client';

const port = parseInt(process.argv[2]);
if (!port) { console.error('Usage: bun run tests/ts_client.ts <port>'); process.exit(1); }

const client = new StringSpaceClient('127.0.0.1', port);

function assert(condition: boolean, message: string) {
    if (!condition) { console.error(`FAIL: ${message}`); process.exit(1); }
    console.log(`  ✓ ${message}`);
}

// Prefix search
let results = await client.prefixSearch('hel');
assert(results.length > 0, `Prefix 'hel': ${results.length} results`);

// Best completions with limit
results = await client.bestCompletions('hel', 5);
assert(results.length <= 5, `Best completions 'hel' limit 5: ${results.length} results`);
assert(results.length > 0, `Best completions 'hel': has results`);

// ... (substring, similar, fuzzy-subsequence follow same pattern)

// Insert
const insertResult = await client.insert(['testword1', 'testword2']);
assert(insertResult.includes('OK'), `Insert: ${insertResult}`);

// Insert dedup
const dedupResult = await client.insert(['testword1']);
assert(dedupResult.includes('0 of 1'), `Insert dedup: ${dedupResult}`);

// Add words from text
const addResult = await client.addWordsFromText("The quick brown fox's jump");
assert(addResult.length > 0, `Add words from text: inserted`);

// Data file
const dataFile = await client.dataFile();
assert(dataFile.length > 0, `Data file: ${dataFile}`);

// Availability
const available = await client.isAvailable();
assert(available === true, 'isAvailable: true');

console.log('\n=== All smoke tests passed ===');
```

#### `tests/ts_best_completions.ts` — Comprehensive Best-Completions

Mirrors `tests/test_best_completions_integration.py`.

| Test Suite | What it covers |
|------------|---------------|
| Basic queries | Simple prefix, query with limit, short queries (1–2 chars) |
| Query lengths | 1-char through 7-char queries, verify limit respected |
| Edge cases | Non-existent query, max-length query (50 chars), empty query, special characters |
| Response format | All results are strings, no RS/EOT bytes in content |
| Performance | Each query completes in <1000ms |
| Concurrent clients | 5 concurrent `bestCompletions` calls via `Promise.all` on separate client instances, all succeed |
| Consistency | 3 identical requests return the same result set |

#### `tests/ts_add_words.ts` — Word Extraction + Batch Insert

Tests the `addWordsFromText` pipeline end-to-end.

| Test | Input | Asserts |
|------|-------|----------|
| Basic extraction | `'hello world foo bar baz'` | Words inserted, searchable by prefix |
| Apostrophe handling | `'don\'t won\'t can\'t'` | Full words preserved (not split) |
| Min length filter | `'a ab abc abcd'` | Only `abc`, `abcd` inserted |
| Max length filter | 51-char string | Rejected (0 inserted) |
| Dedup | `'hello hello hello'` | Only 1 instance inserted |
| Leading apostrophe | `'\'s \'t hello'` | `\'s` and `\'t` skipped, `hello` inserted |
| Large batch | 100 words | All inserted successfully |

#### `tests/ts_protocol.ts` — Raw Protocol Validation

Mirrors `tests/test_protocol_validation.py`. Tests the wire format directly using Node `net` module, independent of the client class.

| Test | What it validates |
|------|-----------------|
| Raw request format | RS-separated elements, EOT terminator produce valid response |
| Raw response format | Response ends with EOT, no RS bytes in content |
| Error response | Unknown command → response starts with `ERROR -` |
| Malformed request | Missing separators → server handles gracefully |
| UTF-8 encoding | Unicode query (e.g., `café`) handled without crash |
| EOT split across chunks | Server response >4096 bytes, verify client reassembles correctly |

### Integration with `make test` and `make ts-test`

**`make ts-test`** — dedicated target for TypeScript integration tests only. Runs `tests/run_ts_tests.sh`, which builds the server, starts a daemon on port 9898, runs all `ts_*.ts` scripts, then stops the server. Use during TS client development for fast iteration.

**`make test`** — full suite (existing). TypeScript test runs are appended after the Python tests, reusing the same daemon on port 9898. No new server start/stop cycles.

#### `tests/run_ts_tests.sh`

```bash
#!/bin/bash
set -e

cargo build
EXECUTABLE=target/debug/string_space
PORT=9898

# Clean up any existing server
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null || true

# Start daemon
RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE start test/word_list.txt -p $PORT -d
echo "Server started on port $PORT"

# Trap ensures cleanup on exit
trap 'RUST_BACKTRACE=full SS_TEST=true $EXECUTABLE stop 2>/dev/null' EXIT

# Run all TypeScript test scripts
for script in tests/ts_*.ts; do
    echo "\n=== Running $script ==="
    bun run "$script" $PORT
    echo "✓ $script passed"
done

echo "\n=== All TypeScript tests passed ==="
```

#### Addition to `tests/run_tests.sh` (full suite)

Appended after the Python test runs, inside the existing daemon lifecycle on port 9898:

```bash
# --- TypeScript tests (new) ---
for script in tests/ts_*.ts; do
    echo "\n=== Running $script ==="
    bun run "$script" 9898
    echo "✓ $script passed"
done
```

### Edge Cases Covered Across All Scripts

| Edge case | Covered by |
|-----------|-----------|
| Server not running | `isAvailable()` returns `false`; search throws after retry exhaustion |
| Empty query | `ts_best_completions.ts` — server returns error |
| Query too long (>50 chars) | `ts_best_completions.ts` — server truncates or rejects |
| Special characters in query | `ts_best_completions.ts` — graceful handling |
| Unicode / emoji | `ts_protocol.ts` — UTF-8 encoding |
| Large insert batch | `ts_add_words.ts` — 100 words |
| Duplicate inserts | `ts_client.ts` — dedup response check |
| Concurrent requests | `ts_best_completions.ts` — `Promise.all` on separate clients |
| EOT split across TCP chunks | `ts_protocol.ts` — large response reassembly |
| Word extraction edge cases | `ts_add_words.ts` — apostrophes, min/max length, dedup, leading apostrophe |

---

## Future: Packaging

Once the client is finalized and proven:

1. Create `typescript/package.json` with name `string-space-client`
2. Add proper exports, types, and README
3. Push to GitHub
4. Consumers install via: `"string-space-client": "github:user/repo#path=typescript"`
5. Extension switches from raw file import to package import

---

## Protocol Gotchas Summary

1. **One-shot connections**: Connect → send → receive → disconnect per request
2. **SEND_METADATA is false**: Responses contain only word text, not frequency/age
3. **Insert word joining**: Words joined with `\n` for insert, not RS
4. **EOT may be split across TCP chunks**: Must accumulate until EOT found
5. **3-second timeout**: Server read/write timeout; don't exceed on client
6. **No concurrency**: Server handles one connection at a time
7. **String length limits**: MIN_CHARS=3, MAX_CHARS=50
8. **Case sensitivity**: `best_completions` does case-insensitive prefix matching, bubbles case-sensitive matches above
9. **Insert comma handling**: The server also replaces commas with spaces during insert (in addition to newlines). Words containing commas will be split.
