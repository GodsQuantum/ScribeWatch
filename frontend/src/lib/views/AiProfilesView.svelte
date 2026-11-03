<script lang="ts">
  import { api } from '$lib/api';
  import { onMount } from 'svelte';
  import type { LlmProvider, LlmProviderInput, StructureProfile } from '$lib/types';

  export let llmProviders:LlmProvider[]=[];
  export let structureProfiles:StructureProfile[]=[];
  export let refresh:()=>Promise<void>=async()=>{};
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};

  const blankProvider=():LlmProviderInput=>({
    id:'',name:'',chatCompletionsUrl:'',model:'',apiKey:'',timeoutSeconds:180,enabled:true
  });
  const blankProfile=():StructureProfile=>({
    id:'',name:'',description:'',prompt:'',outputTemplate:'',outputExample:'',providerId:'',model:'',builtIn:false
  });

  let providerId='';
  let providerDraft=blankProvider();
  let providerBusy=false;
  let providerModels:string[]=[];
  let desktopAvailable=false;
  let detectedEngines:Array<{name:string;chatCompletionsUrl:string;models:string[]}>=[];
  let discoveryBusy=false;
  let installBusy=false;
  let installStatus='';
  let installModel='lfm3-1.7b';
  let downloadStatus='';
  let isWindows=false;

  onMount(()=>{
    isWindows=navigator.userAgent.toLowerCase().includes('windows');
    void scanLocalEngines();
  });
  async function scanLocalEngines(){
    discoveryBusy=true;
    try {
      const result=await api.llmLocalEngines();
      desktopAvailable=result.desktop;
      detectedEngines=result.engines;
    } catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{discoveryBusy=false;}
  }
  function useEngine(engine:{name:string;chatCompletionsUrl:string;models:string[]}){
    newProvider();
    providerDraft.name=engine.name;
    providerDraft.chatCompletionsUrl=engine.chatCompletionsUrl;
    providerDraft.model=engine.models[0]??'';
    providerModels=engine.models;
  }
  async function discoverDraft(){
    if(!providerDraft.chatCompletionsUrl.trim())return;
    discoveryBusy=true;
    try{
      providerModels=(await api.llmDiscoverDraft(providerDraft.chatCompletionsUrl,providerDraft.apiKey??'')).models;
      if(providerModels.length&&!providerModels.includes(providerDraft.model))providerDraft.model=providerModels[0];
      if(!providerModels.length)notify('error','No installed chat models returned by this provider.');
    }catch(e){notify('error',e instanceof Error?e.message:String(e));}
    finally{discoveryBusy=false;}
  }
  async function installEngine(){
    if(!confirm('Install AMD Lemonade Server on this Windows computer via WinGet, then download the recommended Liquid LFM3-1.7B Q4_K_M model (~1.1 GB)? Normal Windows installer prompts still apply.'))return;
    installBusy=true;
    installStatus='Installation via WinGet…';
    try{
      const done=await api.llmInstallEngine();
      installStatus=done.status==='installed'?'Lemonade installed. Checking model installation…':done.status;
      await scanLocalEngines();
      if(detectedEngines.some(engine=>engine.name==='Lemonade Server')){
        const job=await api.llmInstallModel('lfm3-1.7b');
        installStatus='Lemonade installed; Liquid LFM3-1.7B download '+job.status+'. Check progress to confirm completion.';
      }else{
        installStatus='Lemonade installed. Start Lemonade, scan engines, then click Install selected model.';
      }
    }catch(e){installStatus=e instanceof Error?e.message:String(e);}
    finally{installBusy=false;}
  }
  async function installRecommendedModel(){
    const description=installModel==='lfm3-1.7b'?'Liquid LFM3-1.7B Q4_K_M (~1.1 GB)':'Qwen3.5-4B Q4_K_M (~3 GB)';
    if(!confirm('Download '+description+' into the local Lemonade cache?'))return;
    installBusy=true;installStatus='Starting model download…';
    try{
      const job=await api.llmInstallModel(installModel);
      installStatus=job.model+': download '+job.status+'. Check progress, then rescan engines.';
    }catch(e){installStatus=e instanceof Error?e.message:String(e);}
    finally{installBusy=false;}
  }
  async function checkDownloads(){
    try{
      const jobs=await api.llmDownloads();
      downloadStatus=jobs.length?jobs.map(job=>(job.model_name??'Model')+': '+(job.status??'pending')+' ('+(job.percent??0)+'%)').join(' • '):'No ongoing downloads.';
      await scanLocalEngines();
    }catch(e){downloadStatus=e instanceof Error?e.message:String(e);}
  }

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
      try{providerModels=(await api.llmModels(current.id)).models;}catch{/* Nonstandard model endpoints remain manually configurable. */}
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

  <section class="card">
    <div class="card-header">
      <strong>Local AI engine discovery</strong>
      <button class="btn" disabled={discoveryBusy} on:click={scanLocalEngines}>Scan this computer</button>
    </div>
    <div class="card-body stack">
      {#if desktopAvailable}
        <p class="muted small">Native desktop: check Lemonade (13305), LM Studio (1234), Ollama (11434) and llama.cpp (8080). No download happens during detection.</p>
        {#if detectedEngines.length}
          <div class="tag-list">
            {#each detectedEngines as engine}
              <button class="chip-button" on:click={()=>useEngine(engine)}>
                Use {engine.name} ({engine.models.length} models)
              </button>
            {/each}
          </div>
        {:else}
          <span class="muted small">No local engine detected. Start a current engine or install one below.</span>
        {/if}
        {#if isWindows}
          <div class="stack">
            <strong>Optional Windows engine installation</strong>
            <p class="muted small">WinGet installs the official AMD Lemonade Server package only after your confirmation. Windows may display normal installer prompts.</p>
            <div class="row wrap">
              <button class="btn" disabled={installBusy} on:click={installEngine}>Install Lemonade + LFM3 (WinGet)</button>
              <a href="https://github.com/lemonade-sdk/lemonade/releases/latest/download/lemonade.msi" target="_blank" rel="noopener noreferrer">Official MSI</a>
            </div>
          </div>
        {/if}
        {#if detectedEngines.some(engine=>engine.name==='Lemonade Server')}
          <div class="stack">
            <strong>Download a recommended LLM</strong>
            <div class="row wrap">
              <select class="select" bind:value={installModel} aria-label="Recommended LLM">
                <option value="lfm3-1.7b">Liquid LFM3-1.7B Q4_K_M — small</option>
                <option value="qwen3.5-4b">Qwen3.5-4B Q4_K_M — stronger</option>
              </select>
              <button class="btn primary" disabled={installBusy} on:click={installRecommendedModel}>Install selected model</button>
              <button class="btn" disabled={installBusy} on:click={checkDownloads}>Check progress</button>
            </div>
            {#if downloadStatus}<p class="muted small">{downloadStatus}</p>{/if}
          </div>
        {/if}
        {#if installStatus}<p class="muted small">{installStatus}</p>{/if}
      {:else}
        <p class="muted small">Automatic host discovery and installation are for the native desktop app only. In server deployments, enter your existing inference endpoint below.</p>
      {/if}
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
            {#if desktopAvailable}<button class="btn" disabled={discoveryBusy||!providerDraft.chatCompletionsUrl} on:click={discoverDraft}>Find models from URL</button>{/if}
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
          <div class="field"><label for="profile-template">Modèle de sortie (facultatif)</label>
            <textarea id="profile-template" class="input profile-prompt" rows="7" bind:value={profileDraft.outputTemplate} maxlength="12000" placeholder={'## Bilan de consultation\n## Propos rapportés par le patient\n## Constatations indiquées par le praticien\n## Explications et options discutées\n## Suites convenues\n## Points à vérifier'}></textarea>
            <span class="help">Définit la structure attendue. Les rubriques sans preuve dans la transcription ne sont pas remplies artificiellement.</span>
          </div>
          <div class="field"><label for="profile-example">Exemple de rendu (facultatif)</label>
            <textarea id="profile-example" class="input profile-prompt" rows="7" bind:value={profileDraft.outputExample} maxlength="12000" placeholder="Collez un exemple anonymisé ou entièrement fictif ; utilisé pour le style uniquement."></textarea>
            <span class="help">Référence de ton et de présentation, jamais source de faits. N'insérez aucune donnée de patient réel dans l'exemple.</span>
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
