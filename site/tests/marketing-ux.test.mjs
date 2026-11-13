import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root=path.resolve(import.meta.dirname,'..');
const site=(locale)=>fs.readFileSync(path.join(root,'public',locale,'index.html'),'utf8');

test('public-facing site starts with everyday language and an honest local-privacy qualification',()=>{
 for(const l of ['fr','en','zh-CN']){
  const html=site(l);
  assert.match(html,/<h1>[^<]{8,}/);
  assert.match(html,/id="privacy"/);
  assert.match(html,/id="requirements"/);
  assert.match(html,/id="downloads"/);
  assert.match(html,/<details class="advanced">/);
  assert.match(html,/id="image-dialog"/);
  assert.equal((html.match(/data-zoom="/g)||[]).length,4);
  assert.match(html,/data-download="windows-full"/);
  assert.ok(!/Parakeet Ultra|Qwen3\.5|MTP\d+|GGUF/.test(html),'Jargon belongs in advanced documentation, not public marketing');
  assert.ok(!/radial-gradient|linear-gradient|gradient-text|<marquee/i.test(html));
  assert.ok(!html.includes('google-analytics') && !html.includes('gtag('),'No telemetry injected into static pages');
  assert.match(html,/unslop/);
 }
});

test('product-led dark hero, three local-first trust points and natural translation survive build',()=>{
 const css=fs.readFileSync(path.join(root,'src/style.css'),'utf8');
 assert.match(css,/\.hero\{[^}]*background:#142c2c/);
 assert.match(css,/\.site-header\{[^}]*background:#142c2c/);
 assert.match(css,/\.section-install\{[^}]*background:#1b3833/);
 assert.match(css,/\.proofrail-inner\{[^}]*grid-template-columns:repeat\(3/);
 for(const lang of ['fr','en','zh-CN']){
  const html=site(lang);
  assert.match(html,/class="proofrail"/);
  assert.equal((html.match(/class="proofrail-item"/g)||[]).length,3);
  assert.ok(html.indexOf('class="proofrail"')>html.indexOf('class="hero"'));
  assert.ok(html.indexOf('class="proofrail"')<html.indexOf('id="privacy"'));
  assert.equal((html.match(/data-zoom="/g)||[]).length,4);
  assert.ok(html.includes('assets/screens/'+lang+'-home.png'));
 }
 assert.match(site('fr'),/Traitement sur votre ordinateur|Transcription sur votre ordinateur/);
 assert.match(site('en'),/On-device transcription/);
 assert.match(site('zh-CN'),/电脑本地转写/);
});

test('local model privacy is not generalized to advanced remote providers',()=>{
 assert.match(site('fr'),/modèles locaux/);
 assert.match(site('fr'),/service externe/);
 assert.match(site('en'),/external provider/);
 assert.match(site('zh-CN'),/外部服务/);
});

test('requirements separate transcription and report drafting and light editions stay in advanced details',()=>{
 const html=site('en');
 assert.match(html,/To transcribe/);
 assert.match(html,/To write structured reports/);
 assert.match(html,/16 GB RAM/);
 const advanced=html.indexOf('<details class="advanced">');
 const light=html.indexOf('data-download="windows-light"');
 assert.ok(light>advanced);
 assert.ok(html.indexOf('data-download="windows-full"')<advanced);
});

test('all large captures are served from local ScribeWatch assets and have accessible controls',()=>{
 for(const l of ['fr','en','zh-CN']){
  const html=site(l);
  for(const name of ['home','record','documents','settings']){
   assert.match(html,new RegExp('src="/assets/screens/'+l+'-'+name+'\\.png"'));
  }
  assert.match(html,/data-close/);
  assert.match(html,/data-prev/);
  assert.match(html,/data-next/);
 }
});
