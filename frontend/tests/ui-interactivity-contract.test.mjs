import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync, readdirSync} from 'node:fs';
import {join, relative} from 'node:path';

const root = new URL('../src/', import.meta.url);
const read = (name)=>readFileSync(new URL(name,root),'utf8');

function collectSvelte(dir){
  const all=[];
  for(const item of readdirSync(dir,{withFileTypes:true})){
    const path=join(dir,item.name);
    if(item.isDirectory()) all.push(...collectSvelte(path));
    else if(item.isFile()&&path.endsWith('.svelte')) all.push(path);
  }
  return all;
}

test('all ScribeWatch buttons trigger a click action or form submit',()=>{
  let count=0;
  const rootPath=new URL('.',root).pathname;
  for(const file of collectSvelte(rootPath)){
    const source=readFileSync(file,'utf8');
    for(const match of source.matchAll(/<button\b[^>]*>/gs)){
      count++;
      const tag=match[0];
      assert.ok(
        /\bon:click\s*=/.test(tag)||/\btype\s*=\s*["']submit["']/.test(tag),
        'Unbound button in '+relative(rootPath,file)+': '+tag.slice(0,150)
      );
    }
  }
  assert.ok(count>140,'The UI button inventory unexpectedly shrank');
});

test('REC uses responsive layouts, keyboard focus and reduced motion',()=>{
  const css=read('app.css');
  assert.match(css, /@media\s*\(max-width:\s*680px\)/);
  assert.match(css, /@media\s*\(max-width:\s*980px\)/);
  assert.match(css, /focus-visible/);
  assert.match(css, /prefers-reduced-motion/);
  assert.match(css, /overflow-wrap/);
});

test('native first run blocks REC until both models are ready',()=>{
  const recorder=read('lib/views/LiveRecordView.svelte');
  assert.match(recorder, /!simpleMode\|\|nativeSetup==='not-managed'\|\|nativeSetup==='ready'/);
  assert.match(recorder, /api\.embeddedModels\('stt'\)/);
  assert.match(recorder, /api\.embeddedModels\('llm'\)/);
  assert.match(recorder, /scribewatch-native-parakeet/);
  assert.match(recorder, /clearTimeout\(nativeSetupTimer\)/);
});
