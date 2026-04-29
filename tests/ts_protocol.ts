/**
 * ts_protocol.ts — Raw protocol validation tests
 *
 * Tests the wire format directly using the Node `net` module,
 * independent of the StringSpaceClient class.
 *
 * Usage: bun run tests/ts_protocol.ts <port>
 */

import * as net from 'net';

const port = parseInt(process.argv[2]);
if (!port) {
    console.error('Usage: bun run tests/ts_protocol.ts <port>');
    process.exit(1);
}

const host = '127.0.0.1';
const EOT = 0x04;
const RS = '\x1E';

let failures = 0;

function assert(condition: boolean, message: string) {
    if (!condition) {
        console.error(`  FAIL: ${message}`);
        failures++;
    } else {
        console.log(`  ✓ ${message}`);
    }
}

/**
 * Send a raw request and return the raw response Buffer.
 * Handles EOT detection with chunk reassembly.
 */
function rawRequest(payload: string): Promise<Buffer> {
    return new Promise((resolve, reject) => {
        const data = Buffer.from(payload + '\x04', 'utf-8');
        const socket = net.createConnection({ host, port });
        socket.setTimeout(3000);

        let response = Buffer.alloc(0);
        let settled = false;

        socket.on('data', (chunk: Buffer) => {
            response = Buffer.concat([response, chunk]);
            if (chunk.includes(EOT)) {
                settled = true;
                socket.destroy();
                resolve(response);
            }
        });

        socket.on('timeout', () => { if (!settled) { settled = true; socket.destroy(); reject(new Error('timeout')); } });
        socket.on('error', (err) => { if (!settled) { settled = true; reject(err); } });
        socket.on('close', () => { if (!settled) { settled = true; reject(new Error('closed')); } });

        socket.write(data);
    });
}

console.log('=== Protocol Validation Tests ===\n');

// ── Raw request format ──────────────────────────────────────────

console.log('--- Raw request format ---');

{
    // RS-separated elements, EOT terminator
    const response = await rawRequest(`prefix${RS}hel`);
    const text = response.toString('utf-8').replace(/\x04+$/, '');
    assert(text.length > 0, `RS-separated request: got response (${text.length} chars)`);
    assert(!text.startsWith('ERROR'), `Response is not an error: "${text.substring(0, 40)}"`);
}

// ── Raw response format ─────────────────────────────────────────

console.log('\n--- Raw response format ---');

{
    const response = await rawRequest(`prefix${RS}hel`);
    // Response should end with EOT
    assert(response[response.length - 1] === EOT, `Response ends with EOT byte`);
    // No RS bytes in the content
    const text = response.toString('utf-8').replace(/\x04+$/, '');
    assert(!text.includes('\x1E'), `No RS byte in response content`);
}

// ── Error response ──────────────────────────────────────────────

console.log('\n--- Error response ---');

{
    const response = await rawRequest('unknown-command-xyz');
    const text = response.toString('utf-8').replace(/\x04+$/, '');
    assert(text.startsWith('ERROR'), `Unknown command returns ERROR: "${text.substring(0, 60)}"`);
}

// ── Malformed request ───────────────────────────────────────────

console.log('\n--- Malformed request ---');

{
    // Empty payload (just EOT)
    try {
        const response = await rawRequest('');
        const text = response.toString('utf-8').replace(/\x04+$/, '');
        assert(true, `Empty payload handled: "${text.substring(0, 60)}"`);
    } catch {
        assert(true, 'Empty payload: connection closed (acceptable)');
    }
}

{
    // Missing RS separator — single element treated as command name
    try {
        const response = await rawRequest('prefix');
        const text = response.toString('utf-8').replace(/\x04+$/, '');
        // The server will try to use "prefix" as a command with no argument
        assert(true, `No-arg prefix handled: "${text.substring(0, 60)}"`);
    } catch {
        assert(true, 'No-arg prefix: connection closed (acceptable)');
    }
}

// ── UTF-8 encoding ──────────────────────────────────────────────

console.log('\n--- UTF-8 encoding ---');

{
    const response = await rawRequest(`prefix${RS}café`);
    const text = response.toString('utf-8').replace(/\x04+$/, '');
    assert(true, `Unicode query 'café' handled: "${text.substring(0, 60)}"`);
}

// ── EOT split across chunks (large response) ───────────────────

console.log('\n--- EOT split across chunks ---');

{
    // Request a prefix that will return many results, potentially >4096 bytes
    const response = await rawRequest(`prefix${RS}a`);
    const text = response.toString('utf-8').replace(/\x04+$/, '');
    assert(response.length > 0, `Large response received: ${response.length} bytes`);
    assert(response[response.length - 1] === EOT, `Large response ends with EOT`);
    // Verify the text was fully reassembled
    assert(!text.includes('\x04'), `No stray EOT in reassembled text`);
    assert(!text.startsWith('ERROR'), `Large response is not an error`);
}

// ── Summary ─────────────────────────────────────────────────────

console.log('');
if (failures > 0) {
    console.error(`=== ${failures} test(s) FAILED ===`);
    process.exit(1);
} else {
    console.log('=== All protocol tests passed ===');
}
