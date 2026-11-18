/** Prefer native browser encoders over an extra transcription runtime. */
export const recorderFormats = ['audio/webm;codecs=opus','audio/mp4','audio/ogg;codecs=opus'] as const;
export function chooseRecorderMime(supports:(mime:string)=>boolean):string {
  return recorderFormats.find(supports)??'';
}
export function recordedFile(chunks:Blob[], mime:string, date=new Date()):File {
  const blob=new Blob(chunks,{type:mime||chunks[0]?.type||'audio/webm'});
  if(!blob.size)throw new Error('Aucun son enregistré.');
  if(blob.size>12*1024*1024)throw new Error('Dictée trop longue : maximum 12 Mio.');
  const ext=blob.type.includes('mp4')?'m4a':blob.type.includes('ogg')?'ogg':'webm';
  const name='dictation-'+date.toISOString().replace(/[:.]/g,'-')+'.'+ext;
  return new File([blob],name,{type:blob.type});
}
