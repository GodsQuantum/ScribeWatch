import test from 'node:test';
import assert from 'node:assert/strict';
import { OpfsLiveRecoveryStore } from '../src/lib/live-recovery.ts';

class FakeFileHandle {
  constructor(path, fs) { this.path = path; this.fs = fs; this.kind = 'file'; this.data = new Blob([]); }
  async createWritable() {
    const handle = this;
    let pending = new Blob([]);
    return {
      async write(value) {
        if (handle.fs.failSuffix && handle.path.endsWith(handle.fs.failSuffix)) {
          throw new Error('simulated durable write failure');
        }
        pending = value instanceof Blob ? value : new Blob([value]);
      },
      async close() {
        handle.data = pending;
        handle.fs.log.push(`close:${handle.path}`);
      },
    };
  }
  async getFile() { return this.data; }
}

class FakeDirectoryHandle {
  constructor(path, fs) { this.path = path; this.fs = fs; this.kind = 'directory'; this.children = new Map(); }
  async getDirectoryHandle(name, { create = false } = {}) {
    const found = this.children.get(name);
    if (found?.kind === 'directory') return found;
    if (!create) throw new Error('directory not found');
    const dir = new FakeDirectoryHandle(`${this.path}/${name}`, this.fs);
    this.children.set(name, dir);
    return dir;
  }
  async getFileHandle(name, { create = false } = {}) {
    const found = this.children.get(name);
    if (found?.kind === 'file') return found;
    if (!create) throw new Error('file not found');
    const file = new FakeFileHandle(`${this.path}/${name}`, this.fs);
    this.children.set(name, file);
    return file;
  }
  async removeEntry(name) {
    if (!this.children.delete(name)) throw new Error('entry not found');
  }
  async *entries() { for (const pair of this.children.entries()) yield pair; }
}

class FakeHandleStore {
  constructor() { this.map = new Map(); }
  async put(id, handle) { this.map.set(id, handle); }
  async get(id) { return this.map.get(id); }
  async delete(id) { this.map.delete(id); }
}

function env() {
  const fs = { log: [], failSuffix: undefined };
  return { fs, root: new FakeDirectoryHandle('', fs), handles: new FakeHandleStore() };
}

function manifest(id = '550e8400-e29b-41d4-a716-446655440000') {
  return {
    schemaVersion: 1,
    id,
    state: 'recording',
    createdAtMs: 100,
    startedAtMs: 100,
    updatedAtMs: 100,
    mimeType: 'audio/webm;codecs=opus',
    audioBitsPerSecond: 64000,
    segments: [{
      id: 1,
      mimeType: 'audio/webm;codecs=opus',
      startedAtMs: 100,
      chunkCount: 0,
      bytes: 0,
    }],
    totalBytes: 0,
    options: { outputKind: 'client', frontmatter: true, paragraphs: true },
    retention: { mode: 'none' },
  };
}

test('append chunk updates manifest only after durable write', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  e.fs.log.length = 0;
  const saved = await store.appendChunk(manifest().id, 1, new Blob(['abc']), 200);
  assert.equal(saved.totalBytes, 3);
  assert.equal(saved.segments[0].chunkCount, 1);
  assert.equal(saved.lastCheckpointAtMs, 200);
  const chunkClose = e.fs.log.findIndex(v => v.endsWith('/chunk-000001.blob'));
  const manifestClose = e.fs.log.findIndex(v => v.endsWith('/manifest.json'));
  assert.ok(chunkClose >= 0 && manifestClose > chunkClose, e.fs.log.join('\n'));
});

test('recording manifest is returned as interrupted after recovery scan', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  const recovered = await store.listRecoverable();
  assert.equal(recovered.length, 1);
  assert.equal(recovered[0].state, 'interrupted');
  assert.equal((await store.getSession(manifest().id)).state, 'interrupted');
});

test('segment blob preserves chunk order', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  await store.appendChunk(manifest().id, 1, new Blob(['one']), 200);
  await store.appendChunk(manifest().id, 1, new Blob(['two']), 300);
  const blob = await store.buildSegmentBlob(manifest().id, 1);
  assert.equal(await blob.text(), 'onetwo');
  assert.equal(blob.type, 'audio/webm;codecs=opus');
});

test('delete session removes manifest chunks and saved handle', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  await store.saveClientDirectoryHandle(manifest().id, { name: 'picked' });
  await store.deleteSession(manifest().id);
  assert.equal(await store.getSession(manifest().id), undefined);
  assert.equal(await store.loadClientDirectoryHandle(manifest().id), undefined);
});

test('write failure keeps previous checkpoint metadata unchanged', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  e.fs.failSuffix = 'chunk-000001.blob';
  await assert.rejects(
    store.appendChunk(manifest().id, 1, new Blob(['lost']), 200),
    /simulated durable write failure/,
  );
  const saved = await store.getSession(manifest().id);
  assert.equal(saved.totalBytes, 0);
  assert.equal(saved.segments[0].chunkCount, 0);
  assert.equal(saved.lastCheckpointAtMs, undefined);
});

test('corrupted current manifest falls back to previous durable checkpoint', async () => {
  const e = env();
  const store = new OpfsLiveRecoveryStore(() => Promise.resolve(e.root), e.handles);
  await store.createSession(manifest());
  await store.appendChunk(manifest().id, 1, new Blob(['one']), 200);
  await store.appendChunk(manifest().id, 1, new Blob(['two']), 300);

  const app = e.root.children.get('scribewatch-live');
  const schema = app.children.get('v1');
  const session = schema.children.get(manifest().id);
  session.children.get('manifest.json').data = new Blob(['{"broken":']);

  const recovered = await store.getSession(manifest().id);
  assert.equal(recovered.totalBytes, 3);
  assert.equal(recovered.segments[0].chunkCount, 1);
  assert.equal(recovered.lastCheckpointAtMs, 200);
});
