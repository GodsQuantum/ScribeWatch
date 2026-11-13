import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import path from 'node:path';

const root=path.resolve(import.meta.dirname,'..');
const github='https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/';

test('browser detects Windows and falls back to actual GitHub release when Worker API is missing',async()=>{
  const assets=[
    {name:'ScribeWatch-0.6.2-windows-amd64-standalone-setup.exe',size:12000,browser_download_url:github+'ScribeWatch-0.6.2-windows-amd64-standalone-setup.exe'},
    {name:'ScribeWatch-0.6.2-x86_64.AppImage',size:14000,browser_download_url:github+'ScribeWatch-0.6.2-x86_64.AppImage'}
  ];
  const elements={
    'smart-download':{href:'#downloads',textContent:''},
    'platform-note':{textContent:''},
    'release-tag':{textContent:''}
  };
  const requests=[];
  const ctx={
    document:{
      documentElement:{lang:'fr'},
      getElementById:(k)=>elements[k]??null,
      querySelector:(selector)=>{
        if(selector==='[data-download="windows-full"]')return {href:'',closest:()=>({hidden:false})};
        return null;
      }
    },
    navigator:{userAgent:'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',maxTouchPoints:0},
    fetch:async(url)=>{
      requests.push(url);
      if(url==='/api/release')return {ok:false};
      if(url==='/assets/release-map.json')return {ok:true,json:async()=>({'windows-full':'-windows-amd64-standalone-setup\.exe$','linux-amd64':'-x86_64\.AppImage$'})};
      if(url==='https://api.github.com/repos/GodsQuantum/ScribeWatch/releases/latest'){
        return {ok:true,json:async()=>({tag_name:'v0.6.2',draft:false,prerelease:false,assets})};
      }
      throw Error('Unexpected request '+url);
    }
  };
  vm.runInNewContext(fs.readFileSync(path.join(root,'src/client.js'),'utf8'),ctx);
  await new Promise(resolve=>setTimeout(resolve,50));
  assert.deepEqual(requests,['/api/release','/assets/release-map.json','https://api.github.com/repos/GodsQuantum/ScribeWatch/releases/latest']);
  assert.equal(elements['smart-download'].href,assets[0].browser_download_url);
  assert.equal(elements['release-tag'].textContent,'v0.6.2');
});

test('static HTML links remain safe when scripts or release API are unavailable',()=>{
  for(const locale of ['en','fr','zh-CN']){
    const html=fs.readFileSync(path.join(root,'public',locale,'index.html'),'utf8');
    const hrefs=[...html.matchAll(/data-download="[^"]+"[^>]*href="([^"]+)"/g)].map(x=>x[1]);
    assert.equal(hrefs.length,12);
    assert.ok(hrefs.every(x=>x==='https://github.com/GodsQuantum/ScribeWatch/releases/latest'));
  }
});

test('static homepage routes French and Chinese browsers to localized pages',()=>{
  const html=fs.readFileSync(path.join(root,'public/index.html'),'utf8');
  assert.match(html,/navigator\.language/);
  assert.match(html,/\/fr\//);
  assert.match(html,/\/zh-CN\//);
  assert.match(html,/\/en\//);
});
