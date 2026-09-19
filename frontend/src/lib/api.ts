import { parseApiResponse } from './api-response.js';
import type {
  BrowseResponse, DashboardStats, Job, Provider, ProviderInput, QuickOptions, Workflow
} from './types';

export class ApiError extends Error {
  constructor(public status: number, message: string) { super(message); }
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
}
function markdownFilename(response: Response): string {
  const disposition = response.headers.get('content-disposition') ?? '';
  const encoded = disposition.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
  if (encoded) {
    try { return decodeURIComponent(encoded); } catch { /* fallback below */ }
  }
  const simple = disposition.match(/filename="([^"]+)"/i)?.[1];
  return simple || 'transcription.md';
}

export const api = {
  dashboard: () => request<DashboardStats>('/api/v1/dashboard'),
  providers: () => request<Provider[]>('/api/v1/providers'),
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
  jobMarkdown: async (id: string) => {
    const response = await fetch(`/api/v1/jobs/${id}/markdown`);
    if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
    return { blob: await response.blob(), filename: markdownFilename(response) };
  },
  browse: (path = '', mode: 'file'|'directory'|'any' = 'any', extensions = '') =>
    request<BrowseResponse>(`/api/v1/browse?path=${encodeURIComponent(path)}&mode=${mode}&extensions=${encodeURIComponent(extensions)}`)
};
