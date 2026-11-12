import test from 'node:test';
import assert from 'node:assert/strict';
import worker, {selectAsset, ASSETS} from '../src/worker.mjs';
const base='https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/';
const names={'windows-full':'ScribeWatch-0.6.2-windows-amd64-standalone-setup.exe','windows-light':'ScribeWatch-0.6.2-windows-amd64-setup.exe','mac-arm64':'ScribeWatch-0.6.2-darwin-arm64.dmg','mac-amd64':'ScribeWatch-0.6.2-darwin-amd64.dmg','linux-amd64':'ScribeWatch-0.6.2-x86_64.AppImage','linux-arm64':'ScribeWatch-0.6.2-aarch64.AppImage'};
const mock={tag_name:'v0.6.2',draft:false,prerelease:false,assets:Object.values(names).map((name)=>({name,size:12345,browser_download_url:base+name}))};
test('All six exact release assets match the correct platform',()=>{
  assert.equal(Object.keys(ASSETS).length,6);
  for(const [key,name] of Object.entries(names))assert.equal(selectAsset(mock,key)?.name,name);
});
test('Never match SHA sidecars, untrusted domain or arbitrary target',()=>{
  assert.equal(selectAsset({assets:[{name:names['windows-full']+'.sha256',browser_download_url:base+names['windows-full']+'.sha256'}]},'windows-full'),null);
  assert.equal(selectAsset({assets:[{name:names['windows-full'],browser_download_url:'https://evil.example/installer.exe'}]},'windows-full'),null);
  assert.equal(selectAsset(mock,'../../api'),null);
});
test('Redirect locale respects French, Chinese and English',async()=>{
  for(const [value,want] of [['fr-FR,fr;q=0.8','fr'],['zh-CN,zh;q=0.9','zh-CN'],['de-DE,en;q=0.8','en']]){
    const r=await worker.fetch(new Request('https://scribewatch.arezkichougar.com/',{headers:{'accept-language':value}}),{ASSETS:{fetch:()=>new Response('ok')}});
    assert.equal(r.status,302);
    assert.equal(new URL(r.headers.get('location')).pathname,'/'+want+'/');
  }
});
test('Download redirect chooses official GitHub URL',async(t)=>{
  const original=globalThis.fetch;globalThis.fetch=async()=>Response.json(mock);
  t.after(()=>{globalThis.fetch=original;});
  const r=await worker.fetch(new Request('https://scribewatch.arezkichougar.com/download/windows-full'),{ASSETS:{fetch:()=>new Response('ok')}});
  assert.equal(r.status,302);assert.equal(r.headers.get('location'),base+names['windows-full']);
  const api=await worker.fetch(new Request('https://scribewatch.arezkichougar.com/api/release'),{ASSETS:{fetch:()=>new Response('ok')}});
  assert.equal(api.status,200);assert.equal((await api.json()).assets['mac-arm64'].name,names['mac-arm64']);
});
test('Unsupported download platform fails closed',async()=>{
  const r=await worker.fetch(new Request('https://scribewatch.arezkichougar.com/download/windows-arm64'),{ASSETS:{fetch:()=>new Response('ok')}});
  assert.equal(r.status,404);
});
