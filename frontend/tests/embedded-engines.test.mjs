import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const root = new URL('../', import.meta.url);
const read = (file) => readFileSync(new URL(file, root), 'utf8');

test('the same ScribeWatch UI exposes separately selectable STT and LLM engines', () => {
  const page = read('src/lib/views/AiProfilesView.svelte');
  assert.match(page, /Moteurs intégrés/);
  assert.match(page, /engine.kind==='stt'/);
  assert.match(page, /engine.kind==='llm'/);
  for (const mode of ['internal', 'external', 'disabled']) {
    assert.ok(page.includes('value="' + mode + '"'), 'Expected explicit mode ' + mode);
  }
  assert.match(page, /setEmbeddedMode/);
  assert.match(page, /Télécharger le modèle ASR/);
  assert.match(page, /Télécharger le modèle LLM/);
  assert.match(page, /Installer pack complet ScribeWatch/);
  assert.match(page, /Rechercher les quantifications GGUF/);
});

test('external providers remain part of the UI after choosing embedded engine', () => {
  const page = read('src/lib/views/AiProfilesView.svelte');
  assert.match(page, /saveLlmProvider/);
  assert.match(page, /saveProvider/);
  assert.match(page, /connectEmbeddedStt/);
  assert.match(page, /selectEmbeddedLlm/);
  assert.match(page, /Fournisseur de transcription interne créé/);
  assert.match(page, /fournisseurs externes déjà configurés ne sont jamais effacés/);
});

test('native engine operations are backend-owned and require explicit clicks', () => {
  const client = read('src/lib/api.ts');
  for (const route of [
    '/api/v1/embedded/status',
    '/api/v1/embedded/mode',
    '/api/v1/embedded/download',
    '/api/v1/embedded/models/',
    '/api/v1/embedded/stt/variants',
    '/api/v1/embedded/stt/install',
  ]) {
    assert.ok(client.includes(route), 'Required embedded API missing ' + route);
  }
  const page = read('src/lib/views/AiProfilesView.svelte');
  assert.match(page, /confirm\('Démarrer le moteur '/);
  assert.match(page, /confirm\('Télécharger le modèle '/);
  assert.match(page, /embeddedBusy/);
});

test('model weights are downloaded after user action, never at startup', () => {
  const page = read('src/lib/views/AiProfilesView.svelte');
  const mounted = page.slice(page.indexOf('onMount('), page.indexOf('async function scanLocalEngines('));
  assert.doesNotMatch(mounted, /embeddedDownload|embeddedAsrInstall|llmInstallModel/);
  assert.match(page, /selectedLLM='empero-ai\/Qwen3\.8-4B-Distill-GGUF:Q4_K_M'/);
});
