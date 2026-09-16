export function canSaveToDirectory(env: any = globalThis) {
  return env?.isSecureContext === true && typeof env?.showDirectoryPicker === 'function';
}

export async function saveMarkdownLocally(filename: string, blob: Blob, env: any = globalThis) {
  if (canSaveToDirectory(env)) {
    const directory = await env.showDirectoryPicker({ mode: 'readwrite' });
    const handle = await directory.getFileHandle(filename, { create: true });
    const writable = await handle.createWritable();
    await writable.write(blob);
    await writable.close();
    return 'directory';
  }

  const url = env.URL.createObjectURL(blob);
  const anchor = env.document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  env.document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  env.URL.revokeObjectURL(url);
  return 'download';
}
