<script lang="ts">
  import { api } from '$lib/api';
  import PathPicker from '$lib/components/PathPicker.svelte';
  import TagEditor from '$lib/components/TagEditor.svelte';
  import type { Provider, Workflow } from '$lib/types';
  export let workflows:Workflow[]=[];
  export let providers:Provider[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  const blank=():Workflow=>({
    id:'', name:'', watchDir:'', outputDir:undefined, archiveDir:'', tags:[],
    providerId:providers[0]?.id||'', model:'', language:undefined,
    markdown:{frontmatter:true,transcriptHeading:'Transcript'}, enabled:true
  });
  let selectedId='';
  let draft=blank();
  let busy=false;
  let picker:''|'watch'|'output'|'archive'='';

  function createNew(){ selectedId=''; draft=blank(); picker=''; }
  function select(workflow:Workflow){ selectedId=workflow.id; draft=structuredClone({...workflow,tags:workflow.tags??[]}); picker=''; }
  function choosePath(path:string){
    if(picker==='watch') draft.watchDir=path;
    if(picker==='output') draft.outputDir=path;
    if(picker==='archive') draft.archiveDir=path;
    picker='';
  }
  async function save(){
    busy=true;
    try {
      const payload={...draft,outputDir:draft.outputDir?.trim()||undefined,tags:draft.tags??[]};
      const saved=await api.saveWorkflow(payload); await refresh(); select(saved); notify('success','Workflow saved.');
    } catch(e){ notify('error',e instanceof Error?e.message:String(e)); }
    finally{ busy=false; }
  }
  async function scan(){ if(!selectedId)return; try{await api.scanWorkflow(selectedId);notify('success','Scan requested.');}catch(e){notify('error',e instanceof Error?e.message:String(e));} }
  async function remove(){
    if(!selectedId||!confirm('Delete this workflow? Existing history is kept.'))return;
    try{await api.deleteWorkflow(selectedId);createNew();await refresh();notify('success','Workflow deleted.');}
    catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
  $: pickerTitle=picker==='watch'?'Choose Watch folder':picker==='output'?'Choose Markdown folder':'Choose Audio archive';
  $: pickerInitial=picker==='watch'?draft.watchDir:picker==='output'?(draft.outputDir??draft.watchDir):draft.archiveDir;
</script>

<div class="page">
  <div class="page-head">
    <div><p class="page-kicker">AUTOMATION</p><h1 class="page-title">Workflows</h1><p class="page-copy">Watch audio continuously, publish the note where you keep knowledge, then archive the source only after publication succeeds.</p></div>
    <div class="page-actions"><button class="btn primary" on:click={createNew}>+ Add workflow</button></div>
  </div>
  {#if providers.length===0}<div class="notice warning">Create a transcription provider before enabling a workflow.</div>{/if}
  <div class="split-editor">
    <div class="list-pane">
      {#if workflows.length===0}<div class="empty">No workflows yet.</div>{/if}
      {#each workflows as workflow}
        <button class="list-item" class:active={workflow.id===selectedId} on:click={()=>select(workflow)}>
          <strong>{workflow.name}</strong><span>{workflow.enabled?'watching':'disabled'} · {workflow.watchDir}</span>
        </button>
      {/each}
    </div>
    <section class="card">
      <div class="card-header"><strong>{selectedId?'Edit workflow':'New workflow'}</strong><span class="status-pill {draft.enabled?'done':'interrupted'}">{draft.enabled?'enabled':'disabled'}</span></div>
      <div class="card-body stack">
        <div class="grid two">
          <div class="field"><label for="workflow-name">Name</label><input id="workflow-name" class="input" bind:value={draft.name} placeholder="Voice notes inbox" /></div>
          <div class="field"><label for="workflow-provider">Transcription provider</label><select id="workflow-provider" class="select" bind:value={draft.providerId}><option value="">Choose provider</option>{#each providers as provider}<option value={provider.id}>{provider.name}</option>{/each}</select></div>
        </div>
        <div class="folder-flow">
          <div class="field folder-step"><span class="step-no">1</span><label for="workflow-watch">Watch folder</label><div class="path-control"><input id="workflow-watch" class="input mono" bind:value={draft.watchDir} placeholder="/media/inbox" /><button class="btn" on:click={()=>picker='watch'}>Browse</button></div><span class="help">Stable audio files detected here are queued automatically.</span></div>
          <div class="flow-arrow">↓</div>
          <div class="field folder-step"><span class="step-no">2</span><label for="workflow-output">Markdown folder</label><div class="path-control"><input id="workflow-output" class="input mono" bind:value={draft.outputDir} placeholder="Same as Watch folder" /><button class="btn" on:click={()=>picker='output'}>Browse</button></div><span class="help">Point this at an Obsidian vault folder, or leave empty to publish beside the audio.</span></div>
          <div class="flow-arrow">↓</div>
          <div class="field folder-step"><span class="step-no">3</span><label for="workflow-archive">Audio archive</label><div class="path-control"><input id="workflow-archive" class="input mono" bind:value={draft.archiveDir} placeholder="/media/archive" /><button class="btn" on:click={()=>picker='archive'}>Browse</button></div><span class="help">The source moves here only after the Markdown note is safely published.</span></div>
        </div>
        <div class="grid two">
          <div class="field"><label for="workflow-model">Model override</label><input id="workflow-model" class="input" bind:value={draft.model} placeholder="Use provider default" /><span class="help">Leave empty to inherit the provider model.</span></div>
          <div class="field"><label for="workflow-language">Language</label><input id="workflow-language" class="input" bind:value={draft.language} placeholder="auto, fr, en…" /><span class="help">Empty or “auto” lets the provider detect it.</span></div>
        </div>
        <div class="subsection stack">
          <div><strong>Obsidian / Markdown</strong><p class="help">Every note gets a transcript-derived title. YAML properties stay optional.</p></div>
          <label class="check"><input type="checkbox" bind:checked={draft.markdown.frontmatter} /> Add YAML properties</label>
          <TagEditor value={draft.tags} onchange={(tags)=>draft={...draft,tags}} />
        </div>
        <label class="check"><input type="checkbox" bind:checked={draft.enabled} disabled={providers.length===0} /> Watch this folder continuously</label>
        <div class="form-actions">
          {#if selectedId}<button class="btn danger" on:click={remove}>Delete</button><button class="btn" on:click={scan}>Scan now</button>{/if}
          <span class="spacer"></span><button class="btn primary" disabled={busy||!draft.name||!draft.watchDir||!draft.archiveDir||!draft.providerId} on:click={save}>{busy?'Saving…':'Save workflow'}</button>
        </div>
      </div>
    </section>
  </div>
</div>
<PathPicker open={picker!==''} title={pickerTitle} initialPath={pickerInitial} mode="directory" onselect={choosePath} onclose={()=>picker=''} />
