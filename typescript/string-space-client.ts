/**
 * String Space TypeScript Client
 *
 * TCP client for the string-space server — an in-memory word database
 * providing multi-algorithm completion.
 *
 * Wire format:
 *   Request:  <elem1><RS><elem2><RS>...<elemN><EOT>
 *   Response: <text><EOT>
 *
 * Constants:
 *   RS  = 0x1E (record separator)
 *   EOT = 0x04 (end of transmission)
 *
 * @module string-space-client
 */

import * as net from 'net';

// ── Protocol constants ──────────────────────────────────────────────
const EOT = 0x04;
const RS = '\x1E';
const CONNECTION_TIMEOUT_MS = 3000;
const HEALTH_CHECK_TIMEOUT_MS = 2000;

// ── Serialization lock ─────────────────────────────────────────────
// The server handles one client at a time, so we serialize requests
// through a promise chain.

/**
 * TypeScript TCP client for the string-space server.
 *
 * Default connection: 127.0.0.1:7878
 *
 * ```ts
 * const client = new StringSpaceClient('127.0.0.1', 7878);
 * const results = await client.bestCompletions('hel', 5);
 * ```
 */
export class StringSpaceClient {
    private host: string;
    private port: number;
    private debug: boolean;

    // Promise-chain lock for request serialization
    private queue: Promise<void> = Promise.resolve();

    // Availability cache
    private lastAvailableCheck: number = 0;
    private cachedAvailable: boolean = false;
    private readonly RETRY_INTERVAL_MS = 30_000;

    constructor(host: string = '127.0.0.1', port: number = 7878, debug: boolean = false) {
        this.host = host;
        this.port = port;
        this.debug = debug;
    }

    // ── Search Methods ──────────────────────────────────────────────

    /**
     * Intelligent multi-algorithm completion.
     * Combines prefix, fuzzy-subsequence, Jaro-Winkler similarity, and substring
     * searches with progressive execution and dynamic weighting.
     */
    async bestCompletions(query: string, limit: number = 10): Promise<string[]> {
        const response = await this.serializedRequest(['best-completions', query, String(limit)]);
        return response.split('\n').filter(line => line.length > 0);
    }

    /** Search by exact prefix match. */
    async prefixSearch(prefix: string): Promise<string[]> {
        const response = await this.serializedRequest(['prefix', prefix]);
        return response.split('\n').filter(line => line.length > 0);
    }

    /** Search by substring occurrence. */
    async substringSearch(substring: string): Promise<string[]> {
        const response = await this.serializedRequest(['substring', substring]);
        return response.split('\n').filter(line => line.length > 0);
    }

    /** Fuzzy similarity search using Jaro-Winkler distance. */
    async similarSearch(word: string, threshold: number): Promise<string[]> {
        const response = await this.serializedRequest(['similar', word, String(threshold)]);
        return response.split('\n').filter(line => line.length > 0);
    }

    /** Character order-preserving fuzzy subsequence search. */
    async fuzzySubsequenceSearch(query: string): Promise<string[]> {
        const response = await this.serializedRequest(['fuzzy-subsequence', query]);
        return response.split('\n').filter(line => line.length > 0);
    }

    // ── Mutation Methods ────────────────────────────────────────────

    /**
     * Insert one or more words into the database.
     * Words must be 3–50 characters; the server silently filters invalid ones.
     * @returns Server response string (e.g. "OK\nInserted 2 of 2 words")
     */
    async insert(words: string[]): Promise<string> {
        // Match Python client: words joined with \n, NOT RS
        const response = await this.serializedRequest(['insert', words.join('\n')]);
        return response;
    }

