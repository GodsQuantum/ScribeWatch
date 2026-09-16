import test from 'node:test';
import assert from 'node:assert/strict';
import { parseApiResponse } from '../src/lib/api-response.js';

test('successful empty responses do not require JSON', async () => {
  assert.equal(await parseApiResponse(new Response(null, { status: 204 })), undefined);
  assert.equal(await parseApiResponse(new Response('', { status: 200 })), undefined);
});

test('JSON responses are parsed once', async () => {
  const value = await parseApiResponse(new Response('{"status":"ok"}', {
    status: 200,
    headers: { 'content-type': 'application/json' }
  }));
  assert.deepEqual(value, { status: 'ok' });
});
