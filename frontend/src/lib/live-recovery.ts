import type { QuickOptions } from './types';

export type LiveSessionState =
  | 'new'
  | 'recording'
  | 'interrupted'
  | 'finalizing'
  | 'upload_pending'
  | 'uploading'
  | 'accepted'
  | 'completed';

export type LiveRetention =
  | { mode: 'none' }
  | { mode: 'server'; directory: string }
  | { mode: 'client'; handleKey: string };

export interface LiveSegmentManifest {
  id: number;
  mimeType: string;
  startedAtMs: number;
  endedAtMs?: number;
  lastCheckpointAtMs?: number;
  chunkCount: number;
  bytes: number;
  gapMsBefore?: number;
}

export interface LiveSessionManifest {
  schemaVersion: 1;
  id: string;
  state: LiveSessionState;
  createdAtMs: number;
  startedAtMs: number;
  updatedAtMs: number;
  lastCheckpointAtMs?: number;
  mimeType: string;
  audioBitsPerSecond: 64000;
  segments: LiveSegmentManifest[];
  totalBytes: number;
  options: QuickOptions;
  retention: LiveRetention;
  acceptedJobId?: string;
}

export interface ClientHandleStore {
  put(id: string, handle: FileSystemDirectoryHandle): Promise<void>;
  get(id: string): Promise<FileSystemDirectoryHandle | undefined>;
  delete(id: string): Promise<void>;
}

export interface LiveRecoveryStore {
  createSession(manifest: LiveSessionManifest): Promise<void>;
  getSession(id: string): Promise<LiveSessionManifest | undefined>;
  listRecoverable(): Promise<LiveSessionManifest[]>;
  appendChunk(
    sessionId: string,
    segmentId: number,
    blob: Blob,
    checkpointAtMs: number,
  ): Promise<LiveSessionManifest>;
  updateSession(manifest: LiveSessionManifest): Promise<void>;
  buildSegmentBlob(sessionId: string, segmentId: number): Promise<Blob>;
  deleteSession(id: string): Promise<void>;
  saveClientDirectoryHandle(id: string, handle: FileSystemDirectoryHandle): Promise<void>;
  loadClientDirectoryHandle(id: string): Promise<FileSystemDirectoryHandle | undefined>;
}

type RootProvider = () => Promise<FileSystemDirectoryHandle>;

const APP_DIR = 'scribewatch-live';
const SCHEMA_DIR = 'v1';
const MANIFEST = 'manifest.json';
const PREVIOUS_MANIFEST = 'manifest.previous.json';
const HANDLE_DB = 'scribewatch-live-handles';
const HANDLE_STORE = 'directories';

function cloneManifest(manifest: LiveSessionManifest): LiveSessionManifest {
  return JSON.parse(JSON.stringify(manifest)) as LiveSessionManifest;
}

async function defaultRootProvider(): Promise<FileSystemDirectoryHandle> {
  const storage = (globalThis.navigator as any)?.storage;
  if (!storage || typeof storage.getDirectory !== 'function') {
    throw new Error('Origin Private File System is unavailable in this browser.');
  }
  return storage.getDirectory();
}

function openHandleDatabase(): Promise<IDBDatabase> {
  if (typeof indexedDB === 'undefined') {
    return Promise.reject(new Error('IndexedDB is unavailable in this browser.'));
  }
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(HANDLE_DB, 1);
    request.onerror = () => reject(request.error ?? new Error('Could not open IndexedDB.'));
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(HANDLE_STORE)) {
        request.result.createObjectStore(HANDLE_STORE);
      }
    };
    request.onsuccess = () => resolve(request.result);
  });
}

