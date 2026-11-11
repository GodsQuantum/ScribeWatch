<script lang="ts">
  import ProvidersView from '$lib/views/ProvidersView.svelte';
  import AiProfilesView from '$lib/views/AiProfilesView.svelte';
  import type { LlmProvider, Provider, StructureProfile } from '$lib/types';
  import type { UiMode } from '$lib/ui-mode';

  export let uiMode:UiMode='simple';
  export let onmode:(mode:UiMode)=>void=()=>{};
  export let providers:Provider[]=[];
  export let llmProviders:LlmProvider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};
  let selected:'stt'|'ai'='stt';
  function openModelSetup(){selected='ai';onmode('advanced');}
  $: recommendedStt=providers.find(p=>p.enabled&&/parakeet.ultra/i.test(p.model))
    ??providers.find(p=>p.enabled);
  $: defaultDental=structureProfiles.find(p=>p.id==='dental-consultation-complete-fr');
</script>

<div class="page settings-page">
  <div class="page-head">
    <div>
      <p class="page-kicker">LOCAL-FIRST · SETTINGS</p>
      <h1 class="page-title">Settings</h1>
      <p class="page-copy">Simple mode uses the preferred configured models on this computer. Advanced mode preserves every existing setting.</p>
    </div>
  </div>
  <div class="card">
    <div class="card-body stack">
      <div class="row wrap" style="justify-content:space-between;align-items:center">
        <strong>Configuration</strong>
        <div class="segmented" aria-label="Configuration mode">
          <button class:active={uiMode==='simple'} aria-pressed={uiMode==='simple'} on:click={()=>onmode('simple')}>Simple</button>
          <button class:active={uiMode==='advanced'} aria-pressed={uiMode==='advanced'} on:click={()=>onmode('advanced')}>Advanced</button>
        </div>
      </div>
      {#if uiMode==='simple'}
        <p class="muted">Configured models are selected automatically. Open REC, choose the report profile and destination folders, then start recording.</p>
        <div class="grid two">
          <div class="notice compact">
            <strong>Transcription</strong>
            <span>{recommendedStt?.model??'No transcription engine configured'}</span>
            <span class="help">Parakeet Ultra is preferred when installed and enabled.</span>
          </div>
          <div class="notice compact">
            <strong>Dental report</strong>
            <span>{defaultDental?.providerId&&defaultDental?.model?defaultDental.model:'Local model not configured'}</span>
            <span class="help">The report is a draft and requires clinical review before sharing.</span>
          </div>
        </div>
        {#if !recommendedStt||!defaultDental?.providerId}
          <div class="notice warning compact">
            <span>Setup incomplete. Choose Advanced to install or connect the transcription and writing models.</span>
            <button class="btn primary" on:click={openModelSetup}>Configure local models</button>
          </div>
        {/if}
      {:else}
        <p class="muted">Manage STT engines, LLM providers and prompts per profile. Your existing customizations are preserved.</p>
        <div class="segmented" aria-label="Settings sections">
          <button class:active={selected==='stt'} aria-pressed={selected==='stt'} on:click={()=>selected='stt'}>Transcription (STT)</button>
          <button class:active={selected==='ai'} aria-pressed={selected==='ai'} on:click={()=>selected='ai'}>AI Profiles</button>
        </div>
      {/if}
    </div>
  </div>
  {#if uiMode==='advanced'}
    {#if selected==='stt'}
      <ProvidersView {providers} {refresh} {notify}/>
    {:else}
      <AiProfilesView {llmProviders} {structureProfiles} {refresh} {notify}/>
    {/if}
  {/if}
</div>
