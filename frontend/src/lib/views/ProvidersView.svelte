<script lang="ts">
  import { api } from '$lib/api';
  import type { Provider, ProviderInput } from '$lib/types';
  export let providers: Provider[] = [];
  export let refresh: () => Promise<void> = async () => {};
  export let notify: (type:'success'|'error', message:string) => void = () => {};

  const blank = ():ProviderInput => ({ id:'', name:'', transcriptionUrl:'', model:'', apiKey:'', timeoutSeconds:600, enabled:true });
  let selectedId = '';
  let draft = blank();
  let models:string[] = [];
  let busy = false;

  function select(provider:Provider) {
    selectedId = provider.id;
    draft = { id:provider.id, name:provider.name, transcriptionUrl:provider.transcriptionUrl, model:provider.model, apiKey:'', timeoutSeconds:provider.timeoutSeconds, enabled:provider.enabled };
    models = [];
  }
  function createNew() { selectedId=''; draft=blank(); models=[]; }
  async function save() {
    busy=true;
    try { const saved=await api.saveProvider(draft); await refresh(); select(saved); notify('success','Provider saved.'); }
    catch(e){ notify('error',e instanceof Error?e.message:String(e)); }
    finally{ busy=false; }
  }
  async function discoverModels() {
    if (!selectedId) { notify('error','Save this provider before discovering models.'); return; }
    busy=true;
    try { models=(await api.models(selectedId)).models; if(models.length===0) notify('error','Provider returned no models.'); }
    catch(e){ notify('error',e instanceof Error?e.message:String(e)); }
    finally{ busy=false; }
  }
  async function remove() {
    if (!selectedId || !confirm('Delete this provider? Workflows using it must be changed first.')) return;
    try { await api.deleteProvider(selectedId); createNew(); await refresh(); notify('success','Provider deleted.'); }
    catch(e){ notify('error',e instanceof Error?e.message:String(e)); }
  }
  $: selected = providers.find((provider)=>provider.id===selectedId);
</script>

<div class="page">
  <div class="page-head"><div><h1 class="page-title">Transcription providers</h1><p class="page-kicker">Any OpenAI-compatible audio transcription endpoint can be local or remote. ScribeWatch stores secrets server-side.</p></div><div class="page-actions"><button class="btn primary" on:click={createNew}>+ Add provider</button></div></div>
  <div class="split-editor">
    <div class="list-pane">
      {#if providers.length===0}<div class="empty">No providers configured.</div>{/if}
      {#each providers as provider}
        <button class="list-item" class:active={provider.id===selectedId} on:click={()=>select(provider)}><strong>{provider.name}</strong><span>{provider.model || 'No model'} · {provider.enabled?'enabled':'disabled'}</span></button>
      {/each}
    </div>
    <section class="card">
      <div class="card-header"><strong>{selectedId ? 'Edit provider' : 'New provider'}</strong>{#if selected?.hasApiKey}<span class="secret-badge">Secret stored</span>{/if}</div>
      <div class="card-body stack">
        <div class="grid two">
          <div class="field"><label for="provider-name">Name</label><input id="provider-name" class="input" bind:value={draft.name} placeholder="Local Speaches" /></div>
          <div class="field"><label for="provider-model">Default model</label><input id="provider-model" class="input" bind:value={draft.model} placeholder="whisper-1" /></div>
        </div>
        <div class="field"><label for="provider-url">Transcription URL</label><input id="provider-url" class="input mono" bind:value={draft.transcriptionUrl} placeholder="http://host:8000/v1/audio/transcriptions" /><span class="help">Use the complete OpenAI-compatible transcription endpoint. Nothing is hardcoded to a specific service.</span></div>
        <div class="grid two">
          <div class="field"><label for="provider-key">API key</label><input id="provider-key" class="input" type="password" bind:value={draft.apiKey} placeholder={selected?.hasApiKey?'Leave empty to keep stored key':'Optional'} /><span class="help">The stored value is never returned to this browser.</span></div>
          <div class="field"><label for="provider-timeout">Timeout (seconds)</label><input id="provider-timeout" class="input" type="number" min="1" bind:value={draft.timeoutSeconds} /></div>
        </div>
        <label class="check"><input type="checkbox" bind:checked={draft.enabled} /> Provider enabled</label>
        {#if models.length>0}<div class="model-list"><span class="field-label">Models reported by provider</span><div class="row wrap">{#each models as model}<button class="chip-button" on:click={()=>draft.model=model}>{model}</button>{/each}</div></div>{/if}
        <div class="form-actions">
          {#if selectedId}<button class="btn danger" on:click={remove}>Delete</button>{/if}
          <span class="spacer"></span>
          {#if selectedId}<button class="btn" disabled={busy} on:click={discoverModels}>Discover models</button>{/if}
          <button class="btn primary" disabled={busy || !draft.name || !draft.transcriptionUrl || !draft.model} on:click={save}>{busy?'Working…':'Save provider'}</button>
        </div>
      </div>
    </section>
  </div>
</div>
