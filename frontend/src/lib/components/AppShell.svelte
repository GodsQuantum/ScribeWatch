<script lang="ts">
  type View = 'home'|'quick'|'workflows'|'providers'|'jobs';
  export let active: View = 'home';
  export let onnav: (view: View) => void = () => {};
  const items = [
    { id: 'home', label: 'Home', glyph: '⌂' },
    { id: 'quick', label: 'Quick Transcribe', glyph: '✦' },
    { id: 'workflows', label: 'Workflows', glyph: '↻' },
    { id: 'providers', label: 'Providers', glyph: '◎' },
    { id: 'jobs', label: 'Jobs', glyph: '≡' }
  ] as const;
</script>

<div class="app-shell">
  <aside class="app-rail">
    <button class="brand-lockup brand-button" on:click={()=>onnav('home')} aria-label="ScribeWatch home">
      <img class="brand-mark-img" src="/logo.svg" alt="" />
      <div><div class="brand-title">ScribeWatch</div><div class="brand-sub">voice → Markdown</div></div>
    </button>
    <nav class="nav-list" aria-label="Main navigation">
      {#each items as item}
        <button class="nav-button" class:active={active===item.id} on:click={()=>onnav(item.id)}>
          <span class="nav-icon">{item.glyph}</span><span class="nav-label">{item.label}</span>
        </button>
      {/each}
    </nav>
    <div class="rail-spacer"></div>
    <div class="rail-note"><strong>Publish first. Archive second.</strong><br/>Workflow audio moves only after its Markdown note is safely written.</div>
  </aside>
  <main class="app-main"><slot /></main>
</div>
<nav class="bottom-nav" aria-label="Mobile navigation">
  {#each items as item}
    <button class:active={active===item.id} on:click={()=>onnav(item.id)}><span class="nav-icon">{item.glyph}</span><span>{item.label}</span></button>
  {/each}
</nav>
