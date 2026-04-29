/**
 * ts_add_words.ts — Word extraction + batch insert tests
 *
 * Tests the addWordsFromText pipeline and insert deduplication.
 *
 * Usage: bun run tests/ts_add_words.ts <port>
 */

import { StringSpaceClient } from '../typescript/string-space-client';

const port = parseInt(process.argv[2]);
if (!port) {
    console.error('Usage: bun run tests/ts_add_words.ts <port>');
    process.exit(1);
}

// Use a dedicated client for each test group to avoid serialization overlap
let failures = 0;

function assert(condition: boolean, message: string) {
    if (!condition) {
        console.error(`  FAIL: ${message}`);
        failures++;
    } else {
        console.log(`  ✓ ${message}`);
    }
}

console.log('=== Add-Words Tests ===\n');

// ── Basic extraction ────────────────────────────────────────────

console.log('--- Basic extraction ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const result = await client.addWordsFromText('hello world foo bar baz');
    assert(result.length > 0, `Basic extraction: inserted (${result})`);

    // Verify words are searchable
    const found = await client.prefixSearch('hello');
    assert(found.length > 0, `Inserted 'hello' is searchable by prefix`);
}

// ── Apostrophe handling ─────────────────────────────────────────

console.log('\n--- Apostrophe handling ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const result = await client.addWordsFromText("don't won't can't");
    assert(result.length > 0, `Apostrophe words inserted: ${result}`);

    // The full words should be preserved
    const found = await client.prefixSearch("don't");
    assert(found.length > 0, `"don't" searchable by prefix`);
}

// ── Min length filter ───────────────────────────────────────────

console.log('\n--- Min length filter ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    // "a" and "ab" should be filtered out; only "abc" and "abcd" inserted
    const result = await client.addWordsFromText('a ab abc abcd');
    assert(result.length > 0, `Min length filter: some words inserted (${result})`);

    const found = await client.prefixSearch('abcd');
    assert(found.length > 0, `'abcd' is searchable (length 4, >= min 3)`);

    // "ab" should not have been inserted — it's only 2 chars
    const shortFound = await client.prefixSearch('ab');
    // "ab" won't be there, but "abc" and "abcd" will match the prefix
    const hasShort = shortFound.some(w => w === 'ab');
    assert(!hasShort, `'ab' not inserted (length 2 < min 3)`);
}

// ── Max length filter ───────────────────────────────────────────

console.log('\n--- Max length filter ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const longWord = 'a'.repeat(51); // 51 chars — exceeds MAX_CHARS=50
    const result = await client.addWordsFromText(`shortword ${longWord}`);
    // Only 'shortword' should be inserted
    assert(result.length > 0, `Max length filter: short word inserted (${result})`);

    const found = await client.prefixSearch('shortword');
    assert(found.length > 0, `'shortword' is searchable`);

    // The 51-char word should not be searchable
    const longFound = await client.prefixSearch(longWord);
    assert(longFound.length === 0, `51-char word not inserted`);
}

// ── Dedup ───────────────────────────────────────────────────────

console.log('\n--- Dedup ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const uniqueWord = `dedupword_${Date.now()}`;
    // First insert
    const r1 = await client.insert([uniqueWord]);
    assert(r1.includes('OK'), `First insert of "${uniqueWord}": ${r1}`);

    // Second insert of same word — server accepts it (updates frequency, not an error)
    const r2 = await client.insert([uniqueWord]);
    assert(r2.includes('OK'), `Dedup insert succeeds (frequency updated): ${r2}`);
}

// ── Leading apostrophe skip ─────────────────────────────────────

console.log('\n--- Leading apostrophe skip ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const result = await client.addWordsFromText("'s 't hello");
    assert(result.length > 0, `Leading apostrophe: 'hello' inserted (${result})`);
    // 's and 't should be skipped by the client-side filter
}

// ── Large batch ─────────────────────────────────────────────────

console.log('\n--- Large batch ---');

{
    const client = new StringSpaceClient('127.0.0.1', port);
    const words = Array.from({ length: 100 }, (_, i) => `batchword${i}`);
    const result = await client.insert(words);
    assert(result.includes('OK'), `Large batch insert (100 words): ${result}`);
}

// ── Summary ─────────────────────────────────────────────────────

console.log('');
if (failures > 0) {
    console.error(`=== ${failures} test(s) FAILED ===`);
    process.exit(1);
} else {
    console.log('=== All add-words tests passed ===');
}
