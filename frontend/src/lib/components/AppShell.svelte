<script lang="ts">
  import { onMount } from 'svelte';
  import { initLocale, locale, localize, setLocale, type Locale } from '$lib/i18n';
  import type { Job, Space, SpaceKind, SpaceMembership } from '$lib/types';

  type View='home'|'quick'|'live'|'workflows'|'providers'|'ai'|'jobs';
  type Theme='night'|'arctic'|'slate'|'sand'|'violet';
  export let active:View='home';
  export let onnav:(view:View)=>void=()=>{};
  export let spaces:Space[]=[];
  export let jobs:Job[]=[];
  export let assignments:SpaceMembership[]=[];
  export let selectedSpace='all';
  export let onspace:(id:string)=>void=()=>{};
  export let oncreateSpace:(name:string,kind:SpaceKind)=>Promise<void>=async()=>{};
  export let ondeleteSpace:(id:string)=>Promise<void>=async()=>{};
  export let searchQuery='';
  export let onsearch:(query:string)=>void=()=>{};

  const items=[
    {id:'home',label:'Home',glyph:'⌂'},
    {id:'quick',label:'Quick',glyph:'✦'},
    {id:'live',label:'Live',glyph:'●'},
    {id:'workflows',label:'Workflows',glyph:'↻'},
    {id:'providers',label:'STT',glyph:'◎'},
    {id:'ai',label:'AI Profiles',glyph:'◇'},
    {id:'jobs',label:'Jobs',glyph:'≡'}
  ] as const;
  const themes:{id:Theme;name:string}[]=[
    {id:'night',name:'Midnight'}, {id:'arctic',name:'Arctic'},
    {id:'slate',name:'Graphite'}, {id:'sand',name:'Ivory'},
    {id:'violet',name:'Iris'}
  ];
  const allowedThemes=new Set(themes.map(item=>item.id));
  let theme:Theme='night';
  let addingSpace=false;
  let newSpaceName='';
  let newSpaceKind:SpaceKind='general';
  let savingSpace=false;
  let searchInput:HTMLInputElement|undefined;

  $: recent=jobs.slice().sort((a,b)=>b.updatedAtMs-a.updatedAtMs);
  $: membershipIndex=new Map(assignments.map(item=>[item.jobId,item.spaceId]));
  $: selectedJobs=(selectedSpace==='all'?recent:recent.filter(job=>
    selectedSpace==='unassigned'
      ? !membershipIndex.has(job.id)
      : membershipIndex.get(job.id)===selectedSpace
  )).slice(0,6);
  $: counts=(()=>{
    const map=new Map<string,number>();
    map.set('all',jobs.length);
    for(const job of jobs){
      const space=membershipIndex.get(job.id)??'unassigned';
      map.set(space,(map.get(space)??0)+1);
    }
    return map;
  })();
  const count=(id:string)=>counts.get(id)??0;

  function applyTheme(value:Theme){
    theme=value;
    if(typeof document!=='undefined')document.documentElement.dataset.swTheme=value;
    try{localStorage.setItem('scribewatch:theme',value);}catch{/* private mode */}
  }
  onMount(()=>{
    initLocale();
    try{
      const saved=localStorage.getItem('scribewatch:theme');
      if(saved&&allowedThemes.has(saved as Theme))applyTheme(saved as Theme);
    }catch{/* private mode */}
    function shortcut(event:KeyboardEvent){
      if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'){
        event.preventDefault();
        searchInput?.focus();
      }
    }
    window.addEventListener('keydown',shortcut);
    return()=>window.removeEventListener('keydown',shortcut);
  });
  function changeLocale(event:Event){setLocale((event.currentTarget as HTMLSelectElement).value as Locale);}
  async function addSpace(){
    if(savingSpace||!newSpaceName.trim())return;
    savingSpace=true;
    try{
      await oncreateSpace(newSpaceName,newSpaceKind);
      newSpaceName='';
      newSpaceKind='general';
      addingSpace=false;
    }catch{/* the parent displays the API error */}
    finally{savingSpace=false;}
  }
  function chooseSpace(id:string){onspace(id);onnav('jobs');}
  async function removeSelectedSpace(){
    const target=spaces.find(space=>space.id===selectedSpace);
    if(!target)return;
    if(!confirm('Remove this space? Recordings and generated documents will remain in the Inbox.'))return;
    await ondeleteSpace(target.id);
  }
</script>

