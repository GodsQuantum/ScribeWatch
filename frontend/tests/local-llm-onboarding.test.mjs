import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const root = new URL('../../', import.meta.url);
const read = (file) => readFileSync(new URL(file, root), 'utf8');

test('provider wizard exposes local discovery and explicit opt-in install', () => {
  const ui = read('frontend/src/lib/views/AiProfilesView.svelte');
  assert.match(ui, /scanLocalEngines/);
  assert.match(ui, /useEngine/);
  assert.match(ui, /discoverDraft/);
  assert.match(ui, /confirm\('Install AMD Lemonade Server/);
  assert.match(ui, /installRecommendedModel/);
  assert.match(ui, /LFM3-1\.7B Q4_K_M/);
  assert.match(ui, /Qwen3\.5-4B Q4_K_M/);
  assert.match(ui, /isWindows/);
});

test('onboard discovery uses the ScribeWatch API and does not send transcripts', () => {
  const client = read('frontend/src/lib/api.ts');
  for (const path of [
    '/api/v1/llm-local',
    '/api/v1/llm-local/discover',
    '/api/v1/llm-local/pull',
    '/api/v1/llm-local/downloads',
    '/api/v1/llm-local/install-engine',
  ]) {
    assert.ok(client.includes(path), 'Missing API path: ' + path);
  }
  assert.ok(!client.slice(client.indexOf('llmLocalEngines:'), client.indexOf('structureProfiles:')).includes('transcript'));
});

test('Windows installer requires explicit UI confirmation and desktop gate', () => {
  const ui = read('frontend/src/lib/views/AiProfilesView.svelte');
  assert.match(ui, /desktopAvailable/);
  assert.match(ui, /isWindows/);
  assert.match(ui, /confirm\('Install AMD Lemonade Server/);
  assert.match(ui, /api\.llmInstallEngine\(\)/);
  assert.match(ui, /Install Lemonade \+ LFM3/);
});

test('recommended models are selected explicitly via the local provider API', () => {
  const ui = read('frontend/src/lib/views/AiProfilesView.svelte');
  const client = read('frontend/src/lib/api.ts');
  assert.match(ui, /confirm\('Download '/);
  assert.match(ui, /api\.llmInstallModel\(installModel\)/);
  assert.match(ui, /lfm3-1\.7b/);
  assert.match(ui, /qwen3\.5-4b/);
  assert.match(client, /llmInstallModel:.*\/api\/v1\/llm-local\/pull/);
});
