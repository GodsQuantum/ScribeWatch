import test from 'node:test';
import assert from 'node:assert/strict';
import { chooseRecorderMime, recordedFile } from '../src/lib/dictation.ts';

test('native browser encoder selection chooses supported codec only',()=>{
  assert.equal(chooseRecorderMime(m=>m==='audio/mp4'),'audio/mp4');
  assert.equal(chooseRecorderMime(()=>false),'');
  assert.equal(chooseRecorderMime(()=>true),'audio/webm;codecs=opus');
});
test('dictation builds one bounded audio file without disk persistence',()=>{
  const file=recordedFile([new Blob(['sound'],{type:'audio/webm'})], 'audio/webm', new Date('2026-11-18T08:00:00Z'));
  assert.equal(file.type,'audio/webm');
  assert.equal(file.size,5);
  assert.match(file.name,/dictation-2026-11-18T08-00-00-000Z.webm/);
  assert.throws(()=>recordedFile([], ''),/Aucun son/);
  assert.throws(()=>recordedFile([new Blob([new Uint8Array(12*1024*1024+1)])],''),/maximum 12/);
});
test('dictation UI reuses providers and optional structure instead of a second engine',async()=>{
  const fs=await import('node:fs/promises');
  const body=await fs.readFile(new URL('../src/lib/views/DictationView.svelte',import.meta.url),'utf8');
  assert.match(body,/TranscriptionChainEditor/);
  assert.match(body,/api\.dictationTranscribe\(/);
  assert.match(body,/api\.structureExisting\(/);
  assert.match(body,/releaseMicrophone/);
  assert.match(body,/navigator\.clipboard\.writeText/);
});
