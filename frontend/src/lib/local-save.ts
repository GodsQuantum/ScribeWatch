export function canSaveToDirectory(env: any = globalThis) {
  return env?.isSecureContext === true && typeof env?.showDirectoryPicker === 'function';
}

async function downloadBlob(filename: string, blob: Blob, env: any) {
  const url = env.URL.createObjectURL(blob);
  const anchor = env.document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  env.document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  env.URL.revokeObjectURL(url);
  return 'download' as const;
}

export async function saveMarkdownLocally(filename: string, blob: Blob, env: any = globalThis) {
  if (canSaveToDirectory(env)) {
    const directory = await env.showDirectoryPicker({ mode: 'readwrite' });
    const handle = await directory.getFileHandle(filename, { create: true });
    const writable = await handle.createWritable();
    await writable.write(blob);
    await writable.close();
    return 'directory' as const;
  }
  return downloadBlob(filename, blob, env);
}

/** Select a non-existing name without silently replacing an earlier patient
 * report or source note. A NotFoundError is the only proof of vacancy. */
export async function uniqueDirectoryFilename(directory:FileSystemDirectoryHandle,filename:string):Promise<string>{
  const dot=filename.lastIndexOf('.');
  const base=dot>=0?filename.slice(0,dot):filename;
  const suffix=dot>=0?filename.slice(dot):'';
  for(let attempt=0;attempt<999;attempt++){
    const candidate=attempt?base+'_'+(attempt+1)+suffix:filename;
    try{
      await directory.getFileHandle(candidate,{create:false});
    }catch(error){
      if(error instanceof DOMException&&error.name==='NotFoundError')return candidate;
      throw error;
    }
  }
  throw new Error('Too many files with the same name in this directory');
}

export async function ensureDirectoryPermission(handle: any): Promise<boolean> {
  if (!handle) return false;
  if (typeof handle.queryPermission !== 'function') return true;
  const current = await handle.queryPermission({ mode: 'readwrite' });
  if (current === 'granted') return true;
  if (typeof handle.requestPermission !== 'function') return false;
  return (await handle.requestPermission({ mode: 'readwrite' })) === 'granted';
}

export async function saveResponseToDirectory(
  response: Response,
  filename: string,
  directory?: FileSystemDirectoryHandle,
  env: any = globalThis,
): Promise<'directory'|'download'> {
  let target: any = directory;
  if (target && !(await ensureDirectoryPermission(target))) {
    throw new Error('Permission to the selected audio folder is no longer granted.');
  }
  if (!target && canSaveToDirectory(env)) {
    target = await env.showDirectoryPicker({ mode: 'readwrite' });
  }
  if (target) {
    const file = await target.getFileHandle(filename, { create: true });
    const writable = await file.createWritable();
    if (response.body && typeof response.body.pipeTo === 'function') {
      await response.body.pipeTo(writable as WritableStream<Uint8Array>);
    } else {
      await writable.write(await response.blob());
      await writable.close();
    }
    return 'directory';
  }
  return downloadBlob(filename, await response.blob(), env);
}
