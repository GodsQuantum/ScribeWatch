<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api';
  import { canSaveToDirectory, saveMarkdownLocally } from '$lib/local-save';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { Job, Provider, QuickOptions, StructureProfile, TranscriptionRoute } from '$lib/types';

  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let jobs:Job[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  let devices:MediaDeviceInfo[]=[];
  let selectedDeviceId='';
  let permissionState:'idle'|'requesting'|'ready'|'denied'='idle';
  let recording=false;
  let processing=false;
  let elapsed=0;
  let timer:ReturnType<typeof setInterval>|undefined;
  let recorder:MediaRecorder|undefined;
  let stream:MediaStream|undefined;
  let audioContext:AudioContext|undefined;
  let animationFrame=0;
  let levels=Array(20).fill(0.08) as number[];
  let chunks:Blob[]=[];
  let mimeType='';
  let submitted:Job|undefined;

  let outputMode:'computer'|'server'='server';
  let outputDir='';
  let picker=false;
  let transcriptionChain:TranscriptionRoute[]=[{providerId:'',model:''}];
  let chainInitialized=false;
  let language='';
  let structureProfileId='';
  let frontmatter=true;
  let paragraphs=true;

  $: if(!chainInitialized&&providers.length){
    transcriptionChain=[{providerId:providers.find((provider)=>provider.enabled)?.id??providers[0].id,model:''}];
    chainInitialized=true;
  }
  $: current=submitted ? (jobs.find((job)=>job.id===submitted?.id)??submitted) : undefined;
  $: chainReady=transcriptionChain.length>0&&transcriptionChain.every((route)=>!!route.providerId&&!!route.model);
  $: outputReady=outputMode==='computer'||!!outputDir;
  $: canRecord=permissionState==='ready'&&chainReady&&outputReady&&!processing;
  $: localDirectoryAvailable=typeof window!=='undefined'&&canSaveToDirectory(window);

  const mimeCandidates=[
    'audio/webm;codecs=opus',
    'audio/ogg;codecs=opus',
    'audio/mp4',
    'audio/webm',
    'audio/ogg'
  ];

  function chooseMime(){
    if(typeof MediaRecorder==='undefined')return '';
    return mimeCandidates.find((candidate)=>MediaRecorder.isTypeSupported(candidate))??'';
  }
  function extensionForMime(value:string){
    if(value.includes('mp4'))return 'm4a';
    if(value.includes('ogg'))return 'ogg';
    if(value.includes('webm'))return 'webm';
    return 'audio';
  }
  function stopStream(){
    if(stream){for(const track of stream.getTracks())track.stop();}
    stream=undefined;
    if(audioContext){void audioContext.close();}
    audioContext=undefined;
    if(animationFrame)cancelAnimationFrame(animationFrame);
    animationFrame=0;
    levels=Array(20).fill(0.08);
  }
  function stopTimer(){if(timer)clearInterval(timer);timer=undefined;}
  function attachMeter(active:MediaStream){
    const Ctx=window.AudioContext??(window as typeof window & {webkitAudioContext?:typeof AudioContext}).webkitAudioContext;
    if(!Ctx)return;
    audioContext=new Ctx();
    const analyser=audioContext.createAnalyser();
    analyser.fftSize=64;
    analyser.smoothingTimeConstant=.72;
    audioContext.createMediaStreamSource(active).connect(analyser);
    const data=new Uint8Array(analyser.frequencyBinCount);
    const tick=()=>{
      analyser.getByteFrequencyData(data);
      levels=Array.from({length:20},(_,index)=>Math.max(.07,(data[index%data.length]??0)/255));
      animationFrame=requestAnimationFrame(tick);
    };
    tick();
  }
  async function detectMicrophones(){
    if(!navigator.mediaDevices?.getUserMedia){
      permissionState='denied';
      notify('error','Microphone capture is unavailable in this browser/context.');
      return;
    }
    permissionState='requesting';
    try{
      const probe=await navigator.mediaDevices.getUserMedia({audio:true});
      for(const track of probe.getTracks())track.stop();
      devices=(await navigator.mediaDevices.enumerateDevices()).filter((device)=>device.kind==='audioinput');
      selectedDeviceId=selectedDeviceId||devices[0]?.deviceId||'';
      permissionState='ready';
    }catch(e){
      permissionState='denied';
      notify('error',e instanceof Error?e.message:'Microphone permission denied.');
    }
  }
  async function startRecording(){
    if(!canRecord||recording)return;
    const constraints:MediaTrackConstraints={
      echoCancellation:{ideal:true},
      noiseSuppression:{ideal:true},
      autoGainControl:{ideal:true}
    };
    if(selectedDeviceId)constraints.deviceId={exact:selectedDeviceId};
    try{
      stream=await navigator.mediaDevices.getUserMedia({audio:constraints});
      mimeType=chooseMime();
      recorder=mimeType?new MediaRecorder(stream,{mimeType}):new MediaRecorder(stream);
      mimeType=recorder.mimeType||mimeType||'audio/webm';
      chunks=[];
      recorder.ondataavailable=(event)=>{if(event.data.size>0)chunks.push(event.data);};
      recorder.onstop=()=>void finishRecording();
      attachMeter(stream);
      elapsed=0;
      timer=setInterval(()=>elapsed+=1,1000);
      recorder.start(1000);
      recording=true;
      submitted=undefined;
    }catch(e){stopStream();notify('error',e instanceof Error?e.message:String(e));}
  }
  function stopRecording(){
    if(!recording||!recorder)return;
    recording=false;
    stopTimer();
    recorder.stop();
  }
  async function finishRecording(){
    stopStream();
    if(chunks.length===0){notify('error','The browser returned an empty recording.');return;}
    processing=true;
    const ext=extensionForMime(mimeType);
    const stamp=new Date().toISOString().replace(/[:.]/g,'-');
    const file=new File(chunks,`live-${stamp}.${ext}`,{type:mimeType});
    const primary=transcriptionChain[0];
    const options:QuickOptions={
      providerId:primary?.providerId,
      model:primary?.model,
      transcriptionChain,
      language:language.trim()||undefined,
      outputKind:outputMode==='server'?'server':'client',
      outputDir:outputMode==='server'?outputDir:undefined,
      frontmatter,
      paragraphs,
      structureProfileId:structureProfileId||undefined
    };
    try{
      submitted=await api.quickUpload(file,options);
      await refresh();
      notify('success','Recording queued for transcription.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{processing=false;chunks=[];}
  }
  async function saveResult(){
    if(!current||current.status!=='done')return;
    try{
      const result=await api.jobExport(current.id,'md');
      const method=await saveMarkdownLocally(result.filename,result.blob);
      notify('success',method==='directory'?'Markdown saved to your folder.':'Markdown downloaded.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
  const duration=(seconds:number)=>`${String(Math.floor(seconds/60)).padStart(2,'0')}:${String(seconds%60).padStart(2,'0')}`;

  onMount(()=>{void detectMicrophones();});
  onDestroy(()=>{
    stopTimer();
    const activeRecorder=recorder;
    if(activeRecorder&&activeRecorder.state!=='inactive'){
      activeRecorder.onstop=null;
      activeRecorder.stop();
    }
    stopStream();
  });
</script>

<div class="page live-page">
  <div class="page-head">
    <div>
      <p class="page-kicker">BROWSER MICROPHONE</p>
      <h1 class="page-title">Live Recorder</h1>
      <p class="page-copy">Record from a browser microphone, stop when you are done, then run the exact same resilient transcription and optional AI-structure pipeline as a watched file.</p>
    </div>
  </div>

  {#if providers.length===0}<div class="notice warning">Add a transcription provider first.</div>{/if}

  <div class="live-grid">
    <section class="card recorder-card">
      <div class="card-body stack">
        <div class="recorder-stage" class:recording>
          <div class="recording-time">{duration(elapsed)}</div>
          <div class="live-wave" aria-hidden="true">
            {#each levels as level}<i style:height={`${Math.max(8,Math.round(level*76))}px`}></i>{/each}
          </div>
          <button class="record-button" class:recording disabled={!canRecord&& !recording} on:click={()=>recording?stopRecording():startRecording()} aria-label={recording?'Stop recording':'Start recording'}>
            <span></span>
          </button>
          <strong>{recording?'Recording — press again to stop':processing?'Uploading recording…':permissionState==='requesting'?'Requesting microphone…':permissionState==='denied'?'Microphone unavailable':'Ready to record'}</strong>
          <span class="muted small">{recording?'Your audio stays in this browser until you stop.':'The recording is uploaded only after Stop, then the temporary audio is deleted after processing.'}</span>
        </div>

        <div class="grid two">
          <div class="field">
            <label for="live-microphone">Microphone</label>
            <select id="live-microphone" class="select" bind:value={selectedDeviceId} disabled={recording||permissionState!=='ready'}>
              {#each devices as device}<option value={device.deviceId}>{device.label||'Microphone'}</option>{/each}
            </select>
          </div>
          <div class="field"><label for="live-language">Language</label><input id="live-language" class="input" bind:value={language} placeholder="auto, fr, en…" /></div>
        </div>

        {#if permissionState==='denied'}<button class="btn" on:click={detectMicrophones}>Request microphone again</button>{/if}

        <TranscriptionChainEditor value={transcriptionChain} {providers} {notify} onchange={(routes)=>transcriptionChain=routes} />

        <div class="field">
          <label for="live-structure">AI structure <span class="muted">optional</span></label>
          <select id="live-structure" class="select" bind:value={structureProfileId}>
            <option value="">None — deterministic transcript only</option>
            {#each structureProfiles.filter((profile)=>!!profile.providerId) as profile}<option value={profile.id}>{profile.name}</option>{/each}
          </select>
          <span class="help">The profile creates an additional structured view. The complete transcript remains in the same Markdown file.</span>
        </div>

        <div class="subsection stack">
          <strong>Destination</strong>
          <div class="segmented"><button class:active={outputMode==='server'} on:click={()=>outputMode='server'}>Server folder</button><button class:active={outputMode==='computer'} on:click={()=>outputMode='computer'}>This computer</button></div>
          {#if outputMode==='server'}
            <div class="field"><label for="live-output">Markdown destination</label><div class="path-control"><input id="live-output" class="input mono" value={outputDir} readonly placeholder="Choose a folder" /><button class="btn" on:click={()=>picker=true}>Browse</button></div></div>
          {:else}
            <div class="client-output-note"><span>⌁</span><div><strong>{localDirectoryAvailable?'Save into a local folder after transcription':'Download the result after transcription'}</strong><p>Browser security requires the final local save to remain an explicit action.</p></div></div>
          {/if}
          <div class="row wrap"><label class="check"><input type="checkbox" bind:checked={frontmatter}/> YAML properties</label><label class="check"><input type="checkbox" bind:checked={paragraphs}/> Readable deterministic paragraphs</label></div>
        </div>
      </div>
    </section>

    <aside class="card result-card">
      <div class="card-header"><strong>Recording result</strong>{#if current}<span class="status-pill {current.status}">{current.status}</span>{/if}</div>
      <div class="card-body result-body">
        {#if !current}
          <div class="empty result-empty"><span>REC</span><strong>Press the record button.</strong><p>Stop once. ScribeWatch uploads the completed browser recording, normalizes it with FFmpeg, transcribes it, and optionally structures it.</p></div>
        {:else if current.status==='error'||current.status==='interrupted'||current.status==='cancelled'}
          <div class="error-box"><strong>Processing did not finish</strong><span>{current.error??current.status}</span></div>
        {:else if current.status==='done'}
          <div class="result-success"><span class="result-check">✓</span><h2>{current.quick?.resultName??'Note ready'}</h2>
            {#if current.structuringError}<div class="notice warning compact">Transcript succeeded; AI structuring failed: {current.structuringError}</div>{/if}
            {#if current.quick?.outputKind==='client'}<button class="btn primary" on:click={saveResult}>{localDirectoryAvailable?'Save Markdown…':'Download Markdown'}</button>
            {:else if current.markdownPath}<code class="result-path">{current.markdownPath}</code>{/if}
          </div>
        {:else}
          <div class="processing-state"><div class="pulse-ring"></div><strong>{current.status==='transcribing'?'Transcribing…':current.status==='structuring'?'Structuring with AI…':current.status==='publishing'?'Writing Markdown…':'Queued…'}</strong></div>
        {/if}
      </div>
    </aside>
  </div>
</div>

<PathPicker open={picker} title="Choose Markdown folder" mode="directory" initialPath={outputDir} onselect={(path)=>{outputDir=path;picker=false}} onclose={()=>picker=false} />
