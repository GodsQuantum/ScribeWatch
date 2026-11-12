import test from 'node:test';
import assert from 'node:assert/strict';
import { autoSaveFormats } from '../src/lib/live-auto-save.ts';

test('no automatic writes without an explicitly chosen local directory', () => {
  assert.deepEqual(autoSaveFormats('done',false,true,true),[]);
  assert.deepEqual(autoSaveFormats('structuring',true,true,true),[]);
});

test('patient PDF auto-saves only if the AI structure exists and is complete', () => {
  assert.deepEqual(autoSaveFormats('done',true,true,true),['pdf','md']);
  assert.deepEqual(autoSaveFormats('done',true,true,false),['md']);
  assert.deepEqual(autoSaveFormats('error',true,true,true),[]);
});

test('other profile outputs write the Markdown note without a clinical PDF',()=>{
  assert.deepEqual(autoSaveFormats('done',true,false,true),['md']);
});

test('simple recorder persists only authorized destination handles and rechecks write access on REC', async () => {
  const {readFile}=await import('node:fs/promises');
  const live=await readFile(new URL('../src/lib/views/LiveRecordView.svelte',import.meta.url),'utf8');
  assert.ok(live.includes('saveClientDirectoryHandle(simpleDirectoryKey,selected)'));
  assert.ok(live.includes('loadClientDirectoryHandle(simpleDirectoryKey)'));
  assert.ok(live.includes('ensureDirectoryPermission(simpleDirectory)'));
  assert.ok(!live.includes('localStorage.setItem(simpleDirectoryKey'));
});


test('simple recorder retains final audio in the SAME authorized destination as clinical PDF and Markdown', async () => {
  const {readFile}=await import('node:fs/promises');
  const live=await readFile(new URL('../src/lib/views/LiveRecordView.svelte',import.meta.url),'utf8');
  assert.ok(live.includes("if(simpleMode&&simpleDirectory)return {mode:'client',handleKey:'pending'}"));
  assert.ok(live.includes("const finalAudioDirectory=simpleMode?simpleDirectory:clientAudioDirectory"));
  assert.ok(live.includes("saveClientDirectoryHandle(manifest.id,finalAudioDirectory)"));
  assert.ok(live.includes("(!simpleMode||!!simpleDirectory)"));
});


test('simple mode starts on REC without an extra navigation click', async () => {
  const {readFile}=await import('node:fs/promises');
  const page=await readFile(new URL('../src/routes/+page.svelte',import.meta.url),'utf8');
  assert.ok(page.includes("active=uiMode==='simple'?'live'"));
  assert.ok(page.includes("catch{uiMode='simple';active='live';}"));
});
