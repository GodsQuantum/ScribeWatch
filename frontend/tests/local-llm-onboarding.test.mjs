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

test('host installs are restricted to desktop launcher and explicit selection', () => {
  const launcher = read('packaging/desktop/launcher/src/main.rs');
  const server = read('src/api/local_llm.rs');
  assert.match(launcher, /SCRIBEWATCH_DESKTOP_MODE/);
  assert.match(server, /same_origin\(&headers\)/);
  assert.match(server, /AMD\.LemonadeServer/);
  assert.match(server, /cfg!\(target_os = "windows"\)/);
  assert.match(server, /No user-supplied command/);
  assert.doesNotMatch(server, /sh -c|powershell -Command/);
});

test('recommended models download through the Lemonade native API', () => {
  const server = read('src/api/local_llm.rs');
  assert.match(server, /\/v1\/pull/);
  assert.match(server, /"subscribe": false/);
  assert.match(server, /LiquidAI\/LFM3-1\.7B-GGUF:Q4_K_M/);
  assert.match(server, /unsloth\/Qwen3\.5-4B-GGUF:Q4_K_M/);
});
