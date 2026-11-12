/** llama.cpp may list a model while its weights are still downloading. */
export function defaultQwen4bModelReady(model:{
  id?:string;model?:string;status?:{value?:string}|string
}):boolean{
  const id=(model.id??model.model??'').toLowerCase();
  const exact=/^(?:unsloth\/qwen3\.5-4b-gguf:q4_k_m|qwen3\.5-4b-q4_k_m(?:\.gguf)?)$/.test(id);
  const state=typeof model.status==='string'?model.status:model.status?.value;
  return exact&&(state==='loaded'||state==='unloaded'||state==='sleeping');
}
