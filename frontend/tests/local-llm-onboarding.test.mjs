import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const root = new URL('../', import.meta.url);
const read = (file) => readFileSync(new URL(file, root), 'utf8');

test('AI page uses native llama.cpp and optional Atomic Chat instead of recommending Lemonade', () => {
  const ui = read('src/lib/views/AiProfilesView.svelte');
  assert.match(ui, /scanLocalEngines/);
  assert.match(ui, /useEngine/);
  assert.match(ui, /discoverDraft/);
  assert.match(ui, /Atomic Chat/);
  assert.match(ui, /llama\.cpp/);
  assert.match(ui, /openModelsDirectory/);
  assert.match(ui, /Lister les modèles disponibles/);
  assert.doesNotMatch(ui, />Install Lemonade/);
});

test('discovery and model directory use ScribeWatch desktop API without transmitting notes', () => {
  const client = read('src/lib/api.ts');
  for (const path of [
    '/api/v1/llm-local',
    '/api/v1/llm-local/discover',
    '/api/v1/embedded/llm/models-directory/open',
    '/api/v1/embedded/download',
  ]) {
    assert.ok(client.includes(path), 'Missing API path: '+path);
  }
  const slice=client.slice(client.indexOf('llmLocalEngines:'),client.indexOf('structureProfiles:'));
  assert.ok(!slice.includes('transcript'));
});

test('local GGUF folder opening is deliberate and native-only', () => {
  const ui=read('src/lib/views/AiProfilesView.svelte');
  assert.match(ui,/api\.openLocalLlmModelsDirectory\(\)/);
  assert.match(ui,/Ouvrir le dossier GGUF/);
  assert.match(ui,/--models-dir/);
});

test('Parakeet Ultra is preferred in both recording and quick transcription', () => {
  const live=read('src/lib/views/LiveRecordView.svelte');
  const quick=read('src/lib/views/QuickTranscribeView.svelte');
  assert.match(live,/parakeet\.ultra/i);
  assert.match(quick,/parakeet\.ultra/i);
  assert.match(quick,/markdownDestination/);
  assert.match(live,/markdownDestination/);
});

test('manually dropped GGUF models are visible after explicit refresh', () => {
  const backend = read('../src/api/embedded.rs');
  const ui = read('src/lib/views/AiProfilesView.svelte');
  assert.match(backend, /\/models\?reload=1/);
  assert.match(ui, /sans redémarrer ScribeWatch/);
});
