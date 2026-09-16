<script lang="ts">
  import { api } from '$lib/api';
  import type { BrowseEntry } from '$lib/types';
  export let open = false;
  export let initialPath = '';
  export let title = 'Choose folder';
  export let mode: 'file'|'directory' = 'directory';
  export let extensions = '';
  export let onselect: (path:string) => void = () => {};
  export let onclose: () => void = () => {};

  let currentPath = '';
  let parentPath: string | undefined;
  let roots: string[] = [];
  let entries: BrowseEntry[] = [];
  let loading = false;
  let error = '';
  let filter = '';
  let lastOpen = false;

  async function load(path = '') {
    loading = true; error = '';
    try {
      const data = await api.browse(path, mode, extensions);
      currentPath = data.currentPath;
      parentPath = data.parentPath;
      roots = data.roots;
      entries = data.entries;
    } catch (e) { error = e instanceof Error ? e.message : String(e); }
    finally { loading = false; }
  }
  function choose(path: string) { onselect(path); onclose(); }
  function activate(entry: BrowseEntry) {
    if (entry.isDir) load(entry.path);
    else if (entry.selectable) choose(entry.path);
  }
  $: filtered = entries.filter((entry) => entry.name.toLowerCase().includes(filter.toLowerCase()));
  $: if (open && !lastOpen) { filter = ''; load(initialPath && mode==='directory' ? initialPath : ''); }
  $: lastOpen = open;
</script>

{#if open}
  <div class="modal-backdrop" role="presentation" on:click={(e)=>e.currentTarget===e.target && onclose()}>
    <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
      <div class="modal-head"><strong>{title}</strong><button class="btn icon ghost" on:click={onclose} aria-label="Close">×</button></div>
      <div class="picker-path mono">{currentPath || 'Loading…'}</div>
      <div class="picker-list">
        <div class="row picker-tools">
          {#if parentPath}<button class="btn" on:click={()=>load(parentPath)}>← Back</button>{/if}
          <input class="input" bind:value={filter} placeholder={mode==='file'?'Filter audio files':'Filter folders'} aria-label="Filter" />
          <button class="btn icon" on:click={()=>load(currentPath)} aria-label="Refresh">↻</button>
        </div>
        {#if roots.length > 1}<div class="row wrap roots">{#each roots as root}<button class="btn ghost mono small" on:click={()=>load(root)}>{root}</button>{/each}</div>{/if}
        {#if loading}<div class="empty">Loading…</div>
        {:else if error}<div class="empty error-text">{error}</div>
        {:else if filtered.length===0}<div class="empty">Nothing matching here.</div>
        {:else}
          {#each filtered as entry}
            <button class="picker-row" class:selectable={entry.selectable} on:click={()=>activate(entry)}>
              <span>{entry.isDir ? '▰' : '♪'}</span>
              <span><strong>{entry.name}</strong><span class="meta mono">{entry.path}</span></span>
              <span class="meta">{entry.isDir ? 'Open' : entry.selectable ? 'Choose' : ''}</span>
            </button>
          {/each}
        {/if}
      </div>
      <div class="modal-foot">
        <span class="muted small">Only configured allowed roots are visible.</span>
        <div class="row">
          <button class="btn" on:click={onclose}>Cancel</button>
          {#if mode==='directory'}<button class="btn primary" disabled={!currentPath} on:click={()=>choose(currentPath)}>Choose this folder</button>{/if}
        </div>
      </div>
    </div>
  </div>
{/if}
