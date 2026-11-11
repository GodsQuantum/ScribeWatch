import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read = (name) => readFileSync(new URL(name, import.meta.url), 'utf8');

test('existing ASR import goes through native structuring without STT or ChatGPT', () => {
  const view=read('../src/lib/views/QuickTranscribeView.svelte');
  const client=read('../src/lib/api.ts');
  assert.match(view, /api\.structureExisting\(importedProfileId,source\)/);
  assert.match(view, /createMedicalPdf\(importedResult\.markdown/);
  assert.match(client, /\/api\/v1\/structure\/existing/);
  assert.match(view, /Aucun second passage|aucun second passage/);
  assert.doesNotMatch(view.slice(view.indexOf('async function structureImportedTranscript'),view.indexOf('async function exportImportedTranscript')), /localStorage|openai\.com|api\.quickUpload/);
});
