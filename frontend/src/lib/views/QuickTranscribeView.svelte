<script lang="ts">
  import { api } from '$lib/api';
  import { canSaveToDirectory, saveMarkdownLocally } from '$lib/local-save';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import type { Job, Provider, QuickOptions } from '$lib/types';
  export let providers:Provider[]=[];
  export let jobs:Job[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  let sourceMode:'computer'|'server'='computer';
  let outputMode:'computer'|'server'='computer';
  let localFile:File|null=null;
  let serverFile='';
  let outputDir='';
  let providerId='';
  let model='';
  let language='';
  let frontmatter=true;
  let advanced=false;
  let picker:''|'source'|'output'='';
  let busy=false;
  let submitted:Job|undefined;
  let drag=false;

  $: if(!providerId && providers.length) providerId=providers.find((p)=>p.enabled)?.id??providers[0].id;
  $: current=submitted ? (jobs.find((job)=>job.id===submitted?.id)??submitted) : undefined;
  $: sourceReady=sourceMode==='computer' ? !!localFile : !!serverFile;
  $: outputReady=outputMode==='computer' || !!outputDir;
  $: canSubmit=sourceReady&&outputReady&&!!providerId&&!busy;
  $: localDirectoryAvailable=typeof window!=='undefined'&&canSaveToDirectory(window);

  function chooseFile(file:File|undefined){ if(file){localFile=file;submitted=undefined;} }
  function dropped(event:DragEvent){ event.preventDefault();drag=false;chooseFile(event.dataTransfer?.files?.[0]); }
  async function submit(){
    if(!canSubmit)return;
    busy=true; submitted=undefined;
    const options:QuickOptions={
      providerId, model:model.trim()||undefined, language:language.trim()||undefined,
      outputKind:outputMode==='server'?'server':'client', outputDir:outputMode==='server'?outputDir:undefined,
      frontmatter
    };
    try{
      submitted=sourceMode==='computer'&&localFile
        ? await api.quickUpload(localFile,options)
        : await api.quickServer(serverFile,options);
      await refresh();
      notify('success','Transcription queued.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{busy=false;}
  }
  async function saveResult(){
    if(!current||current.status!=='done')return;
    try{
      const result=await api.jobMarkdown(current.id);
      const method=await saveMarkdownLocally(result.filename,result.blob);
      notify('success',method==='directory'?'Markdown saved to your folder.':'Markdown downloaded.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
  function reset(){localFile=null;serverFile='';submitted=undefined;}
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
        <div class="segmented source-tabs"><button class:active={sourceMode==='computer'} on:click={()=>{sourceMode='computer';submitted=undefined}}>This computer</button><button class:active={sourceMode==='server'} on:click={()=>{sourceMode='server';submitted=undefined}}>Server</button></div>
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

        <div class="quick-step"><span>02</span><div><strong>Transcription</strong><p>Use any configured OpenAI-compatible transcription provider.</p></div></div>
        <div class="grid two">
          <div class="field"><label for="quick-provider">Provider</label><select id="quick-provider" class="select" bind:value={providerId}><option value="">Choose provider</option>{#each providers.filter((p)=>p.enabled) as provider}<option value={provider.id}>{provider.name}</option>{/each}</select></div>
          <div class="field"><label for="quick-language">Language</label><input id="quick-language" class="input" bind:value={language} placeholder="auto, fr, en…" /></div>
        </div>
        <button class="advanced-toggle" on:click={()=>advanced=!advanced} aria-expanded={advanced}>{advanced?'−':'+'} Advanced</button>
        {#if advanced}<div class="grid two advanced-panel"><div class="field"><label for="quick-model">Model override</label><input id="quick-model" class="input" bind:value={model} placeholder="Provider default" /></div><label class="check"><input type="checkbox" bind:checked={frontmatter} /> YAML / Obsidian properties</label></div>{/if}

        <div class="quick-step"><span>03</span><div><strong>Save the note</strong><p>Publish on the server or bring the Markdown back to this computer.</p></div></div>
        <div class="segmented source-tabs"><button class:active={outputMode==='computer'} on:click={()=>outputMode='computer'}>This computer</button><button class:active={outputMode==='server'} on:click={()=>outputMode='server'}>Server folder</button></div>
        {#if outputMode==='server'}
          <div class="field"><label for="quick-output-dir">Markdown destination</label><div class="path-control"><input id="quick-output-dir" class="input mono" value={outputDir} readonly placeholder="Choose a notes folder" /><button class="btn" on:click={()=>picker='output'}>Browse</button></div></div>
        {:else}
          <div class="client-output-note"><span>⌁</span><div><strong>{localDirectoryAvailable?'Save directly into a local folder':'Download the .md file'}</strong><p>{localDirectoryAvailable?'After transcription, choose the local folder from the Save button.':'This HTTP/browser context cannot write directly to folders; the Markdown will download normally.'}</p></div></div>
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
            {#if current.quick?.outputKind==='client'}<button class="btn primary" on:click={saveResult}>{localDirectoryAvailable?'Save Markdown…':'Download Markdown'}</button>
            {:else if current.markdownPath}<code class="result-path">{current.markdownPath}</code>{/if}
            <button class="btn ghost" on:click={reset}>Transcribe another</button>
          </div>
        {:else}<div class="processing-state"><div class="pulse-ring"></div><strong>{current.status==='transcribing'?'Listening…':current.status==='publishing'?'Writing Markdown…':'Queued…'}</strong><span>{current.originalName}</span></div>{/if}
      </div>
    </aside>
  </div>
</div>
<PathPicker open={picker==='source'} title="Choose server audio" mode="file" extensions={audioExtensions} onselect={(path)=>{serverFile=path;picker=''}} onclose={()=>picker=''} />
<PathPicker open={picker==='output'} title="Choose Markdown folder" mode="directory" initialPath={outputDir} onselect={(path)=>{outputDir=path;picker=''}} onclose={()=>picker=''} />
