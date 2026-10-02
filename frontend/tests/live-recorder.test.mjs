import test from 'node:test';
import assert from 'node:assert/strict';
import { LiveRecorderController } from '../src/lib/live-recorder.ts';

const flush = async () => {
  await Promise.resolve();
  await new Promise((resolve) => setImmediate(resolve));
  await Promise.resolve();
};

class MemoryStore {
  constructor() {
    this.sessions = new Map();
    this.failNext = false;
    this.pendingAppend = null;
  }
  async createSession(manifest) { this.sessions.set(manifest.id, structuredClone(manifest)); }
  async getSession(id) {
    const value = this.sessions.get(id);
    return value ? structuredClone(value) : undefined;
  }
  async listRecoverable() { return [...this.sessions.values()].map(v => structuredClone(v)); }
  async appendChunk(id, segmentId, blob, checkpointAtMs) {
    if (this.pendingAppend) await this.pendingAppend;
    if (this.failNext) { this.failNext = false; throw new Error('disk full'); }
    const value = structuredClone(this.sessions.get(id));
    const segment = value.segments.find((s) => s.id === segmentId);
    segment.chunkCount += 1;
    segment.bytes += blob.size;
    segment.lastCheckpointAtMs = checkpointAtMs;
    value.totalBytes += blob.size;
    value.lastCheckpointAtMs = checkpointAtMs;
    value.updatedAtMs = checkpointAtMs;
    this.sessions.set(id, value);
    return structuredClone(value);
  }
  async updateSession(manifest) { this.sessions.set(manifest.id, structuredClone(manifest)); }
  async buildSegmentBlob() { return new Blob([]); }
  async deleteSession(id) { this.sessions.delete(id); }
  async saveClientDirectoryHandle() {}
  async loadClientDirectoryHandle() { return undefined; }
}

class FakeTrack {
  constructor() { this.listeners = new Map(); this.stopped = false; }
  addEventListener(name, fn) { this.listeners.set(name, fn); }
  removeEventListener(name, fn) { if (this.listeners.get(name) === fn) this.listeners.delete(name); }
  stop() { this.stopped = true; }
  end() { this.listeners.get('ended')?.(); }
}
class FakeStream {
  constructor() { this.track = new FakeTrack(); }
  getTracks() { return [this.track]; }
}

class FakeRecorder {
  static instances = [];
  static failStart = false;
  constructor(stream, options) {
    this.stream = stream;
    this.options = options;
    this.state = 'inactive';
    this.mimeType = options.mimeType || 'audio/webm';
    this.ondataavailable = null;
    this.onstop = null;
    this.onerror = null;
    this.requestDataCount = 0;
    FakeRecorder.instances.push(this);
  }
  start(timeslice) {
    if (FakeRecorder.failStart) throw new Error('recorder start failed');
    this.timeslice = timeslice;
    this.state = 'recording';
  }
  requestData() { this.requestDataCount += 1; }
  stop() {
    if (this.state === 'inactive') return;
    this.state = 'inactive';
    queueMicrotask(() => this.onstop?.());
  }
  emit(text) { this.ondataavailable?.({ data: new Blob([text], { type: this.mimeType }) }); }
  fail(message='recorder failed') { this.onerror?.({ error: new Error(message) }); }
}

class FakeDocument {
  constructor() { this.visibilityState = 'visible'; this.listeners = new Map(); }
  addEventListener(name, fn) { this.listeners.set(name, fn); }
  removeEventListener(name, fn) { if (this.listeners.get(name) === fn) this.listeners.delete(name); }
  hide() { this.visibilityState = 'hidden'; this.listeners.get('visibilitychange')?.(); }
}

function options() {
  return { outputKind:'client', frontmatter:true, paragraphs:true };
}
function config() {
  return {
    options: options(),
    retention: { mode:'none' },
    mimeType: 'audio/webm;codecs=opus',
    audioBitsPerSecond: 64000,
  };
}
function controller({ store = new MemoryStore(), nowRef = { value: 1_000 }, document = new FakeDocument() } = {}) {
  FakeRecorder.instances.length = 0;
  FakeRecorder.failStart = false;
  const instance = new LiveRecorderController({
    store,
    MediaRecorderCtor: FakeRecorder,
    now: () => nowRef.value,
    document,
    wakeLock: undefined,
    idFactory: () => '550e8400-e29b-41d4-a716-446655440000',
    setInterval: () => 1,
    clearInterval: () => {},
  });
  return { instance, store, nowRef, document };
}

test('checkpoint is persisted before protected bytes advance', async () => {
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const store = new MemoryStore();
  store.pendingAppend = gate;
  const { instance } = controller({ store });
  await instance.startSession(config(), new FakeStream());
  FakeRecorder.instances[0].emit('abc');
  await flush();
  assert.equal(instance.getSnapshot().protectedBytes, 0);
  release();
  await flush();
  assert.equal(instance.getSnapshot().protectedBytes, 3);
});