<div class="localized-app" use:localize>
  <div class="app-shell">
    <aside class="app-rail">
      <button class="brand-lockup brand-button" on:click={()=>onnav('home')} aria-label="ScribeWatch home">
        <img class="brand-mark-img" src="/logo.svg" alt="" />
        <span><strong class="brand-title">ScribeWatch</strong><span class="brand-sub">audio → knowledge</span></span>
      </button>
      <nav class="nav-list" aria-label="Main navigation">
        {#each items as item}
          <button class="nav-button" class:active={active===item.id}
            aria-current={active===item.id?'page':undefined} on:click={()=>onnav(item.id)}>
            <span class="nav-icon" aria-hidden="true">{item.glyph}</span><span class="nav-label">{item.label}</span>
          </button>
        {/each}
      </nav>

      <section class="spaces-rail" aria-label="Recording spaces">
        <div class="spaces-heading">
          <strong>Spaces</strong>
          <button class="spaces-add" title="Create a space" aria-label="Create a space"
            aria-expanded={addingSpace} on:click={()=>addingSpace=!addingSpace}>+</button>
        </div>
        {#if addingSpace}
          <form class="space-create" on:submit|preventDefault={addSpace}>
            <input class="input" maxlength="100" required bind:value={newSpaceName}
              aria-label="Space name" placeholder="Space name"/>
            <select class="select" bind:value={newSpaceKind} aria-label="Space type">
              <option value="general">General</option>
              <option value="project">Project</option>
              <option value="meeting">Meeting</option>
              <option value="case">Case / dossier</option>
            </select>
            <div class="row">
              <button class="btn primary" type="submit" disabled={savingSpace||!newSpaceName.trim()}>Create</button>
              <button class="btn ghost" type="button" on:click={()=>addingSpace=false}>Cancel</button>
            </div>
          </form>
        {/if}
        <div class="space-items">
          <button class:active={selectedSpace==='all'} on:click={()=>chooseSpace('all')}>
            <span>All recordings</span><span class="space-count">{count('all')}</span>
          </button>
          <button class:active={selectedSpace==='unassigned'} on:click={()=>chooseSpace('unassigned')}>
            <span>Inbox</span><span class="space-count">{count('unassigned')}</span>
          </button>
          {#each spaces as space (space.id)}
            <button class:active={selectedSpace===space.id} title={space.name}
              on:click={()=>chooseSpace(space.id)}>
              <span class="space-item-name">{space.name}</span><span class="space-count">{count(space.id)}</span>
            </button>
          {/each}
        </div>
        {#if selectedSpace!=='all'}
          {#if spaces.some(space=>space.id===selectedSpace)}
            <button class="spaces-remove" on:click={removeSelectedSpace}>Remove space (keep recordings)</button>
          {/if}
          {#if selectedJobs.length}
            <p class="space-list-label">Recent recordings</p>
            <div class="space-recordings">
              {#each selectedJobs as job (job.id)}
                <button title={job.originalName} on:click={()=>onnav('jobs')}>
                  <span class="space-recording-dot" aria-hidden="true"></span>
                  <span>{job.originalName}</span>
                </button>
              {/each}
            </div>
          {/if}
        {/if}
      </section>

      <div class="rail-spacer"></div>
      <div class="rail-preferences">
        <label class="locale-control" aria-label="Color theme">
          <span>Theme</span>
          <select class="select compact-select" bind:value={theme} on:change={(event)=>applyTheme(event.currentTarget.value as Theme)}>
            {#each themes as option}<option value={option.id}>{option.name}</option>{/each}
          </select>
        </label>
        <label class="locale-control" aria-label="Interface language">
          <span>Language</span>
          <select class="select compact-select" value={$locale} on:change={changeLocale}>
            <option value="en">English</option><option value="fr">Français</option><option value="zh-CN">中文</option>
          </select>
        </label>
      </div>
    </aside>
    <div class="app-content">
      <header class="app-toolbar">
        <div class="toolbar-context"><span aria-hidden="true">●</span> ScribeWatch</div>
        <label class="toolbar-search">
          <span aria-hidden="true">⌕</span>
          <input bind:this={searchInput} value={searchQuery}
            on:input={(event)=>{onsearch(event.currentTarget.value);onnav('jobs');}}
            placeholder="Search recordings" aria-label="Search recordings"/>
          <kbd>Ctrl K</kbd>
        </label>
        <span class="toolbar-info">Local-first · Private</span>
      </header>
      <main class="app-main"><slot /></main>
    </div>
  </div>
  <nav class="bottom-nav" aria-label="Mobile navigation">
    {#each items as item}
      <button class:active={active===item.id} on:click={()=>onnav(item.id)}>
        <span class="nav-icon">{item.glyph}</span><span>{item.label}</span>
      </button>
    {/each}
    <select class="bottom-theme" aria-label="Color theme" bind:value={theme}
      on:change={(event)=>applyTheme(event.currentTarget.value as Theme)}>
      {#each themes as option}<option value={option.id}>{option.name}</option>{/each}
    </select>
    <select class="bottom-locale" aria-label="Interface language" value={$locale} on:change={changeLocale}>
      <option value="en">EN</option><option value="fr">FR</option><option value="zh-CN">中文</option>
    </select>
  </nav>
</div>
