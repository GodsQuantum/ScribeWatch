/** Reusable session form schemas, independent of ASR/LLM and document type.
 * Persist professional presets and FIELD DEFINITIONS only, never session values.
 */
export type SessionFieldType='text'|'date'|'number'|'multiline';
export interface SessionFieldDefinition {id:string;label:string;type:SessionFieldType}
export interface PracticePreset {id:string;name:string;address:string;contact:string}
export type SessionFormSchemas=Record<string,SessionFieldDefinition[]>;
const fieldTypes=new Set<SessionFieldType>(['text','date','number','multiline']);
export function sanitizeSessionSchemas(input:unknown):SessionFormSchemas {
  const result:SessionFormSchemas=Object.create(null);
  if(!input||typeof input!=='object'||Array.isArray(input))return result;
  for(const [key,fields] of Object.entries(input).slice(0,64)){
    if(!key||key==='__proto__'||key==='constructor'||key.length>150||!Array.isArray(fields))continue;
    const ids=new Set<string>();
    result[key]=fields.slice(0,24).flatMap(field=>{
      if(!field||typeof field!=='object')return [];
      const id=String(field.id||'').trim().slice(0,80);
      const label=String(field.label||'').trim().slice(0,90);
      const type=String(field.type||'');
      if(!id||!label||ids.has(id)||!fieldTypes.has(type as SessionFieldType))return [];
      ids.add(id);
      return [{id,label,type:type as SessionFieldType}];
    });
  }
  return result;
}
export function sanitizePracticePresets(input:unknown):PracticePreset[]{
  if(!Array.isArray(input))return [];
  const ids=new Set<string>();
  return input.slice(0,60).flatMap(value=>{
    if(!value||typeof value!=='object')return [];
    const id=String(value.id||'').trim().slice(0,80);
    const name=String(value.name||'').trim().slice(0,160);
    if(!id||!name||ids.has(id))return [];
    ids.add(id);
    return [{id,name,address:String(value.address||'').trim().slice(0,240),
      contact:String(value.contact||'').trim().slice(0,200)}];
  });
}
export function sessionFieldPairs(schema:SessionFieldDefinition[],values:Record<string,string>):
Array<{label:string;value:string}> {
  return schema.slice(0,24).flatMap(field=>{
    const value=(values[field.id]||'').trim().slice(0,500);
    return value?[{label:field.label,value}]:[];
  });
}
