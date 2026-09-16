<script lang="ts">
  import type { DashboardStats, Job, Workflow } from '$lib/types';
  export let stats:DashboardStats={workflows:0,enabledWorkflows:0,activeJobs:0,completedJobs:0,failedJobs:0};
  export let jobs:Job[]=[];
  export let workflows:Workflow[]=[];
  export let onnav:(view:'quick'|'workflows'|'providers'|'jobs')=>void=()=>{};
  const terminal=new Set(['done','error','cancelled','interrupted']);
  $: activeJobs=jobs.filter((job)=>!terminal.has(job.status));
  $: recent=jobs.filter((job)=>job.status==='done').slice().sort((a,b)=>b.updatedAtMs-a.updatedAtMs).slice(0,5);
  const workflowName=(id?:string)=>workflows.find((w)=>w.id===id)?.name??'Workflow';
  const when=(ms:number)=>new Date(ms).toLocaleString();
</script>

<div class="page home-page">
  <section class="home-hero">
    <div class="hero-copy">
      <div class="eyebrow"><span class="live-dot"></span> SELF-HOSTED AUDIO NOTES</div>
      <h1>Turn voice notes into Markdown before you forget them.</h1>
      <p>ScribeWatch listens to your folders — or one file right now — and turns audio into titled, Obsidian-ready notes using your own transcription provider.</p>
      <div class="hero-actions"><button class="btn primary hero-btn" on:click={()=>onnav('quick')}>✦ Quick Transcribe</button><button class="btn hero-btn" on:click={()=>onnav('workflows')}>Set up a watch folder</button></div>
      <div class="hero-proof"><span>✓ No LLM required</span><span>✓ OpenAI-compatible</span><span>✓ Original audio stays safe</span></div>
    </div>
    <div class="hero-visual" aria-hidden="true">
      <div class="voice-card"><div class="wave-mini"><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div><span>voice-note.m4a</span></div>
      <div class="hero-arrow">→</div>
      <div class="note-preview"><span class="md-badge">MD</span><strong>Remember to book the train…</strong><small># voice-note · # transcription</small><div class="note-lines"><i></i><i></i><i></i></div></div>
    </div>
  </section>
  <div class="kpi-row home-kpis">
    <div class="kpi"><span>WATCHING</span><strong>{stats.enabledWorkflows}</strong><small>active workflows</small></div>
    <div class="kpi"><span>NOW</span><strong>{stats.activeJobs}</strong><small>processing</small></div>
    <div class="kpi"><span>NOTES</span><strong>{stats.completedJobs}</strong><small>completed</small></div>
    <div class="kpi"><span>ATTENTION</span><strong>{stats.failedJobs}</strong><small>failed</small></div>
  </div>
  <div class="grid two dashboard-grid">
    <section class="card">
      <div class="card-header"><div><strong>What’s happening</strong><span class="card-sub">Live transcription activity</span></div><button class="btn ghost" on:click={()=>onnav('jobs')}>All jobs →</button></div>
      <div class="card-body stack">
        {#if activeJobs.length===0}<div class="empty compact"><strong>Quiet right now.</strong><br/>Drop a voice note into Quick Transcribe or an enabled watch folder.</div>
        {:else}{#each activeJobs as job}<div class="activity-row"><span class="status-dot-large {job.status}"></span><div class="grow"><strong>{job.originalName}</strong><span>{job.kind==='quick'?'Quick Transcribe':workflowName(job.workflowId)}</span></div><span class="status-pill {job.status}">{job.status}</span></div>{/each}{/if}
      </div>
    </section>
    <section class="card">
      <div class="card-header"><div><strong>Recent notes</strong><span class="card-sub">Latest Markdown published</span></div></div>
      <div class="card-body stack">
        {#if recent.length===0}<div class="empty compact">Your first finished note will appear here.</div>
        {:else}{#each recent as job}<div class="recent-note"><span class="md-mini">MD</span><div class="grow"><strong>{job.quick?.resultName??job.markdownPath?.split('/').pop()??job.originalName}</strong><span>{job.kind==='quick'?'Quick Transcribe':workflowName(job.workflowId)} · {when(job.updatedAtMs)}</span></div></div>{/each}{/if}
      </div>
    </section>
  </div>
</div>
