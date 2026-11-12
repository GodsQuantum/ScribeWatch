<script lang="ts">
  import { api } from '$lib/api';
  import ReportEditor from '$lib/components/ReportEditor.svelte';
  import {buildSavedClinicalPdf} from '$lib/clinical-report-export';
  import { canSaveToDirectory, saveResponseToDirectory } from '$lib/local-save';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import TranscriptionChainEditor from '$lib/components/TranscriptionChainEditor.svelte';
  import type { ExportFormat, Job, Provider, QuickOptions, StructureProfile, TranscriptionRoute } from '$lib/types';
  export let providers:Provider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let jobs:Job[]=[];
  export let selectedSpace='all';
  export let onassign:(jobId:string,spaceId:string|null)=>Promise<void>=async()=>{};

  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  let sourceMode:'computer'|'server'='computer';
  let outputMode:'computer'|'server'='computer';
  let markdownDestination:FileSystemDirectoryHandle|undefined;
  let markdownDestinationLabel='';
  async function chooseMarkdownFolder(){
    if(!localDirectoryAvailable)return;
    try{
      markdownDestination=await (window as any).showDirectoryPicker({mode:'readwrite'}) as FileSystemDirectoryHandle;
      markdownDestinationLabel=markdownDestination?.name??'';
    }catch(e){if(!(e instanceof DOMException&&e.name==='AbortError'))notify('error',e instanceof Error?e.message:String(e));}
  }
  let localFile:File|null=null;
  let serverFile='';
  let outputDir='';
  let transcriptionChain:TranscriptionRoute[]=[{providerId:'',model:''}];
  let chainInitialized=false;
  let language='';
  let structureProfileId='';
  let frontmatter=true;
  let paragraphs=true;
  let advanced=false;
  let picker:''|'source'|'output'='';
  let busy=false;
  let exporting='';
  let submitted:Job|undefined;
  let drag=false;
  const exportFormats:ExportFormat[]=['md','txt','html','docx','odt','pdf'];

  $: if(!chainInitialized&&providers.length){
    transcriptionChain=[{providerId:providers.find((provider)=>provider.enabled&&/parakeet.ultra/i.test(provider.model))?.id??providers.find((provider)=>provider.enabled)?.id??providers[0].id,model:''}];
    chainInitialized=true;
  }
  $: current=submitted ? (jobs.find((job)=>job.id===submitted?.id)??submitted) : undefined;
  $: sourceReady=sourceMode==='computer' ? !!localFile : !!serverFile;
  $: outputReady=outputMode==='computer' || !!outputDir;
  $: chainReady=transcriptionChain.length>0&&transcriptionChain.every((route)=>!!route.providerId);
  $: canSubmit=sourceReady&&outputReady&&chainReady&&!busy;
  $: localDirectoryAvailable=typeof window!=='undefined'&&canSaveToDirectory(window);

  function chooseFile(file:File|undefined){ if(file){localFile=file;submitted=undefined;} }
  function dropped(event:DragEvent){ event.preventDefault();drag=false;chooseFile(event.dataTransfer?.files?.[0]); }
  async function submit(){
    if(!canSubmit)return;
    busy=true; submitted=undefined;
    const primary=transcriptionChain[0];
    const options:QuickOptions={
      providerId:primary?.providerId, model:primary?.model, transcriptionChain,
      language:language.trim()||undefined,
      outputKind:outputMode==='server'?'server':'client', outputDir:outputMode==='server'?outputDir:undefined,
      frontmatter, paragraphs, structureProfileId:structureProfileId||undefined
    };
    try{
      submitted=sourceMode==='computer'&&localFile
        ? await api.quickUpload(localFile,options)
        : await api.quickServer(serverFile,options);
      if(submitted && selectedSpace!=='all' && selectedSpace!=='unassigned'){
        try{await onassign(submitted.id,selectedSpace);}catch{/* job itself remains queued */}
      }
      await refresh();
      notify('success','Transcription queued.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{busy=false;}
  }
  async function exportResult(format:ExportFormat):Promise<boolean>{
    if(!current||current.status!=='done')return false;
    exporting=format;
    try{
      const clinical=!!current.structuredProfileId&&
        /dental|medical|consultation/i.test(current.structuredProfileId);
      if(format==='pdf'&&clinical&&current.structuredProfileName&&!current.structuringError){
        const [report,encounter]=await Promise.all([api.jobReport(current.id),api.jobEncounter(current.id)]);
        const result=await buildSavedClinicalPdf(current,report,encounter);
        const method=await saveResponseToDirectory(new Response(result.blob),result.filename,markdownDestination,window);
        notify('success',method==='directory'?'PDF clinique corrigé enregistré.':'PDF clinique téléchargé.');
      }else if(format==='pdf'){
        const saved=await api.jobReport(current.id);
        const {buildSavedNotePdf}=await import('$lib/note-pdf');
        const result=await buildSavedNotePdf(saved);
        const method=await saveResponseToDirectory(new Response(result.blob),result.filename,markdownDestination,window);
        notify('success',method==='directory'?'PDF corrigé enregistré.':'PDF corrigé téléchargé.');
      }else{
        const result=await api.jobExport(current.id,format);
        const method=await saveResponseToDirectory(new Response(result.blob),result.filename,markdownDestination,window);
        notify('success',method==='directory'?format.toUpperCase()+' saved to your folder.':format.toUpperCase()+' downloaded.');
      }
      return true;
    }catch(e){notify('error',e instanceof Error?e.message:String(e));return false;}
    finally{exporting='';}
  }
  function reset(){localFile=null;serverFile='';submitted=undefined;}

  // Reuse the genuine native structuring pipeline without performing a second
  // ASR pass. Patient transcripts and outputs only live in component memory.
  let importedTranscriptFile:File|null=null;
  let importedProfileId='';
  let importedBusy=false;
  let importedExporting:''|'md'|'pdf'='';
  let importedResult:{markdown:string;elapsedMs:number;profileId:string;model:string}|null=null;
  async function structureImportedTranscript(){
    if(!importedTranscriptFile||!importedProfileId||importedBusy)return;
    importedBusy=true;
    importedResult=null;
    try{
      const source=await importedTranscriptFile.text();
      if(source.length>250_000||source.length<50)throw Error('Transcription : entre 50 et 250 000 caractères.');
      importedResult=await api.structureExisting(importedProfileId,source);
      notify('success', 'Structuration locale terminée : '+(importedResult.elapsedMs/1000).toFixed(1)+' s.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{importedBusy=false;}
  }
  async function exportImportedTranscript(format:'md'|'pdf'){
    if(!importedResult||importedExporting)return;
    importedExporting=format;
    try{
      const {createMedicalPdf}=await import('$lib/medical-report');
      const content=format==='pdf'
        ?await createMedicalPdf(importedResult.markdown,{surname:'',forename:'',dateOfBirth:''})
        :new Blob([importedResult.markdown],{type:'text/markdown;charset=utf-8'});
      const filename='rapport-structure-'+new Date().toISOString().slice(0,10)+'.'+format;
      const method=await saveResponseToDirectory(new Response(content),filename,markdownDestination,window);
      notify('success',method==='directory'?'Rapport enregistré dans le dossier choisi.':'Rapport téléchargé.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{importedExporting='';}
  }
  const audioExtensions='mp3,wav,m4a,flac,ogg,opus,aac,wma,aiff,aif,caf,webm';
</script>

<div class="page quick-page">
  <div class="page-head">
    <div><p class="page-kicker">ONE FILE · RIGHT NOW</p><h1 class="page-title">Quick Transcribe</h1><p class="page-copy">Turn one recording into a clean Markdown note without creating a workflow.</p></div>
  </div>
  {#if providers.length===0}<div class="notice warning">Add a transcription provider first.</div>{/if}
  <div class="quick-layout">
    <section class="card quick-card">
      <div class="card-body stack">
        <div class="quick-step"><span>01</span><div><strong>Choose your audio</strong><p>Upload from this computer or use a file already mounted on the server.</p></div></div>
        <div class="segmented source-tabs"><button class:active={sourceMode==='computer'} on:click={()=>{sourceMode='computer';submitted=undefined}}>Choisir un audio depuis le navigateur</button><button class:active={sourceMode==='server'} on:click={()=>{sourceMode='server';submitted=undefined}}>Audio déjà présent dans ScribeWatch</button></div>
        {#if sourceMode==='computer'}
          <label class="drop-zone" class:drag on:dragover={(e)=>{e.preventDefault();drag=true}} on:dragleave={()=>drag=false} on:drop={dropped}>
            <input type="file" accept="audio/*,.m4a,.flac,.ogg,.opus,.aac,.wma,.aiff,.aif,.caf,.webm" on:change={(e)=>chooseFile(e.currentTarget.files?.[0])} />
            <span class="drop-icon">↥</span>
            {#if localFile}<strong>{localFile.name}</strong><span>{(localFile.size/1024/1024).toFixed(1)} MB · click to replace</span>
            {:else}<strong>Drop an audio note here</strong><span>or click to choose a file from this computer</span>{/if}
          </label>
        {:else}
          <div class="field"><label for="quick-server-file">Server audio file</label><div class="path-control"><input id="quick-server-file" class="input mono" value={serverFile} readonly placeholder="Choose an allowed server file" /><button class="btn" on:click={()=>picker='source'}>Browse</button></div><span class="help">Only explicitly mounted and allowed server roots are visible.</span></div>
        {/if}

        <div class="quick-step"><span>02</span><div><strong>Transcription</strong><p>Pick an exact provider + model chain. Failed routes automatically fall through.</p></div></div>
        <TranscriptionChainEditor value={transcriptionChain} {providers} {notify} onchange={(routes)=>transcriptionChain=routes} />
        <div class="grid two">
          <div class="field"><label for="quick-language">Language</label><input id="quick-language" class="input" bind:value={language} placeholder="auto, fr, en…" /></div>
          <div class="field"><label for="quick-structure">AI structure <span class="muted">optional</span></label><select id="quick-structure" class="select" bind:value={structureProfileId}><option value="">None — transcript only</option>{#each structureProfiles as profile}<option value={profile.id} disabled={!profile.providerId}>{profile.name}{profile.providerId?'':' — sans LLM'}</option>{/each}</select><span class="help">Adds a structured view and always preserves the full transcript.</span></div>
        </div>
        <button class="advanced-toggle" on:click={()=>advanced=!advanced} aria-expanded={advanced}>{advanced?'−':'+'} Advanced</button>
        {#if advanced}<div class="advanced-panel"><label class="check"><input type="checkbox" bind:checked={frontmatter} /> YAML / Obsidian properties</label><label class="check"><input type="checkbox" bind:checked={paragraphs} /> Readable deterministic paragraphs</label><span class="help">Uses Unicode sentence boundaries and length rules only. No LLM rewrites the transcript.</span></div>{/if}

        <div class="quick-step"><span>03</span><div><strong>Save the note</strong><p>Publish on the server or bring the Markdown back to this computer.</p></div></div>
        <div class="segmented source-tabs"><button class:active={outputMode==='computer'} on:click={()=>outputMode='computer'}>Dossier choisi via navigateur</button><button class:active={outputMode==='server'} on:click={()=>outputMode='server'}>Dossier géré par ScribeWatch</button></div>
        {#if outputMode==='server'}
          <div class="field"><label for="quick-output-dir">Markdown destination</label><div class="path-control"><input id="quick-output-dir" class="input mono" value={outputDir} readonly placeholder="Choose a notes folder" /><button class="btn" on:click={()=>picker='output'}>Browse</button></div></div>
        {:else}
          <div class="client-output-note"><span>⌁</span><div><strong>{markdownDestinationLabel||'Choisir un dossier avant la transcription'}</strong><p>Le résultat sera sauvegardé dans ce dossier au clic sur Export ; sinon téléchargement classique.</p></div>{#if localDirectoryAvailable}<button class="btn compact" on:click={chooseMarkdownFolder}>Choisir un dossier</button>{/if}</div>
        {/if}
        <button class="btn primary quick-submit" disabled={!canSubmit} on:click={submit}>{busy?'Uploading…':'Transcribe to Markdown →'}</button>
      </div>
    </section>

    <aside class="card result-card">
      <div class="card-header"><strong>Result</strong>{#if current}<span class="status-pill {current.status}">{current.status}</span>{/if}</div>
      <div class="card-body result-body">
        {#if !current}<div class="empty result-empty"><span>MD</span><strong>Your note appears here.</strong><p>The title comes from the first useful words of the transcription — ready for Obsidian, any Markdown vault, or plain files.</p></div>
        {:else if current.status==='error'||current.status==='interrupted'||current.status==='cancelled'}<div class="error-box"><strong>Transcription did not finish</strong><span>{current.error??current.status}</span></div>
        {:else if current.status==='done'}
          <div class="result-success"><span class="result-check">✓</span><h2>{current.quick?.resultName??'Markdown ready'}</h2><p>{current.originalName} was transcribed. Quick Transcribe never archives or deletes the original source.</p>
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
            <ReportEditor jobId={current.id} {notify}
              clinical={!!current.structuredProfileId&&/dental|medical|consultation/i.test(current.structuredProfileId)}
              onexport={async(format)=>{
                if(!(await exportResult(format)))throw Error('Export failed');
              }}/>
            <button class="btn ghost" on:click={reset}>Transcribe another</button>
          </div>
        {:else}<div class="processing-state"><div class="pulse-ring"></div><strong>{current.status==='transcribing'?'Listening…':current.status==='structuring'?'Structuring with AI…':current.status==='publishing'?'Writing Markdown…':'Queued…'}</strong><span>{current.originalName}</span></div>{/if}
      </div>
    </aside>
  </div>

  <section class="card">
    <div class="card-body stack">
      <strong>Structurer une transcription existante</strong>
      <p class="help">Pour un fichier Parakeet, Whisper ou tout autre ASR déjà produit : aucun second passage de transcription. Le compte rendu utilise directement le profil IA sélectionné et le moteur local. Les fichiers ne sont pas sauvegardés dans les préférences.</p>
      <div class="grid two">
        <div class="field">
          <label for="existing-asr-file">Transcription source (.md, .txt)</label>
          <input id="existing-asr-file" class="input" type="file" accept=".md,.txt,text/plain,text/markdown"
            on:change={(event)=>{importedTranscriptFile=event.currentTarget.files?.[0]??null;importedResult=null}}/>
        </div>
        <div class="field">
          <label for="existing-ai-profile">Profil IA</label>
          <select id="existing-ai-profile" class="select" bind:value={importedProfileId} on:change={()=>importedResult=null}>
            <option value="">Choisir un profil configuré</option>
            {#each structureProfiles.filter(profile=>profile.providerId) as profile}
              <option value={profile.id}>{profile.name}</option>
            {/each}
          </select>
        </div>
      </div>
      <button class="btn primary" disabled={!importedTranscriptFile||!importedProfileId||importedBusy}
        on:click={structureImportedTranscript}>{importedBusy?'Structuration par le moteur local…':'Créer le compte rendu depuis cette transcription'}</button>
      {#if importedResult}
        <div class="result-export-panel stack">
          <span>Compte rendu terminé en {(importedResult.elapsedMs/1000).toFixed(1)} secondes. Brouillon à valider avant usage clinique.</span>
          <div class="row wrap">
            <button class="btn compact" disabled={!!importedExporting} on:click={()=>exportImportedTranscript('md')}>Exporter Markdown</button>
            <button class="btn compact" disabled={!!importedExporting} on:click={()=>exportImportedTranscript('pdf')}>Exporter PDF</button>
          </div>
        </div>
      {/if}
    </div>
  </section>
</div>
<PathPicker open={picker==='source'} title="Choose server audio" mode="file" extensions={audioExtensions} onselect={(path)=>{serverFile=path;picker=''}} onclose={()=>picker=''} />
<PathPicker open={picker==='output'} title="Choose Markdown folder" mode="directory" initialPath={outputDir} onselect={(path)=>{outputDir=path;picker=''}} onclose={()=>picker=''} />
