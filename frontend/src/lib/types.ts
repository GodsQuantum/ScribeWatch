export type JobStatus =
  | 'pending' | 'transcribing' | 'publishing' | 'archiving'
  | 'done' | 'error' | 'cancelled' | 'interrupted';
export type JobKind = 'workflow' | 'quick';
export type QuickSourceKind = 'server' | 'upload';
export type QuickOutputKind = 'server' | 'client';

export interface TranscriptionRoute {
  providerId: string;
  model: string;
  fallbackAfterSeconds?: number;
}

export type TranscriptionAttemptOutcome = 'success' | 'failed' | 'timed_out' | 'cancelled';

export interface TranscriptionAttempt {
  run: number;
  routeIndex: number;
  providerId: string;
  providerName: string;
  model: string;
  startedAtMs: number;
  finishedAtMs: number;
  outcome: TranscriptionAttemptOutcome;
  error?: string;
}

export interface Provider {
  id: string;
  name: string;
  transcriptionUrl: string;
  model: string;
  timeoutSeconds: number;
  enabled: boolean;
  hasApiKey: boolean;
}

export interface ProviderInput {
  id: string;
  name: string;
  transcriptionUrl: string;
  model: string;
  apiKey?: string;
  timeoutSeconds: number;
  enabled: boolean;
}

export interface MarkdownOptions {
  frontmatter: boolean;
  transcriptHeading: string;
}
export interface Workflow {
  id: string;
  name: string;
  watchDir: string;
  outputDir?: string;
  archiveDir: string;
  tags: string[];
  providerId: string;
  model: string;
  transcriptionChain?: TranscriptionRoute[];
  language?: string;
  markdown: MarkdownOptions;
  enabled: boolean;
}

export interface QuickJobMeta {
  sourceKind: QuickSourceKind;
  outputKind: QuickOutputKind;
  outputDir?: string;
  resultName?: string;
  frontmatter: boolean;
}

export interface Job {
  id: string;
  kind: JobKind;
  workflowId?: string;
  quick?: QuickJobMeta;
  providerId: string;
  transcriptionChain: TranscriptionRoute[];
  transcriptionAttempts: TranscriptionAttempt[];
  usedProviderId?: string;
  usedProviderName?: string;
  usedModel?: string;
  originalName: string;
  sourcePath: string;
  sourceSize: number;
  sourceMtimeNs: number;
  model: string;
  language?: string;
  status: JobStatus;
  attempts: number;
  error?: string;
  markdownPath?: string;
  archivePath?: string;
  markdownPublished: boolean;
  createdAtMs: number;
  updatedAtMs: number;
}

export interface QuickOptions {
  providerId?: string;
  model?: string;
  transcriptionChain?: TranscriptionRoute[];
  language?: string;
  outputKind: QuickOutputKind;
  outputDir?: string;
  frontmatter: boolean;
}

export interface DashboardStats {
  workflows: number;
  enabledWorkflows: number;
  activeJobs: number;
  completedJobs: number;
  failedJobs: number;
}

export interface BrowseEntry {
  name: string;
  path: string;
  isDir: boolean;
  size?: number;
  modifiedMs?: number;
  selectable: boolean;
}
export interface BrowseResponse {
  currentPath: string;
  parentPath?: string;
  entries: BrowseEntry[];
  roots: string[];
}
