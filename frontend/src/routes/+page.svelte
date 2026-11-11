<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { getLiveRecorderController } from '$lib/live-recorder';
  import type { DashboardStats, Job, LlmProvider, Provider, StructureProfile, Workflow, Space, SpaceKind, SpaceMembership } from '$lib/types';
  import AppShell from '$lib/components/AppShell.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import type { Toast } from '$lib/toast';
  import DashboardView from '$lib/views/DashboardView.svelte';
  import QuickTranscribeView from '$lib/views/QuickTranscribeView.svelte';
  import LiveRecordView from '$lib/views/LiveRecordView.svelte';
  import WorkflowsView from '$lib/views/WorkflowsView.svelte';
  import SettingsView from '$lib/views/SettingsView.svelte';
  import JobsView from '$lib/views/JobsView.svelte';

  import {navigationAllows,normalizeMode,normalizeView,type UiMode,type View} from '$lib/ui-mode';
  getLiveRecorderController();
  const emptyStats:DashboardStats={workflows:0,enabledWorkflows:0,activeJobs:0,completedJobs:0,failedJobs:0};
  let active:View='home';
  let uiMode:UiMode='simple';
  let stats=emptyStats;
  let providers:Provider[]=[];
  let llmProviders:LlmProvider[]=[];
  let structureProfiles:StructureProfile[]=[];
  let workflows:Workflow[]=[];
  let jobs:Job[]=[];
  let spaces:Space[]=[];
  let assignments:SpaceMembership[]=[];
  let selectedSpace='all';
  let searchQuery='';
  let loading=true; let initialError=''; let toasts:Toast[]=[]; let toastSeq=0;

  function notify(type:Toast['type'],message:string){const id=++toastSeq;toasts=[...toasts,{id,type,message}];setTimeout(()=>toasts=toasts.filter(t=>t.id!==id),4500);}
  function nav(view:View|'providers'|'ai'){
    const target=view==='providers'||view==='ai'?'settings':view;
    active=navigationAllows(uiMode,target)?target:'live';
    try{localStorage.setItem('scribewatch:view',active);}catch{/* storage unavailable */}
  }
  function changeUiMode(next:UiMode){
    uiMode=next;
    try{localStorage.setItem('scribewatch:uiMode',next);}catch{/* storage unavailable */}
    if(!navigationAllows(uiMode,active))nav('live');
  }
  function chooseSpace(id:string){
    selectedSpace=id;
    try{localStorage.setItem('scribewatch:space',id);}catch{/* storage unavailable */}
  }
  async function createSpace(name:string,kind:SpaceKind){
    try{
      const created=await api.createSpace(name,kind);
      spaces=[...spaces,created];
      chooseSpace(created.id);
      nav('jobs');
      notify('success','Space created.');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));throw error;}
  }
  async function deleteSpace(id:string){
    try{
      await api.deleteSpace(id);
      spaces=spaces.filter(space=>space.id!==id);
      assignments=assignments.filter(item=>item.spaceId!==id);
      chooseSpace('unassigned');
      notify('success','Space removed. Recordings kept.');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
  }
  async function assignJob(jobId:string,spaceId:string|null){
    try{
      await api.assignJobSpace(jobId,spaceId);
      assignments=[...assignments.filter(item=>item.jobId!==jobId),...(spaceId?[{jobId,spaceId}]:[])];
      notify('success','Recording space saved.');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));throw error;}
  }

  async function loadAll(initial=false){
    if(initial)loading=true;
    try{
      [stats,providers,llmProviders,structureProfiles,workflows,jobs,spaces,assignments]=await Promise.all([
        api.dashboard(),api.providers(),api.llmProviders(),api.structureProfiles(),
        api.workflows(),api.jobs(),api.spaces(),api.spaceAssignments()
      ]);
      if(selectedSpace!=='all'&&selectedSpace!=='unassigned'&&!spaces.some(space=>space.id===selectedSpace))
        chooseSpace('unassigned');
      initialError='';
    }catch(e){initialError=e instanceof Error?e.message:String(e);}
    finally{loading=false;}
  }
  async function refreshJobs(){
    try{[stats,jobs,assignments]=await Promise.all([api.dashboard(),api.jobs(),api.spaceAssignments()]);}
    catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
  onMount(()=>{
    try{selectedSpace=localStorage.getItem('scribewatch:space')??'all';}catch{/* storage unavailable */}
    try{
      uiMode=normalizeMode(localStorage.getItem('scribewatch:uiMode'));
      active=normalizeView(localStorage.getItem('scribewatch:view')??'home',uiMode);
    }catch{uiMode='simple';active='home';}
    loadAll(true);
    let timer:ReturnType<typeof setTimeout>|undefined;
    const events=new EventSource('/api/v1/events');
    events.addEventListener('job',()=>{if(timer)clearTimeout(timer);timer=setTimeout(refreshJobs,120);});
    events.onerror=()=>{};
    return()=>{events.close();if(timer)clearTimeout(timer);};
  });
</script>

<svelte:head>
  <title>ScribeWatch — Audio to structured notes</title>
  <meta name="description" content="Self-hosted audio transcription, browser recording and optional AI-structured notes with a canonical transcript."/>
  <meta name="theme-color" content="#09100f"/>
</svelte:head>
<AppShell {active} {uiMode} onmode={changeUiMode} onnav={nav} {spaces} {jobs} {assignments} {selectedSpace}
  onspace={chooseSpace} oncreateSpace={createSpace} ondeleteSpace={deleteSpace} onlocale={loadAll}
  {searchQuery} onsearch={(value)=>searchQuery=value}>
  {#if !initialError&&!loading}
    <!-- Keep REC mounted while navigating so a completed recording is still exported. -->
    <div style:display={active==='live'?'contents':'none'}>
      <LiveRecordView simpleMode={uiMode==='simple'} {providers} {structureProfiles} {jobs} {selectedSpace} onassign={assignJob} refresh={refreshJobs} {notify}/>
    </div>
  {/if}
  {#if initialError}<div class="page"><div class="notice danger"><strong>Cannot reach ScribeWatch.</strong><span>{initialError}</span><button class="btn" on:click={()=>loadAll(true)}>Retry</button></div></div>
  {:else if loading}<div class="page"><div class="empty loading-state">Loading ScribeWatch…</div></div>
  {:else if active==='home'}<DashboardView {stats} {jobs} {workflows} onnav={nav}/>
  {:else if active==='quick'}<QuickTranscribeView {providers} {structureProfiles} {jobs} {selectedSpace} onassign={assignJob} refresh={refreshJobs} {notify}/>
  {:else if active==='live'}<!-- REC is rendered persistently above. -->
  {:else if active==='workflows'}<WorkflowsView {workflows} {providers} {structureProfiles} refresh={loadAll} {notify}/>
  {:else if active==='settings'}<SettingsView {uiMode} onmode={changeUiMode} {providers} {llmProviders} {structureProfiles} refresh={loadAll} {notify}/>
  {:else}<JobsView {jobs} {workflows} {spaces} {assignments} {selectedSpace} {searchQuery}
      onspace={chooseSpace} onassign={assignJob} oncreateSpace={createSpace} ondeleteSpace={deleteSpace}
      refresh={refreshJobs} {notify}/>{/if}
</AppShell>
<ToastHost {toasts}/>
