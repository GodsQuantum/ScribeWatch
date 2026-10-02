import {
  OpfsLiveRecoveryStore,
  type LiveRecoveryStore,
  type LiveRetention,
  type LiveSessionManifest,
} from './live-recovery.ts';
import type { QuickOptions } from './types.ts';

export interface LiveStartConfig {
  options: QuickOptions;
  retention: LiveRetention;
  mimeType: string;
  audioBitsPerSecond: 64000;
}

export interface LiveRecorderSnapshot {
  activeSessionId?: string;
  recording: boolean;
  elapsedMs: number;
  protectedBytes: number;
  stream?: MediaStream;
  error?: string;
}

type RecorderLike = {
  state: string;
  mimeType: string;
  ondataavailable: ((event: { data: Blob }) => void) | null;
  onstop: (() => void) | null;
  onerror: ((event: { error?: Error }) => void) | null;
  start(timeslice?: number): void;
  stop(): void;
  requestData(): void;
};

type RecorderCtor = new (
  stream: MediaStream,
  options?: MediaRecorderOptions,
) => RecorderLike;

interface ControllerDependencies {
  store?: LiveRecoveryStore;
  MediaRecorderCtor?: RecorderCtor;
  now?: () => number;
  document?: {
    visibilityState?: string;
    addEventListener?: (name: string, listener: () => void) => void;
    removeEventListener?: (name: string, listener: () => void) => void;
  };
  wakeLock?: {
    request?: (type: 'screen') => Promise<{
      release?: () => Promise<void>;
      addEventListener?: (name: string, listener: () => void) => void;
    }>;
  };
  idFactory?: () => string;
  setInterval?: (handler: () => void, delay: number) => any;
  clearInterval?: (id: any) => void;
}

function defaultRecorderCtor(): RecorderCtor {
  const ctor = (globalThis as any).MediaRecorder;
  if (!ctor) throw new Error('MediaRecorder is unavailable in this browser.');
  return ctor as RecorderCtor;
}

