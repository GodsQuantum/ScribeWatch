import {test} from 'node:test';
import assert from 'node:assert/strict';
import {defaultQwen4bModelReady} from '../src/lib/native-model-ready.ts';

test('Qwen4B readiness requires a fully downloaded model, not catalog visibility',()=>{
  const hf='unsloth/Qwen3.5-4B-GGUF:Q4_K_M';
  for(const state of ['pending','downloading','incomplete','error',undefined]){
    assert.equal(defaultQwen4bModelReady({
      id:hf,status:state?{value:state}:undefined
    }),false);
  }
  for(const id of [hf,'Qwen3.5-4B-Q4_K_M','Qwen3.5-4B-Q4_K_M.gguf']){
    for(const value of ['loaded','unloaded','sleeping']){
      assert.equal(defaultQwen4bModelReady({id,status:{value}}),true);
    }
  }
  assert.equal(defaultQwen4bModelReady({
    id:'Qwen3.5-9B-Q4_K_M',status:{value:'unloaded'}
  }),false);
});
