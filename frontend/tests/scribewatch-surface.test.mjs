import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const root = new URL('../src/', import.meta.url);
const read = (path) => readFile(new URL(path, root), 'utf8');

test('UI exposes only ScribeWatch product areas', async () => {
  const shell = await read('lib/components/AppShell.svelte');
  for (const label of ['Home', 'Quick', 'Live', 'Workflows', 'STT', 'AI Profiles', 'Jobs']) assert.ok(shell.includes(label));
  for (const obsolete of ['AutoSubs', 'Presets', 'Brands', 'Editor', 'Queue']) assert.ok(!shell.includes(obsolete));
});

test('client uses the focused ScribeWatch API surface', async () => {
  const api = await read('lib/api.ts');
  for (const endpoint of ['/api/v1/dashboard','/api/v1/providers','/api/v1/llm-providers','/api/v1/structure-profiles','/api/v1/workflows','/api/v1/jobs','/api/v1/browse','/api/v1/quick/upload','/api/v1/quick/server','/export/']) {
    assert.ok(api.includes(endpoint), `missing endpoint ${endpoint}`);
  }
});

test('named job SSE events refresh job state', async () => {
  const page = await read('routes/+page.svelte');
  assert.ok(page.includes("new EventSource('/api/v1/events')"));
  assert.ok(page.includes("addEventListener('job'"));
});

test('provider form never binds an existing API secret from server data', async () => {
  const view = await read('lib/views/ProvidersView.svelte');
  assert.ok(view.includes("apiKey:''"));
  assert.ok(view.includes('hasApiKey'));
  assert.ok(!view.includes('provider.apiKey'));
});

test('workflow UI separates Watch, Markdown and Archive folders', async () => {
  const view = await read('lib/views/WorkflowsView.svelte');
  for (const label of ['Watch folder','Markdown folder','Audio archive']) assert.ok(view.includes(label));
});

test('quick and workflow expose deterministic paragraph formatting', async () => {
  const quick = await read('lib/views/QuickTranscribeView.svelte');
  const workflows = await read('lib/views/WorkflowsView.svelte');
  const api = await read('lib/api.ts');
  const types = await read('lib/types.ts');
  for (const view of [quick, workflows]) {
    assert.ok(view.includes('Readable deterministic paragraphs'));
  }
  assert.ok(api.includes("form.append('paragraphs'"));
  assert.ok(types.includes('paragraphs: boolean'));
});

test('product promise is visible on Home', async () => {
  const view = await read('lib/views/DashboardView.svelte');
  assert.ok(view.includes('Turn voice notes into Markdown before you forget them.'));
});


test('quick transport exposes transcription chains', async () => {
  const api = await read('lib/api.ts');
  const types = await read('lib/types.ts');
  assert.ok(api.includes("form.append('transcriptionChain'"));
  assert.ok(types.includes('interface TranscriptionRoute'));
  assert.ok(types.includes('transcriptionChain'));
  assert.ok(types.includes('transcriptionAttempts'));
  assert.ok(types.includes('usedProviderId'));
});


test('workflow and quick share the fallback chain editor with live model discovery', async () => {
  const editor = await read('lib/components/TranscriptionChainEditor.svelte');
  const workflows = await read('lib/views/WorkflowsView.svelte');
  const quick = await read('lib/views/QuickTranscribeView.svelte');
  const providers = await read('lib/views/ProvidersView.svelte');
  assert.ok(editor.includes('api.models'));
  assert.ok(editor.includes('+ Add fallback'));
  assert.ok(editor.includes('Primary'));
  assert.ok(editor.includes('Fallback'));
  assert.ok(workflows.includes('TranscriptionChainEditor'));
  assert.ok(quick.includes('TranscriptionChainEditor'));
  assert.ok(!workflows.includes('Model override'));
  assert.ok(!quick.includes('Model override'));
  assert.ok(providers.includes('Model discovery timeout'));
});

test('live recorder negotiates browser media and reuses the quick pipeline', async () => {
  const live = await read('lib/views/LiveRecordView.svelte');
  for (const token of ['getUserMedia','MediaRecorder.isTypeSupported','enumerateDevices','TranscriptionChainEditor','api.quickUpload','structureProfileId']) {
    assert.ok(live.includes(token), 'missing live recorder behavior ' + token);
  }
  assert.ok(live.includes('activeRecorder.onstop=null'), 'navigation must not upload an unfinished recording');
});

test('AI structure stays optional and preserves canonical transcript semantics', async () => {
  const quick = await read('lib/views/QuickTranscribeView.svelte');
  const workflows = await read('lib/views/WorkflowsView.svelte');
  const ai = await read('lib/views/AiProfilesView.svelte');
  for (const view of [quick, workflows]) assert.ok(view.includes('structureProfile'));
  assert.ok(ai.includes('canonical transcript is always preserved'));
});

test('jobs expose all supported document exports', async () => {
  const jobs = await read('lib/views/JobsView.svelte');
  for (const format of ['md','txt','html','docx','odt','pdf']) assert.ok(jobs.includes(format));
  assert.ok(jobs.includes('api.jobExport'));
});