    /**
     * Extract words from free-form text and insert them.
     * Applies the same filtering rules as the Python client:
     *   - Split on non-word characters (keeps apostrophes, hyphens, underscores)
     *   - Filter: length 3–50, no leading apostrophe, deduplicated
     */
    async addWordsFromText(text: string): Promise<string> {
        const words = text
            .split(/[^\w_\-']+/)
            .filter(w => w.length >= 3)
            .filter(w => w.length <= 50)
            .filter(w => !w.startsWith("'"))
            .filter((w, i, arr) => arr.indexOf(w) === i); // unique
        if (words.length === 0) return '';
        return this.insert(words);
    }

    // ── Health & Utility ────────────────────────────────────────────

    /** Returns the server's data file path. */
    async dataFile(): Promise<string> {
        return this.serializedRequest(['data-file']);
    }

    /**
     * Check if the server is reachable.
     * Results are cached for 30 seconds to avoid hammering the port.
     */
    async isAvailable(): Promise<boolean> {
        const now = Date.now();
        if (this.cachedAvailable && now - this.lastAvailableCheck < this.RETRY_INTERVAL_MS) {
            return true;
        }

        try {
            await new Promise<void>((resolve, reject) => {
                const socket = net.createConnection({ host: this.host, port: this.port });
                socket.setTimeout(HEALTH_CHECK_TIMEOUT_MS);
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

    // ── Internal: Request Pipeline ──────────────────────────────────

    /**
     * Serialized request: ensures only one request is in-flight at a time
     * (the server is single-client).
     */
    private async serializedRequest(elements: string[]): Promise<string> {
        let resolver!: () => void;
        const waiter = new Promise<void>(r => { resolver = r; });
        const prev = this.queue;
        this.queue = waiter;

        await prev;
        try {
            return await this.requestWithRetry(elements);
        } finally {
            resolver();
        }
    }

    /**
     * Retry up to 2 times with exponential backoff on connection-level errors.
     */
    private async requestWithRetry(elements: string[], maxRetries = 2): Promise<string> {
        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                return await this.request(elements);
            } catch (err) {
                if (attempt === maxRetries) throw err;
                if (!this.isConnectionError(err)) throw err;
                if (this.debug) {
                    console.error(`[DEBUG] Retry attempt ${attempt + 1} after error: ${err}`);
                }
                await new Promise(r => setTimeout(r, 1000 * Math.pow(2, attempt)));
            }
        }
        throw new Error('Unreachable');
    }

    /**
     * Core transport: connect → send → receive → disconnect.
     */
    private request(elements: string[]): Promise<string> {
        return new Promise<string>((resolve, reject) => {
            const payload = elements.join(RS);
            const serialized = Buffer.from(payload + '\x04', 'utf-8');

            if (this.debug) {
                console.error(`[DEBUG] Sending: ${payload} (+EOT)`);
            }

            const socket = net.createConnection({ host: this.host, port: this.port });
            socket.setTimeout(CONNECTION_TIMEOUT_MS);

            let response = Buffer.alloc(0);
            let settled = false;

            const settle = (fn: () => void) => {
                if (settled) return;
                settled = true;
                fn();
            };

            socket.on('data', (chunk: Buffer) => {
                response = Buffer.concat([response, chunk]);
                // Check if the EOT byte has arrived (may be split across chunks)
                if (chunk.includes(EOT)) {
                    settle(() => {
                        socket.destroy();
                        const text = response.toString('utf-8').replace(/\x04+$/, '');
                        if (this.debug) {
                            console.error(`[DEBUG] Response: ${text}`);
                        }
                        if (text.startsWith('ERROR')) {
                            reject(new Error(text));
                        } else {
                            resolve(text);
                        }
                    });
                }
            });

            socket.on('timeout', () => {
                settle(() => {
                    socket.destroy();
                    reject(new Error('Connection timeout'));
                });
            });

            socket.on('error', (err) => {
                settle(() => {
                    reject(err);
                });
            });

            socket.on('close', () => {
                settle(() => {
                    reject(new Error('Connection closed by server'));
                });
            });

            socket.write(serialized);
        });
    }

    /**
     * Determine whether an error is retryable (connection-level).
     */
    private isConnectionError(err: unknown): boolean {
        if (err instanceof Error) {
            const msg = err.message;
            return (
                msg.includes('ECONNREFUSED') ||
                msg.includes('ECONNRESET') ||
                msg.includes('ETIMEDOUT') ||
                msg.includes('Connection timeout') ||
                msg.includes('EPIPE') ||
                msg.includes('Connection closed by server')
            );
        }
        return false;
    }
}
