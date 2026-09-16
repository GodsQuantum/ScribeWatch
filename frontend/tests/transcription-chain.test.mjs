import test from 'node:test';
import assert from 'node:assert/strict';
import { addRoute, removeRoute, moveRoute, updateRoute } from '../src/lib/transcription-chain.js';

test('fallback routes preserve complete tuples while reordering', () => {
  const a = { providerId: 'p', model: 'large' };
  const b = { providerId: 'p', model: 'distil', fallbackAfterSeconds: 1800 };
  const source = [a, b];
  const moved = moveRoute(source, 1, 0);
  assert.deepEqual(moved, [b, a]);
  assert.deepEqual(source, [a, b]);
  assert.notEqual(moved, source);
});

test('add and remove are immutable and never leave an empty chain', () => {
  const source = [{ providerId: 'p', model: 'large' }];
  const added = addRoute(source);
  assert.deepEqual(added, [source[0], { providerId: '', model: '' }]);
  assert.deepEqual(source, [{ providerId: 'p', model: 'large' }]);
  assert.deepEqual(removeRoute(source, 0), source);
  assert.deepEqual(removeRoute(added, 1), source);
});

test('updateRoute changes exactly one tuple without mutating source', () => {
  const source = [
    { providerId: 'a', model: 'm1' },
    { providerId: 'b', model: 'm2', fallbackAfterSeconds: 600 },
  ];
  const updated = updateRoute(source, 1, { model: 'm3' });
  assert.deepEqual(updated, [source[0], { providerId: 'b', model: 'm3', fallbackAfterSeconds: 600 }]);
  assert.equal(source[1].model, 'm2');
});
