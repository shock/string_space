/**
 * ts_client.ts — Smoke test for StringSpaceClient
 *
 * Exercises every public API method once against a server loaded
 * with test/word_list.txt.
 *
 * Usage: bun run tests/ts_client.ts <port>
 */

import { StringSpaceClient } from '../typescript/string-space-client';

const port = parseInt(process.argv[2]);
if (!port) {
    console.error('Usage: bun run tests/ts_client.ts <port>');
    process.exit(1);
}

const client = new StringSpaceClient('127.0.0.1', port);

function assert(condition: boolean, message: string) {
    if (!condition) {
        console.error(`FAIL: ${message}`);
        process.exit(1);
    }
    console.log(`  ✓ ${message}`);
}

console.log('=== Smoke Test: StringSpaceClient ===\n');

// ── Search methods ──────────────────────────────────────────────

console.log('--- Search methods ---');

let results: string[];

// Prefix search
results = await client.prefixSearch('hel');
assert(results.length > 0, `prefixSearch('hel'): ${results.length} results`);

// Substring search
results = await client.substringSearch('lo');
assert(results.length > 0, `substringSearch('lo'): ${results.length} results`);

// Similar search
results = await client.similarSearch('hello', 0.6);
assert(results.length > 0, `similarSearch('hello', 0.6): ${results.length} results`);

// Fuzzy-subsequence search
results = await client.fuzzySubsequenceSearch('hl');
assert(results.length > 0, `fuzzySubsequenceSearch('hl'): ${results.length} results`);

// Best completions with limit
results = await client.bestCompletions('hel', 5);
assert(results.length > 0, `bestCompletions('hel', 5): has results (${results.length})`);
assert(results.length <= 5, `bestCompletions('hel', 5): respects limit (got ${results.length})`);

// ── Mutation methods ────────────────────────────────────────────

console.log('\n--- Mutation methods ---');

// Insert
const insertResult = await client.insert(['testword1', 'testword2']);
assert(insertResult.includes('OK'), `insert(['testword1','testword2']): ${insertResult}`);

// Insert dedup — server accepts duplicate inserts (updates frequency)
const dedupResult = await client.insert(['testword1']);
assert(dedupResult.includes('OK'), `insert dedup accepted: ${dedupResult}`);

// Add words from text
const addResult = await client.addWordsFromText("The quick brown fox's jump over the lazy dog");
assert(addResult.length > 0, `addWordsFromText(...): inserted (${addResult})`);

// ── Health & utility ────────────────────────────────────────────

console.log('\n--- Health & utility ---');

// Data file
const dataFile = await client.dataFile();
assert(dataFile.length > 0, `dataFile(): ${dataFile}`);

// Availability
const available = await client.isAvailable();
assert(available === true, 'isAvailable(): true');

console.log('\n=== All smoke tests passed ===');
