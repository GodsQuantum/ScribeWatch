import { parseApiResponse } from './api-response.js';
import type {
  BrowseResponse, DashboardStats, ExportFormat, Job, LiveUploadManifest, LlmProvider, LlmProviderInput,
  Provider, ProviderInput, QuickOptions, StructureProfile, Workflow
} from './types';

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function errorMessage(response: Response): Promise<string> {
  let message = `HTTP ${response.status}`;
  try {
    const body = await response.json();
    message = body?.error?.message ?? message;
  } catch { /* non-JSON error */ }
  return message;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  if (init?.body && !(init.body instanceof FormData)) headers.set('Content-Type', 'application/json');
  const response = await fetch(path, { ...init, headers });
  if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
  return parseApiResponse(response) as Promise<T>;
}

function appendQuickOptions(form: FormData, options: QuickOptions) {
  if (options.providerId) form.append('providerId', options.providerId);
  if (options.model) form.append('model', options.model);
  if (options.transcriptionChain) form.append('transcriptionChain', JSON.stringify(options.transcriptionChain));
  form.append('language', options.language ?? '');
  form.append('outputKind', options.outputKind);
  form.append('outputDir', options.outputDir ?? '');
  form.append('frontmatter', String(options.frontmatter));
  form.append('paragraphs', String(options.paragraphs));
  form.append('structureProfileId', options.structureProfileId ?? '');
}
export function responseFilename(response: Response, fallback = 'transcription.md'): string {
  const disposition = response.headers.get('content-disposition') ?? '';
  const encoded = disposition.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
  if (encoded) {
    try { return decodeURIComponent(encoded); } catch { /* fallback below */ }
  }
  const simple = disposition.match(/filename="([^"]+)"/i)?.[1];
  return simple || fallback;
}

export function buildLiveUploadForm(
  manifest: LiveUploadManifest,
  segments: Array<{id:number;blob:Blob}>,
): FormData {
  const form = new FormData();
  form.append('manifest', JSON.stringify(manifest));
  for (const segment of segments) {
    const name = `segment-${String(segment.id).padStart(4,'0')}`;
    const ext = segment.blob.type.includes('mp4')
      ? 'm4a'
      : segment.blob.type.includes('ogg')
        ? 'ogg'
        : segment.blob.type.includes('wav')
          ? 'wav'
          : 'webm';
    form.append(name, segment.blob, `${name}.${ext}`);
  }
  return form;
}

export const api = {
  dashboard: () => request<DashboardStats>('/api/v1/dashboard'),
  providers: () => request<Provider[]>('/api/v1/providers'),
  llmProviders: () => request<LlmProvider[]>('/api/v1/llm-providers'),
  saveLlmProvider: (provider: LlmProviderInput) => request<LlmProvider>('/api/v1/llm-providers', {
    method: 'POST', body: JSON.stringify(provider)
  }),
  deleteLlmProvider: (id: string) => request<void>(`/api/v1/llm-providers/${id}`, { method: 'DELETE' }),
  llmModels: (id: string) => request<{models:string[]}>(`/api/v1/llm-providers/${id}/models`),
  structureProfiles: () => request<StructureProfile[]>('/api/v1/structure-profiles'),
  saveStructureProfile: (profile: Omit<StructureProfile,'builtIn'>) => request<StructureProfile>('/api/v1/structure-profiles', {
    method: 'POST', body: JSON.stringify(profile)
  }),
  deleteStructureProfile: (id: string) => request<void>(`/api/v1/structure-profiles/${id}`, { method: 'DELETE' }),
  saveProvider: (provider: ProviderInput) => request<Provider>('/api/v1/providers', {
    method: 'POST', body: JSON.stringify(provider)
  }),
  deleteProvider: (id: string) => request<void>(`/api/v1/providers/${id}`, { method: 'DELETE' }),
  models: (id: string) => request<{models:string[]}>(`/api/v1/providers/${id}/models`),
  workflows: () => request<Workflow[]>('/api/v1/workflows'),
  saveWorkflow: (workflow: Workflow) => request<Workflow>('/api/v1/workflows', {
    method: 'POST', body: JSON.stringify(workflow)
  }),
  deleteWorkflow: (id: string) => request<void>(`/api/v1/workflows/${id}`, { method: 'DELETE' }),
  scanWorkflow: (id: string) => request<void>(`/api/v1/workflows/${id}/scan`, { method: 'PUT' }),
  jobs: () => request<Job[]>('/api/v1/jobs'),
  job: (id: string) => request<Job>(`/api/v1/jobs/${id}`),
  retryJob: (id: string) => request<Job>(`/api/v1/jobs/${id}/retry`, { method: 'POST' }),
  cancelJob: (id: string) => request<Job>(`/api/v1/jobs/${id}/cancel`, { method: 'POST' }),
  deleteJob: (id: string) => request<void>(`/api/v1/jobs/${id}`, { method: 'DELETE' }),
  quickServer: (sourcePath: string, options: QuickOptions) => request<Job>('/api/v1/quick/server', {
    method: 'POST', body: JSON.stringify({ sourcePath, ...options })
  }),
  quickUpload: async (file: File, options: QuickOptions) => {
    const form = new FormData();
    appendQuickOptions(form, options);
    form.append('file', file, file.name);
    const response = await fetch('/api/v1/quick/upload', { method: 'POST', body: form });
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return response.json() as Promise<Job>;
  },
  liveUpload: async (
    manifest: LiveUploadManifest,
    segments: Array<{id:number;blob:Blob}>,
  ) => {
    const response = await fetch('/api/v1/live/upload', {
      method: 'POST',
      body: buildLiveUploadForm(manifest, segments),
    });
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return response.json() as Promise<Job>;
  },
  jobAudio: async (id: string) => {
    const response = await fetch(`/api/v1/jobs/${id}/audio`);
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return response;
  },
  jobExport: async (id: string, format: ExportFormat) => {
    const response = await fetch(`/api/v1/jobs/${id}/export/${format}`);
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return { blob: await response.blob(), filename: responseFilename(response) };
  },
  jobMarkdown: async (id: string) => {
    const response = await fetch(`/api/v1/jobs/${id}/markdown`);
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return { blob: await response.blob(), filename: responseFilename(response) };
  },
  browse: (path = '', mode: 'file'|'directory'|'any' = 'any', extensions = '') =>
    request<BrowseResponse>(`/api/v1/browse?path=${encodeURIComponent(path)}&mode=${mode}&extensions=${encodeURIComponent(extensions)}`)
};
