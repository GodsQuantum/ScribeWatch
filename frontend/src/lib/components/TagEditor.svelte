<script lang="ts">
  export let value: string[] = [];
  export let onchange: (tags:string[]) => void = () => {};
  let draft = '';

  function normalized(raw:string) {
    return raw.trim().replace(/^#+/, '').replace(/\s+/g, '-');
  }
  function add(raw = draft) {
    const next = raw.split(',').map(normalized).filter(Boolean);
    const seen = new Set(value.map((tag)=>tag.toLowerCase()));
    const merged = [...value];
    for (const tag of next) if (!seen.has(tag.toLowerCase())) { seen.add(tag.toLowerCase()); merged.push(tag); }
    draft = '';
    onchange(merged);
  }
  function remove(index:number) { onchange(value.filter((_, i)=>i!==index)); }
  function keydown(event:KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ',') { event.preventDefault(); add(); }
  }
</script>

<div class="tag-editor">
  <div class="tag-list">
    {#each value as tag, index}
      <span class="tag-chip">#{tag}<button type="button" on:click={()=>remove(index)} aria-label={`Remove ${tag}`}>×</button></span>
    {/each}
  </div>
  <div class="row">
    <input class="input" bind:value={draft} on:keydown={keydown} on:blur={()=>draft.trim()&&add()} placeholder="voice-note, ideas, inbox" aria-label="Workflow tags" />
    <button type="button" class="btn" disabled={!draft.trim()} on:click={()=>add()}>Add</button>
  </div>
  <span class="help">Optional Obsidian tags. Comma or Enter adds a tag.</span>
</div>
