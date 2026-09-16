<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { DashboardStats, Job, Provider, Workflow } from '$lib/types';
  import AppShell from '$lib/components/AppShell.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import type { Toast } from '$lib/toast';
  import DashboardView from '$lib/views/DashboardView.svelte';
  import QuickTranscribeView from '$lib/views/QuickTranscribeView.svelte';
  import WorkflowsView from '$lib/views/WorkflowsView.svelte';
  import ProvidersView from '$lib/views/ProvidersView.svelte';
  import JobsView from '$lib/views/JobsView.svelte';

  type View='home'|'quick'|'workflows'|'providers'|'jobs';
  const emptyStats:DashboardStats={workflows:0,enabledWorkflows:0,activeJobs:0,completedJobs:0,failedJobs:0};
  let active:View='home';
  let stats=emptyStats; let providers:Provider[]=[]; let workflows:Workflow[]=[]; let jobs:Job[]=[];
  let loading=true; let initialError=''; let toasts:Toast[]=[]; let toastSeq=0;

  function notify(type:Toast['type'],message:string){const id=++toastSeq;toasts=[...toasts,{id,type,message}];setTimeout(()=>toasts=toasts.filter(t=>t.id!==id),4500);}
  function nav(view:View){active=view;localStorage.setItem('scribewatch:view',view);}
  async function loadAll(initial=false){if(initial)loading=true;try{[stats,providers,workflows,jobs]=await Promise.all([api.dashboard(),api.providers(),api.workflows(),api.jobs()]);initialError='';}catch(e){initialError=e instanceof Error?e.message:String(e);}finally{loading=false;}}
  async function refreshJobs(){try{[stats,jobs]=await Promise.all([api.dashboard(),api.jobs()]);}catch(e){notify('error',e instanceof Error?e.message:String(e));}}
  onMount(()=>{
    const saved=localStorage.getItem('scribewatch:view');
    if(saved==='dashboard') active='home';
    else if(saved&&['home','quick','workflows','providers','jobs'].includes(saved)) active=saved as View;
    loadAll(true);
    let timer:ReturnType<typeof setTimeout>|undefined;
    const events=new EventSource('/api/v1/events');
    events.addEventListener('job',()=>{if(timer)clearTimeout(timer);timer=setTimeout(refreshJobs,120);});
    events.onerror=()=>{};
    return()=>{events.close();if(timer)clearTimeout(timer);};
  });
</script>

<svelte:head>
  <title>ScribeWatch — Voice notes to Markdown</title>
  <meta name="description" content="Self-hosted voice-note and audio transcription to titled, Obsidian-ready Markdown."/>
  <meta name="theme-color" content="#09100f"/>
</svelte:head>
<AppShell {active} onnav={nav}>
  {#if initialError}<div class="page"><div class="notice danger"><strong>Cannot reach ScribeWatch.</strong><span>{initialError}</span><button class="btn" on:click={()=>loadAll(true)}>Retry</button></div></div>
  {:else if loading}<div class="page"><div class="empty loading-state">Loading ScribeWatch…</div></div>
  {:else if active==='home'}<DashboardView {stats} {jobs} {workflows} onnav={nav}/>
  {:else if active==='quick'}<QuickTranscribeView {providers} {jobs} refresh={refreshJobs} {notify}/>
  {:else if active==='workflows'}<WorkflowsView {workflows} {providers} refresh={loadAll} {notify}/>
  {:else if active==='providers'}<ProvidersView {providers} refresh={loadAll} {notify}/>
  {:else}<JobsView {jobs} {workflows} refresh={refreshJobs} {notify}/>{/if}
</AppShell>
<ToastHost {toasts}/>