class IndexedDbClientHandleStore implements ClientHandleStore {
  async put(id: string, handle: FileSystemDirectoryHandle): Promise<void> {
    const db = await openHandleDatabase();
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(HANDLE_STORE, 'readwrite');
      tx.objectStore(HANDLE_STORE).put(handle, id);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('Could not save directory handle.'));
      tx.onabort = () => reject(tx.error ?? new Error('Could not save directory handle.'));
    });
    db.close();
  }

  async get(id: string): Promise<FileSystemDirectoryHandle | undefined> {
    const db = await openHandleDatabase();
    const value = await new Promise<FileSystemDirectoryHandle | undefined>((resolve, reject) => {
      const tx = db.transaction(HANDLE_STORE, 'readonly');
      const request = tx.objectStore(HANDLE_STORE).get(id);
      request.onsuccess = () => resolve(request.result as FileSystemDirectoryHandle | undefined);
      request.onerror = () => reject(request.error ?? new Error('Could not load directory handle.'));
    });
    db.close();
    return value;
  }

  async delete(id: string): Promise<void> {
    if (typeof indexedDB === 'undefined') return;
    const db = await openHandleDatabase();
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(HANDLE_STORE, 'readwrite');
      tx.objectStore(HANDLE_STORE).delete(id);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('Could not delete directory handle.'));
      tx.onabort = () => reject(tx.error ?? new Error('Could not delete directory handle.'));
    });
    db.close();
  }
}

export class OpfsLiveRecoveryStore implements LiveRecoveryStore {
  private rootProvider: RootProvider;
  private handleStore: ClientHandleStore;

  constructor(
    rootProvider: RootProvider = defaultRootProvider,
    handleStore: ClientHandleStore = new IndexedDbClientHandleStore(),
  ) {
    this.rootProvider = rootProvider;
    this.handleStore = handleStore;
  }

  private async schemaDirectory(create: boolean): Promise<FileSystemDirectoryHandle> {
    const root = await this.rootProvider();
    const app = await root.getDirectoryHandle(APP_DIR, { create });
    return app.getDirectoryHandle(SCHEMA_DIR, { create });
  }

  private async sessionDirectory(id: string, create: boolean): Promise<FileSystemDirectoryHandle> {
    const schema = await this.schemaDirectory(create);
    return schema.getDirectoryHandle(id, { create });
  }

  private async segmentDirectory(
    id: string,
    segmentId: number,
    create: boolean,
  ): Promise<FileSystemDirectoryHandle> {
    const session = await this.sessionDirectory(id, create);
    const segments = await session.getDirectoryHandle('segments', { create });
    return segments.getDirectoryHandle(String(segmentId).padStart(4, '0'), { create });
  }

  private async readManifestFile(
    session: FileSystemDirectoryHandle,
    name: string,
  ): Promise<LiveSessionManifest | undefined> {
    try {
      const handle = await session.getFileHandle(name);
      const file = await handle.getFile();
      return JSON.parse(await file.text()) as LiveSessionManifest;
    } catch {
      return undefined;
    }
  }

  private async writeManifestFile(
    session: FileSystemDirectoryHandle,
    name: string,
    manifest: LiveSessionManifest,
  ): Promise<void> {
    const handle = await session.getFileHandle(name, { create: true });
    const writable = await handle.createWritable();
    await writable.write(JSON.stringify(manifest));
    await writable.close();
  }

  private async writeManifest(manifest: LiveSessionManifest): Promise<void> {
    const session = await this.sessionDirectory(manifest.id, true);
    const current = await this.readManifestFile(session, MANIFEST);
    if (current) {
      await this.writeManifestFile(session, PREVIOUS_MANIFEST, current);
    }
    await this.writeManifestFile(session, MANIFEST, manifest);
  }

  async createSession(manifest: LiveSessionManifest): Promise<void> {
    await this.sessionDirectory(manifest.id, true);
    for (const segment of manifest.segments) {
      await this.segmentDirectory(manifest.id, segment.id, true);
    }
    await this.writeManifest(cloneManifest(manifest));
  }

  async getSession(id: string): Promise<LiveSessionManifest | undefined> {
    let session: FileSystemDirectoryHandle;
    try {
      session = await this.sessionDirectory(id, false);
    } catch {
      return undefined;
    }
    return (
      (await this.readManifestFile(session, MANIFEST)) ??
      (await this.readManifestFile(session, PREVIOUS_MANIFEST))
    );
  }

