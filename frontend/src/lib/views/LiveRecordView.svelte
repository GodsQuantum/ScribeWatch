<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api';
  import { canSaveToDirectory, saveMarkdownLocally } from '$lib/local-save';
  import { buildLocalProxyInstaller, detectLocalProxyPlatform, inferLocalProxyTarget, localProxyFilename, type LocalProxyPlatform } from '$lib/local-loopback-installer';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { ExportFormat, Job, Provider, QuickOptions, StructureProfile, TranscriptionRoute } from '$lib/types';

  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let jobs:Job[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  let devices:MediaDeviceInfo[]=[];
  let selectedDeviceId='';
  let permissionState:'idle'|'requesting'|'ready'|'denied'='idle';
  let permissionName:'granted'|'prompt'|'denied'|'unknown'='unknown';
  let secureContext=true;
  let mediaError='';
  let permissionStatus:PermissionStatus|undefined;
  let recording=false;
  let processing=false;
  let exporting='';
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
  let outputDir='/media/notes';
  let picker=false;
  let transcriptionChain:TranscriptionRoute[]=[{providerId:'',model:''}];
  let chainInitialized=false;
  let language='';
  let structureProfileId='';
  let frontmatter=true;
  let paragraphs=true;
  let localProxyPanel=false;
  let localProxyPlatform:LocalProxyPlatform='linux';
  let localProxyHost='';
  let localProxyTargetPort=3052;
  let localProxyPort=3052;
  let localProxyNeedsTarget=false;

  const exportFormats:ExportFormat[]=['md','txt','html','docx','odt','pdf'];

  $: if(!chainInitialized&&providers.length){
    const provider=providers.find((candidate)=>candidate.enabled)??providers[0];
    transcriptionChain=[{providerId:provider.id,model:provider.model}];
    chainInitialized=true;
  }
  $: current=submitted ? (jobs.find((job)=>job.id===submitted?.id)??submitted) : undefined;
  $: chainReady=transcriptionChain.length>0&&transcriptionChain.every((route)=>!!route.providerId&&!!route.model);
  $: outputReady=outputMode==='computer'||!!outputDir;
  $: canRecord=chainReady&&outputReady&&!processing;
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
    let lastPaint=0;
    const tick=(now:number)=>{
      if(now-lastPaint>=50){
        analyser.getByteFrequencyData(data);
        levels=Array.from({length:20},(_,index)=>Math.max(.07,(data[index%data.length]??0)/255));
        lastPaint=now;
      }
      animationFrame=requestAnimationFrame(tick);
    };
    animationFrame=requestAnimationFrame(tick);
  }

  async function refreshMicrophones(){
    if(!navigator.mediaDevices?.enumerateDevices){
      mediaError='Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.';
      return;
    }
    const next=(await navigator.mediaDevices.enumerateDevices()).filter((device)=>device.kind==='audioinput');
    devices=next;
    if(!next.some((device)=>device.deviceId===selectedDeviceId)) selectedDeviceId=next[0]?.deviceId??'';
    if(permissionName==='granted') permissionState=next.length?'ready':'denied';
    if(permissionName==='granted'&&!next.length) mediaError='No audio input device is currently visible to the browser.';
  }

  function microphoneError(error:unknown){
    if(!(error instanceof DOMException))return error instanceof Error?error.message:String(error);
    if(error.name==='NotAllowedError')return 'Microphone permission is blocked. Allow it in the browser site permissions, then retry.';
    if(error.name==='NotFoundError')return 'No audio input device is currently visible to the browser.';
    if(error.name==='NotReadableError')return 'The microphone exists but the browser cannot read it. Close other exclusive audio users and retry.';
    if(error.name==='OverconstrainedError')return 'The selected microphone is no longer available. Refresh the device list and retry.';
    return error.message||error.name;
  }

  async function detectMicrophones():Promise<boolean>{
    mediaError='';
    secureContext=window.isSecureContext;
    if(!navigator.mediaDevices?.getUserMedia){
      permissionState='denied';
      mediaError='Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.';
      return false;
    }
    permissionState='requesting';
    try{
      const probe=await navigator.mediaDevices.getUserMedia({audio:true});
      for(const track of probe.getTracks())track.stop();
      permissionName='granted';
      await refreshMicrophones();
      if(devices.length){
        permissionState='ready';
        mediaError='';
        return true;
      }
      permissionState='denied';
      mediaError='No audio input device is currently visible to the browser.';
      return false;
    }catch(error){
      permissionState='denied';
      mediaError=microphoneError(error);
      try{
        const status=await (navigator.permissions as any)?.query?.({name:'microphone'});
        if(status)permissionName=status.state;
      }catch{/* Permissions API is optional */}
      notify('error',mediaError);
      return false;
    }
  }

  function captureConstraints(includeSelected=true):MediaTrackConstraints{
    const supported=navigator.mediaDevices?.getSupportedConstraints?.()??{};
    const constraints:MediaTrackConstraints={};
    if(supported.echoCancellation)constraints.echoCancellation={ideal:true};
    if(supported.noiseSuppression)constraints.noiseSuppression={ideal:true};
    if(supported.autoGainControl)constraints.autoGainControl={ideal:true};
    if(supported.channelCount)constraints.channelCount={ideal:1};
    if(includeSelected&&selectedDeviceId)constraints.deviceId={exact:selectedDeviceId};
    return constraints;
  }

  async function acquireRecordingStream(){
    try{
      return await navigator.mediaDevices.getUserMedia({audio:captureConstraints(true)});
    }catch(error){
      const retryable=error instanceof DOMException&&selectedDeviceId&&['NotFoundError','OverconstrainedError'].includes(error.name);
      if(!retryable)throw error;
      await refreshMicrophones();
      notify('error','The selected microphone changed; retrying with the current default input.');
      return navigator.mediaDevices.getUserMedia({audio:captureConstraints(false)});
    }
  }

  async function startRecording(){
    if(recording||processing)return;
    if(!chainReady){
      notify('error','Choose a transcription provider and model first.');
      return;
    }
    if(!outputReady){
      notify('error','Choose where to save the recording result first.');
      return;
    }
    if(permissionState!=='ready'&&!(await detectMicrophones()))return;
    try{
      stream=await acquireRecordingStream();
      mimeType=chooseMime();
      const options:MediaRecorderOptions={audioBitsPerSecond:64000};
      if(mimeType)options.mimeType=mimeType;
      recorder=new MediaRecorder(stream,options);
      mimeType=recorder.mimeType||mimeType||'audio/webm';
      chunks=[];
      recorder.ondataavailable=(event)=>{if(event.data.size>0)chunks.push(event.data);};
      recorder.onstop=()=>void finishRecording();
      attachMeter(stream);
      elapsed=0;
      timer=setInterval(()=>elapsed+=1,1000);
      recorder.start(5000);
      recording=true;
      submitted=undefined;
      mediaError='';
    }catch(error){
      stopStream();
      mediaError=microphoneError(error);
      notify('error',mediaError);
    }
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
  async function exportResult(format:ExportFormat){
    if(!current||current.status!=='done')return;
    exporting=format;
    try{
      const result=await api.jobExport(current.id,format);
      await saveMarkdownLocally(result.filename,result.blob);
      notify('success',`${format.toUpperCase()} export saved.`);
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{exporting='';}
  }
  function downloadLocalProxyInstaller(){
    try{
      const source=buildLocalProxyInstaller(localProxyPlatform,localProxyHost,Number(localProxyTargetPort),Number(localProxyPort));
      const blob=new Blob([source],{type:'text/plain;charset=utf-8'});
      const url=URL.createObjectURL(blob);
      const anchor=document.createElement('a');
      anchor.href=url;
      anchor.download=localProxyFilename(localProxyPlatform);
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      notify('success','Local microphone installer downloaded. Run it once on this computer.');
    }catch(error){
      notify('error',error instanceof Error?error.message:String(error));
    }
  }
  const duration=(seconds:number)=>`${String(Math.floor(seconds/60)).padStart(2,'0')}:${String(seconds%60).padStart(2,'0')}`;

  onMount(()=>{
    secureContext=window.isSecureContext;
    localProxyPlatform=detectLocalProxyPlatform(navigator as Navigator & {userAgentData?:{platform?:string}});
    const inferredTarget=inferLocalProxyTarget(window.location);
    localProxyHost=inferredTarget.host;
    localProxyTargetPort=inferredTarget.port;
    localProxyNeedsTarget=!inferredTarget.host;
    if(!navigator.mediaDevices?.getUserMedia){
      permissionState='denied';
      mediaError='Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.';
    }else{
      void (async()=>{
        try{
          permissionStatus=await (navigator.permissions as any)?.query?.({name:'microphone'});
          if(permissionStatus){
            permissionName=permissionStatus.state as typeof permissionName;
            permissionStatus.onchange=()=>{
              permissionName=permissionStatus?.state as typeof permissionName;
              if(permissionName==='granted')void refreshMicrophones();
              if(permissionName==='denied'){permissionState='denied';mediaError='Microphone permission is blocked. Allow it in the browser site permissions, then retry.';}
            };
          }
        }catch{/* Some browsers do not expose microphone via Permissions API. */}
        if(permissionName==='granted'){await refreshMicrophones();if(devices.length)permissionState='ready';}
      })();
      navigator.mediaDevices.addEventListener?.('devicechange',refreshMicrophones);
    }
  });
  onDestroy(()=>{
    stopTimer();
    if(permissionStatus)permissionStatus.onchange=null;
    navigator.mediaDevices?.removeEventListener?.('devicechange',refreshMicrophones);
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
  {#if mediaError}<div class="notice warning microphone-warning"><strong>Microphone</strong><span>{mediaError}</span></div>{/if}

  <div class="microphone-diagnostics" aria-label="Microphone diagnostics">
    <span><strong>Secure context</strong> {secureContext?'yes':'no'}</span>
    <span><strong>Permission</strong> {permissionName}</span>
    <span><strong>Devices</strong> {devices.length}</span>
    <span class="spacer"></span>
    {#if permissionState!=='ready'}<button class="btn primary" on:click={detectMicrophones}>Enable microphone</button>{/if}
    <button class="btn" on:click={refreshMicrophones}>Refresh microphones</button>
    <button class="btn" class:primary={!secureContext} on:click={()=>localProxyPanel=!localProxyPanel}>Local microphone mode…</button>
  </div>

  {#if localProxyPanel}
    <section class="card local-proxy-card">
      <div class="card-header"><strong>Install local microphone mode</strong><button class="btn compact" on:click={()=>localProxyPanel=false}>Close</button></div>
      <div class="card-body stack">
        <p class="muted">This does not install another ScribeWatch. It creates a localhost-only proxy on this computer to the existing ScribeWatch HTTP service on your NAS, then you use <code>http://127.0.0.1:{localProxyPort}</code>.</p>
        <div class="segmented local-proxy-platforms" aria-label="Client operating system">
          <button class:active={localProxyPlatform==='linux'} on:click={()=>localProxyPlatform='linux'}>Linux</button>
          <button class:active={localProxyPlatform==='windows'} on:click={()=>localProxyPlatform='windows'}>Windows</button>
          <button class:active={localProxyPlatform==='macos'} on:click={()=>localProxyPlatform='macos'}>macOS</button>
        </div>
        {#if localProxyNeedsTarget}<div class="notice warning compact">You opened ScribeWatch through HTTPS. Enter the NAS local hostname or IPv4 address and its plain HTTP ScribeWatch port below; do not enter the public HTTPS reverse-proxy address.</div>{/if}
        <div class="grid three local-proxy-fields">
          <div class="field"><label for="proxy-host">NAS hostname / IPv4</label><input id="proxy-host" class="input mono" bind:value={localProxyHost} placeholder="192.168.1.217 or nas.local" /></div>
          <div class="field"><label for="proxy-target-port">ScribeWatch HTTP port</label><input id="proxy-target-port" class="input mono" type="number" min="1" max="65535" bind:value={localProxyTargetPort} /></div>
          <div class="field"><label for="proxy-local-port">Local port</label><input id="proxy-local-port" class="input mono" type="number" min="1" max="65535" bind:value={localProxyPort} /></div>
        </div>
        <div class="local-proxy-install-row">
          <button class="btn primary" disabled={!localProxyHost.trim()} on:click={downloadLocalProxyInstaller}><span>Download installer</span> · {localProxyPlatform==='macos'?'macOS':localProxyPlatform==='windows'?'Windows':'Linux'}</button>
          <span class="help">
            {#if localProxyPlatform==='windows'}Run the downloaded .cmd once; Windows will request administrator approval.
            {:else if localProxyPlatform==='macos'}Run once in Terminal with <code>zsh ~/Downloads/{localProxyFilename('macos')}</code>. It installs a per-user LaunchAgent.
            {:else}Run once with <code>bash ~/Downloads/{localProxyFilename('linux')}</code>. It installs a per-user systemd socket.
            {/if}
          </span>
        </div>
        <p class="help">The generated installer is specific to the NAS address above, binds only to 127.0.0.1, persists across logins, opens the local URL after installation, and supports <code>--uninstall</code> to remove itself.</p>
      </div>
    </section>
  {/if}

  <div class="live-grid">
    <section class="card recorder-card">
      <div class="card-body stack">
        <div class="recorder-stage" class:recording>
          <div class="recording-time">{duration(elapsed)}</div>
          <div class="live-wave" aria-hidden="true">
            {#each levels as level}<i style:height={`${Math.max(8,Math.round(level*76))}px`}></i>{/each}
          </div>
          <button class="record-button" class:recording disabled={processing&&!recording} on:click={()=>recording?stopRecording():startRecording()} aria-label={recording?'Stop recording':'Start recording'}>
            <span></span>
          </button>
          <strong>{recording?'Recording — press again to stop':processing?'Uploading recording…':permissionState==='requesting'?'Requesting microphone…':permissionState==='denied'?'Microphone unavailable':permissionState==='idle'?'Enable microphone':'Ready to record'}</strong>
          <span class="muted small">{recording?'Your audio stays in this browser until you stop.':'The recording is uploaded only after Stop, then the temporary audio is deleted after processing.'}</span>
        </div>

        <div class="grid two">
          <div class="field">
            <label for="live-microphone">Microphone</label>
            <select id="live-microphone" class="select" bind:value={selectedDeviceId} disabled={recording||permissionState!=='ready'}>
              {#if devices.length===0}<option value="">No audio input device is currently visible to the browser.</option>{/if}
              {#each devices as device,index}<option value={device.deviceId}>{device.label||`Microphone ${index+1}`}</option>{/each}
            </select>
          </div>
          <div class="field"><label for="live-language">Language</label><input id="live-language" class="input" bind:value={language} placeholder="auto, fr, en…" /></div>
        </div>

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
            {#if current.quick?.outputKind==='server'&&current.markdownPath}<code class="result-path">{current.markdownPath}</code>{/if}
            <div class="result-export-panel">
              <strong>Export result</strong><span class="help">Download / save as</span>
              <div class="export-actions">
                {#each exportFormats as format}
                  <button class="btn compact" class:primary={format==='pdf'} disabled={!!exporting} on:click={()=>exportResult(format)}>{exporting===format?'…':format.toUpperCase()}</button>
                {/each}
              </div>
            </div>
          </div>
        {:else}
          <div class="processing-state"><div class="pulse-ring"></div><strong>{current.status==='transcribing'?'Transcribing…':current.status==='structuring'?'Structuring with AI…':current.status==='publishing'?'Writing Markdown…':'Queued…'}</strong></div>
        {/if}
      </div>
    </aside>
  </div>
</div>

<PathPicker open={picker} title="Choose Markdown folder" mode="directory" initialPath={outputDir} onselect={(path)=>{outputDir=path;picker=false}} onclose={()=>picker=false} />
