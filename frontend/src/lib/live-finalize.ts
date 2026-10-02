import type {
  Job,
  LiveAudioRetention,
  LiveUploadManifest,
} from './types.ts';
import type {
  LiveRecoveryStore,
  LiveSessionManifest,
} from './live-recovery.ts';

export interface FinalizeLiveDependencies {
  store: Pick<
    LiveRecoveryStore,
    'getSession' | 'updateSession' | 'buildSegmentBlob' | 'deleteSession' | 'loadClientDirectoryHandle'
  >;
  sessionId: string;
  upload: (
    manifest: LiveUploadManifest,
    segments: Array<{ id: number; blob: Blob }>,
  ) => Promise<Job>;
  fetchAudio: (jobId: string) => Promise<Response>;
  saveClient: (
    response: Response,
    filename: string,
    handle?: FileSystemDirectoryHandle,
  ) => Promise<unknown>;
}

export function serverRetention(manifest: LiveSessionManifest): LiveAudioRetention {
  switch (manifest.retention.mode) {
    case 'server':
      return { mode: 'server', directory: manifest.retention.directory };
    case 'client':
      return { mode: 'client' };
    default:
      return { mode: 'none' };
  }
}

export function toLiveUploadManifest(manifest: LiveSessionManifest): LiveUploadManifest {
  return {
    schemaVersion: 1,
    sessionId: manifest.id,
    startedAtMs: manifest.startedAtMs,
    segments: manifest.segments.map((segment) => ({
      id: segment.id,
      mimeType: segment.mimeType,
      startedAtMs: segment.startedAtMs,
      endedAtMs: segment.endedAtMs,
      gapMsBefore: segment.gapMsBefore,
    })),
    retention: serverRetention(manifest),
    options: manifest.options,
  };
}

export async function finalizeLiveSession(
  dependencies: FinalizeLiveDependencies,
): Promise<Job> {
  const { store, sessionId, upload, fetchAudio, saveClient } = dependencies;
  let manifest = await store.getSession(sessionId);
  if (!manifest) throw new Error('LIVE recovery session does not exist.');
  if (manifest.segments.length === 0 || manifest.segments.some((segment) => segment.chunkCount < 1)) {
    throw new Error('LIVE recovery session has no complete audio segment to upload.');
  }

  manifest.state = 'upload_pending';
  manifest.updatedAtMs = Date.now();
  await store.updateSession(manifest);

  const segments: Array<{ id: number; blob: Blob }> = [];
  for (const segment of manifest.segments) {
    segments.push({
      id: segment.id,
      blob: await store.buildSegmentBlob(sessionId, segment.id),
    });
  }

  let job: Job;
  try {
    manifest.state = 'uploading';
    manifest.updatedAtMs = Date.now();
    await store.updateSession(manifest);
    job = await upload(toLiveUploadManifest(manifest), segments);
  } catch (error) {
    manifest.state = 'upload_pending';
    manifest.updatedAtMs = Date.now();
    await store.updateSession(manifest);
    throw error;
  }

  manifest.state = 'accepted';
  manifest.acceptedJobId = job.id;
  manifest.updatedAtMs = Date.now();
  await store.updateSession(manifest);

  if (manifest.retention.mode === 'client') {
    const response = await fetchAudio(job.id);
    const filename =
      job.quick?.live?.audioResultName ??
      `live-${manifest.startedAtMs}.m4a`;
    const handle = await store.loadClientDirectoryHandle(sessionId);
    await saveClient(response, filename, handle);
  }

  await store.deleteSession(sessionId);
  return job;
}

export function seekLastSeconds(
  audio: Pick<HTMLAudioElement, 'duration' | 'currentTime'> | { duration: number; currentTime: number },
  seconds = 30,
): void {
  const duration = Number.isFinite(audio.duration) ? audio.duration : 0;
  audio.currentTime = Math.max(0, duration - Math.max(0, seconds));
}
