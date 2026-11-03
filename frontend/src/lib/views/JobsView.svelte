<script lang="ts">
  import { api } from '$lib/api';
  import { saveMarkdownLocally } from '$lib/local-save';
  import type { ExportFormat, Job, TranscriptionAttempt, Workflow, Space, SpaceMembership, SpaceKind } from '$lib/types';
  export let jobs:Job[]=[];
  export let spaces:Space[]=[];
  export let assignments:SpaceMembership[]=[];
  export let selectedSpace='all';
  export let searchQuery='';
  export let onspace:(id:string)=>void=()=>{};
  export let onassign:(jobId:string,spaceId:string|null)=>Promise<void>=async()=>{};
  export let oncreateSpace:(name:string,kind:SpaceKind)=>Promise<void>=async()=>{};
  export let ondeleteSpace:(id:string)=>Promise<void>=async()=>{};
  let newSpaceName='';
  let newSpaceKind:SpaceKind='general';
  let addingSpace=false;
  let savingSpace=false;
  let shown=30;
  let lastListKey='';

  let assigningJob='';

  export let workflows:Workflow[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};
  let filter='all';
  let exporting='';
  const workflowName=(id?:string)=>workflows.find((w)=>w.id===id)?.name??'Deleted workflow';
  const terminal=new Set(['done','error','cancelled','interrupted']);
  $: membership=new Map(assignments.map(item=>[item.jobId,item.spaceId]));
  $: sorted=jobs.slice().sort((a,b)=>b.updatedAtMs-a.updatedAtMs);
  $: byStatus=filter==='all'?sorted:filter==='active'?sorted.filter((j)=>!terminal.has(j.status)):
    filter==='failed'?sorted.filter((j)=>j.status==='error'||j.status==='interrupted'):sorted.filter((j)=>j.status==='done');
  $: bySpace=selectedSpace==='all'?byStatus:byStatus.filter(job=>selectedSpace==='unassigned'
    ? !membership.has(job.id):membership.get(job.id)===selectedSpace);
  const normalized=(value:string)=>value.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLocaleLowerCase();
  $: visible=searchQuery.trim()?bySpace.filter(job=>{
    const query=normalized(searchQuery.trim());
    return normalized(job.originalName).includes(query)
      || normalized(workflowName(job.workflowId)).includes(query);
  }):bySpace;
  $: {
    const key=selectedSpace+'|'+filter+'|'+searchQuery;
    if(key!==lastListKey){lastListKey=key;shown=30;}
  }
  $: paged=visible.slice(0,shown);
  async function addSpace(){
    if(savingSpace||!newSpaceName.trim())return;
    savingSpace=true;
    try{await oncreateSpace(newSpaceName,newSpaceKind);newSpaceName='';addingSpace=false;}
    catch{/* parent reports validation/API error */}
    finally{savingSpace=false;}
  }
  async function removeSpace(){
    if(!spaces.some(space=>space.id===selectedSpace))return;
    if(!confirm('Remove this space? Recording history and documents are kept.'))return;
    await ondeleteSpace(selectedSpace);
  }
  async function assign(jobId:string,value:string){
    assigningJob=jobId;
    try{await onassign(jobId,value||null);}catch{/* parent already reported error */}
    finally{assigningJob='';}
  }
  async function retry(id:string){try{await api.retryJob(id);await refresh();notify('success','Job queued for retry.');}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function cancel(id:string){try{await api.cancelJob(id);await refresh();notify('success','Cancellation requested.');}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function remove(id:string){if(!confirm('Delete this history record? Media files are not deleted.'))return;try{await api.deleteJob(id);await refresh();}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function exportResult(id:string,format:ExportFormat){
    exporting=`${id}:${format}`;
    try{
      const result=await api.jobExport(id,format);
      await saveMarkdownLocally(result.filename,result.blob);
      notify('success',`${format.toUpperCase()} export saved.`);
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{exporting='';}
  }
  const when=(ms:number)=>new Date(ms).toLocaleString();
  const size=(bytes:number)=>bytes<1024*1024?`${Math.max(1,Math.round(bytes/1024))} KB`:`${(bytes/1024/1024).toFixed(1)} MB`;
  const elapsed=(attempt:TranscriptionAttempt)=>{
    const ms=Math.max(0,attempt.finishedAtMs-attempt.startedAtMs);
    return ms<1000?`${ms} ms`:ms<60_000?`${(ms/1000).toFixed(1)} s`:`${(ms/60_000).toFixed(1)} min`;
  };
</script>

<div class="page">
  <div class="page-head"><div><p class="page-kicker">HISTORY</p><h1 class="page-title">Jobs</h1><p class="page-copy">Every automatic and one-off transcription keeps a durable lifecycle, including retries, fallback attempts and restart recovery.</p></div><div class="segmented"><button class:active={filter==='all'} on:click={()=>filter='all'}>All</button><button class:active={filter==='active'} on:click={()=>filter='active'}>Current</button><button class:active={filter==='failed'} on:click={()=>filter='failed'}>Errors</button><button class:active={filter==='done'} on:click={()=>filter='done'}>Done</button></div></div>
  <div class="jobs-toolbar">
    <div class="field">
      <label class="field-label" for="jobs-space">Space</label>
      <select id="jobs-space" class="select" value={selectedSpace} on:change={(event)=>onspace(event.currentTarget.value)}>
        <option value="all">All recordings</option>
        <option value="unassigned">Inbox — unassigned</option>
        {#each spaces as space (space.id)}
          <option value={space.id}>{space.name}</option>
        {/each}
      </select>
    </div>
    <p class="small muted">{visible.length} of {jobs.length} recordings</p>
    <button class="btn" on:click={()=>addingSpace=!addingSpace}>+ Create space</button>
    {#if spaces.some(space=>space.id===selectedSpace)}
      <button class="btn danger" on:click={removeSpace}>Remove space</button>
    {/if}
  </div>
  {#if addingSpace}
    <form class="card space-create-inline" on:submit|preventDefault={addSpace}>
      <label for="new-job-space" class="field-label">Space name</label>
      <input id="new-job-space" class="input" maxlength="100" required bind:value={newSpaceName}
        placeholder="e.g. Team meeting, Project A"/>
      <label for="new-job-space-kind" class="field-label">Space type</label>
      <select id="new-job-space-kind" class="select" bind:value={newSpaceKind}>
        <option value="general">General</option><option value="project">Project</option>
        <option value="meeting">Meeting</option><option value="case">Case / dossier</option>
      </select>
      <div class="row"><button class="btn primary" disabled={savingSpace||!newSpaceName.trim()} type="submit">Create</button>
      <button class="btn ghost" on:click={()=>addingSpace=false} type="button">Cancel</button></div>
    </form>
  {/if}
  <div class="job-list">
    {#if visible.length===0}<div class="empty">No recordings here yet. Select another space or clear your search.</div>{/if}
    {#each paged as job (job.id)}
      <article class="job-card">
        <div class="job-top">
          <div class="job-title"><div class="row"><span class="job-kind {job.kind}">{job.kind==='quick'?'QUICK':'WATCH'}</span><strong>{job.originalName}</strong></div><span>{job.kind==='quick'?'Quick Transcribe':workflowName(job.workflowId)} · {size(job.sourceSize)} · {when(job.updatedAtMs)}</span></div>
          <span class="status-pill {job.status}">{job.status}</span>
        </div>
        <div class="job-paths">
          <div><span>Source</span><code>{job.sourcePath}</code></div>
          {#if job.markdownPath}<div><span>Markdown</span><code>{job.quick?.outputKind==='client'?(job.quick.resultName??'Client result'):job.markdownPath}</code></div>{/if}
          {#if job.archivePath}<div><span>Archive</span><code>{job.archivePath}</code></div>{/if}
        </div>
        {#if job.error}<div class="error-box"><strong>Processing error</strong><span>{job.error}</span></div>{/if}
        {#if job.structuredProfileName}<div class="notice compact"><span class="status-pill done">AI STRUCTURED</span><span>{job.structuredProfileName} · <code>{job.structuredModel}</code></span></div>{/if}
        {#if job.structuringError}<div class="notice warning compact"><strong>Transcript published without AI structure.</strong><span>{job.structuringError}</span></div>{/if}
        {#if (job.transcriptionAttempts??[]).length>0}
          <details class="attempt-details">
            <summary>Transcription attempts ({job.transcriptionAttempts.length})</summary>
            <div class="attempt-list">
              {#each job.transcriptionAttempts as attempt}
                <div class="attempt-row">
                  <span class="attempt-outcome {attempt.outcome}">{attempt.outcome.replace('_',' ')}</span>
                  <span>Run {attempt.run} · {attempt.providerName} / <code>{attempt.model}</code> · {elapsed(attempt)}</span>
                  {#if attempt.error}<span class="attempt-error">{attempt.error}</span>{/if}
                </div>
              {/each}
            </div>
          </details>
        {/if}
        <div class="job-footer">
          <div class="field">
            <label class="field-label" for={'job-space-'+job.id}>Space</label>
            <select id={'job-space-'+job.id} class="select space-job-select"
              value={membership.get(job.id)??''} disabled={assigningJob===job.id}
              on:change={(event)=>assign(job.id,event.currentTarget.value)}>
              <option value="">Inbox — unassigned</option>
              {#each spaces as space (space.id)}
                <option value={space.id}>{space.name}</option>
              {/each}
            </select>
          </div>
          <span class="muted small">{job.usedProviderName&&job.usedModel?`Run ${job.attempts} · used ${job.usedProviderName} / ${job.usedModel}`:`Run ${job.attempts} · model ${job.model}`}{job.language?` · ${job.language}`:' · auto language'}</span>
          <div class="row wrap">
            {#if job.status==='done'&&job.markdownPublished}
              <div class="export-actions" aria-label="Export result">
                {#each ['md','txt','html','docx','odt','pdf'] as format}
                  <button class="btn compact" disabled={!!exporting} on:click={()=>exportResult(job.id,format as ExportFormat)}>{exporting===`${job.id}:${format}`?'…':format.toUpperCase()}</button>
                {/each}
              </div>
            {/if}
            {#if job.status==='error'||job.status==='interrupted'||job.status==='cancelled'}<button class="btn primary" on:click={()=>retry(job.id)}>Retry</button>{/if}
            {#if !terminal.has(job.status)}<button class="btn danger" on:click={()=>cancel(job.id)}>Cancel</button>{/if}
            {#if terminal.has(job.status)}<button class="btn ghost" on:click={()=>remove(job.id)}>Remove record</button>{/if}
          </div>
        </div>
      </article>
    {/each}
    {#if paged.length<visible.length}
      <button class="btn jobs-more" on:click={()=>shown=Math.min(visible.length,shown+30)}>
        Show more recordings ({visible.length-paged.length} remaining)
      </button>
    {/if}
  </div>
</div>
