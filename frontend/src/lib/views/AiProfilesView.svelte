<script lang="ts">
  import { api } from '$lib/api';
  import type { LlmProvider, LlmProviderInput, StructureProfile } from '$lib/types';

  export let llmProviders:LlmProvider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  const blankProvider=():LlmProviderInput=>({
    id:'',name:'',chatCompletionsUrl:'',model:'',apiKey:'',timeoutSeconds:180,enabled:true
  });
  const blankProfile=():StructureProfile=>({
    id:'',name:'',description:'',prompt:'',providerId:'',model:'',builtIn:false
  });

  let providerId='';
  let providerDraft=blankProvider();
  let providerBusy=false;
  let providerModels:string[]=[];
  let profileId='';
  let profileDraft=blankProfile();
  let profileBusy=false;
  let profileModels:string[]=[];

  function newProvider(){providerId='';providerDraft=blankProvider();providerModels=[];}
  function editProvider(provider:LlmProvider){
    providerId=provider.id;
    providerDraft={...provider,apiKey:''};
    providerModels=[];
  }
  async function saveProvider(){
    providerBusy=true;
    try{
      const saved=await api.saveLlmProvider(providerDraft);
      await refresh();
      const current=llmProviders.find((provider)=>provider.id===saved.id)??saved;
      editProvider(current);
      notify('success','LLM provider saved.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{providerBusy=false;}
  }
  async function removeProvider(){
    if(!providerId||!confirm('Delete this LLM provider?'))return;
    try{await api.deleteLlmProvider(providerId);newProvider();await refresh();notify('success','LLM provider deleted.');}
    catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
  async function discoverProviderModels(){
    if(!providerId)return;
    providerBusy=true;
    try{providerModels=(await api.llmModels(providerId)).models;}
    catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{providerBusy=false;}
  }

  function newProfile(){profileId='';profileDraft=blankProfile();profileModels=[];}
  function editProfile(profile:StructureProfile){
    profileId=profile.id;
    profileDraft=structuredClone(profile);
    profileModels=[];
  }
  async function chooseProfileProvider(id:string){
    profileDraft={...profileDraft,providerId:id,model:llmProviders.find((provider)=>provider.id===id)?.model??''};
    profileModels=[];
    if(id){
      try{profileModels=(await api.llmModels(id)).models;}catch{/* manual model remains valid */}
    }
  }
  async function saveProfile(){
    profileBusy=true;
    try{
      const {builtIn,...payload}=profileDraft;
      const saved=await api.saveStructureProfile(payload);
      await refresh();
      editProfile(saved);
      notify('success','Structure profile saved.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{profileBusy=false;}
  }
  async function removeProfile(){
    if(!profileId||profileDraft.builtIn||!confirm('Delete this structure profile?'))return;
    try{await api.deleteStructureProfile(profileId);newProfile();await refresh();notify('success','Profile deleted.');}
    catch(e){notify('error',e instanceof Error?e.message:String(e));}
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <p class="page-kicker">OPTIONAL AI POST-PROCESSING</p>
      <h1 class="page-title">AI structure profiles</h1>
      <p class="page-copy">Keep transcription deterministic, then optionally create a second structured view with an OpenAI-compatible LLM. The canonical transcript is always preserved.</p>
    </div>
  </div>

  <section class="card ai-principle">
    <div class="card-body row wrap">
      <span class="status-pill done">TRANSCRIPT FIRST</span>
      <span>Audio → canonical transcript → optional LLM structure → Markdown with both views.</span>
      <span class="muted small">If the LLM fails, ScribeWatch still publishes the transcript.</span>
    </div>
  </section>

  <div class="ai-grid">
    <section class="card">
      <div class="card-header"><strong>LLM providers</strong><button class="btn" on:click={newProvider}>+ Provider</button></div>
      <div class="card-body ai-editor-grid">
        <div class="list-pane">
          {#if llmProviders.length===0}<div class="empty compact">No LLM provider configured.</div>{/if}
          {#each llmProviders as provider}
            <button class="list-item" class:active={provider.id===providerId} on:click={()=>editProvider(provider)}>
              <strong>{provider.name}</strong><span>{provider.model} · {provider.enabled?'enabled':'disabled'}</span>
            </button>
          {/each}
        </div>
        <div class="stack">
          <div class="field"><label for="llm-provider-name">Name</label><input id="llm-provider-name" class="input" bind:value={providerDraft.name} placeholder="Local LLM / OpenAI-compatible / Ollama…" /></div>
          <div class="field"><label for="llm-provider-url">Chat completions URL</label><input id="llm-provider-url" class="input mono" bind:value={providerDraft.chatCompletionsUrl} placeholder="http://host:port/v1/chat/completions" /></div>
          <div class="field"><label for="llm-provider-model">Default model</label><input id="llm-provider-model" class="input mono" bind:value={providerDraft.model} placeholder="provider/model" /></div>
          <div class="field"><label for="llm-provider-key">API key</label><input id="llm-provider-key" class="input" type="password" bind:value={providerDraft.apiKey} placeholder={providerId?'Leave empty to keep stored key':'Optional'} /></div>
          <div class="grid two">
            <div class="field"><label for="llm-provider-timeout">Timeout (seconds)</label><input id="llm-provider-timeout" class="input" type="number" min="1" bind:value={providerDraft.timeoutSeconds} /></div>
            <label class="check"><input type="checkbox" bind:checked={providerDraft.enabled}/> Provider enabled</label>
          </div>
          {#if providerModels.length}<div class="tag-list">{#each providerModels as model}<button class="chip-button" on:click={()=>providerDraft.model=model}>{model}</button>{/each}</div>{/if}
          <div class="form-actions">
            {#if providerId}<button class="btn danger" on:click={removeProvider}>Delete</button><button class="btn" disabled={providerBusy} on:click={discoverProviderModels}>Discover models</button>{/if}
            <span class="spacer"></span><button class="btn primary" disabled={providerBusy||!providerDraft.name||!providerDraft.chatCompletionsUrl||!providerDraft.model} on:click={saveProvider}>Save provider</button>
          </div>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-header"><strong>Structure profiles</strong><button class="btn" on:click={newProfile}>+ Custom profile</button></div>
      <div class="card-body ai-editor-grid">
        <div class="list-pane">
          {#each structureProfiles as profile}
            <button class="list-item" class:active={profile.id===profileId} on:click={()=>editProfile(profile)}>
              <strong>{profile.name}</strong><span>{profile.builtIn?'built-in · ':''}{profile.providerId?'ready':'needs LLM provider'}</span>
            </button>
          {/each}
        </div>
        <div class="stack">
          {#if profileDraft.builtIn}<div class="notice compact"><strong>Built-in template</strong><span class="muted small">Editable for your deployment, but protected from deletion.</span></div>{/if}
          <div class="field"><label for="profile-name">Name</label><input id="profile-name" class="input" bind:value={profileDraft.name} placeholder="e.g. Production meeting" /></div>
          <div class="field"><label for="profile-description">Description</label><input id="profile-description" class="input" bind:value={profileDraft.description} placeholder="When should this profile be used?" /></div>
          <div class="grid two">
            <div class="field"><label for="profile-provider">LLM provider</label>
              <select id="profile-provider" class="select" value={profileDraft.providerId} on:change={(e)=>chooseProfileProvider(e.currentTarget.value)}>
                <option value="">Not connected</option>
                {#each llmProviders.filter((provider)=>provider.enabled) as provider}<option value={provider.id}>{provider.name}</option>{/each}
              </select>
            </div>
            <div class="field"><label for="profile-model">Model</label><input id="profile-model" class="input mono" bind:value={profileDraft.model} list="profile-models" placeholder="Exact model" />
              <datalist id="profile-models">{#each profileModels as model}<option value={model}></option>{/each}</datalist>
            </div>
          </div>
          <div class="field"><label for="profile-prompt">Structure prompt</label><textarea id="profile-prompt" class="input profile-prompt" rows="14" bind:value={profileDraft.prompt} placeholder="Describe the exact structure, constraints and what must never be invented."></textarea>
            <span class="help">ScribeWatch adds its own immutable anti-hallucination / prompt-injection system instruction before this profile.</span>
          </div>
          <div class="form-actions">
            {#if profileId&&!profileDraft.builtIn}<button class="btn danger" on:click={removeProfile}>Delete</button>{/if}
            <span class="spacer"></span><button class="btn primary" disabled={profileBusy||!profileDraft.name||!profileDraft.prompt} on:click={saveProfile}>Save profile</button>
          </div>
        </div>
      </div>
    </section>
  </div>
</div>
