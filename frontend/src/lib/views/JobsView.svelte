<script lang="ts">
  import { api } from '$lib/api';
  import { saveMarkdownLocally } from '$lib/local-save';
  import type { Job, Workflow } from '$lib/types';
  export let jobs:Job[]=[];
  export let workflows:Workflow[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};
  let filter='all';
  const workflowName=(id?:string)=>workflows.find((w)=>w.id===id)?.name??'Deleted workflow';
  const terminal=new Set(['done','error','cancelled','interrupted']);
  $: sorted=jobs.slice().sort((a,b)=>b.updatedAtMs-a.updatedAtMs);
  $: visible=filter==='all'?sorted:filter==='active'?sorted.filter((j)=>!terminal.has(j.status)):filter==='failed'?sorted.filter((j)=>j.status==='error'||j.status==='interrupted'):sorted.filter((j)=>j.status==='done');
  async function retry(id:string){try{await api.retryJob(id);await refresh();notify('success','Job queued for retry.');}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function cancel(id:string){try{await api.cancelJob(id);await refresh();notify('success','Cancellation requested.');}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function remove(id:string){if(!confirm('Delete this history record? Media files are not deleted.'))return;try{await api.deleteJob(id);await refresh();}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  async function download(id:string){try{const result=await api.jobMarkdown(id);await saveMarkdownLocally(result.filename,result.blob);notify('success','Markdown saved.');}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  const when=(ms:number)=>new Date(ms).toLocaleString();
  const size=(bytes:number)=>bytes<1024*1024?`${Math.max(1,Math.round(bytes/1024))} KB`:`${(bytes/1024/1024).toFixed(1)} MB`;
</script>

<div class="page">
  <div class="page-head"><div><p class="page-kicker">HISTORY</p><h1 class="page-title">Jobs</h1><p class="page-copy">Every automatic and one-off transcription keeps a durable lifecycle, including retries and restart recovery.</p></div><div class="segmented"><button class:active={filter==='all'} on:click={()=>filter='all'}>All</button><button class:active={filter==='active'} on:click={()=>filter='active'}>Current</button><button class:active={filter==='failed'} on:click={()=>filter='failed'}>Errors</button><button class:active={filter==='done'} on:click={()=>filter='done'}>Done</button></div></div>
  <div class="job-list">
    {#if visible.length===0}<div class="empty">No jobs in this view.</div>{/if}
    {#each visible as job}
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
        <div class="job-footer">
          <span class="muted small">Attempt {job.attempts} · model {job.model}{job.language?` · ${job.language}`:' · auto language'}</span>
          <div class="row wrap">
            {#if job.kind==='quick'&&job.status==='done'&&job.quick?.outputKind==='client'}<button class="btn primary" on:click={()=>download(job.id)}>Save Markdown</button>{/if}
            {#if job.status==='error'||job.status==='interrupted'||job.status==='cancelled'}<button class="btn primary" on:click={()=>retry(job.id)}>Retry</button>{/if}
            {#if !terminal.has(job.status)}<button class="btn danger" on:click={()=>cancel(job.id)}>Cancel</button>{/if}
            {#if terminal.has(job.status)}<button class="btn ghost" on:click={()=>remove(job.id)}>Remove record</button>{/if}
          </div>
        </div>
      </article>
    {/each}
  </div>
</div>