  async listRecoverable(): Promise<LiveSessionManifest[]> {
    let schema: FileSystemDirectoryHandle;
    try {
      schema = await this.schemaDirectory(false);
    } catch {
      return [];
    }
    const recovered: LiveSessionManifest[] = [];
    for await (const [id, handle] of (schema as any).entries()) {
      if (handle?.kind !== 'directory') continue;
      const manifest = await this.getSession(id);
      if (!manifest || manifest.state === 'completed') continue;
      if (manifest.state === 'recording') {
        manifest.state = 'interrupted';
        manifest.updatedAtMs = Date.now();
        await this.updateSession(manifest);
      }
      recovered.push(manifest);
    }
    recovered.sort((a, b) => b.updatedAtMs - a.updatedAtMs);
    return recovered;
  }

  async appendChunk(
    sessionId: string,
    segmentId: number,
    blob: Blob,
    checkpointAtMs: number,
  ): Promise<LiveSessionManifest> {
    const current = await this.getSession(sessionId);
    if (!current) throw new Error('LIVE recovery session does not exist.');
    const segment = current.segments.find((entry) => entry.id === segmentId);
    if (!segment) throw new Error(`LIVE segment ${segmentId} does not exist.`);
    const directory = await this.segmentDirectory(sessionId, segmentId, true);
    const nextIndex = segment.chunkCount + 1;
    const handle = await directory.getFileHandle(
      `chunk-${String(nextIndex).padStart(6, '0')}.blob`,
      { create: true },
    );
    const writable = await handle.createWritable();
    await writable.write(blob);
    await writable.close();

    const next = cloneManifest(current);
    const nextSegment = next.segments.find((entry) => entry.id === segmentId)!;
    nextSegment.chunkCount = nextIndex;
    nextSegment.bytes += blob.size;
    nextSegment.lastCheckpointAtMs = checkpointAtMs;
    next.totalBytes += blob.size;
    next.lastCheckpointAtMs = checkpointAtMs;
    next.updatedAtMs = checkpointAtMs;
    await this.writeManifest(next);
    return next;
  }

  async updateSession(manifest: LiveSessionManifest): Promise<void> {
    for (const segment of manifest.segments) {
      await this.segmentDirectory(manifest.id, segment.id, true);
    }
    await this.writeManifest(cloneManifest(manifest));
  }

  async buildSegmentBlob(sessionId: string, segmentId: number): Promise<Blob> {
    const manifest = await this.getSession(sessionId);
    if (!manifest) throw new Error('LIVE recovery session does not exist.');
    const segment = manifest.segments.find((entry) => entry.id === segmentId);
    if (!segment) throw new Error(`LIVE segment ${segmentId} does not exist.`);
    const directory = await this.segmentDirectory(sessionId, segmentId, false);
    const parts: Blob[] = [];
    for (let index = 1; index <= segment.chunkCount; index += 1) {
      const handle = await directory.getFileHandle(
        `chunk-${String(index).padStart(6, '0')}.blob`,
      );
      parts.push(await handle.getFile());
    }
    return new Blob(parts, { type: segment.mimeType });
  }

  async deleteSession(id: string): Promise<void> {
    try {
      const schema = await this.schemaDirectory(false);
      await schema.removeEntry(id, { recursive: true });
    } catch {
      // Already absent is an idempotent delete.
    }
    await this.handleStore.delete(id);
  }

  async saveClientDirectoryHandle(
    id: string,
    handle: FileSystemDirectoryHandle,
  ): Promise<void> {
    await this.handleStore.put(id, handle);
  }

  async loadClientDirectoryHandle(id: string): Promise<FileSystemDirectoryHandle | undefined> {
    return this.handleStore.get(id);
  }
}

export async function requestPersistentStorage(env: any = globalThis): Promise<boolean | undefined> {
  const storage = env?.navigator?.storage;
  if (!storage || typeof storage.persist !== 'function') return undefined;
  return storage.persist();
}

export async function storageEstimate(
  env: any = globalThis,
): Promise<{ usage?: number; quota?: number }> {
  const storage = env?.navigator?.storage;
  if (!storage || typeof storage.estimate !== 'function') return {};
  const estimate = await storage.estimate();
  return { usage: estimate.usage, quota: estimate.quota };
}