function defaultIdFactory(): string {
  const cryptoObject = (globalThis as any).crypto;
  if (cryptoObject?.randomUUID) return cryptoObject.randomUUID();
  throw new Error('crypto.randomUUID is unavailable in this browser.');
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function stopSuppliedStream(stream: MediaStream): void {
  for (const track of stream.getTracks?.() ?? []) track.stop();
}

export class LiveRecorderController {
  private store: LiveRecoveryStore;
  private recorderCtor?: RecorderCtor;
  private now: () => number;
  private doc?: ControllerDependencies['document'];
  private wakeLock?: ControllerDependencies['wakeLock'];
  private idFactory: () => string;
  private intervalFn: (handler: () => void, delay: number) => any;
  private clearIntervalFn: (id: any) => void;

  private listeners = new Set<(snapshot: LiveRecorderSnapshot) => void>();
  private recorder?: RecorderLike;
  private stream?: MediaStream;
  private manifest?: LiveSessionManifest;
  private segmentId?: number;
  private writeQueue: Promise<void> = Promise.resolve();
  private stopResolver?: () => void;
  private stopping = false;
  private durabilityFailed = false;
  private currentError = '';
  private timer: any;
  private wakeSentinel?: {
    release?: () => Promise<void>;
    addEventListener?: (name: string, listener: () => void) => void;
  };
  private visibilityListener: () => void;
  private trackEndedListener?: () => void;

  constructor(dependencies: ControllerDependencies = {}) {
    this.store = dependencies.store ?? new OpfsLiveRecoveryStore();
    this.recorderCtor = dependencies.MediaRecorderCtor;
    this.now = dependencies.now ?? (() => Date.now());
    this.doc = dependencies.document ?? ((globalThis as any).document as any);
    this.wakeLock =
      dependencies.wakeLock ?? ((globalThis as any).navigator?.wakeLock as any);
    this.idFactory = dependencies.idFactory ?? defaultIdFactory;
    this.intervalFn =
      dependencies.setInterval ??
      ((handler, delay) => globalThis.setInterval(handler, delay));
    this.clearIntervalFn =
      dependencies.clearInterval ??
      ((id) => globalThis.clearInterval(id));
    this.visibilityListener = () => {
      if (this.doc?.visibilityState === 'hidden') {
        this.requestCheckpoint();
      } else if (this.isRecording()) {
        void this.acquireWakeLock();
      }
    };
    this.doc?.addEventListener?.('visibilitychange', this.visibilityListener);
  }

  getSnapshot(): LiveRecorderSnapshot {
    return {
      activeSessionId: this.manifest?.id,
      recording: this.isRecording(),
      elapsedMs: this.elapsedMs(),
      protectedBytes: this.manifest?.totalBytes ?? 0,
      stream: this.stream,
      error: this.currentError || undefined,
    };
  }

  subscribe(listener: (snapshot: LiveRecorderSnapshot) => void): () => void {
    this.listeners.add(listener);
    listener(this.getSnapshot());
    return () => {
      this.listeners.delete(listener);
    };
  }

  private emit(): void {
    const snapshot = this.getSnapshot();
    for (const listener of this.listeners) listener(snapshot);
  }

  private isRecording(): boolean {
    return !!this.recorder && this.recorder.state === 'recording';
  }

  private elapsedMs(): number {
    const manifest = this.manifest;
    if (!manifest) return 0;
    let total = 0;
    for (const segment of manifest.segments) {
      const end =
        segment.endedAtMs ??
        (segment.id === this.segmentId && this.isRecording()
          ? this.now()
          : segment.lastCheckpointAtMs ?? segment.startedAtMs);
      total += Math.max(0, end - segment.startedAtMs);
    }
    return total;
  }

  private startTimer(): void {
    this.stopTimer();
    this.timer = this.intervalFn(() => this.emit(), 250);
  }

  private stopTimer(): void {
    if (this.timer !== undefined) {
      this.clearIntervalFn(this.timer);
      this.timer = undefined;
    }
  }

  private async acquireWakeLock(): Promise<void> {
    if (!this.wakeLock?.request || this.doc?.visibilityState === 'hidden') return;
    if (this.wakeSentinel) return;
    try {
      const sentinel = await this.wakeLock.request('screen');
      this.wakeSentinel = sentinel;
      sentinel.addEventListener?.('release', () => {
        if (this.wakeSentinel === sentinel) this.wakeSentinel = undefined;
      });
    } catch {
      // Wake Lock is advisory; durable storage is the safety mechanism.
    }
  }

  private async releaseWakeLock(): Promise<void> {
    const sentinel = this.wakeSentinel;
    this.wakeSentinel = undefined;
    try {
      await sentinel?.release?.();
    } catch {
      // Releasing an already-released sentinel is harmless.
    }
  }

  private stopTracks(): void {
    if (this.trackEndedListener && this.stream) {
      for (const track of this.stream.getTracks()) {
        track.removeEventListener?.('ended', this.trackEndedListener);
      }
    }
    for (const track of this.stream?.getTracks?.() ?? []) track.stop();
    this.trackEndedListener = undefined;
    this.stream = undefined;
  }

  private makeRecorder(stream: MediaStream, config: LiveStartConfig): RecorderLike {
    const Ctor = this.recorderCtor ?? defaultRecorderCtor();
    const options: MediaRecorderOptions = {
      audioBitsPerSecond: config.audioBitsPerSecond,
    };
    if (config.mimeType) options.mimeType = config.mimeType;
    return new Ctor(stream, options);
  }

  private async beginRecorder(
    manifest: LiveSessionManifest,
    segmentId: number,
    stream: MediaStream,
    config: LiveStartConfig,
  ): Promise<LiveSessionManifest> {
    if (this.isRecording() || this.stopping) {
      throw new Error('A LIVE recording is already active.');
    }
    const recorder = this.makeRecorder(stream, config);
    this.manifest = manifest;
    this.segmentId = segmentId;
    this.stream = stream;
    this.recorder = recorder;
    this.currentError = '';
    this.durabilityFailed = false;
    this.writeQueue = Promise.resolve();

    this.trackEndedListener = () => {
      void this.interrupt('Microphone input ended.');
    };
    for (const track of stream.getTracks()) {
      track.addEventListener?.('ended', this.trackEndedListener);
    }

    recorder.ondataavailable = (event) => {
      if (!event.data || event.data.size <= 0 || this.durabilityFailed) return;
      const checkpointAtMs = this.now();
      const sessionId = manifest.id;
      const activeSegmentId = segmentId;
      this.writeQueue = this.writeQueue.then(async () => {
        if (this.durabilityFailed) return;
        try {
          this.manifest = await this.store.appendChunk(
            sessionId,
            activeSegmentId,
            event.data,
            checkpointAtMs,
          );
          this.emit();
        } catch (error) {
          this.durabilityFailed = true;
          this.currentError =
            error instanceof Error ? error.message : String(error);
          queueMicrotask(() => {
            void this.interrupt(this.currentError, false);
          });
        }
      });
    };
    recorder.onstop = () => {
      const resolve = this.stopResolver;
      this.stopResolver = undefined;
      resolve?.();
    };
    recorder.onerror = (event) => {
      const message = event.error?.message || 'MediaRecorder failed.';
      void this.interrupt(message);
    };

    recorder.start(5000);
    this.startTimer();
    void this.acquireWakeLock();
    this.emit();
    return clone(manifest);
  }

  async startSession(
    config: LiveStartConfig,
    stream: MediaStream,
  ): Promise<LiveSessionManifest> {
    if (this.isRecording() || this.stopping) {
      throw new Error('A LIVE recording is already active.');
    }
    const now = this.now();
    const id = this.idFactory();
    const manifest: LiveSessionManifest = {
      schemaVersion: 1,
      id,
      state: 'recording',
      createdAtMs: now,
      startedAtMs: now,
      updatedAtMs: now,
      mimeType: config.mimeType,
      audioBitsPerSecond: 64000,
      segments: [
        {
          id: 1,
          mimeType: config.mimeType,
          startedAtMs: now,
          chunkCount: 0,
          bytes: 0,
        },
      ],
      totalBytes: 0,
      options: clone(config.options),
      retention: clone(config.retention),
    };
    let created = false;
    try {
      await this.store.createSession(manifest);
      created = true;
      return await this.beginRecorder(manifest, 1, stream, config);
    } catch (error) {
      if (created) {
        manifest.state = 'interrupted';
        manifest.updatedAtMs = this.now();
        try {
          await this.store.updateSession(manifest);
        } catch {
          // Preserve the original start failure; recovery storage reports its own failure on retry.
        }
      }
      if (this.stream === stream) this.stopTracks();
      else stopSuppliedStream(stream);
      this.recorder = undefined;
      this.segmentId = undefined;
      this.manifest = created ? manifest : undefined;
      this.currentError = error instanceof Error ? error.message : String(error);
      this.emit();
      throw error;
    }
  }

  async continueSession(
    sessionId: string,
    stream: MediaStream,
  ): Promise<LiveSessionManifest> {
    if (this.isRecording() || this.stopping) {
      throw new Error('A LIVE recording is already active.');
    }
    const manifest = await this.store.getSession(sessionId);
    if (!manifest) throw new Error('LIVE recovery session does not exist.');
    if (manifest.state !== 'interrupted') {
      throw new Error('Only an interrupted LIVE session can be continued.');
    }
    const now = this.now();
    const previous = manifest.segments.at(-1);
    const lastDurable =
      previous?.lastCheckpointAtMs ??
      previous?.endedAtMs ??
      manifest.lastCheckpointAtMs ??
      now;
    if (previous && previous.endedAtMs === undefined) {
      previous.endedAtMs = lastDurable;
    }
    const segmentId = manifest.segments.length + 1;
    manifest.segments.push({
      id: segmentId,
      mimeType: manifest.mimeType,
      startedAtMs: now,
      chunkCount: 0,
      bytes: 0,
      gapMsBefore: Math.max(0, now - lastDurable),
    });
    manifest.state = 'recording';
    manifest.updatedAtMs = now;
    await this.store.updateSession(manifest);
    const config: LiveStartConfig = {
      options: clone(manifest.options),
      retention: clone(manifest.retention),
      mimeType: manifest.mimeType,
      audioBitsPerSecond: 64000,
    };
    try {
      return await this.beginRecorder(manifest, segmentId, stream, config);
    } catch (error) {
      const pending = manifest.segments.at(-1);
      if (pending?.id === segmentId && pending.chunkCount === 0) {
        manifest.segments.pop();
      }
      manifest.state = 'interrupted';
      manifest.updatedAtMs = this.now();
      await this.store.updateSession(manifest);
      if (this.stream === stream) this.stopTracks();
      else stopSuppliedStream(stream);
      this.recorder = undefined;
      this.segmentId = undefined;
      this.manifest = manifest;
      this.currentError = error instanceof Error ? error.message : String(error);
      this.emit();
      throw error;
    }
  }

  requestCheckpoint(): void {
    if (!this.isRecording()) return;
    try {
      this.recorder?.requestData();
    } catch {
      // A concurrent stop can make requestData invalid; stop owns finalization.
    }
  }

  private async waitForRecorderStop(requestFinalData: boolean): Promise<void> {
    const recorder = this.recorder;
    if (!recorder || recorder.state === 'inactive') return;
    if (requestFinalData) this.requestCheckpoint();
    const stopped = new Promise<void>((resolve) => {
      this.stopResolver = resolve;
    });
    recorder.stop();
    await stopped;
  }

  private async settle(
    state: 'finalizing' | 'interrupted',
    reason?: string,
    requestFinalData = true,
  ): Promise<LiveSessionManifest> {
    const manifest = this.manifest;
    if (!manifest) throw new Error('No LIVE recording is active.');
    if (this.stopping) {
      await this.writeQueue;
      const current = await this.store.getSession(manifest.id);
      return current ?? manifest;
    }
    this.stopping = true;
    this.stopTimer();
    try {
      await this.waitForRecorderStop(requestFinalData);
      await this.writeQueue;
      const latest = (await this.store.getSession(manifest.id)) ?? manifest;
      const segment = latest.segments.find((entry) => entry.id === this.segmentId);
      if (segment && segment.endedAtMs === undefined) {
        segment.endedAtMs =
          segment.lastCheckpointAtMs ??
          latest.lastCheckpointAtMs ??
          this.now();
      }
      latest.state = state;
      latest.updatedAtMs = this.now();
      this.manifest = latest;
      if (reason) this.currentError = reason;
      await this.store.updateSession(latest);
      this.recorder = undefined;
      this.segmentId = undefined;
      this.stopTracks();
      await this.releaseWakeLock();
      this.emit();
      return clone(latest);
    } finally {
      this.stopping = false;
    }
  }

  async stop(): Promise<LiveSessionManifest> {
    if (!this.manifest || !this.isRecording()) {
      throw new Error('No LIVE recording is active.');
    }
    return this.settle('finalizing');
  }

  async interrupt(reason: string, requestFinalData = true): Promise<void> {
    if (!this.manifest) return;
    if (!this.isRecording() && !this.stopping) {
      const manifest = (await this.store.getSession(this.manifest.id)) ?? this.manifest;
      manifest.state = 'interrupted';
      manifest.updatedAtMs = this.now();
      this.currentError = reason;
      this.manifest = manifest;
      await this.store.updateSession(manifest);
      this.emit();
      return;
    }
    await this.settle('interrupted', reason, requestFinalData);
  }
}

let singleton: LiveRecorderController | undefined;

export function getLiveRecorderController(): LiveRecorderController {
  if (!singleton) singleton = new LiveRecorderController();
  return singleton;
}
