import test from 'node:test';
import assert from 'node:assert/strict';
import { canSaveToDirectory, saveMarkdownLocally } from '../src/lib/local-save.ts';

test('directory save requires secure context and picker support', () => {
  assert.equal(canSaveToDirectory({ isSecureContext: true, showDirectoryPicker(){} }), true);
  assert.equal(canSaveToDirectory({ isSecureContext: false, showDirectoryPicker(){} }), false);
  assert.equal(canSaveToDirectory({ isSecureContext: true }), false);
});

test('download fallback preserves requested filename', async () => {
  let clicked = false;
  const anchor = { href:'', download:'', click(){ clicked = true; }, remove(){} };
  const env = {
    isSecureContext: false,
    URL: { createObjectURL(){ return 'blob:test'; }, revokeObjectURL(){} },
    document: { createElement(){ return anchor; }, body: { appendChild(){} } }
  };
  const result = await saveMarkdownLocally('Ma note.md', new Blob(['hello']), env);
  assert.equal(result, 'download');
  assert.equal(anchor.download, 'Ma note.md');
  assert.equal(clicked, true);
});
