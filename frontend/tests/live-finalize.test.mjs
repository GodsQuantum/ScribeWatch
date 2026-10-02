import test from 'node:test';
import assert from 'node:assert/strict';
import {
  finalizeLiveSession,
  seekLastSeconds,
} from '../src/lib/live-finalize.ts';
import { buildLiveUploadForm } from '../src/lib/api.ts';

function session(mode='none') {
  return {
    schemaVersion:1,
    id:'550e8400-e29b-41d4-a716-446655440000',
    state:'finalizing',
    createdAtMs:100,
    startedAtMs:100,
    updatedAtMs:300,
    lastCheckpointAtMs:300,
    mimeType:'audio/webm',
    audioBitsPerSecond:64000,
    segments:[
      {id:1,mimeType:'audio/webm',startedAtMs:100,endedAtMs:200,lastCheckpointAtMs:200,chunkCount:1,bytes:3},
      {id:2,mimeType:'audio/webm',startedAtMs:250,endedAtMs:300,lastCheckpointAtMs:300,chunkCount:1,bytes:3,gapMsBefore:50},
    ],
    totalBytes:6,
    options:{outputKind:'client',frontmatter:true,paragraphs:true},
    retention: mode==='server'
      ? {mode:'server',directory:'/media/audio'}
      : mode==='client'
        ? {mode:'client',handleKey:'550e8400-e29b-41d4-a716-446655440000'}
        : {mode:'none'},
  };
}

class Store {
  constructor(value){this.value=structuredClone(value);this.deleted=false;}
  async getSession(){return this.deleted?undefined:structuredClone(this.value);}
  async updateSession(v){this.value=structuredClone(v);}
  async buildSegmentBlob(_id,segmentId){return new Blob([segmentId===1?'one':'two'],{type:'audio/webm'});}
  async deleteSession(){this.deleted=true;}
  async loadClientDirectoryHandle(){return {name:'chosen'};}
}

test('live upload form puts manifest before numbered segments', () => {
  const manifest={
    schemaVersion:1,
    sessionId:'550e8400-e29b-41d4-a716-446655440000',
    startedAtMs:100,
    segments:[
      {id:1,mimeType:'audio/webm',startedAtMs:100,endedAtMs:200},
      {id:2,mimeType:'audio/webm',startedAtMs:250,endedAtMs:300,gapMsBefore:50},
    ],
    retention:{mode:'none'},
    options:{outputKind:'client',frontmatter:true,paragraphs:true},
  };
  const form=buildLiveUploadForm(manifest,[
    {id:1,blob:new Blob(['one'],{type:'audio/webm'})},
    {id:2,blob:new Blob(['two'],{type:'audio/webm'})},
  ]);
  const entries=[...form.entries()];
  assert.deepEqual(entries.map(([name])=>name),['manifest','segment-0001','segment-0002']);
  assert.equal(JSON.parse(entries[0][1]).sessionId,manifest.sessionId);
});

test('failed live upload keeps recovery session', async () => {
  const store=new Store(session('none'));
  await assert.rejects(
    finalizeLiveSession({
      store,
      sessionId:store.value.id,
      upload:async()=>{throw new Error('network down');},
      fetchAudio:async()=>new Response(),
      saveClient:async()=>{},
    }),
    /network down/,
  );
  assert.equal(store.deleted,false);
  assert.equal((await store.getSession()).state,'upload_pending');
});

test('accepted none or server session is deleted after 202', async () => {
  for(const mode of ['none','server']){
    const store=new Store(session(mode));
    const job=await finalizeLiveSession({
      store,
      sessionId:store.value.id,
      upload:async()=>({id:'job-1',quick:{live:{audioResultName:'live.m4a'}}}),
      fetchAudio:async()=>new Response(),
      saveClient:async()=>{},
    });
    assert.equal(job.id,'job-1');
    assert.equal(store.deleted,true);
  }
});

test('client save failure keeps recovery session', async () => {
  const store=new Store(session('client'));
  await assert.rejects(
    finalizeLiveSession({
      store,
      sessionId:store.value.id,
      upload:async()=>({id:'job-1',quick:{live:{audioResultName:'live.m4a'}}}),
      fetchAudio:async()=>new Response(new Blob(['audio'])),
      saveClient:async()=>{throw new Error('permission lost');},
    }),
    /permission lost/,
  );
  assert.equal(store.deleted,false);
  assert.equal(store.value.acceptedJobId,'job-1');
  assert.equal(store.value.state,'accepted');
});

test('seek last 30 seconds clamps to zero', () => {
  const short={duration:12,currentTime:7};
  seekLastSeconds(short,30);
  assert.equal(short.currentTime,0);
  const long={duration:90,currentTime:0};
  seekLastSeconds(long,30);
  assert.equal(long.currentTime,60);
});
