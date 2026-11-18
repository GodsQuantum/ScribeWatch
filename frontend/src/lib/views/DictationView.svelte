<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { chooseRecorderMime, recordedFile } from '$lib/dictation';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { Provider, StructureProfile, TranscriptionRoute } from '$lib/types';

  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  let routes:TranscriptionRoute[]=[{providerId:'',model:''}];
  let initialized=false;
  let language='fr';
  let profileId='';
  let recording=false;
  let processing=false;
  let structuring=false;
  let transcript='';
  let corrected='';
  let elapsed=0;
  let model='';
  let recorder:MediaRecorder|null=null;
  let stream:MediaStream|null=null;
  let chunks:Blob[]=[];
  let disposed=false;

  $: if(!initialized&&providers.length){
    const preferred=providers.find(p=>p.enabled&&/parakeet.ultra/i.test(p.model))
      ??providers.find(p=>p.enabled)??providers[0];
    routes=[{providerId:preferred.id,model:''}];
    initialized=true;
  }
  function releaseMicrophone(){
    stream?.getTracks().forEach(track=>track.stop());
    stream=null;
  }
  async function start(){
    if(recording||processing||structuring||routes.some(r=>!r.providerId))return;
    if(!navigator.mediaDevices?.getUserMedia||typeof MediaRecorder==='undefined'){
      notify('error','Microphone indisponible dans ce navigateur.');
      return;
    }
    try{
      stream=await navigator.mediaDevices.getUserMedia({
        audio:{echoCancellation:true,noiseSuppression:true,autoGainControl:true}
      });
      if(disposed){releaseMicrophone();return;}
      const mime=chooseRecorderMime(type=>MediaRecorder.isTypeSupported(type));
      recorder=new MediaRecorder(stream,mime?{mimeType:mime}:undefined);
      chunks=[];
      recorder.ondataavailable=event=>{if(event.data.size)chunks.push(event.data);};
      recorder.onerror=()=>{
        releaseMicrophone();
        recording=false;
        notify('error',"Erreur d'enregistrement du microphone.");
      };
      recorder.onstop=()=>{
        releaseMicrophone();
        recording=false;
        if(!disposed)void transcribe();
      };
      recorder.start();
      recording=true;
      transcript='';corrected='';model='';elapsed=0;
    }catch(error){
      releaseMicrophone();
      recorder=null;
      notify('error',error instanceof Error?error.message:String(error));
    }
  }
  function stop(){
    if(recorder?.state==='recording')recorder.stop();
  }
  async function transcribe(){
    processing=true;
    try{
      const file=recordedFile(chunks,recorder?.mimeType??'');
      const result=await api.dictationTranscribe(file,{transcriptionChain:routes,language});
      if(disposed)return;
      transcript=result.text;
      elapsed=result.elapsedMs;
      model=result.model;
      notify('success','Dictée transcrite.');
      if(profileId)await applyProfile();
    }catch(error){
      if(!disposed)notify('error',error instanceof Error?error.message:String(error));
    }finally{
      chunks=[];recorder=null;processing=false;
    }
  }
  async function applyProfile(){
    if(!profileId||!transcript.trim()||structuring)return;
    structuring=true;
    try{
      const result=await api.structureExisting(profileId,transcript);
      if(!disposed)corrected=result.markdown;
    }catch(error){
      if(!disposed)notify('error',error instanceof Error?error.message:String(error));
    }finally{structuring=false;}
  }
  async function copy(content:string){
    try{
      await navigator.clipboard.writeText(content);
      notify('success','Texte copié, prêt à coller dans votre application.');
    }catch{
      notify('error','Copie indisponible : sélectionnez le texte manuellement.');
    }
  }
  onMount(()=>{
    disposed=false;
    function shortcut(event:KeyboardEvent){
      if(event.ctrlKey&&event.shiftKey&&event.code==='KeyD'
        &&!(event.target instanceof HTMLInputElement)
        &&!(event.target instanceof HTMLTextAreaElement)){
        event.preventDefault();
        if(recording)stop();else if(!processing)void start();
      }
    }
    window.addEventListener('keydown',shortcut);
    return()=>{
      disposed=true;
      window.removeEventListener('keydown',shortcut);
      if(recorder?.state==='recording'){
        recorder.onstop=null;
        recorder.stop();
      }
      releaseMicrophone();
    };
  });
</script>

<div class="page quick-page">
  <div class="page-head">
    <div>
      <p class="page-kicker">DICTÉE · LOCAL OU DISTANT</p>
      <h1 class="page-title">Dictée</h1>
      <p class="page-copy">Enregistrez quelques secondes, puis copiez le texte. Aucun enregistrement ni job permanent.</p>
    </div>
  </div>
  <div class="quick-layout">
    <section class="card quick-card">
      <div class="card-body stack">
        <strong>1. Moteur de transcription et repli</strong>
        <TranscriptionChainEditor value={routes} {providers} {notify} onchange={next=>routes=next}/>
        <div class="grid two">
          <div class="field">
            <label for="dictation-language">Langue</label>
            <select id="dictation-language" class="select" bind:value={language}>
              <option value="fr">Français</option>
              <option value="en">English</option>
              <option value="auto">Automatique</option>
            </select>
          </div>
          <div class="field">
            <label for="dictation-profile">Correction / mise en page IA (facultative)</label>
            <select id="dictation-profile" class="select" bind:value={profileId}>
              <option value="">Texte original, sans LLM</option>
              {#each structureProfiles.filter(profile=>profile.providerId) as profile}
                <option value={profile.id}>{profile.name}</option>
              {/each}
            </select>
          </div>
        </div>
        <strong>2. Microphone</strong>
        <p class="help">Ctrl+Maj+D dans cette fenêtre. Le raccourci système global nécessite une extension du lanceur natif.</p>
        {#if recording}
          <button class="btn primary" on:click={stop}>■ Arrêter et transcrire</button>
          <span class="help">Enregistrement en cours…</span>
        {:else}
          <button class="btn primary" disabled={processing||structuring||!routes[0]?.providerId} on:click={start}>
            {processing?'Transcription en cours…':structuring?'Traitement IA…':'● Commencer la dictée'}
          </button>
        {/if}
      </div>
    </section>
    <aside class="card result-card">
      <div class="card-header">
        <strong>3. Texte</strong>
        {#if model}<span class="help">{model} · {(elapsed/1000).toFixed(1)} s</span>{/if}
      </div>
      <div class="card-body stack">
        <label for="dictation-original">Transcription originale</label>
        <textarea id="dictation-original" class="input" rows="9" bind:value={transcript}
          placeholder="Le texte apparaît ici après l'arrêt."></textarea>
        <button class="btn" disabled={!transcript} on:click={()=>copy(transcript)}>Copier la transcription</button>
        {#if corrected}
          <label for="dictation-corrected">Version IA (original conservé)</label>
          <textarea id="dictation-corrected" class="input" rows="9" bind:value={corrected}></textarea>
          <button class="btn primary" on:click={()=>copy(corrected)}>Copier le texte IA</button>
        {:else if transcript&&profileId}
          <button class="btn" disabled={structuring} on:click={applyProfile}>
            {structuring?'Traitement IA…':'Appliquer le profil IA'}
          </button>
        {/if}
        <p class="help">La conversion audio→texte et le LLM utilisent vos fournisseurs ScribeWatch existants.</p>
      </div>
    </aside>
  </div>
</div>
