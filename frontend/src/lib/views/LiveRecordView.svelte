<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api';
  import ReportEditor from '$lib/components/ReportEditor.svelte';
  import { locale, t } from '$lib/i18n';
  import { canSaveToDirectory, ensureDirectoryPermission, saveMarkdownLocally, saveResponseToDirectory, uniqueDirectoryFilename } from '$lib/local-save';
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
  import {sanitizePracticePresets,sanitizeSessionSchemas,sessionFieldPairs,
    type PracticePreset,type SessionFieldType,type SessionFormSchemas} from '$lib/session-fields';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import {autoSaveFormats} from '$lib/live-auto-save';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { ExportFormat, Job, Provider, QuickOptions, StructureProfile, TranscriptionRoute } from '$lib/types';

  export let simpleMode=false;
  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let jobs:Job[]=[];
  export let selectedSpace='all';
  export let onassign:(jobId:string,spaceId:string|null)=>Promise<void>=async()=>{};

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

  let outputMode:'computer'|'server'='computer';
  let outputDir='';
  let picker=false;
  let transcriptionChain:TranscriptionRoute[]=[{providerId:'',model:''}];
  let chainInitialized=false;
  let language='';
  let structureProfileId='';
  let medicalReportEnabled=false;
  let simpleProfileTouched=false;
  const simpleProfileKey='scribewatch.live.simpleProfile.v1';
  function profileIsClinical(id:string){
    return id==='dental-consultation-complete-fr'||id==='medical-consultation';
  }
  function selectStructureProfile(id:string){
    structureProfileId=id;
    // Switching a document profile never carries patient-specific custom
    // field answers into the next profile. Only reusable definitions persist.
    sessionValues={};
    selectPracticeForCurrentProfile();
    if(!simpleMode)return;
    simpleProfileTouched=true;
    medicalReportEnabled=profileIsClinical(id);
    try{localStorage.setItem(simpleProfileKey,id);}catch{/* storage unavailable */}
  }

  let patientSurname='';
  let patientForename='';
  let patientBirthDate='';
  let practitionerName='';
  let practiceAddress='';
  let practiceContact='';
  let consultationDate='';
  type EncounterMetadata={
    surname:string;forename:string;dateOfBirth:string;consultationDate:string;
    practitioner:string;practiceAddress:string;practiceContact:string;
    sessionFields:Array<{label:string;value:string}>;
    medicalReportEnabled:boolean;
  };
  // A processing job must never inherit another patient's subsequently edited
  // form. Only keep snapshots in memory; no patient data in localStorage.
  const metadataByJob=new Map<string,EncounterMetadata>();
  function snapshotEncounter():EncounterMetadata {
    return {
      surname:patientSurname,forename:patientForename,dateOfBirth:patientBirthDate,
      consultationDate,practitioner:practitionerName,practiceAddress,practiceContact,
      sessionFields:sessionFieldPairs(currentSessionFields,sessionValues),
      medicalReportEnabled
    };
  }
  function clearCurrentEncounterForm(){
    if(recording||processing)return;
    patientSurname='';patientForename='';patientBirthDate='';
    consultationDate='';sessionValues={};
    notify('success','Champs patient et réponses de séance effacés. Le praticien reste sélectionné.');
  }
  const practiceIdentityKey='scribewatch.practice.identity.v1'; // legacy migration only
  const practiceDirectoryKey='scribewatch.practice.directory.v2';
  const practiceSelectionKey='scribewatch.practice.selection.v2';
  const sessionSchemasKey='scribewatch.session.schemas.v1';
  const profileScope=()=>structureProfileId||'__default__';
  const newPresetId=()=>crypto.randomUUID?.()??('local-'+Date.now().toString(36)+'-'+Math.random().toString(36).slice(2));
  let practicePresets:PracticePreset[]=[];
  let selectedPracticeId='';
  let selectedPracticeByProfile:Record<string,string>={};
  let sessionSchemas:SessionFormSchemas={};
  $: currentSessionFields=sessionSchemas[structureProfileId||'__default__']??[];
  let sessionValues:Record<string,string>={};
  let newFieldLabel='';
  let newFieldType:SessionFieldType='text';
  function selectPracticeForCurrentProfile(){
    const chosen=selectedPracticeByProfile[profileScope()]??'';
    const preset=practicePresets.find(p=>p.id===chosen);
    selectedPracticeId=preset?.id??'';
    practitionerName=preset?.name??'';
    practiceAddress=preset?.address??'';
    practiceContact=preset?.contact??'';
  }
  function choosePractice(id:string){
    selectedPracticeByProfile={...selectedPracticeByProfile,[profileScope()]:id};
    try{localStorage.setItem(practiceSelectionKey,JSON.stringify(selectedPracticeByProfile));}
    catch{/* preferred practitioner is optional */}
    selectPracticeForCurrentProfile();
  }
  function startNewPractice(){
    choosePractice('');
    practitionerName='';practiceAddress='';practiceContact='';
  }
  // Store ONLY reusable professional information. No patient/session answers.
  function savePracticeIdentity(){
    const name=practitionerName.trim().slice(0,160);
    if(!name)return;
    const id=selectedPracticeId||newPresetId();
    const entry={id,name,address:practiceAddress.trim().slice(0,240),
      contact:practiceContact.trim().slice(0,200)};
    const updated=[...practicePresets.filter(p=>p.id!==id),entry];
    try{
      localStorage.setItem(practiceDirectoryKey,JSON.stringify(updated));
      practicePresets=updated;
      choosePractice(id);
      notify('success','Fiche praticien enregistrée localement, sans données patient.');
    }catch{notify('error','Impossible d’enregistrer la fiche praticien.');}
  }
  function clearPracticeIdentity(){
    if(!selectedPracticeId)return;
    const updated=practicePresets.filter(p=>p.id!==selectedPracticeId);
    try{
      localStorage.setItem(practiceDirectoryKey,JSON.stringify(updated));
      practicePresets=updated;
      startNewPractice();
      notify('success','Fiche praticien retirée de cet ordinateur.');
    }catch{notify('error','Impossible de supprimer cette fiche.');}
  }
  function persistSessionSchemas(next:SessionFormSchemas){
    try{
      const sanitized=sanitizeSessionSchemas(next);
      localStorage.setItem(sessionSchemasKey,JSON.stringify(sanitized));
      sessionSchemas=sanitized;
      return true;
    }catch{notify('error','Impossible d’enregistrer les champs de ce profil.');return false;}
  }
  function addSessionField(){
    const label=newFieldLabel.trim().slice(0,90);
    if(!label||currentSessionFields.length>=24)return;
    const id=newPresetId();
    const fields=[...currentSessionFields,{id,label,type:newFieldType}];
    if(persistSessionSchemas({...sessionSchemas,[profileScope()]:fields}))newFieldLabel='';
  }
  function removeSessionField(id:string){
    if(persistSessionSchemas({...sessionSchemas,
      [profileScope()]:currentSessionFields.filter(field=>field.id!==id)})){
      const {[id]:_deleted,...kept}=sessionValues;
      sessionValues=kept;
    }
  }
  let markdownDestination:FileSystemDirectoryHandle|undefined;
  let markdownDestinationLabel='';
  async function chooseMarkdownFolder(){
    if(!localDirectoryAvailable)return;
    try{
      markdownDestination=await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle;
      markdownDestinationLabel=markdownDestination?.name??'';
    }catch(e){if(!(e instanceof DOMException&&e.name==='AbortError'))
      notify('error',e instanceof Error?e.message:String(e));}
  }
  let pdfDestination:FileSystemDirectoryHandle|undefined;
  let pdfDestinationLabel='';
  let simpleDirectory:FileSystemDirectoryHandle|undefined;
  let simpleDirectoryName='';
  const simpleDirectoryKey='ui-default-report-output-v1';
  let lastAutoSavedSession='';
  let autoSaving=false;
  async function chooseSimpleDestination(){
    if(!localDirectoryAvailable||recording)return;
    try{
      const selected=await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle;
      simpleDirectory=selected;
      simpleDirectoryName=selected.name;
      pdfDestination=selected;
      pdfDestinationLabel=selected.name;
      markdownDestination=selected;
      markdownDestinationLabel=selected.name;
      // A selected local folder overrides any previous advanced server preset.
      outputMode='computer';
      try{await recoveryStore.saveClientDirectoryHandle(simpleDirectoryKey,selected);}
      catch{notify('error','Folder selected for this session, but the browser could not remember it for the next launch.');}
    }catch(error){
      if(!(error instanceof DOMException&&error.name==='AbortError'))
        notify('error',error instanceof Error?error.message:String(error));
    }
  }

  type LivePreset={
    name:string;transcriptionChain:TranscriptionRoute[];language:string;
    structureProfileId:string;outputMode:'computer'|'server';outputDir:string;
    frontmatter:boolean;paragraphs:boolean;medicalReportEnabled:boolean;
    audioRetentionMode:'none'|'server'|'client';audioServerDir:string;
  };
  const presetStorageKey='scribewatch.live.presets.v1';
  let presets:LivePreset[]=[];
  let presetName='';
  let presetSelected='';
  function savePreset(){
    const name=presetName.trim().slice(0,64);
    if(!name){notify('error','Nom de profil requis');return;}
    const item:LivePreset={
      name,transcriptionChain:structuredClone(transcriptionChain),language,
      structureProfileId,outputMode,outputDir,frontmatter,paragraphs,
      medicalReportEnabled,audioRetentionMode,audioServerDir
    };
    presets=[...presets.filter(p=>p.name!==name),item].slice(-30);
    try{localStorage.setItem(presetStorageKey,JSON.stringify(presets));presetSelected=name;
      notify('success','Profil de session enregistre sans donnees patient.');
    }catch(e){notify('error','Enregistrement du profil impossible');}
  }
  function applyPreset(name:string){
    presetSelected=name;
    const p=presets.find(p=>p.name===name);
    if(!p)return;
    transcriptionChain=structuredClone(p.transcriptionChain);
    chainInitialized=true;
    language=p.language;structureProfileId=p.structureProfileId;
    outputMode=p.outputMode;outputDir=p.outputDir;
    frontmatter=p.frontmatter;paragraphs=p.paragraphs;
    medicalReportEnabled=p.medicalReportEnabled;
    audioRetentionMode=p.audioRetentionMode;audioServerDir=p.audioServerDir;
  }
  function deletePreset(){
    if(!presetSelected)return;
    presets=presets.filter(p=>p.name!==presetSelected);
    localStorage.setItem(presetStorageKey,JSON.stringify(presets));
    presetSelected='';presetName='';
  }

  async function choosePdfFolder(){
    if(!localDirectoryAvailable)return;
    try{
      pdfDestination=await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle;
      pdfDestinationLabel=pdfDestination.name;
    }catch(error){
      if(!(error instanceof DOMException&&error.name==='AbortError'))
        notify('error',error instanceof Error?error.message:String(error));
    }
  }
  function enableMedicalReport(checked:boolean){
    medicalReportEnabled=checked;
    if(checked){
      const premium=structureProfiles.find(p=>p.id==='dental-consultation-complete-fr'&&p.providerId);
      const general=structureProfiles.find(p=>p.id==='medical-consultation'&&p.providerId);
      const current=structureProfiles.find(p=>p.id===structureProfileId);
      if(!current?.providerId)structureProfileId=(premium??general)?.id??'';
    }
  }
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
    const provider=providers.find((candidate)=>candidate.enabled && /parakeet.ultra/i.test(candidate.model) && /127\.0\.0\.1:38005|localhost:38005/i.test(candidate.transcriptionUrl))??providers.find((candidate)=>candidate.enabled && /parakeet.ultra/i.test(candidate.model))??providers.find((candidate)=>candidate.enabled)??providers[0];
    transcriptionChain=[{providerId:provider.id,model:provider.model}];
    chainInitialized=true;
  }
  // In simple mode preselect the built-in dental profile when it is actually
  // wired to a provider. Never override an existing choice or patient data.
  $: if(simpleMode&&!recording){
    if(outputMode==='server')outputMode='computer';
    if(audioRetentionMode==='server'&&!audioServerDir)audioRetentionMode='none';
  }
  $: if(simpleMode&&!recording&&!structureProfileId&&!simpleProfileTouched){
    const dental=structureProfiles.find(p=>p.id==='dental-consultation-complete-fr'&&p.providerId&&p.model);
    if(dental){
      structureProfileId=dental.id;
      medicalReportEnabled=true;
    }
  }
  $: current=submitted ? (jobs.find((job)=>job.id===submitted?.id)??submitted) : undefined;
  // Only results from a session recorded in this view can trigger auto-export.
  // The directory must have been explicitly authorized using the picker.
  $: if(simpleMode&&submitted&&current?.id===submitted.id&&current.status==='done'
      &&simpleDirectory&&!autoSaving&&lastAutoSavedSession!==current.id){
    void autoSaveCompletedSession(current.id);
  }
  async function autoSaveCompletedSession(jobId:string){
    if(autoSaving||lastAutoSavedSession===jobId)return;
    lastAutoSavedSession=jobId; // do not retry silently after a permission/error
    autoSaving=true;
    const isClinical=metadataByJob.get(jobId)?.medicalReportEnabled??medicalReportEnabled;
    const formats=autoSaveFormats(current?.status??'pending',!!simpleDirectory,
      isClinical,!!current?.structuredProfileName&&!current?.structuringError);
    try{
      for(const format of formats){
        if(!(await exportResult(format,true))){
          notify('error',t($locale,'Automatic save was incomplete. Use the export buttons to save the remaining files.'));
          return;
        }
      }
      if(formats.length)notify('success',t($locale,'Results saved to {folder} (clinical PDF remains a draft).').replace('{folder}',simpleDirectoryName));
      if(isClinical&&!formats.includes('pdf'))
        notify('error',t($locale,'Clinical AI report unavailable: the Markdown transcript was saved, but no patient PDF was generated.'));
    }finally{autoSaving=false;}
  }

  $: chainReady=transcriptionChain.length>0&&transcriptionChain.every((route)=>!!route.providerId);
  $: outputReady=outputMode==='computer'||!!outputDir;
  $: audioRetentionReady=audioRetentionMode!=='server'||!!audioServerDir;
  // Simple mode never starts unless its destination can retain the audio and documents.
  $: canRecord=chainReady&&outputReady&&audioRetentionReady&&!processing&&(!simpleMode||!!simpleDirectory);
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
    // The single authorized Simple-mode folder holds the final M4A as well as PDF/MD.
    if(simpleMode&&simpleDirectory)return {mode:'client',handleKey:'pending'};
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
    if(!chainReady){notify('error','Choose a transcription provider first.');return;}
    if(!outputReady){notify('error','Choose where to save the recording result first.');return;}
    if(!audioRetentionReady){notify('error','Choose a server folder for the final audio or another retention mode.');return;}
    if(simpleMode&&!simpleDirectory){notify('error','Choose the destination folder before recording (audio, PDF and Markdown).');return;}
    if(simpleMode&&simpleDirectory){
      try{
        if(!(await ensureDirectoryPermission(simpleDirectory))){
          notify('error',t($locale,'Allow access to your previously chosen destination folder before recording.'));
          return;
        }
      }catch(error){
        notify('error',t($locale,'The destination folder is no longer available. Select it again.'));
        return;
      }
    }
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
      const finalAudioDirectory=simpleMode?simpleDirectory:clientAudioDirectory;
      if((simpleMode&&simpleDirectory||audioRetentionMode==='client')&&finalAudioDirectory){
        await recoveryStore.saveClientDirectoryHandle(manifest.id,finalAudioDirectory);
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
    // Freeze this consultation before asynchronous upload and processing.
    const encounter=snapshotEncounter();
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
      if(submitted){
        metadataByJob.set(submitted.id,encounter);
        if(encounter.medicalReportEnabled){
          try{await api.saveJobEncounter(submitted.id,encounter);}
          catch(error){
            notify('error','Compte-rendu en cours, mais les informations patient ne sont pas encore conservées : '+String(error));
          }
        }
      }
      if(submitted && selectedSpace!=='all' && selectedSpace!=='unassigned'){
        try{await onassign(submitted.id,selectedSpace);}catch{/* audio and job remain safe */}
      }
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

  async function exportResult(format:ExportFormat,silent=false):Promise<boolean>{
    if(!current||current.status!=='done')return false;
    exporting=format;
    const originalEncounter=metadataByJob.get(current.id)??snapshotEncounter();
    const encounter=(format==='pdf'&&originalEncounter.medicalReportEnabled
      ?await api.jobEncounter(current.id).catch(()=>null):null)??originalEncounter;
    try{
      if(format==='pdf'&&encounter.medicalReportEnabled){
        if(current.structuringError||!current.structuredProfileName)throw new Error('Clinical AI draft is incomplete: a patient PDF cannot be generated from the raw transcript.');
        const {createMedicalPdf,medicalFilename,uniqueMedicalFilename}=await import('$lib/medical-report');
        const source=await api.jobMarkdown(current.id);
        const sourceMarkdown=await source.blob.text();
        // Content-derived link to the separately preserved Markdown note, not a signature.
        const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(sourceMarkdown));
        const sourceDocumentSha256=Array.from(new Uint8Array(digest),value=>value.toString(16).padStart(2,'0')).join('');
        const patient={surname:encounter.surname,forename:encounter.forename,
          dateOfBirth:encounter.dateOfBirth,
          practitioner:encounter.practitioner,practiceAddress:encounter.practiceAddress,
          practiceContact:encounter.practiceContact,
          sourceSessionRef:current.quick?.live?.sessionId??current.id,
          sourceDocumentSha256,structuredProfile:current.structuredProfileName,
          structuredModel:current.structuredModel,
          sessionFields:encounter.sessionFields};
        const sessionMs=current.quick?.live?.startedAtMs??current.createdAtMs;
        const sessionDate=Number.isFinite(sessionMs)&&sessionMs>0?new Date(sessionMs):new Date();
        // A reprocessed recording may be older than the processing date.
        const dateOverride=/^\d{4}-\d{2}-\d{2}$/.test(encounter.consultationDate)
          ?new Date(encounter.consultationDate+'T12:00:00'):undefined;
        const documentDate=dateOverride&&!Number.isNaN(dateOverride.getTime())?dateOverride:sessionDate;
        const blob=await createMedicalPdf(sourceMarkdown,patient,documentDate);
        const target=pdfDestination??(localDirectoryAvailable?await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle:undefined);
        const filename=target?await uniqueMedicalFilename(target,medicalFilename(patient,documentDate)):medicalFilename(patient,documentDate);
        await saveResponseToDirectory(new Response(blob),filename,target,window);
        if(!silent)notify('success','Brouillon PDF créé. Vérifier les faits, archiver le Markdown source et faire valider par le praticien.');
      }else if(format==='pdf'){
        const saved=await api.jobReport(current.id);
        const {buildSavedNotePdf}=await import('$lib/note-pdf');
        const file=await buildSavedNotePdf(saved);
        const target=pdfDestination??markdownDestination;
        const filename=target?await uniqueDirectoryFilename(target,file.filename):file.filename;
        await saveResponseToDirectory(new Response(file.blob),filename,target,window);
        if(!silent)notify('success','PDF corrigé enregistré.');
      }else{
        const result=await api.jobExport(current.id,format);
        const filename=silent&&markdownDestination
          ?await uniqueDirectoryFilename(markdownDestination,result.filename)
          :result.filename;
        await saveResponseToDirectory(new Response(result.blob),filename,markdownDestination,window);
        if(!silent)notify('success',format.toUpperCase()+' export saved.');
      }
    return true;
    }catch(error){notify('error',error instanceof Error?error.message:String(error));return false;}
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
    if(bytes===undefined)return '—';
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
    if(simpleMode){
      void recoveryStore.loadClientDirectoryHandle(simpleDirectoryKey).then(savedFolder=>{
        if(!savedFolder||!localDirectoryAvailable)return;
        simpleDirectory=savedFolder;
        simpleDirectoryName=savedFolder.name;
        pdfDestination=savedFolder;pdfDestinationLabel=savedFolder.name;
        markdownDestination=savedFolder;markdownDestinationLabel=savedFolder.name;
      }).catch(()=>{/* optional folder persistence; manual export stays available */});
      try{
        const saved=localStorage.getItem(simpleProfileKey);
        if(saved!==null&&(saved===''||structureProfiles.some(p=>p.id===saved&&!!p.providerId))){
          structureProfileId=saved;
          simpleProfileTouched=true;
          medicalReportEnabled=profileIsClinical(saved);
        }
      }catch{/* browser storage unavailable */}
    }
    try{
      const stored=JSON.parse(localStorage.getItem(presetStorageKey)||'[]');
      if(Array.isArray(stored))presets=stored.filter(p=>p&&typeof p.name==='string'&&Array.isArray(p.transcriptionChain)).slice(-30);
    }catch{presets=[];}
    try{
      const directory=localStorage.getItem(practiceDirectoryKey);
      if(directory!==null){
        practicePresets=sanitizePracticePresets(JSON.parse(directory));
      }else{
        // Non-destructive one-way migration from the former single cabinet.
        const legacy=JSON.parse(localStorage.getItem(practiceIdentityKey)||'null');
        if(legacy&&typeof legacy.practitioner==='string'&&legacy.practitioner.trim()){
          practicePresets=sanitizePracticePresets([{
            id:'imported-practice',name:legacy.practitioner,
            address:legacy.address||'',contact:legacy.contact||''
          }]);
          localStorage.setItem(practiceDirectoryKey,JSON.stringify(practicePresets));
          selectedPracticeByProfile={[profileScope()]:practicePresets[0].id};
        }
      }
      const selections=JSON.parse(localStorage.getItem(practiceSelectionKey)||'{}');
      if(selections&&typeof selections==='object'&&!Array.isArray(selections)){
        const validSelections:Record<string,string>={};
        for(const [key,id] of Object.entries(selections)){
          if(key.length<=150&&typeof id==='string'&&id.length<=80)
            validSelections[key]=id;
        }
        selectedPracticeByProfile={...selectedPracticeByProfile,...validSelections};
      }
      sessionSchemas=sanitizeSessionSchemas(JSON.parse(localStorage.getItem(sessionSchemasKey)||'{}'));
      selectPracticeForCurrentProfile();
    }catch{/* Optional professional presets and field schemas. Patient data not stored. */}
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
      <h1 class="page-title">{simpleMode?'Record a consultation':'Live Recorder'}</h1>
      <p class="page-copy">{simpleMode?'Choose a report profile and destination folder, then press REC. Recording is protected as it progresses.':'Long recordings are checkpointed locally while you speak. A reload or browser crash can recover the audio already delivered by the browser, then Continue in a new segment.'}</p>
    </div>
  </div>

  {#if providers.length===0}<div class="notice warning">Add a transcription provider first.</div>{/if}
  {#if mediaError}<div class="notice warning microphone-warning"><strong>Microphone</strong><span>{mediaError}</span></div>{/if}
  {#if recoveryError}<div class="notice warning"><strong>Recovery storage</strong><span>{recoveryError}</span></div>{/if}

  {#if simpleMode&&permissionState!=='ready'}<div class="row wrap"><button class="btn primary" on:click={detectMicrophones}>Enable microphone</button>{#if !secureContext}<button class="btn" on:click={()=>localProxyPanel=!localProxyPanel}>Local microphone mode…</button>{/if}</div>{/if}
  {#if !simpleMode}<div class="microphone-diagnostics" aria-label="Microphone diagnostics">
    <span><strong>Secure context</strong> {secureContext?'yes':'no'}</span>
    <span><strong>Permission</strong> {permissionName}</span>
    <span><strong>Devices</strong> {devices.length}</span>
    <span><strong>Recovery</strong> {persistenceState==='persistent'?'Persistent':persistenceState==='best-effort'?'Best effort':persistenceState==='unsupported'?'Unavailable':'Checking…'}</span>
    <span><strong>Protected</strong> {formatBytes(protectedBytes)}</span>
    {#if storageQuota!==undefined}<span><strong>Browser storage</strong> {formatBytes(storageUsage)} / {formatBytes(storageQuota)}</span>{/if}
    <span class="spacer"></span>
    {#if permissionState!=='ready'}<button class="btn primary" on:click={detectMicrophones}>Enable microphone</button>{/if}
    <button class="btn" on:click={refreshMicrophones}>Refresh microphones</button>
    <button class="btn" class:primary={!secureContext} on:click={()=>localProxyPanel=!localProxyPanel}>Local microphone mode…</button>
  </div>

  {/if}

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
          <div class="field"><label for="proxy-host">NAS hostname / IPv4</label><input id="proxy-host" class="input mono" bind:value={localProxyHost} placeholder="nas.local or nas.local" /></div>
          <div class="field"><label for="proxy-target-port">ScribeWatch HTTP port</label><input id="proxy-target-port" class="input mono" type="number" min="1" max="65535" bind:value={localProxyTargetPort} /></div>
          <div class="field"><label for="proxy-local-port">Local port</label><input id="proxy-local-port" class="input mono" type="number" min="1" max="65535" bind:value={localProxyPort} /></div>
        </div>
        <div class="local-proxy-install-row">
          <button class="btn primary" disabled={!localProxyHost.trim()} on:click={downloadLocalProxyInstaller}><span>Download installer</span> · {localProxyPlatform==='macos'?'macOS':localProxyPlatform==='windows'?'Windows':'Linux'}</button>
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
              {new Date(session.startedAtMs).toLocaleString()} · {duration(recoveryDuration(session))}
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
          <strong>{recording?'Recording — press again to stop':processing?'Finalizing recording…':permissionState==='requesting'?'Requesting microphone…':permissionState==='denied'?'Microphone unavailable':permissionState==='idle'?'Enable microphone':simpleMode&&!simpleDirectory?'Choose destination folder':'Ready to record'}</strong>
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
          {#if !simpleMode}<div class="field"><label for="live-language">Language</label><input id="live-language" class="input" bind:value={language} placeholder="auto, fr, en…" disabled={recording}/></div>{/if}
        </div>

        {#if !simpleMode}<TranscriptionChainEditor value={transcriptionChain} {providers} {notify} onchange={(routes)=>transcriptionChain=routes} />{/if}

        <div class="field">
          <label for="live-structure">{simpleMode?'Report profile':'AI structure'} {#if !simpleMode}<span class="muted">optional</span>{/if}</label>
          <select id="live-structure" class="select" bind:value={structureProfileId} on:change={(e)=>selectStructureProfile(e.currentTarget.value)} disabled={recording}>
            <option value="">None — deterministic transcript only</option>
            {#each structureProfiles as profile}<option value={profile.id} disabled={!profile.providerId}>{profile.name}{profile.providerId?'':' — fournisseur LLM à configurer'}</option>{/each}
          </select>
          {#if !simpleMode}<span class="help">The profile creates an additional structured view. The complete transcript remains in the same Markdown file.</span>{/if}
        </div>

        <div class="subsection stack">
          <strong>Profil de sortie de la session</strong>
          {#if !simpleMode}
          <div class="grid two">
            <div class="field"><label for="live-preset">Profil enregistre</label><select id="live-preset" class="select" value={presetSelected} on:change={(e)=>applyPreset(e.currentTarget.value)} disabled={recording}><option value="">Configuration manuelle</option>{#each presets as preset}<option value={preset.name}>{preset.name}</option>{/each}</select></div>
            <div class="field"><label for="live-preset-name">Nom du profil</label><input id="live-preset-name" class="input" bind:value={presetName} disabled={recording} placeholder="Consultation parodontale"/></div>
          </div>
          <div class="row wrap"><button class="btn compact" on:click={savePreset} disabled={recording||!presetName.trim()}>Enregistrer ce profil</button>{#if presetSelected}<button class="btn compact" on:click={deletePreset} disabled={recording}>Supprimer le profil</button>{/if}</div>
          <span class="help">Le profil conserve la chaine de transcription, les modeles et les destinations. Les identites des patients ne sont jamais sauvegardees dans ces profils.</span>
          <span class="help">Transcription et modele de structuration : selections ci-dessus.</span>
          {/if}
          <label class="check"><input type="checkbox" checked={medicalReportEnabled} on:change={(e)=>enableMedicalReport(e.currentTarget.checked)} disabled={recording}/> Compte rendu de consultation (PDF)</label>
          {#if medicalReportEnabled}
            <div class="notice compact"><strong>Brouillon a faire valider par le praticien.</strong><span>La transcription originale reste disponible. Les donnees patient ne sont pas enregistrees dans les preferences.</span></div>
            {#if !structureProfileId}<span class="help">Pour une synthese medicale, associer un LLM au profil medical dans AI structure profiles. Sinon le PDF contiendra la transcription brute.</span>{/if}
            <div class="field">
              <label for="practice-preset">Praticien / cabinet enregistré</label>
              <select id="practice-preset" class="select" value={selectedPracticeId} on:change={(e)=>choosePractice(e.currentTarget.value)}>
                <option value="">Saisie manuelle / nouveau praticien</option>
                {#each practicePresets as preset (preset.id)}
                  <option value={preset.id}>{preset.name}{preset.address?' — '+preset.address:''}</option>
                {/each}
              </select>
            </div>
            <div class="grid two">
              <div class="field"><label for="patient-last">Nom (facultatif)</label><input id="patient-last" class="input" autocomplete="off" bind:value={patientSurname}/></div>
              <div class="field"><label for="patient-first">Prenom (facultatif)</label><input id="patient-first" class="input" autocomplete="off" bind:value={patientForename}/></div>
              <div class="field"><label for="patient-dob">Date de naissance</label><input id="patient-dob" class="input" type="date" bind:value={patientBirthDate}/></div>
              <div class="field"><label for="patient-doctor">Praticien</label><input id="patient-doctor" class="input" autocomplete="off" bind:value={practitionerName}/></div>
              <div class="field"><label for="practice-address">Adresse du cabinet (facultative)</label><input id="practice-address" class="input" autocomplete="off" bind:value={practiceAddress} placeholder="Adresse complète" /></div>
              <div class="field"><label for="practice-contact">Contact professionnel (facultatif)</label><input id="practice-contact" class="input" autocomplete="off" bind:value={practiceContact} placeholder="Téléphone ou e-mail" /></div>
              <div class="field"><label for="consultation-date">Date de consultation (si différente de la date d'enregistrement)</label><input id="consultation-date" class="input" type="date" bind:value={consultationDate}/></div>
            </div>
            <div class="row wrap">
              <button class="btn compact" on:click={savePracticeIdentity} disabled={!practitionerName.trim()}>{selectedPracticeId?'Mettre à jour ce praticien':'Enregistrer comme nouveau praticien'}</button>
              <button class="btn compact" on:click={startNewPractice}>Nouvelle fiche</button>
              <button class="btn compact" on:click={clearPracticeIdentity} disabled={!selectedPracticeId}>Supprimer la fiche sélectionnée</button>
            </div>
            <span class="help">Plusieurs praticiens ou cabinets peuvent être mémorisés et choisis selon le profil de compte rendu. Aucune information patient n'est conservée avec ces fiches.</span>
            <button class="btn compact" on:click={clearCurrentEncounterForm} disabled={recording||processing}>Nouveau dossier : vider les champs patient et de séance</button>
            {#if !simpleMode}
            <div class="row wrap"><button class="btn compact" on:click={choosePdfFolder} disabled={!localDirectoryAvailable}>Dossier PDF</button><span class="help">{pdfDestinationLabel||'Choisir au moment de la sauvegarde'}</span></div>
            {/if}
            <span class="help">Choisissez « Consultation dentaire » ou « Consultation médicale » selon le rendez-vous. Le PDF est un brouillon à valider par le praticien ; conserver la source de l'enregistrement séparément. Les informations du formulaire patient/cabinet sont ajoutées localement au PDF.</span>
            {#if audioRetentionMode==='none'}<span class="help">Attention : aucune copie audio permanente. Pour documenter les échanges, choisir explicitement une conservation de l'audio si elle est autorisée ; le PDF seul ne certifie pas les propos.</span>{/if}
            <span class="help">Nom automatique du PDF : 3 lettres NOM + 3 lettres PRENOM + date. Rendu local sur cet ordinateur, sans Pandoc.</span>
          {/if}
          {#if structureProfileId}
            <div class="subsection stack">
              <strong>Champs personnalisés du profil</strong>
              <span class="help">Les intitulés et types de champs sont mémorisés pour ce profil, quel que soit le LLM choisi. Les réponses sont propres à la session et ne sont pas sauvegardées dans les préférences.</span>
              {#each currentSessionFields as field (field.id)}
                <div class="field">
                  <label for={'profile-field-'+field.id}>{field.label}</label>
                  <div class="row">
                    {#if field.type==='multiline'}
                      <textarea id={'profile-field-'+field.id} rows="2" class="input"
                        value={sessionValues[field.id]??''}
                        on:input={(event)=>sessionValues={...sessionValues,[field.id]:event.currentTarget.value.slice(0,500)}}></textarea>
                    {:else}
                      <input id={'profile-field-'+field.id} class="input"
                        type={field.type==='date'?'date':field.type==='number'?'number':'text'}
                        value={sessionValues[field.id]??''}
                        on:input={(event)=>sessionValues={...sessionValues,[field.id]:event.currentTarget.value.slice(0,500)}} />
                    {/if}
                    <button class="btn compact" disabled={recording} on:click={()=>removeSessionField(field.id)}>Retirer le champ</button>
                  </div>
                </div>
              {/each}
              {#if currentSessionFields.length<24}
                <div class="grid two">
                  <div class="field">
                    <label for="new-field-label">Nouveau champ</label>
                    <input id="new-field-label" class="input" maxlength="90"
                      bind:value={newFieldLabel} placeholder="Référence, motif, service…"/>
                  </div>
                  <div class="field">
                    <label for="new-field-type">Type</label>
                    <select id="new-field-type" class="select" bind:value={newFieldType}>
                      <option value="text">Texte</option>
                      <option value="date">Date</option>
                      <option value="number">Nombre</option>
                      <option value="multiline">Texte long</option>
                    </select>
                  </div>
                </div>
                <button class="btn compact" disabled={recording||!newFieldLabel.trim()} on:click={addSessionField}>Ajouter ce champ au profil</button>
              {/if}
            </div>
          {/if}
        </div>
        <div class="subsection stack">
          <strong>Destination</strong>
          {#if simpleMode}
            <div class="row wrap">
              <button class="btn primary" disabled={!localDirectoryAvailable||recording} on:click={chooseSimpleDestination}>Choose destination folder</button>
              <span class="help">{simpleDirectoryName?`Audio M4A, PDF et transcription Markdown enregistrés dans ${simpleDirectoryName}`:'Choisir une fois le dossier local pour enregistrer automatiquement audio M4A, PDF et transcription Markdown.'}</span>
            </div>
            {#if !localDirectoryAvailable}<div class="notice warning compact">Automatic local folder saving requires this desktop browser to support the Directory Picker API. The manual export buttons remain available.</div>{/if}
            {#if outputMode==='server'}<span class="help">Existing server-side Markdown folder: {outputDir}. Change it in Advanced settings.</span>{/if}
          {/if}
          {#if !simpleMode}<div class="segmented"><button disabled={recording} class:active={outputMode==='server'} on:click={()=>outputMode='server'} >Dossier ScribeWatch (sur ce PC si installé)</button><button disabled={recording} class:active={outputMode==='computer'} on:click={()=>outputMode='computer'}>Dossier choisi via navigateur</button></div>{/if}
          {#if outputMode==='server'}
            <div class="field"><label for="live-output">Markdown destination</label><div class="path-control"><input id="live-output" class="input mono" value={outputDir} readonly placeholder="Choose a folder" /><button class="btn" disabled={recording} on:click={()=>picker=true}>Browse</button></div></div>
          {:else if !simpleMode}
            <div class="client-output-note"><span>⌁</span><div><strong>{markdownDestinationLabel||'Choisir le dossier des notes (.md, .txt, etc.)'}</strong><p>Le dossier peut être choisi avant l’enregistrement ; le fichier est sauvegardé lors de l’export. Sans accès aux dossiers, téléchargement classique.</p></div>{#if localDirectoryAvailable}<button class="btn compact" disabled={recording} on:click={chooseMarkdownFolder}>Choisir le dossier</button>{/if}</div>
          {/if}
          {#if !simpleMode}<div class="row wrap"><label class="check"><input type="checkbox" bind:checked={frontmatter} disabled={recording}/> YAML properties</label><label class="check"><input type="checkbox" bind:checked={paragraphs} disabled={recording}/> Readable deterministic paragraphs</label></div>{/if}
        </div>

        {#if !simpleMode}<div class="subsection stack final-audio-settings">
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
        </div>{/if}
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
          <div class="result-success"><span class="result-check">✓</span><h2>{current.quick?.resultName??'Note ready'}</h2>
            {#if current.structuringError}<div class="notice warning compact">Transcript succeeded; AI structuring failed: {current.structuringError}</div>{/if}
            {#if current.quick?.outputKind==='server'&&current.markdownPath}<code class="result-path">{current.markdownPath}</code>{/if}
            {#if current.quick?.sourceKind==='live'}<button class="btn compact" on:click={saveFinalAudio}>Save final audio…</button>{/if}
            <ReportEditor jobId={current.id} {notify}
              clinical={!!current.structuredProfileId&&/dental|medical|consultation/i.test(current.structuredProfileId)}
              onexport={async(format)=>{
              if(!(await exportResult(format)))throw Error('Export failed; check the notification.');
            }}/>
            <div class="result-export-panel">
              <strong>Export result</strong><span class="help">Download / save as</span>
              <div class="export-actions">
                {#each (simpleMode?(['pdf','md'] as ExportFormat[]):exportFormats) as format}
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
<PathPicker open={audioPicker} title="Choose final audio folder" mode="directory" initialPath={audioServerDir||outputDir} onselect={(path)=>{audioServerDir=path;audioPicker=false}} onclose={()=>audioPicker=false} />
