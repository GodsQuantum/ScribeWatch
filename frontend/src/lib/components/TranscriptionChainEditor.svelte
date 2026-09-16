<script lang="ts">
  import { api } from '$lib/api';
  import { addRoute, moveRoute, removeRoute, updateRoute } from '$lib/transcription-chain.js';
  import type { Provider, TranscriptionRoute } from '$lib/types';

  export let value: TranscriptionRoute[] = [];
  export let providers: Provider[] = [];
  export let onchange: (routes: TranscriptionRoute[]) => void = () => {};
  export let notify: (type:'success'|'error', message:string) => void = () => {};

  let modelsByProvider: Record<string,string[]> = {};
  let loadingByProvider: Record<string,boolean> = {};
  let errorsByProvider: Record<string,string> = {};

  async function discover(providerId:string, force=false) {
    if (!providerId) return;
    if (!force && (modelsByProvider[providerId] || loadingByProvider[providerId] || errorsByProvider[providerId])) return;
    loadingByProvider={...loadingByProvider,[providerId]:true};
    if (force) {
      const next={...errorsByProvider}; delete next[providerId]; errorsByProvider=next;
    }
    try {
      const result=await api.models(providerId);
      modelsByProvider={...modelsByProvider,[providerId]:result.models};
      const next={...errorsByProvider}; delete next[providerId]; errorsByProvider=next;
      if (result.models.length===0) notify('error','Provider returned no models.');
    } catch(e) {
      const message=e instanceof Error?e.message:String(e);
      errorsByProvider={...errorsByProvider,[providerId]:message};
    } finally {
      loadingByProvider={...loadingByProvider,[providerId]:false};
    }
  }

  function visibleProviders(route:TranscriptionRoute) {
    return providers.filter((provider)=>provider.enabled || provider.id===route.providerId);
  }
  function modelOptions(route:TranscriptionRoute) {
    const reported=modelsByProvider[route.providerId]??[];
    return [...new Set([...(route.model?[route.model]:[]),...reported])];
  }
  function update(index:number, patch:Partial<TranscriptionRoute>) {
    onchange(updateRoute(value,index,patch));
  }
  function providerChanged(index:number, event:Event) {
    const providerId=(event.currentTarget as HTMLSelectElement).value;
    update(index,{providerId,model:''});
    void discover(providerId,true);
  }
  function modelChanged(index:number, event:Event) {
    update(index,{model:(event.currentTarget as HTMLSelectElement).value});
  }
  function timeoutChoice(route:TranscriptionRoute) {
    const seconds=route.fallbackAfterSeconds;
    if (!seconds) return 'never';
    if ([600,1800,3600].includes(seconds)) return String(seconds);
    return 'custom';
  }
  function timeoutChanged(index:number,event:Event) {
    const choice=(event.currentTarget as HTMLSelectElement).value;
    if (choice==='never') update(index,{fallbackAfterSeconds:undefined});
    else if (choice==='custom') update(index,{fallbackAfterSeconds:value[index]?.fallbackAfterSeconds||60});
    else update(index,{fallbackAfterSeconds:Number(choice)});
  }
  function customTimeoutChanged(index:number,event:Event) {
    const minutes=Math.max(1,Number((event.currentTarget as HTMLInputElement).value)||1);
    update(index,{fallbackAfterSeconds:Math.round(minutes*60)});
  }
  function add() { onchange(addRoute(value.length?value:[{providerId:'',model:''}])); }
  function remove(index:number) { onchange(removeRoute(value,index)); }
  function move(index:number,delta:number) { onchange(moveRoute(value,index,index+delta)); }

  $: for (const route of value) {
    if (route.providerId) void discover(route.providerId);
  }
</script>

<div class="chain-editor">
  <div class="chain-head">
    <div><strong>Transcription chain</strong><p class="help">Choose the exact provider + model order. If one route fails, ScribeWatch tries the next.</p></div>
  </div>
  {#each value as route,index (index)}
    <div class="chain-route">
      <div class="chain-route-label"><strong>{index===0?'Primary':`Fallback ${index}`}</strong><span>#{index+1}</span></div>
      <div class="field">
        <label for={`route-provider-${index}`}>Provider</label>
        <select id={`route-provider-${index}`} class="select" value={route.providerId} on:change={(event)=>providerChanged(index,event)}>
          <option value="">Choose provider</option>
          {#each visibleProviders(route) as provider}
            <option value={provider.id}>{provider.name}{provider.enabled?'':' (disabled)'}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for={`route-model-${index}`}>Model</label>
        <select id={`route-model-${index}`} class="select" value={route.model} disabled={!route.providerId||loadingByProvider[route.providerId]} on:change={(event)=>modelChanged(index,event)}>
          <option value="">{loadingByProvider[route.providerId]?'Loading models…':'Choose model'}</option>
          {#each modelOptions(route) as model}
            <option value={model}>{model}{errorsByProvider[route.providerId]&&model===route.model?' (saved — discovery unavailable)':''}</option>
          {/each}
        </select>
        {#if errorsByProvider[route.providerId]}
          <span class="help error-inline">Model discovery unavailable. <button class="link-button" on:click={()=>discover(route.providerId,true)}>Retry</button></span>
        {/if}
      </div>
      <div class="field chain-timeout">
        <label for={`route-timeout-${index}`}>Fallback after</label>
        <select id={`route-timeout-${index}`} class="select" value={timeoutChoice(route)} on:change={(event)=>timeoutChanged(index,event)}>
          <option value="never">Never</option>
          <option value="600">10 min</option>
          <option value="1800">30 min</option>
          <option value="3600">60 min</option>
          <option value="custom">Custom</option>
        </select>
        {#if timeoutChoice(route)==='custom'}
          <div class="timeout-custom"><input class="input" type="number" min="1" value={Math.max(1,Math.round((route.fallbackAfterSeconds??60)/60))} on:input={(event)=>customTimeoutChanged(index,event)} /><span>min</span></div>
        {/if}
      </div>
      <div class="chain-actions" aria-label={`Actions for route ${index+1}`}>
        <button class="btn icon-btn" disabled={index===0} title="Move up" on:click={()=>move(index,-1)}>↑</button>
        <button class="btn icon-btn" disabled={index===value.length-1} title="Move down" on:click={()=>move(index,1)}>↓</button>
        <button class="btn icon-btn danger" disabled={value.length===1} title="Remove route" on:click={()=>remove(index)}>×</button>
      </div>
    </div>
  {/each}
  <button class="btn chain-add" on:click={add}>+ Add fallback</button>
</div>
