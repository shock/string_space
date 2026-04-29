# String-Space TypeScript Client: Implementation Plan

**Created:** 2026-04-29
**Status:** Planning — implementation not yet started
**Parent plan:** `admin/string-space-acmpl/master-plan.md`

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
| EOT byte | `0x04` | `src/modules/protocol.rs:36` |
| RS byte | `0x1E` | `src/modules/protocol.rs:37` |
| Connection timeout | 3 seconds | `src/modules/protocol.rs:39` |
| Min word length | 3 chars | `src/modules/string_space/mod.rs:65` |
| Max word length | 50 chars | `src/modules/string_space/mod.rs:66` |

### Commands

| Command | Params | Response |
|---------|--------|----------|
| `insert` | `<word1>\n<word2>\n...` | `OK\nInserted N of M words` |
| `best-completions` | `<query>[<RS><limit>]` | newline-separated matches |
| `prefix` | `<prefix>` | newline-separated matches |
| `substring` | `<substring>` | newline-separated matches |
| `similar` | `<word><RS><threshold>` | newline-separated matches |
| `fuzzy-subsequence` | `<query>` | newline-separated matches |
| `remove` | `<words...>` | — |
| `data-file` | (none) | file path string |
| `len` | (none) | count |
| `empty` | (none) | `true`/`false` |

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
    words = [word for word in words if not word.startswith("'")]
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
    async len(): Promise<number>;
    async empty(): Promise<boolean>;
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

        socket.on('data', (chunk: Buffer) => {
            response = Buffer.concat([response, chunk]);
            if (chunk.includes(EOT)) {
                socket.destroy();
                const text = response.toString('utf-8').replace(/\x04$/, '');
                if (text.startsWith('ERROR')) {
                    reject(new Error(text));
                } else {
                    resolve(text);
                }
            }
        });

        socket.on('timeout', () => { socket.destroy(); reject(new Error('Connection timeout')); });
        socket.on('error', (err) => reject(err));

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
            await new Promise(r => setTimeout(r, 1000 * Math.pow(2, attempt))); // 1s, 2s
        }
    }
    throw new Error('Unreachable');
}
```

### Search Method Implementations

```typescript
async bestCompletions(query: string, limit: number = 10): Promise<string[]> {
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
        .split(/[^\w_-]+/)
        .filter(w => w.length >= 3)
        .filter(w => !w.startsWith("'"))
        .filter((w, i, arr) => arr.indexOf(w) === i); // unique
    if (words.length === 0) return '';
    return this.insert(words);
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

### Manual Testing

1. Start the string-space server: `cargo run -- start`
2. Run a test script that imports the client:

```typescript
import { StringSpaceClient } from './typescript/string-space-client';

const client = new StringSpaceClient('127.0.0.1', 7878);

// Health check
console.log('Available:', await client.isAvailable());

// Insert words
await client.insert(['hello', 'world', 'helicopter', 'helpful', 'hemisphere']);

// Search
console.log('Best completions for "hel":', await client.bestCompletions('hel', 5));
console.log('Prefix "he":', await client.prefixSearch('he'));

// Word learning from text
await client.addWordsFromText('The quick brown fox jumps over the lazy dog');
console.log('Best completions for "qui":', await client.bestCompletions('qui', 5));
```

### Edge Cases to Test

- Server not running → `isAvailable()` returns `false`, search throws with retry exhaustion
- Empty query → server rejects (min 3 chars)
- Query with special characters → server rejects control chars
- Large insert batch → verify all words added
- Concurrent requests → verify serialized (no interleaving)
- Split EOT across chunks → verify response assembly

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