test('continue creates next segment and records gap without silence', async () => {
  const store = new MemoryStore();
  const previous = {
    schemaVersion:1, id:'550e8400-e29b-41d4-a716-446655440000', state:'interrupted',
    createdAtMs:100, startedAtMs:100, updatedAtMs:1_000, lastCheckpointAtMs:1_000,
    mimeType:'audio/webm', audioBitsPerSecond:64000,
    segments:[{id:1,mimeType:'audio/webm',startedAtMs:100,endedAtMs:1_000,lastCheckpointAtMs:1_000,chunkCount:2,bytes:6}],
    totalBytes:6, options:options(), retention:{mode:'none'},
  };
  await store.createSession(previous);
  const nowRef={value:1_500};
  const { instance }=controller({store,nowRef});
  const resumed=await instance.continueSession(previous.id,new FakeStream());
  assert.equal(resumed.segments.length,2);
  assert.equal(resumed.segments[1].gapMsBefore,500);
  assert.equal(resumed.segments[1].startedAtMs,1_500);
});

test('storage write failure interrupts and preserves prior chunks', async () => {
  const store=new MemoryStore();
  const {instance}=controller({store});
  await instance.startSession(config(),new FakeStream());
  store.failNext=true;
  FakeRecorder.instances[0].emit('lost');
  await flush(); await flush();
  const saved=await store.getSession('550e8400-e29b-41d4-a716-446655440000');
  assert.equal(saved.state,'interrupted');
  assert.equal(saved.totalBytes,0);
  assert.match(instance.getSnapshot().error,/disk full/);
});

test('recorder error interrupts recoverably', async () => {
  const {instance,store}=controller();
  await instance.startSession(config(),new FakeStream());
  FakeRecorder.instances[0].fail('boom');
  await flush(); await flush();
  assert.equal((await store.getSession('550e8400-e29b-41d4-a716-446655440000')).state,'interrupted');
  assert.match(instance.getSnapshot().error,/boom/);
});

test('track ended interrupts recoverably', async () => {
  const {instance,store}=controller();
  const stream=new FakeStream();
  await instance.startSession(config(),stream);
  stream.track.end();
  await flush(); await flush();
  assert.equal((await store.getSession('550e8400-e29b-41d4-a716-446655440000')).state,'interrupted');
});

test('second start or continue is rejected while active', async () => {
  const {instance}=controller();
  const stream=new FakeStream();
  await instance.startSession(config(),stream);
  await assert.rejects(instance.startSession(config(),new FakeStream()),/already active/i);
  await assert.rejects(instance.continueSession('550e8400-e29b-41d4-a716-446655440000',new FakeStream()),/already active/i);
});

test('destroying a view subscription does not stop recording', async () => {
  const {instance}=controller();
  await instance.startSession(config(),new FakeStream());
  const unsubscribe=instance.subscribe(()=>{});
  unsubscribe();
  assert.equal(FakeRecorder.instances[0].state,'recording');
  assert.equal(instance.getSnapshot().recording,true);
});

test('hidden page requests checkpoint', async () => {
  const document=new FakeDocument();
  const {instance}=controller({document});
  await instance.startSession(config(),new FakeStream());
  document.hide();
  assert.equal(FakeRecorder.instances[0].requestDataCount,1);
});

test('elapsed uses timestamps not interval tick count', async () => {
  const nowRef={value:1_000};
  const {instance}=controller({nowRef});
  await instance.startSession(config(),new FakeStream());
  nowRef.value=5_500;
  assert.equal(instance.getSnapshot().elapsedMs,4_500);
});

test('session create failure stops the unowned microphone stream', async () => {
  const store = new MemoryStore();
  store.createSession = async () => { throw new Error('opfs unavailable'); };
  const { instance } = controller({ store });
  const stream = new FakeStream();
  await assert.rejects(instance.startSession(config(), stream), /opfs unavailable/);
  assert.equal(stream.track.stopped, true);
  assert.equal(instance.getSnapshot().recording, false);
});

test('failed Continue recorder start restores interrupted state and stops microphone', async () => {
  const store = new MemoryStore();
  const previous = {
    schemaVersion:1, id:'550e8400-e29b-41d4-a716-446655440000', state:'interrupted',
    createdAtMs:100, startedAtMs:100, updatedAtMs:1_000, lastCheckpointAtMs:1_000,
    mimeType:'audio/webm', audioBitsPerSecond:64000,
    segments:[{id:1,mimeType:'audio/webm',startedAtMs:100,endedAtMs:1_000,lastCheckpointAtMs:1_000,chunkCount:2,bytes:6}],
    totalBytes:6, options:options(), retention:{mode:'none'},
  };
  await store.createSession(previous);
  const nowRef={value:1_500};
  const { instance }=controller({store,nowRef});
  const stream=new FakeStream();
  FakeRecorder.failStart=true;
  await assert.rejects(instance.continueSession(previous.id,stream),/recorder start failed/);
  const saved=await store.getSession(previous.id);
  assert.equal(saved.state,'interrupted');
  assert.equal(saved.segments.length,1);
  assert.equal(stream.track.stopped,true);
  assert.equal(instance.getSnapshot().recording,false);
});
