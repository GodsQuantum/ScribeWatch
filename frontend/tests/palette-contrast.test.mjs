import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const css=readFileSync(new URL('../src/app.css',import.meta.url),'utf8');
const palettes=['night','arctic','slate','sand','violet'];
function declarations(name){
  const start=name==='night' ? css.indexOf(':root {') : css.indexOf('html[data-sw-theme="'+name+'"]{');
  assert.notEqual(start,-1,'Missing palette '+name);
  const body=css.slice(start,css.indexOf('}',start));
  return Object.fromEntries([...body.matchAll(/(--[\w-]+)\s*:\s*(#[0-9a-f]{6}|#[0-9a-f]{3})/gi)]
    .map((match)=>[match[1],match[2]]));
}
function lum(hex){
  if(hex.length===4)hex='#'+hex.slice(1).split('').map(char=>char+char).join('');
  const colors=[1,3,5].map(index=>parseInt(hex.slice(index,index+2),16)/255)
    .map(value=>value<=.04045 ? value/12.92 : ((value+.055)/1.055)**2.4);
  return .2126*colors[0]+.7152*colors[1]+.0722*colors[2];
}
function contrast(a,b){
  const values=[lum(a),lum(b)].sort((x,y)=>y-x);
  return (values[0]+.05)/(values[1]+.05);
}
for(const name of palettes){
  test('WCAG AA contrast: '+name+' semantic text and interactive accent',()=>{
    const tokens=declarations(name);
    const accentText=name==='night'?'#062019':tokens['--on-accent'];
    for(const [label,left,right] of [
      ['text/panel',tokens['--text'],tokens['--panel']],
      ['muted/panel',tokens['--muted'],tokens['--panel']],
      ['text/bg',tokens['--text'],tokens['--bg']],
      ['button/accent',accentText,tokens['--accent']],
    ]){
      assert.ok(left&&right,'Missing '+name+' '+label);
      assert.ok(contrast(left,right)>=4.5,
        name+' '+label+' contrast '+contrast(left,right).toFixed(2)+' below WCAG AA 4.5');
    }
  });
}
