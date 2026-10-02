<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api';
  import { canSaveToDirectory, saveMarkdownLocally, saveResponseToDirectory } from '$lib/local-save';
  import { buildLocalProxyInstaller, detectLocalProxyPlatform, inferLocalProxyTarget, localProxyFilename, type LocalProxyPlatform } from '$lib/local-loopback-installer';
  import { finalizeLiveSession, seekLastSeconds } from '$lib/live-finalize';
  import { getLiveRecorderController, type LiveRecorderSnapshot } from '$lib/live-recorder';
  import {
    OpfsLiveRecoveryStore,
    requestPersistentStorage,
    storageEstimate,
    type LiveRetention,
    type LiveSessionManifest
  } from '$lib/live-recovery';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { ExportFormat, Job, Provider, QuickOptions, StructureProfile, TranscriptionRoute } from '$lib/types';

  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let jobs:Job[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  const controller=getLiveRecorderController();
  const recoveryStore=new OpfsLiveRecoveryStore();

  let devices:MediaDeviceInfo[]=[];
  let selectedDeviceId='';
  let permissionState:'idle'|'requesting'|'ready'|'denied'='idle';
  let permissionName:'granted'|'prompt'|'denied'|'unknown'='unknown';
  let secureContext=true;
  let mediaError='';
  let permissionStatus:PermissionStatus|undefined;

  let snapshot:LiveRecorderSnapshot=controller.getSnapshot();
  let recording=snapshot.recording;
  let elapsed=Math.floor(snapshot.elapsedMs/1000);
  let protectedBytes=snapshot.protectedBytes;
  let unsubscribeController:(()=>void)|undefined;

  let processing=false;
  let exporting='';
  let submitted:Job|undefined;

  let audioContext:AudioContext|undefined;
  let meterStream:MediaStream|undefined;
  let animationFrame=0;
  let levels=Array(20).fill(0.08) as number[];

  let outputMode:'computer'|'server'='server';
  let outputDir='/media/notes';
  let picker=false;
  let transcriptionChain:TranscriptionRoute[]=[{providerId:'',model:''}];
  let chainInitialized=false;
  let language='';
  let structureProfileId='';
  let frontmatter=true;
  let paragraphs=true;

  let audioRetentionMode:'none'|'server'|'client'='none';
  let audioServerDir='';
  let audioPicker=false;
  let clientAudioDirectory:FileSystemDirectoryHandle|undefined;
  let clientAudioFolderName='';

  let recoveries:LiveSessionManifest[]=[];
  let selectedRecoveryId='';
  let selectedRecoverySegment=0;
  let recoveryAudioUrl='';
  let recoveryAudio:HTMLAudioElement|undefined;
  let recoveryBusy='';
  let recoveryError='';

  let persistenceState:'checking'|'persistent'|'best-effort'|'unsupported'='checking';
  let storageUsage:number|undefined;
  let storageQuota:number|undefined;

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
  $: audioRetentionReady=audioRetentionMode!=='server'||!!audioServerDir;
  $: canRecord=chainReady&&outputReady&&audioRetentionReady&&!processing;
  $: localDirectoryAvailable=typeof window!=='undefined'&&canSaveToDirectory(window);
  $: selectedRecovery=recoveries.find((session)=>session.id===selectedRecoveryId);
  $: selectedRecoveryPlayable=selectedRecovery?.segments.filter((segment)=>segment.chunkCount>0)??[];

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

  function stopMeter(){
    if(audioContext){void audioContext.close();}
    audioContext=undefined;
    meterStream=undefined;
    if(animationFrame)cancelAnimationFrame(animationFrame);
    animationFrame=0;
    levels=Array(20).fill(0.08);
  }

  function attachMeter(active:MediaStream){
    if(meterStream===active)return;
    stopMeter();
    meterStream=active;
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

  function applySnapshot(next:LiveRecorderSnapshot){
    snapshot=next;
    recording=next.recording;
    elapsed=Math.floor(next.elapsedMs/1000);
    protectedBytes=next.protectedBytes;
    if(next.stream)attachMeter(next.stream);else stopMeter();
    if(next.error)mediaError=next.error;
  }

  async function refreshMicrophones(){
    if(!navigator.mediaDevices?.enumerateDevices){
      mediaError='Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.';
      return;
    }
    const next=(await navigator.mediaDevices.enumerateDevices()).filter((device)=>device.kind==='audioinput');
    devices=next;
    if(!next.some((device)=>device.deviceId===selectedDeviceId))selectedDeviceId=next[0]?.deviceId??'';
    if(permissionName==='granted')permissionState=next.length?'ready':'denied';
    if(permissionName==='granted'&&!next.length)mediaError='No audio input device is currently visible to the browser.';
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
      if(devices.length){permissionState='ready';mediaError='';return true;}
      permissionState='denied';
      mediaError='No audio input device is currently visible to the browser.';
      return false;
    }catch(error){
      permissionState='denied';
      mediaError=microphoneError(error);
      try{
        const status=await (navigator.permissions as any)?.query?.({name:'microphone'});
        if(status)permissionName=status.state;
      }catch{/* optional */}
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

  function quickOptions():QuickOptions{
    const primary=transcriptionChain[0];
    return {
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
  }

  function requestedRetention():LiveRetention{
    if(audioRetentionMode==='server')return {mode:'server',directory:audioServerDir};
    if(audioRetentionMode==='client')return {mode:'client',handleKey:'pending'};
    return {mode:'none'};
  }

  async function chooseClientAudioDirectory(){
    if(!localDirectoryAvailable)return undefined;
    const handle=await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle;
    clientAudioDirectory=handle;
    clientAudioFolderName=handle.name;
    return handle;
  }

  async function startRecording(){
    if(recording||processing)return;
    if(!chainReady){notify('error','Choose a transcription provider and model first.');return;}
    if(!outputReady){notify('error','Choose where to save the recording result first.');return;}
    if(!audioRetentionReady){notify('error','Choose a server folder for the final audio or another retention mode.');return;}
    if(permissionState!=='ready'&&!(await detectMicrophones()))return;
    try{
      if(audioRetentionMode==='client'&&localDirectoryAvailable&&!clientAudioDirectory){
        await chooseClientAudioDirectory();
      }
      const stream=await acquireRecordingStream();
      const mimeType=chooseMime()||'audio/webm';
      const manifest=await controller.startSession({
        options:quickOptions(),
        retention:requestedRetention(),
        mimeType,
        audioBitsPerSecond:64000
      },stream);
      if(audioRetentionMode==='client'&&clientAudioDirectory){
        await recoveryStore.saveClientDirectoryHandle(manifest.id,clientAudioDirectory);
      }
      submitted=undefined;
      recoveries=[];
      mediaError='';
      notify('success','Recording started with durable local recovery.');
    }catch(error){
      const message=microphoneError(error);
      mediaError=message;
      notify('error',message);
    }
  }

  async function finalizeSession(sessionId:string, successMessage='Recording queued for transcription.'){
    processing=true;
    recoveryBusy=sessionId;
    try{
      submitted=await finalizeLiveSession({
        store:recoveryStore,
        sessionId,
        upload:api.liveUpload,
        fetchAudio:api.jobAudio,
        saveClient:(response,filename,handle)=>saveResponseToDirectory(response,filename,handle,window)
      });
      await refresh();
      notify('success',successMessage);
    }catch(error){
      notify('error',error instanceof Error?error.message:String(error));
    }finally{
      processing=false;
      recoveryBusy='';
      await refreshRecoveries();
    }
  }

  async function stopRecording(){
    if(!recording||processing)return;
    processing=true;
    try{
      const manifest=await controller.stop();
      processing=false;
      await finalizeSession(manifest.id);
    }catch(error){
      processing=false;
      notify('error',error instanceof Error?error.message:String(error));
      await refreshRecoveries();
    }
  }

  async function refreshRecoveries(){
    recoveryError='';
    if(controller.getSnapshot().recording){recoveries=[];return;}
    try{
      recoveries=await recoveryStore.listRecoverable();
      if(!recoveries.some((item)=>item.id===selectedRecoveryId)){
        selectedRecoveryId=recoveries[0]?.id??'';
      }
      if(selectedRecoveryId)await prepareRecoveryPlayback(selectedRecoveryId);
      else clearRecoveryAudio();
    }catch(error){
      recoveryError=error instanceof Error?error.message:String(error);
      recoveries=[];
    }
  }

  function clearRecoveryAudio(){
    if(recoveryAudioUrl)URL.revokeObjectURL(recoveryAudioUrl);
    recoveryAudioUrl='';
    selectedRecoverySegment=0;
  }

  async function prepareRecoveryPlayback(sessionId:string, segmentId?:number){
    clearRecoveryAudio();
    const session=recoveries.find((item)=>item.id===sessionId);
    if(!session)return;
    selectedRecoveryId=sessionId;
    const playable=session.segments.filter((segment)=>segment.chunkCount>0);
    if(!playable.length)return;
    const target=playable.find((segment)=>segment.id===segmentId)??playable.at(-1)!;
    selectedRecoverySegment=target.id;
    const blob=await recoveryStore.buildSegmentBlob(sessionId,target.id);
    recoveryAudioUrl=URL.createObjectURL(blob);
  }

  async function continueRecovery(session:LiveSessionManifest){
    if(recording||processing)return;
    if(permissionState!=='ready'&&!(await detectMicrophones()))return;
    recoveryBusy=session.id;
    try{
      const stream=await acquireRecordingStream();
      await controller.continueSession(session.id,stream);
      recoveries=recoveries.filter((item)=>item.id!==session.id);
      clearRecoveryAudio();
      notify('success','Recovered recording continued in a new segment.');
    }catch(error){
      notify('error',error instanceof Error?error.message:String(error));
    }finally{recoveryBusy='';}
  }

  async function finishRecovery(session:LiveSessionManifest){
    await finalizeSession(session.id,'Recovered recording queued for transcription.');
  }

  async function saveRecoveryAudio(session:LiveSessionManifest){
    recoveryBusy=session.id;
    try{
      const updated=structuredClone(session);
      updated.retention={mode:'client',handleKey:session.id};
      updated.updatedAtMs=Date.now();
      if(localDirectoryAvailable){
        const handle=await chooseClientAudioDirectory();
        if(handle)await recoveryStore.saveClientDirectoryHandle(session.id,handle);
      }
      await recoveryStore.updateSession(updated);
      await finalizeSession(session.id,'Recovered recording finalized and saved.');
    }catch(error){
      notify('error',error instanceof Error?error.message:String(error));
      recoveryBusy='';
    }
  }

  async function deleteRecovery(session:LiveSessionManifest){
    if(recording&&snapshot.activeSessionId===session.id)return;
    if(!window.confirm('Delete this recovered recording permanently?'))return;
    recoveryBusy=session.id;
    try{
      await recoveryStore.deleteSession(session.id);
      notify('success','Recovered recording deleted.');
      await refreshRecoveries();
    }catch(error){
      notify('error',error instanceof Error?error.message:String(error));
    }finally{recoveryBusy='';}
  }

  function listenLast30(){
    if(!recoveryAudio)return;
    seekLastSeconds(recoveryAudio,30);
    void recoveryAudio.play();
  }

  async function saveFinalAudio(){
    if(!current?.quick?.live)return;
    try{
      const response=await api.jobAudio(current.id);
      await saveResponseToDirectory(response,current.quick.live.audioResultName,undefined,window);
      notify('success','Final audio saved.');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
  }

  async function exportResult(format:ExportFormat){
    if(!current||current.status!=='done')return;
    exporting=format;
    try{
      const result=await api.jobExport(current.id,format);
      await saveMarkdownLocally(result.filename,result.blob);
      notify('success',`${format.toUpperCase()} export saved.`);
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
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
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
  }

  function duration(seconds:number){
    const hours=Math.floor(seconds/3600);
    const minutes=Math.floor((seconds%3600)/60);
    const secs=seconds%60;
    return hours>0
      ? `${String(hours).padStart(2,'0')}:${String(minutes).padStart(2,'0')}:${String(secs).padStart(2,'0')}`
      : `${String(minutes).padStart(2,'0')}:${String(secs).padStart(2,'0')}`;
  }
  function formatBytes(bytes:number|undefined){
    if(bytes===undefined)return '���';
    if(bytes<1024)return `${bytes} B`;
    if(bytes<1024*1024)return `${(bytes/1024).toFixed(1)} KB`;
    if(bytes<1024*1024*1024)return `${(bytes/1024/1024).toFixed(1)} MB`;
    return `${(bytes/1024/1024/1024).toFixed(1)} GB`;
  }
  function recoveryDuration(session:LiveSessionManifest){
    return Math.round(session.segments.reduce((sum,segment)=>{
      const end=segment.endedAtMs??segment.lastCheckpointAtMs??segment.startedAtMs;
      return sum+Math.max(0,end-segment.startedAtMs);
    },0)/1000);
  }
  function recoveryGap(session:LiveSessionManifest){
    return session.segments.reduce((sum,segment)=>sum+(segment.gapMsBefore??0),0);
  }

  onMount(()=>{
    secureContext=window.isSecureContext;
    unsubscribeController=controller.subscribe(applySnapshot);
    localProxyPlatform=detectLocalProxyPlatform(navigator as Navigator & {userAgentData?:{platform?:string}});
    const inferredTarget=inferLocalProxyTarget(window.location);
    localProxyHost=inferredTarget.host;
    localProxyTargetPort=inferredTarget.port;
    localProxyNeedsTarget=!inferredTarget.host;

    void (async()=>{
      try{
        const persistent=await requestPersistentStorage(window);
        persistenceState=persistent===true?'persistent':persistent===false?'best-effort':'unsupported';
        const estimate=await storageEstimate(window);
        storageUsage=estimate.usage;
        storageQuota=estimate.quota;
      }catch{persistenceState='best-effort';}
      if(!controller.getSnapshot().recording)await refreshRecoveries();
    })();

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
        }catch{/* optional */}
        if(permissionName==='granted'){await refreshMicrophones();if(devices.length)permissionState='ready';}
      })();
      navigator.mediaDevices.addEventListener?.('devicechange',refreshMicrophones);
    }
  });

  onDestroy(()=>{
    if(permissionStatus)permissionStatus.onchange=null;
    navigator.mediaDevices?.removeEventListener?.('devicechange',refreshMicrophones);
    unsubscribeController?.();
    stopMeter();
    clearRecoveryAudio();
  });
</script>

<div class="page live-page">
  <div class="page-head">
    <div>
      <p class="page-kicker">BROWSER MICROPHONE</p>
      <h1 class="page-title">Live Recorder</h1>
      <p class="page-copy">Long recordings are checkpointed locally while you speak. A reload or browser crash can recover the audio already delivered by the browser, then Continue in a new segment.</p>
    </div>
  </div>

  {#if providers.length===0}<div class="notice warning">Add a transcription provider first.</div>{/if}
  {#if mediaError}<div class="notice warning microphone-warning"><strong>Microphone</strong><span>{mediaError}</span></div>{/if}
  {#if recoveryError}<div class="notice warning"><strong>Recovery storage</strong><span>{recoveryError}</span></div>{/if}

  <div class="microphone-diagnostics" aria-label="Microphone diagnostics">
    <span><strong>Secure context</strong> {secureContext?'yes':'no'}</span>
    <span><strong>Permission</strong> {permissionName}</span>
    <span><strong>Devices</strong> {devices.length}</span>
    <span><strong>Recovery</strong> {persistenceState==='persistent'?'Persistent':persistenceState==='best-effort'?'Best effort':persistenceState==='unsupported'?'Unavailable':'Checking���'}</span>
    <span><strong>Protected</strong> {formatBytes(protectedBytes)}</span>
    {#if storageQuota!==undefined}<span><strong>Browser storage</strong> {formatBytes(storageUsage)} / {formatBytes(storageQuota)}</span>{/if}
    <span class="spacer"></span>
    {#if permissionState!=='ready'}<button class="btn primary" on:click={detectMicrophones}>Enable microphone</button>{/if}
    <button class="btn" on:click={refreshMicrophones}>Refresh microphones</button>
    <button class="btn" class:primary={!secureContext} on:click={()=>localProxyPanel=!localProxyPanel}>Local microphone mode���</button>
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
          <button class="btn primary" disabled={!localProxyHost.trim()} on:click={downloadLocalProxyInstaller}><span>Download installer</span> �� {localProxyPlatform==='macos'?'macOS':localProxyPlatform==='windows'?'Windows':'Linux'}</button>
          <span class="help">{localProxyPlatform==='windows'?'Run the downloaded .cmd once; Windows will request administrator approval.':localProxyPlatform==='macos'?'Run once in Terminal; it installs a per-user LaunchAgent.':'Run once with bash; it installs a per-user systemd socket.'}</span>
        </div>
      </div>
    </section>
  {/if}

  {#if recoveries.length>0}
    <section class="card recovery-panel">
      <div class="card-header"><strong>Recovered LIVE recording</strong><span>{recoveries.length} available</span></div>
      <div class="card-body stack">
        <div class="recovery-tabs">
          {#each recoveries as session}
            <button class:active={session.id===selectedRecoveryId} on:click={()=>prepareRecoveryPlayback(session.id)}>
              {new Date(session.startedAtMs).toLocaleString()} �� {duration(recoveryDuration(session))}
            </button>
          {/each}
        </div>
        {#if selectedRecovery}
          <div class="recovery-summary">
            <span><strong>Started</strong>{new Date(selectedRecovery.startedAtMs).toLocaleString()}</span>
            <span><strong>Saved duration</strong>{duration(recoveryDuration(selectedRecovery))}</span>
            <span><strong>Protected audio</strong>{formatBytes(selectedRecovery.totalBytes)}</span>
            <span><strong>Segments</strong>{selectedRecovery.segments.length}</span>
            <span><strong>Interruption gap</strong>~{Math.round(recoveryGap(selectedRecovery)/1000)} s</span>
          </div>
          {#if selectedRecoveryPlayable.length}
            <div class="recovery-player stack">
              {#if selectedRecoveryPlayable.length>1}
                <div class="segmented recovery-segments">
                  {#each selectedRecoveryPlayable as segment}
                    <button class:active={segment.id===selectedRecoverySegment} on:click={()=>prepareRecoveryPlayback(selectedRecovery.id,segment.id)}>Segment {segment.id}</button>
                  {/each}
                </div>
              {/if}
              {#if recoveryAudioUrl}<audio bind:this={recoveryAudio} controls preload="metadata" src={recoveryAudioUrl}></audio>{/if}
              <button class="btn compact" on:click={listenLast30}>Listen to last 30 seconds</button>
            </div>
          {:else}
            <div class="notice warning compact">The session exists, but no browser-delivered audio checkpoint was completed before the interruption.</div>
          {/if}
          <div class="recovery-actions">
            <button class="btn primary" disabled={!!recoveryBusy||recording} on:click={()=>continueRecovery(selectedRecovery)}>Continue</button>
            <button class="btn" disabled={!!recoveryBusy||!selectedRecoveryPlayable.length} on:click={()=>finishRecovery(selectedRecovery)}>Finish and transcribe</button>
            <button class="btn" disabled={!!recoveryBusy||!selectedRecoveryPlayable.length} on:click={()=>saveRecoveryAudio(selectedRecovery)}>Save audio</button>
            <button class="btn danger" disabled={!!recoveryBusy||snapshot.activeSessionId===selectedRecovery.id} on:click={()=>deleteRecovery(selectedRecovery)}>Delete recovery</button>
          </div>
        {/if}
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
          <button class="record-button" class:recording disabled={processing&&!recording||(!recording&&!canRecord)} on:click={()=>recording?stopRecording():startRecording()} aria-label={recording?'Stop recording':'Start recording'}>
            <span></span>
          </button>
          <strong>{recording?'Recording ��� press again to stop':processing?'Finalizing recording���':permissionState==='requesting'?'Requesting microphone���':permissionState==='denied'?'Microphone unavailable':permissionState==='idle'?'Enable microphone':'Ready to record'}</strong>
          <span class="muted small">{recording?`Audio is being checkpointed locally. ${formatBytes(protectedBytes)} is already protected.`:'Browser-delivered audio is stored progressively for crash/reload recovery before the final server upload.'}</span>
        </div>

        <div class="grid two">
          <div class="field">
            <label for="live-microphone">Microphone</label>
            <select id="live-microphone" class="select" bind:value={selectedDeviceId} disabled={recording||permissionState!=='ready'}>
              {#if devices.length===0}<option value="">No audio input device is currently visible to the browser.</option>{/if}
              {#each devices as device,index}<option value={device.deviceId}>{device.label||`Microphone ${index+1}`}</option>{/each}
            </select>
          </div>
          <div class="field"><label for="live-language">Language</label><input id="live-language" class="input" bind:value={language} placeholder="auto, fr, en���" disabled={recording}/></div>
        </div>

        <TranscriptionChainEditor value={transcriptionChain} {providers} {notify} onchange={(routes)=>transcriptionChain=routes} />

        <div class="field">
          <label for="live-structure">AI structure <span class="muted">optional</span></label>
          <select id="live-structure" class="select" bind:value={structureProfileId} disabled={recording}>
            <option value="">None ��� deterministic transcript only</option>
            {#each structureProfiles.filter((profile)=>!!profile.providerId) as profile}<option value={profile.id}>{profile.name}</option>{/each}
          </select>
          <span class="help">The profile creates an additional structured view. The complete transcript remains in the same Markdown file.</span>
        </div>

        <div class="subsection stack">
          <strong>Destination</strong>
          <div class="segmented"><button disabled={recording} class:active={outputMode==='server'} on:click={()=>outputMode='server'}>Server folder</button><button disabled={recording} class:active={outputMode==='computer'} on:click={()=>outputMode='computer'}>This computer</button></div>
          {#if outputMode==='server'}
            <div class="field"><label for="live-output">Markdown destination</label><div class="path-control"><input id="live-output" class="input mono" value={outputDir} readonly placeholder="Choose a folder" /><button class="btn" disabled={recording} on:click={()=>picker=true}>Browse</button></div></div>
          {:else}
            <div class="client-output-note"><span>���</span><div><strong>{localDirectoryAvailable?'Save into a local folder after transcription':'Download the result after transcription'}</strong><p>Browser security requires the final local save to remain an explicit action.</p></div></div>
          {/if}
          <div class="row wrap"><label class="check"><input type="checkbox" bind:checked={frontmatter} disabled={recording}/> YAML properties</label><label class="check"><input type="checkbox" bind:checked={paragraphs} disabled={recording}/> Readable deterministic paragraphs</label></div>
        </div>

        <div class="subsection stack final-audio-settings">
          <strong>Keep final audio</strong>
          <div class="segmented">
            <button disabled={recording} class:active={audioRetentionMode==='none'} on:click={()=>audioRetentionMode='none'}>No copy</button>
            <button disabled={recording} class:active={audioRetentionMode==='server'} on:click={()=>audioRetentionMode='server'}>Server folder</button>
            <button disabled={recording} class:active={audioRetentionMode==='client'} on:click={()=>audioRetentionMode='client'}>This computer</button>
          </div>
          {#if audioRetentionMode==='server'}
            <div class="field"><label for="live-audio-output">Audio destination</label><div class="path-control"><input id="live-audio-output" class="input mono" value={audioServerDir} readonly placeholder="Choose a folder" /><button class="btn" on:click={()=>audioPicker=true}>Browse</button></div></div>
          {:else if audioRetentionMode==='client'}
            <div class="client-output-note"><span>REC</span><div><strong>{clientAudioFolderName|| (localDirectoryAvailable?'Choose a local audio folder':'Final M4A will download after server finalization')}</strong><p>The browser recovery copy is kept until the finalized M4A is successfully saved.</p></div>{#if localDirectoryAvailable}<button class="btn compact" disabled={recording} on:click={chooseClientAudioDirectory}>Choose folder</button>{/if}</div>
          {:else}
            <span class="help">No permanent audio copy. The server still keeps the accepted LIVE source for the 24-hour retry window.</span>
          {/if}
        </div>
      </div>
    </section>

    <aside class="card result-card">
      <div class="card-header"><strong>Recording result</strong>{#if current}<span class="status-pill {current.status}">{current.status}</span>{/if}</div>
      <div class="card-body result-body">
        {#if !current}
          <div class="empty result-empty"><span>REC</span><strong>Press the record button.</strong><p>During a long session, ScribeWatch checkpoints audio locally. Stop finalizes one M4A on Cloud9, then transcribes it.</p></div>
        {:else if current.status==='error'||current.status==='interrupted'||current.status==='cancelled'}
          <div class="error-box"><strong>Processing did not finish</strong><span>{current.error??current.status}</span>{#if current.quick?.sourceKind==='live'}<button class="btn compact" on:click={()=>api.retryJob(current.id).then(refresh).catch((e)=>notify('error',String(e)))}>Retry</button>{/if}</div>
        {:else if current.status==='done'}
          <div class="result-success"><span class="result-check">���</span><h2>{current.quick?.resultName??'Note ready'}</h2>
            {#if current.structuringError}<div class="notice warning compact">Transcript succeeded; AI structuring failed: {current.structuringError}</div>{/if}
            {#if current.quick?.outputKind==='server'&&current.markdownPath}<code class="result-path">{current.markdownPath}</code>{/if}
            {#if current.quick?.sourceKind==='live'}<button class="btn compact" on:click={saveFinalAudio}>Save final audio���</button>{/if}
            <div class="result-export-panel">
              <strong>Export result</strong><span class="help">Download / save as</span>
              <div class="export-actions">
                {#each exportFormats as format}
                  <button class="btn compact" class:primary={format==='pdf'} disabled={!!exporting} on:click={()=>exportResult(format)}>{exporting===format?'���':format.toUpperCase()}</button>
                {/each}
              </div>
            </div>
          </div>
        {:else}
          <div class="processing-state"><div class="pulse-ring"></div><strong>{current.status==='transcribing'?'Transcribing���':current.status==='structuring'?'Structuring with AI���':current.status==='publishing'?'Writing Markdown���':'Queued���'}</strong></div>
        {/if}
      </div>
    </aside>
  </div>
</div>

<PathPicker open={picker} title="Choose Markdown folder" mode="directory" initialPath={outputDir} onselect={(path)=>{outputDir=path;picker=false}} onclose={()=>picker=false} />
<PathPicker open={audioPicker} title="Choose final audio folder" mode="directory" initialPath={audioServerDir||outputDir} onselect={(path)=>{audioServerDir=path;audioPicker=false}} onclose={()=>audioPicker=false} />
