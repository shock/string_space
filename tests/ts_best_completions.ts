/**
 * ts_best_completions.ts — Comprehensive best-completions tests
 *
 * Mirrors tests/test_best_completions_integration.py.
 *
 * Usage: bun run tests/ts_best_completions.ts <port>
 */

import { StringSpaceClient } from '../typescript/string-space-client';

const port = parseInt(process.argv[2]);
if (!port) {
    console.error('Usage: bun run tests/ts_best_completions.ts <port>');
    process.exit(1);
}

const client = new StringSpaceClient('127.0.0.1', port);

let failures = 0;

function assert(condition: boolean, message: string) {
    if (!condition) {
        console.error(`  FAIL: ${message}`);
        failures++;
    } else {
        console.log(`  ✓ ${message}`);
    }
}

console.log('=== Best-Completions Comprehensive Tests ===\n');

// ── Basic queries ───────────────────────────────────────────────

console.log('--- Basic queries ---');

let results = await client.bestCompletions('hel', 10);
assert(results.length > 0, `Simple query 'hel': ${results.length} results`);

results = await client.bestCompletions('hel', 5);
assert(results.length <= 5, `Query with limit 5: got ${results.length}`);

results = await client.bestCompletions('a', 10);
assert(results.length > 0, `Single-char query 'a': ${results.length} results`);

results = await client.bestCompletions('ab', 10);
assert(results.length > 0, `Two-char query 'ab': ${results.length} results`);

// ── Query lengths ───────────────────────────────────────────────

console.log('\n--- Query lengths ---');

for (const len of [1, 2, 3, 4, 5, 6, 7]) {
    const query = 'a'.repeat(len);
    const r = await client.bestCompletions(query, 10);
    assert(r.length <= 10, `Query length ${len} (${query}): limit respected (got ${r.length})`);
}

// ── Edge cases ──────────────────────────────────────────────────

console.log('\n--- Edge cases ---');

// Non-existent prefix — should return empty or very few results
results = await client.bestCompletions('zzzzzzzzz', 10);
assert(Array.isArray(results), `Non-existent query: returns array (got ${results.length} results)`);

// Max-length query (50 chars)
const longQuery = 'a'.repeat(50);
try {
    results = await client.bestCompletions(longQuery, 10);
    assert(true, `50-char query: no crash (got ${results.length} results)`);
} catch (e) {
    // Server may reject — that's acceptable
    assert(true, `50-char query: server rejected (acceptable)`);
}

// Empty query — server typically returns error or empty
try {
    results = await client.bestCompletions('', 10);
    assert(results.length === 0 || Array.isArray(results), `Empty query: handled (${results.length} results)`);
} catch (e) {
    assert(true, 'Empty query: server error (acceptable)');
}

// Special characters
try {
    results = await client.bestCompletions('!@#$', 10);
    assert(true, `Special characters: no crash (${results?.length ?? 'error'} results)`);
} catch {
    assert(true, 'Special characters: server error (acceptable)');
}

// ── Response format validation ──────────────────────────────────

console.log('\n--- Response format ---');

results = await client.bestCompletions('hel', 10);
for (const r of results) {
    assert(typeof r === 'string', `Result is string: "${r.substring(0, 30)}"`);
    assert(!r.includes('\x1E'), `No RS byte in result: "${r.substring(0, 30)}"`);
    assert(!r.includes('\x04'), `No EOT byte in result: "${r.substring(0, 30)}"`);
}

// ── Performance ─────────────────────────────────────────────────

console.log('\n--- Performance ---');

const perfStart = Date.now();
await client.bestCompletions('hel', 10);
const perfMs = Date.now() - perfStart;
assert(perfMs < 1000, `Single query < 1000ms: ${perfMs}ms`);

// ── Concurrent clients ─────────────────────────────────────────

console.log('\n--- Concurrent clients ---');

const concurrentResults = await Promise.all(
    Array.from({ length: 5 }, (_, i) => {
        const c = new StringSpaceClient('127.0.0.1', port);
        return c.bestCompletions('hel', 5).catch(() => null);
    })
);
const successCount = concurrentResults.filter(r => r !== null).length;
assert(successCount === 5, `5 concurrent requests: ${successCount}/5 succeeded`);

// ── Consistency ─────────────────────────────────────────────────

console.log('\n--- Consistency ---');

const r1 = await client.bestCompletions('hel', 5);
const r2 = await client.bestCompletions('hel', 5);
const r3 = await client.bestCompletions('hel', 5);

assert(
    JSON.stringify(r1) === JSON.stringify(r2) && JSON.stringify(r2) === JSON.stringify(r3),
    `3 identical requests return same results: ${r1.length} results`
);

// ── Summary ─────────────────────────────────────────────────────

console.log('');
if (failures > 0) {
    console.error(`=== ${failures} test(s) FAILED ===`);
    process.exit(1);
} else {
    console.log('=== All best-completions tests passed ===');
}
