import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const root = new URL('../src/', import.meta.url);
const read = (path) => readFile(new URL(path, root), 'utf8');

test('UI exposes only ScribeWatch product areas', async () => {
  const shell = await read('lib/components/AppShell.svelte');
  for (const label of ['Home', 'Quick Transcribe', 'Workflows', 'Providers', 'Jobs']) assert.ok(shell.includes(label));
  for (const obsolete of ['AutoSubs', 'Presets', 'Brands', 'Editor', 'Queue']) assert.ok(!shell.includes(obsolete));
});

test('client uses the focused ScribeWatch API surface', async () => {
  const api = await read('lib/api.ts');
  for (const endpoint of ['/api/v1/dashboard','/api/v1/providers','/api/v1/workflows','/api/v1/jobs','/api/v1/browse','/api/v1/quick/upload','/api/v1/quick/server','/markdown']) {
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
