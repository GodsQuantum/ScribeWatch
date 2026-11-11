import test from 'node:test';
import assert from 'node:assert/strict';
import { canSaveToDirectory, saveMarkdownLocally, saveResponseToDirectory } from '../src/lib/local-save.ts';

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


test('unsupported directory picker uses download fallback for audio response', async () => {
  let clicked=false;
  const anchor={href:'',download:'',click(){clicked=true;},remove(){}};
  const env={
    isSecureContext:false,
    URL:{createObjectURL(){return 'blob:audio';},revokeObjectURL(){}},
    document:{createElement(){return anchor;},body:{appendChild(){}}},
  };
  const response=new Response(new Blob(['audio'],{type:'audio/mp4'}));
  const result=await saveResponseToDirectory(response,'consultation.m4a',undefined,env);
  assert.equal(result,'download');
  assert.equal(anchor.download,'consultation.m4a');
  assert.equal(clicked,true);
});

test('automatic note export can select a non-clobbering file name', async () => {
  const {uniqueDirectoryFilename}=await import('../src/lib/local-save.ts');
  const existing=new Set(['consultation.md','consultation_2.md']);
  const handle={
    async getFileHandle(name,{create}={}){
      if(create)throw new Error('lookups must be read only');
      if(existing.has(name))return {name};
      throw new DOMException('not found','NotFoundError');
    }
  };
  assert.equal(await uniqueDirectoryFilename(handle,'consultation.md'),'consultation_3.md');
  await assert.rejects(uniqueDirectoryFilename({
    async getFileHandle(){throw new DOMException('not permitted','NotAllowedError');}
  },'consultation.md'),/not permitted/);
});
