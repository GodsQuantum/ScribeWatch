/** Edit only the report, keeping the canonical transcript byte-identical. */
export interface EditableReport {
  prefix:string;
  body:string;
  suffix:string;
  section:boolean;
}
export function splitEditable(markdown:string):EditableReport{
  const header='## Structured notes\n';
  const start=markdown.indexOf(header);
  const end=start<0?-1:markdown.lastIndexOf('\n## Transcript\n');
  if(start<0||end<start+header.length){
    return {prefix:'',body:markdown,suffix:'',section:false};
  }
  return {
    prefix:markdown.slice(0,start+header.length),
    body:markdown.slice(start+header.length,end).trim(),
    suffix:markdown.slice(end),section:true
  };
}
export function composeEditable(markdown:string,editedBody:string):string{
  const parts=splitEditable(markdown);
  return parts.section
    ?parts.prefix+'\n'+editedBody.trim()+'\n'+parts.suffix
    :editedBody;
}
